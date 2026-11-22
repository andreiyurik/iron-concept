//! Runs `conformance/validate.yaml` and the greenhouse example through the
//! validator.
//!
//! Covers: REQ-TAG-002..007, REQ-RWS-002, REQ-DRV-009, REQ-EXP-001..003,
//! REQ-EXP-005, REQ-CFG-005, REQ-ALM-001, REQ-ALM-004.

use iron_core::{validate, Files, Severity};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The vectors are themselves inside the YAML subset, so the crate's own
/// reader loads them.
#[test]
fn validate_vectors() {
    let src = std::fs::read_to_string(repo().join("conformance/validate.yaml")).unwrap();
    let (root, problems) = iron_core::yaml::parse(&src);
    assert_eq!(
        problems,
        vec![],
        "the vector file must be inside the subset"
    );
    let root = root.unwrap();
    let cases = root.get("cases").unwrap().items().unwrap();
    assert!(
        cases.len() >= 16,
        "expected at least 16 cases, found {}",
        cases.len()
    );
    for case in cases {
        let name = case.get("name").unwrap().as_str().unwrap();
        let files: Files = case
            .get("files")
            .unwrap()
            .entries()
            .unwrap()
            .iter()
            .map(|e| (e.key.clone(), e.value.as_str().unwrap().to_string()))
            .collect();
        let report = validate(&files);
        let got: BTreeSet<(String, String)> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| {
                (
                    d.location.path.clone(),
                    d.requirement.unwrap_or("-").to_string(),
                )
            })
            .collect();
        let want: BTreeSet<(String, String)> = case
            .get("expect")
            .unwrap()
            .items()
            .unwrap_or_default()
            .iter()
            .map(|e| {
                assert_eq!(e.get("severity").unwrap().as_str(), Some("error"), "{name}");
                let field = |k: &str| e.get(k).unwrap().as_str().unwrap().to_string();
                (field("path"), field("requirement"))
            })
            .collect();
        assert_eq!(got, want, "case '{name}':\n{:#?}", report.diagnostics);
    }
}

#[test]
fn greenhouse_example_is_valid() {
    let files = iron_core::project::load(&repo().join("examples/greenhouse")).unwrap();
    let report = validate(&files);
    assert!(report.is_valid(), "{:#?}", report.diagnostics);
    assert_eq!((report.tags, report.commands), (15, 1));
    assert!(
        report.diagnostics.is_empty(),
        "no warnings expected: {:#?}",
        report.diagnostics
    );
}

/// REQ-ALM-001: there is no way to make an alarm act.
#[test]
fn an_alarm_cannot_carry_an_action() {
    let mut files = Files::new();
    files.insert(
        "config/tags/a.yaml".into(),
        "a:\n  t:\n    source: \"modbus://h/holding/0\"\n    type: float32\n    unit: C\n    range: [0, 9]\n    alarms:\n      high: { limit: 5, priority: 1, message: m, action: stop }\n".into(),
    );
    let report = validate(&files);
    let codes: Vec<_> = report
        .diagnostics
        .iter()
        .map(|d| (d.code, d.requirement))
        .collect();
    assert_eq!(codes, vec![("IRON-E043", Some("REQ-ALM-001"))]);
}
