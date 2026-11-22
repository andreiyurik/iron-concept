//! `iron test --sim`: deterministic scenarios (`docs/specs/testing.md`, Level 3).

use crate::diag::Report;
use crate::model::Project;
use crate::validate::{check, Files};
use crate::yaml::{self, Node};
use iron_domain::alarm::{AlarmDef, Detector};
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

pub fn test(files: &Files) -> Result<Vec<Outcome>, Report> {
    let checked = check(files);
    if !checked.report.is_valid() {
        return Err(checked.report);
    }
    let scenarios = files.iter().filter(|(file, _)| file.starts_with("test/sim/scenarios/"));
    Ok(scenarios.map(|(file, src)| run(file, src, &checked.project)).collect())
}

struct Engine {
    samples: BTreeMap<String, Sample>,
    alarms: BTreeMap<(String, String), (AlarmDef, Detector)>,
}

#[must_use]
pub fn run(file: &str, src: &str, project: &Project) -> Outcome {
    let mut name = file.rsplit('/').next().unwrap_or(file).trim_end_matches(".yaml").to_string();
    let (root, problems) = yaml::parse(src);
    let mut failures: Vec<String> =
        problems.iter().map(|p| format!("line {}: {}", yaml::line_col(src, p.span.start).0, p.message)).collect();
    let Some(root) = root.filter(|_| failures.is_empty()) else {
        if failures.is_empty() {
            failures.push("the scenario file is empty".into());
        }
        return Outcome { name, file: file.into(), passed: false, failures };
    };
    for e in root.entries().unwrap_or_default() {
        if !["name", "timeline", "assertions"].contains(&e.key.as_str()) {
            failures.push(format!("unknown key '{}' (expected name, timeline, assertions)", e.key));
        }
    }
    if let Some(n) = root.get("name").and_then(Node::as_str) {
        name = n.to_string();
    }

    // (time, sets before assertions, body)
    let mut steps: Vec<(i64, bool, &Node)> = Vec::new();
    for (key, time_key, body_key) in [("timeline", "t", "set"), ("assertions", "at", "expect")] {
        let Some(items) = root.get(key).map(Node::items) else { continue };
        let Some(items) = items else {
            failures.push(format!("`{key}` must be a list"));
            continue;
        };
        for (i, item) in items.iter().enumerate() {
            for e in item.entries().unwrap_or_default() {
                if ![time_key, body_key].contains(&e.key.as_str()) {
                    failures.push(format!("{key}[{i}]: '{}' is not supported yet", e.key));
                }
            }
            match (item.get(time_key).map(Node::as_ms), item.get(body_key)) {
                (Some(Some(t)), Some(body)) => steps.push((t, key == "assertions", body)),
                (Some(None), _) => failures.push(format!("{key}[{i}]: `{time_key}` is not a duration (write 0s, 500ms, 2m)")),
                _ => failures.push(format!("{key}[{i}]: needs `{time_key}` and `{body_key}`")),
            }
        }
    }
    if !steps.iter().any(|(_, assertion, _)| *assertion) {
        failures.push("a scenario with no assertions checks nothing".into());
    }
    steps.sort_by_key(|(t, assertion, _)| (*t, *assertion));

    let alarms = project
        .tags
        .iter()
        .flat_map(|(path, tag)| tag.alarms.iter().map(move |(name, def)| ((path.clone(), name.clone()), (*def, Detector::default()))));
    let mut engine = Engine { samples: BTreeMap::new(), alarms: alarms.collect() };
    for (t, assertion, body) in steps {
        let at = format!("at {}s", t as f64 / 1000.0);
        if assertion {
            engine.expect(t, &at, body, &mut failures);
        } else {
            engine.set(t, &at, body, project, &mut failures);
        }
    }
    Outcome { name, file: file.into(), passed: failures.is_empty(), failures }
}

impl Engine {
    fn set(&mut self, t: i64, at: &str, body: &Node, project: &Project, failures: &mut Vec<String>) {
        for e in body.entries().unwrap_or_default() {
            let value = match project.tags.get(&e.key) {
                None => Err(format!("unknown tag '{}'", e.key)),
                Some(tag) if tag.derived.is_some() => Err(format!("'{}' is derived and cannot be set", e.key)),
                Some(tag) if tag.ty == TagType::Bool => {
                    e.value.as_bool().map(Value::Bool).ok_or(()).map_err(|()| format!("wrong value type for '{}'", e.key))
                }
                Some(_) => e.value.as_f64().map(Value::Number).ok_or(()).map_err(|()| format!("wrong value type for '{}'", e.key)),
            };
            match value {
                Ok(value) => drop(self.samples.insert(e.key.clone(), Sample { value, quality: Quality::Good })),
                Err(message) => failures.push(format!("{at}: {message}")),
            }
        }
        for path in &project.derived_order {
            let Some(expr) = project.tags.get(path).and_then(|t| t.derived.as_ref()) else { continue };
            let result = expr.evaluate(&|name| self.samples.get(name).copied());
            match result.value {
                Some(value) => drop(self.samples.insert(path.clone(), Sample { value, quality: result.quality })),
                None => drop(self.samples.remove(path)),
            }
        }
        // A tag with no sample is unreadable: BAD, which holds alarm state.
        let bad = Sample { value: Value::Number(0.0), quality: Quality::Bad };
        for ((path, _), (def, detector)) in &mut self.alarms {
            let sample = self.samples.get(path).copied().unwrap_or(bad);
            let _ = detector.update(def, t, sample.value, sample.quality);
        }
    }

    fn expect(&mut self, t: i64, at: &str, body: &Node, failures: &mut Vec<String>) {
        for (def, detector) in self.alarms.values_mut() {
            detector.poll(def, t);
        }
        for e in body.entries().unwrap_or_default() {
            let key = e.key.rsplit_once('.').map(|(tag, alarm)| (tag.to_string(), alarm.to_string()));
            let Some((_, detector)) = key.and_then(|k| self.alarms.get(&k)) else {
                failures.push(format!("{at}: '{}' is not an alarm (write <tag path>.<alarm name>)", e.key));
                continue;
            };
            let got = if detector.active { "active" } else { "normal" };
            match e.value.as_str() {
                Some(want @ ("active" | "normal")) if want == got => {}
                Some(want @ ("active" | "normal")) => failures.push(format!("{at}: expected {} {want}, got {got}", e.key)),
                _ => failures.push(format!("{at}: '{}' must be `active` or `normal`", e.key)),
            }
        }
    }
}
