//! The typed project model, built from files that already passed `validate`.

use crate::validate::{parse_duration_ms, Files};
use crate::yaml::{self, Node};
use iron_domain::alarm::{AlarmDef, Condition};
use iron_domain::expr::{self, Expr};
use iron_domain::TagType;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Tag {
    pub ty: TagType,
    pub derived: Option<Expr>,
    pub alarms: Vec<(String, AlarmDef)>,
}

#[derive(Debug, Clone, Default)]
pub struct Project {
    pub tags: BTreeMap<String, Tag>,
    /// Derived tags, each after everything it depends on.
    pub derived_order: Vec<String>,
}

/// Anything `validate` would have rejected is skipped, not reported.
#[must_use]
pub fn build(files: &Files) -> Project {
    let mut tags = BTreeMap::new();
    for (file, src) in files {
        if !file.starts_with("config/tags/") {
            continue;
        }
        let Some(root) = yaml::parse(src).0 else {
            continue;
        };
        for area in root.entries().unwrap_or_default() {
            for t in area.value.entries().unwrap_or_default() {
                if let Some(tag) = tag(&t.value) {
                    tags.insert(format!("{}.{}", area.key, t.key), tag);
                }
            }
        }
    }
    let mut order = Vec::new();
    let mut done = BTreeSet::new();
    for path in tags.keys() {
        visit(path, &tags, &mut done, &mut order);
    }
    Project {
        tags,
        derived_order: order,
    }
}

fn visit(
    path: &str,
    tags: &BTreeMap<String, Tag>,
    done: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    let Some(expr) = tags.get(path).and_then(|t| t.derived.as_ref()) else {
        return;
    };
    if !done.insert(path.to_string()) {
        return;
    }
    for dep in expr.refs() {
        visit(dep, tags, done, order);
    }
    order.push(path.to_string());
}

fn tag(node: &Node) -> Option<Tag> {
    let ty = node.get("type")?.as_str().and_then(TagType::parse)?;
    let derived = node
        .get("derived")
        .and_then(Node::as_str)
        .and_then(|t| expr::parse(t).ok());
    let ms = |n: &Node, key: &str| {
        n.get(key)
            .and_then(Node::as_str)
            .and_then(parse_duration_ms)
            .map_or(0, |v| v as i64)
    };
    let mut alarms = Vec::new();
    for a in node
        .get("alarms")
        .and_then(Node::entries)
        .unwrap_or_default()
    {
        let condition = match a.value.get("state").and_then(Node::as_bool) {
            Some(state) => Some(Condition::State(state)),
            None => a
                .value
                .get("limit")
                .and_then(Node::as_f64)
                .and_then(|l| AlarmDef::level(&a.key, l)),
        };
        if let Some(condition) = condition {
            let deadband = a
                .value
                .get("deadband")
                .and_then(Node::as_f64)
                .unwrap_or(0.0);
            let def = AlarmDef {
                condition,
                deadband,
                on_delay_ms: ms(&a.value, "on_delay"),
                off_delay_ms: ms(&a.value, "off_delay"),
            };
            alarms.push((a.key.clone(), def));
        }
    }
    Some(Tag {
        ty,
        derived,
        alarms,
    })
}
