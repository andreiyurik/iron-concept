//! The scenario runner: derived tags, delays, and honest failures.

use iron_core::{check, sim, Files};

const TAGS: &str = "\
r:
  temp:
    source: \"sim://x\"
    type: float32
    unit: C
    range: [0, 300]
    alarms:
      high: { limit: 180, priority: 2, message: m, deadband: 1.0, on_delay: 3s }
  ambient: { source: \"sim://y\", type: float32, unit: C, range: [-50, 60] }
  rise:
    derived: \"r.temp - r.ambient\"
    type: float32
    unit: K
    range: [-100, 350]
    alarms:
      high: { limit: 150, priority: 3, message: m }
  power: { source: \"sim://z\", type: bool, alarms: { trip: { state: false, priority: 1, message: m } } }
";

fn run(scenario: &str) -> sim::Outcome {
    let checked = check(&Files::from([("config/tags/r.yaml".to_string(), TAGS.to_string())]));
    assert!(checked.report.is_valid(), "{:#?}", checked.report.diagnostics);
    sim::run("test/sim/scenarios/s.yaml", scenario, &checked.project)
}

#[test]
fn delays_hysteresis_and_derived_alarms() {
    let o = run("\
name: ramp
timeline:
  - { t: 0s, set: { r.temp: 185, r.ambient: 20, r.power: true } }
  - { t: 10s, set: { r.temp: 179.5 } }
  - { t: 12s, set: { r.temp: 178, r.power: false } }
assertions:
  - { at: 2s, expect: { r.temp.high: normal, r.rise.high: active } }
  - { at: 3s, expect: { r.temp.high: active } }
  - { at: 11s, expect: { r.temp.high: active, r.power.trip: normal } }
  - { at: 13s, expect: { r.temp.high: normal, r.power.trip: active } }
");
    assert!(o.passed, "{:#?}", o.failures);
    assert_eq!(o.name, "ramp");
}

/// A derived tag with a missing input is BAD, and BAD holds alarm state (REQ-ALM-008).
#[test]
fn a_derived_tag_without_inputs_raises_nothing() {
    let o = run("\
timeline:
  - { t: 0s, set: { r.temp: 250 } }
assertions:
  - { at: 1s, expect: { r.rise.high: normal } }
");
    assert!(o.passed, "{:#?}", o.failures);
}

#[test]
fn failures_say_what_was_expected() {
    let o = run("\
timeline:
  - { t: 0s, set: { r.temp: 100, r.nope: 1, r.rise: 5 } }
  - { t: 1s, fault: nats_disconnect, set: { r.temp: 100 } }
assertions:
  - { at: 2s, expect: { r.temp.high: active, r.temp: active } }
");
    assert!(!o.passed);
    let all = o.failures.join("\n");
    for needle in [
        "unknown tag 'r.nope'",
        "'r.rise' is derived",
        "'fault' is not supported yet",
        "expected r.temp.high active, got normal",
        "'r.temp' is not an alarm",
    ] {
        assert!(all.contains(needle), "missing {needle:?} in:\n{all}");
    }
}

// A scenario that checks nothing must not pass.
#[test]
fn malformed_scenarios_fail() {
    for (scenario, needle) in [
        ("", "empty"),
        ("name: nothing\n", "no assertions"),
        (
            "timeline:\n  - { t: 0s, set: { r.temp: 1 } }\nassertion:\n  - { at: 1s, expect: { r.temp.high: normal } }\n",
            "unknown key 'assertion'",
        ),
        ("timeline: { t: 0s }\nassertions: 5\n", "must be a list"),
        ("timeline:\n  - { t: 0, set: { r.temp: 1 } }\nassertions:\n  - { at: 1s, expect: { r.temp.high: normal } }\n", "not a duration"),
    ] {
        let o = run(scenario);
        assert!(!o.passed && o.failures.join("\n").contains(needle), "{scenario:?}: {:?}", o.failures);
    }
}

// REQ-ALM-007: no assertion falls between the samples, so nothing polls.
#[test]
fn an_elapsed_delay_is_honoured_without_a_poll() {
    let o = run("\
timeline:
  - { t: 0s, set: { r.temp: 185 } }
  - { t: 10s, set: { r.temp: 170 } }
  - { t: 20s, set: { r.temp: 185 } }
assertions:
  - { at: 25s, expect: { r.temp.high: active } }
");
    assert!(o.passed, "{:#?}", o.failures);
}
