//! Runs `conformance/validate.yaml` and the greenhouse example through the validator.

use iron_core::{validate, Files, Severity};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// The vector file embeds project files as block scalars, which config files may not use.
#[test]
fn validate_vectors() {
    let src = std::fs::read_to_string(repo().join("conformance/validate.yaml")).unwrap();
    let (root, problems) = iron_core::yaml::parse(&src);
    assert!(problems.iter().all(|p| !p.syntax), "{problems:?}");
    let root = root.unwrap();
    let cases = root.get("cases").unwrap().items().unwrap();
    assert!(cases.len() >= 16, "expected at least 16 cases, found {}", cases.len());
    for case in cases {
        let name = case.get("name").unwrap().as_str().unwrap();
        let files: Files =
            case.get("files").unwrap().entries().unwrap().iter().map(|e| (e.key.clone(), e.value.as_str().unwrap().to_string())).collect();
        let report = validate(&files);
        let got: BTreeSet<(String, String)> = report
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| (d.location.path.clone(), d.requirement.unwrap_or("-").to_string()))
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
    assert!(report.diagnostics.is_empty(), "no warnings expected: {:#?}", report.diagnostics);
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
    let codes: Vec<_> = report.diagnostics.iter().map(|d| (d.code, d.requirement)).collect();
    assert_eq!(codes, vec![("IRON-E043", Some("REQ-ALM-001"))]);
}

fn tags(body: &str) -> iron_core::Report {
    validate(&Files::from([("config/tags/a.yaml".to_string(), body.to_string())]))
}

// REQ-EXP-003: linear in the number of tags, and the chain is in dependency order.
#[test]
fn cycles_are_found_fast_and_named_in_order() {
    let leaf = |n: &str| format!("  {n}: {{ source: \"sim://{n}\", type: float32, unit: u, range: [0, 9] }}\n");
    let derived = |n: &str, e: &str| format!("  {n}: {{ derived: \"{e}\", type: float32, unit: u, range: [0, 9] }}\n");
    let mut dag = format!("z:\n{}{}", leaf("x0"), leaf("y0"));
    for i in 1..200 {
        dag += &derived(&format!("x{i}"), &format!("z.x{} + z.y{}", i - 1, i - 1));
        dag += &derived(&format!("y{i}"), &format!("z.x{} - z.y{}", i - 1, i - 1));
    }
    assert!(tags(&dag).is_valid());

    let ring = [("a", "z.c + 1"), ("b", "z.a + 1"), ("c", "z.b + 1"), ("s", "z.s + 1")];
    let report = tags(&format!("z:\n{}", ring.map(|(n, e)| derived(n, e)).concat()));
    let messages: Vec<&str> = report.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, ["derived tags depend on each other: z.a → z.c → z.b → z.a", "derived tags depend on each other: z.s → z.s"]);
}

#[test]
fn a_derived_tag_may_be_named_like_a_field() {
    let report = tags("z:\n  suppressed_when: { derived: \"z.suppressed_when + 1\", type: float32, unit: u, range: [0, 9] }\n");
    assert_eq!(report.diagnostics[0].location.path, "z.suppressed_when.derived");
    assert_eq!(report.diagnostics[0].requirement, Some("REQ-EXP-003"));
}

#[test]
fn malformed_values_are_reported_not_defaulted() {
    let alarms = "      high: { limit: 5, priority: 1, message: m, deadband: abc, adjustable: maybe }\n      low: { limit: 2, priority: 2, message: m, deadband: -3 }\n";
    let report =
        tags(&format!("z:\n  t:\n    source: \"sim://x\"\n    type: float32\n    unit: C\n    range: [0, 10]\n    alarms:\n{alarms}"));
    let paths: Vec<&str> = report.diagnostics.iter().map(|d| d.location.path.as_str()).collect();
    assert_eq!(paths, ["z.t.alarms.high.deadband", "z.t.alarms.high.adjustable", "z.t.alarms.low.deadband"]);

    let files = Files::from([
        ("config/tags/a.yaml".to_string(), "z:\n  t: { source: \"sim://x\", type: bool }\n".to_string()),
        ("config/commands/c.yaml".to_string(), "q: 5\n_s:\n  x: { target: \"modbus://h/coil/2\", type: bool }\n".to_string()),
        (
            "config/connections.yaml".to_string(),
            "plc-01:\n  host: 10.0.0.1\n  prot: 502\n  port: abc\n  max_block: 9999\nplc-03: 5\n".to_string(),
        ),
    ]);
    let paths: Vec<String> = validate(&files).diagnostics.iter().map(|d| d.location.path.clone()).collect();
    assert_eq!(paths, ["q", "_s", "plc-01.prot", "plc-01.port", "plc-01.max_block", "plc-03"]);
}
