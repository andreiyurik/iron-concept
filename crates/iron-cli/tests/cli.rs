//! The `iron` binary end to end: output and exit codes (`docs/specs/cli.md`).

use std::path::PathBuf;
use std::process::Command;

fn iron(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_iron"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn greenhouse() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/greenhouse")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn valid_project_exits_zero() {
    let (code, out, _) = iron(&["validate", &greenhouse()]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("15 tags, 1 command — valid"), "{out}");
}

#[test]
fn json_output_is_the_report() {
    let (code, out, _) = iron(&["validate", &greenhouse(), "--json"]);
    assert_eq!(code, 0);
    let doc: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(doc["status"], "ok");
    assert_eq!(doc["tags"], 15);
    assert_eq!(doc["diagnostics"].as_array().unwrap().len(), 0);
}

#[test]
fn invalid_project_exits_one_with_a_located_diagnostic() {
    let dir = std::env::temp_dir().join(format!("iron-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("config/tags")).unwrap();
    std::fs::write(
        dir.join("config/tags/a.yaml"),
        "a:\n  t:\n    source: \"modbus://h/holding/40001\"\n    type: uint16\n    unit: x\n    range: [0, 9]\n",
    )
    .unwrap();
    let (code, out, _) = iron(&["validate", dir.to_str().unwrap()]);
    let (jcode, json, _) = iron(&["validate", dir.to_str().unwrap(), "--json"]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!((code, jcode), (1, 1), "{out}");
    assert!(out.contains("config/tags/a.yaml:3:13  a.t.source"), "{out}");
    assert!(out.contains("IRON-E013 · REQ-DRV-009"), "{out}");
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc["status"], "invalid");
    assert_eq!(doc["diagnostics"][0]["requirement"], "REQ-DRV-009");
    assert_eq!(doc["diagnostics"][0]["location"]["line"], 3);
}

#[test]
fn environment_and_usage_errors_exit_two() {
    assert_eq!(iron(&["validate", "/nonexistent-iron-project"]).0, 2);
    assert_eq!(iron(&["frobnicate"]).0, 2);
    assert_eq!(iron(&[]).0, 2);
    assert_eq!(iron(&["version"]).0, 0);
}

#[test]
fn simulation_scenarios_run_and_report() {
    let (code, out, _) = iron(&["test", "--sim", &greenhouse()]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("✅ frost_night") && out.contains("1 passed"),
        "{out}"
    );
    let (_, json, _) = iron(&["test", "--sim", &greenhouse(), "--json"]);
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc["status"], "ok");
    assert_eq!(doc["scenarios"][0]["name"], "frost_night");
    assert_eq!(iron(&["test", &greenhouse()]).0, 2);
}
