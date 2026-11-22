//! `iron test --sim`: deterministic scenarios (`docs/specs/testing.md`, Level 3).

use crate::model::Project;
use crate::validate::{parse_duration_ms, Files};
use crate::yaml::{self, Node};
use iron_domain::alarm::Detector;
use iron_domain::{Quality, Sample, TagType, Value};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outcome {
    pub name: String,
    pub file: String,
    pub passed: bool,
    pub failures: Vec<String>,
}

#[must_use]
pub fn run_all(files: &Files, project: &Project) -> Vec<Outcome> {
    files
        .iter()
        .filter(|(file, _)| file.starts_with("test/sim/scenarios/"))
        .map(|(file, src)| run(file, src, project))
        .collect()
}

enum Step<'a> {
    Set(&'a Node),
    Expect(&'a Node),
}

struct Engine<'a> {
    project: &'a Project,
    samples: BTreeMap<String, Sample>,
    detectors: BTreeMap<(String, String), Detector>,
}

#[must_use]
pub fn run(file: &str, src: &str, project: &Project) -> Outcome {
    let fallback = file
        .rsplit('/')
        .next()
        .unwrap_or(file)
        .trim_end_matches(".yaml")
        .to_string();
    let mut failures = Vec::new();
    let (root, problems) = yaml::parse(src);
    failures.extend(problems.iter().map(|p| p.message.clone()));
    let Some(root) = root.filter(|_| failures.is_empty()) else {
        return Outcome {
            name: fallback,
            file: file.into(),
            passed: false,
            failures,
        };
    };
    let name = root
        .get("name")
        .and_then(Node::as_str)
        .map_or(fallback, str::to_string);

    // (time, sets before assertions, position in file)
    let mut steps: Vec<(i64, u8, usize, Step)> = Vec::new();
    for (key, time_key, body_key, rank) in [
        ("timeline", "t", "set", 0),
        ("assertions", "at", "expect", 1),
    ] {
        for (i, item) in root
            .get(key)
            .and_then(Node::items)
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let known = [time_key, body_key];
            for e in item.entries().unwrap_or_default() {
                if !known.contains(&e.key.as_str()) {
                    failures.push(format!("{key}[{i}]: '{}' is not supported yet", e.key));
                }
            }
            let t = item
                .get(time_key)
                .and_then(Node::as_str)
                .and_then(parse_duration_ms);
            match (t, item.get(body_key)) {
                (Some(t), Some(body)) => {
                    let step = if rank == 0 {
                        Step::Set(body)
                    } else {
                        Step::Expect(body)
                    };
                    steps.push((t as i64, rank, i, step));
                }
                _ => failures.push(format!("{key}[{i}]: needs `{time_key}` and `{body_key}`")),
            }
        }
    }
    steps.sort_by_key(|(t, rank, i, _)| (*t, *rank, *i));

    let mut engine = Engine {
        project,
        samples: BTreeMap::new(),
        detectors: BTreeMap::new(),
    };
    for (t, _, _, step) in &steps {
        match step {
            Step::Set(body) => engine.set(*t, body, &mut failures),
            Step::Expect(body) => engine.expect(*t, body, &mut failures),
        }
    }
    Outcome {
        name,
        file: file.into(),
        passed: failures.is_empty(),
        failures,
    }
}

impl Engine<'_> {
    fn set(&mut self, t: i64, body: &Node, failures: &mut Vec<String>) {
        let at = format!("at {}s", t as f64 / 1000.0);
        for e in body.entries().unwrap_or_default() {
            let Some(tag) = self.project.tags.get(&e.key) else {
                failures.push(format!("{at}: unknown tag '{}'", e.key));
                continue;
            };
            if tag.derived.is_some() {
                failures.push(format!("{at}: '{}' is derived and cannot be set", e.key));
                continue;
            }
            let value = if tag.ty == TagType::Bool {
                e.value.as_bool().map(Value::Bool)
            } else {
                e.value.as_f64().map(Value::Number)
            };
            match value {
                Some(value) => {
                    self.samples.insert(
                        e.key.clone(),
                        Sample {
                            value,
                            quality: Quality::Good,
                        },
                    );
                }
                None => failures.push(format!("{at}: wrong value type for '{}'", e.key)),
            }
        }
        for path in &self.project.derived_order {
            let Some(expr) = self.project.tags.get(path).and_then(|t| t.derived.as_ref()) else {
                continue;
            };
            let result = expr.evaluate(&|name| self.samples.get(name).copied());
            match result.value {
                Some(value) => {
                    self.samples.insert(
                        path.clone(),
                        Sample {
                            value,
                            quality: result.quality,
                        },
                    );
                }
                None => {
                    self.samples.remove(path);
                }
            }
        }
        for (path, tag) in &self.project.tags {
            let bad = Sample {
                value: Value::Number(0.0),
                quality: Quality::Bad,
            };
            let sample = self.samples.get(path).copied().unwrap_or(bad);
            for (name, def) in &tag.alarms {
                let detector = self
                    .detectors
                    .entry((path.clone(), name.clone()))
                    .or_default();
                detector.update(def, t, sample.value, sample.quality);
            }
        }
    }

    fn expect(&mut self, t: i64, body: &Node, failures: &mut Vec<String>) {
        for (path, tag) in &self.project.tags {
            for (name, def) in &tag.alarms {
                self.detectors
                    .entry((path.clone(), name.clone()))
                    .or_default()
                    .poll(def, t);
            }
        }
        let at = format!("at {}s", t as f64 / 1000.0);
        for e in body.entries().unwrap_or_default() {
            let found = e
                .key
                .rsplit_once('.')
                .and_then(|(tag, alarm)| self.detectors.get(&(tag.to_string(), alarm.to_string())));
            let Some(detector) = found else {
                failures.push(format!(
                    "{at}: '{}' is not an alarm (write <tag path>.<alarm name>)",
                    e.key
                ));
                continue;
            };
            let got = if detector.active { "active" } else { "normal" };
            match e.value.as_str() {
                Some(want @ ("active" | "normal")) if want == got => {}
                Some(want @ ("active" | "normal")) => {
                    failures.push(format!("{at}: expected {} {want}, got {got}", e.key));
                }
                _ => failures.push(format!("{at}: '{}' must be `active` or `normal`", e.key)),
            }
        }
    }
}
