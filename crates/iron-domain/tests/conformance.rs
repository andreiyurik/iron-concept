//! Runs the language-neutral vectors in `conformance/` against `iron-domain`.

use iron_domain::decode::{decode, ByteOrder};
use iron_domain::expr::{expand, parse, Param, Type};
use iron_domain::publish::{PublishFilter, Published};
use iron_domain::scaling::{convert, Scaling};
use iron_domain::source::parse_modbus;
use iron_domain::{Quality, Sample, TagType, Value};
use serde_json::Value as J;

fn load(name: &str) -> J {
    let path = format!("{}/../../conformance/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    oneyaml::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn quality(j: &J) -> Quality {
    match j.as_str() {
        None | Some("GOOD") => Quality::Good,
        Some("UNCERTAIN") => Quality::Uncertain,
        Some("BAD") => Quality::Bad,
        Some(other) => panic!("unknown quality {other}"),
    }
}

fn value(j: &J) -> Value {
    match j {
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => Value::Number(n.as_f64().unwrap()),
        other => panic!("not a value: {other}"),
    }
}

fn close(case: &J, got: f64, want: f64, label: &str) {
    let tol = case["tol"].as_f64().or(case["expect"]["tol"].as_f64()).unwrap_or(1e-6);
    assert!((got - want).abs() <= tol, "{label}: got {got}, want {want} (tol {tol})");
}

fn label(case: &J) -> String {
    case["name"].as_str().or(case["expr"].as_str()).or(case["template"].as_str()).map_or_else(|| case.to_string(), str::to_string)
}

fn inputs(case: &J) -> impl Fn(&str) -> Option<Sample> + '_ {
    move |name| {
        let i = case["inputs"].get(name)?;
        Some(Sample { value: value(&i["value"]), quality: quality(&i["quality"]) })
    }
}

#[test]
fn expressions_runtime() {
    let doc = load("expressions.yaml");
    let cases = doc["runtime"].as_array().unwrap();
    assert!(cases.len() >= 18);
    for case in cases {
        let name = label(case);
        let expr = parse(case["expr"].as_str().unwrap()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let get = inputs(case);
        expr.check(&|n| get(n).map(|s| if matches!(s.value, Value::Bool(_)) { Type::Bool } else { Type::Number }))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got = expr.evaluate(&get);
        let want = &case["expect"];
        if !want["quality"].is_null() {
            assert_eq!(got.quality, quality(&want["quality"]), "{name}");
        }
        match &want["value"] {
            J::Null => {}
            J::Bool(b) => assert_eq!(got.value, Some(Value::Bool(*b)), "{name}"),
            n => match got.value {
                Some(Value::Number(g)) => close(case, g, n.as_f64().unwrap(), &name),
                other => panic!("{name}: got {other:?}"),
            },
        }
        if got.quality == Quality::Bad {
            assert_eq!(got.value, None, "{name}: a BAD result carries no value");
        }
    }
}

#[test]
fn expressions_suppression() {
    let doc = load("expressions.yaml");
    for case in doc["suppression"].as_array().unwrap() {
        let expr = parse(case["expr"].as_str().unwrap()).unwrap();
        let want = case["expect"]["value"].as_bool().unwrap();
        assert_eq!(expr.suppresses(&inputs(case)), want, "{}", label(case));
    }
}

#[test]
fn expressions_expansion() {
    let doc = load("expressions.yaml");
    for case in doc["expansion"].as_array().unwrap() {
        let name = label(case);
        let params = |p: &str| match case["params"].get(p)? {
            J::String(s) => Some(Param::Text(s.clone())),
            J::Number(n) => Some(Param::Number { value: n.as_f64().unwrap(), int: n.is_i64() || n.is_u64() }),
            other => panic!("{name}: bad param {other}"),
        };
        let got = expand(case["template"].as_str().unwrap(), &params);
        if case["expect_error"].as_bool() == Some(true) {
            // REQ-EXP-004: a non-integer where an address is required.
            let rejected = match &got {
                Err(_) => true,
                Ok(Param::Text(uri)) => parse_modbus(uri).is_err(),
                Ok(Param::Number { int, .. }) => !int,
            };
            assert!(rejected, "{name}: expected a rejection, got {got:?}");
            continue;
        }
        match (&case["expect"], got.unwrap_or_else(|e| panic!("{name}: {e}"))) {
            (J::String(want), Param::Text(text)) => assert_eq!(&text, want, "{name}"),
            (J::Number(want), Param::Number { value, .. }) => {
                close(case, value, want.as_f64().unwrap(), &name);
            }
            (want, got) => panic!("{name}: want {want}, got {got:?}"),
        }
    }
}

#[test]
fn decode_modbus() {
    let doc = load("decode.yaml");
    let cases = doc["modbus"].as_array().unwrap();
    assert!(cases.len() >= 11);
    for case in cases {
        let ty = TagType::parse(case["type"].as_str().unwrap()).unwrap();
        let order = case["order"].as_str().map_or(ByteOrder::Abcd, |o| ByteOrder::parse(o).unwrap());
        let regs: Vec<u16> = case["registers"].as_array().unwrap().iter().map(|r| u16::try_from(r.as_u64().unwrap()).unwrap()).collect();
        let bit = case["bit"].as_u64().map(|b| b as u8);
        let got = decode(ty, order, &regs, bit).unwrap();
        match (&case["expect"], got) {
            (J::Bool(w), Value::Bool(g)) => assert_eq!(g, *w, "{case}"),
            (w, Value::Number(g)) => close(case, g, w.as_f64().unwrap(), &case.to_string()),
            (w, g) => panic!("{case}: want {w}, got {g:?}"),
        }
    }
}

#[test]
fn scaling() {
    let doc = load("decode.yaml");
    let pair = |j: &J| (j[0].as_f64().unwrap(), j[1].as_f64().unwrap());
    for case in doc["scaling"].as_array().unwrap() {
        let scaling = (!case["scaling"].is_null())
            .then(|| Scaling { raw: pair(&case["scaling"]["raw"]), engineering: pair(&case["scaling"]["engineering"]) });
        let range = (!case["range"].is_null()).then(|| pair(&case["range"]));
        let (v, q) = convert(case["raw"].as_f64().unwrap(), scaling, range);
        close(case, v, case["expect"]["value"].as_f64().unwrap(), &label(case));
        assert_eq!(q, quality(&case["expect"]["quality"]), "{}", label(case));
    }
}

#[test]
fn publish_filter() {
    let doc = load("publish.yaml");
    for case in doc["cases"].as_array().unwrap() {
        let cfg = &case["config"];
        let last = &case["last_published"];
        let ms = |j: &J| (j.as_f64().unwrap() * 1000.0) as i64;
        let mut filter = PublishFilter {
            deadband: cfg["deadband"].as_f64().unwrap(),
            max_interval_ms: ms(&cfg["max_publish_interval"]),
            alarm_limits: cfg["alarm_limits"].as_array().map(|a| a.iter().map(|l| l.as_f64().unwrap()).collect()).unwrap_or_default(),
            last: Some(Published { t_ms: ms(&last["t"]), value: last["value"].as_f64().unwrap(), quality: quality(&last["quality"]) }),
        };
        for step in case["steps"].as_array().unwrap() {
            let got = filter.offer(ms(&step["t"]), step["value"].as_f64().unwrap(), quality(&step["quality"]));
            assert_eq!(got, step["publish"].as_bool().unwrap(), "{}: step {step}", label(case));
        }
    }
}

#[test]
fn datasheet_address_notation_is_rejected() {
    // REQ-DRV-009, the source-URI half of the validate.yaml case.
    use iron_domain::source::SourceError;
    assert_eq!(parse_modbus("modbus://10.0.0.5/holding/40001"), Err(SourceError::DatasheetNotation { suggested: 0 }));
    assert_eq!(parse_modbus("modbus://plc/holding/0x9C41").unwrap().address, 0x9C41);
    assert_eq!(parse_modbus("modbus://plc/holding/0x1000?bit=3").unwrap().bit, Some(3));
    assert!(parse_modbus("modbus://plc/holding/256.5").is_err());
    assert_eq!(parse_modbus("modbus://plc/holding/4x0001"), Err(SourceError::DatasheetNotation { suggested: 0 }));
    for bad in ["holding/0x+10", "holding/1?bit=+3", "holding/1?bit=1&bit=2", "coil/1?bit=1"] {
        assert!(parse_modbus(&format!("modbus://plc/{bad}")).is_err(), "{bad}");
    }
}

#[test]
fn size_is_bounded() {
    use iron_domain::expr::{ErrorKind, MAX_DEPTH, MAX_TOKENS};
    let nested = |n: usize| format!("{}1{}", "(".repeat(n), ")".repeat(n));
    assert!(parse(&nested(MAX_DEPTH)).is_ok(), "exactly {MAX_DEPTH} levels are allowed");
    assert_eq!(parse(&nested(MAX_DEPTH + 1)).unwrap_err().kind, ErrorKind::TooDeep);
    // A flat chain is as deep a tree as nesting; it must be bounded too.
    let chain = |terms: usize| vec!["1"; terms].join("+");
    let longest = parse(&chain(MAX_TOKENS / 2)).unwrap();
    assert_eq!(longest.check(&|_| None), Ok(Type::Number));
    assert_eq!(parse(&chain(5000)).unwrap_err().kind, ErrorKind::TooLong);
}

// REQ-EXP-006
#[test]
fn a_non_finite_operand_is_no_value() {
    let nan = |_: &str| Some(Sample { value: Value::Number(f64::NAN), quality: Quality::Good });
    for src in ["a.x", "max(a.x, 1)", "a.x + 1"] {
        let got = parse(src).unwrap().evaluate(&nan);
        assert_eq!((got.value, got.quality), (None, Quality::Bad), "{src}");
    }
}

// REQ-EDG-001..004
#[test]
fn the_first_sample_always_publishes() {
    let mut f = PublishFilter { deadband: 5.0, max_interval_ms: 60_000, alarm_limits: vec![], last: None };
    assert!(f.offer(0, 1.0, Quality::Good));
    assert!(!f.offer(1, 1.0, Quality::Good));
}

// REQ-EXP-006, REQ-EXP-007 as properties; a stand-in for a fuzzer.
#[test]
fn evaluation_is_total() {
    const PIECES: [&str; 40] = [
        "a.x", "a.y", "a.b", "0", "1", "2.5", "0x10", "1000", "true", "false", "+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=",
        "&&", "||", "!", "(", ")", ",", "abs(", "min(", "max(", "clamp(", "sqrt(", "pow(", "exp(", "ln(", "log10(", "if(", "round(",
        "floor(", "ceil(",
    ];
    const NUMS: [f64; 6] = [0.0, -1.0, 1e308, -1e308, 0.5, 1000.0];
    const QUALS: [Quality; 3] = [Quality::Good, Quality::Uncertain, Quality::Bad];
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let (mut parsed, mut evaluated) = (0, 0);
    for _ in 0..200_000 {
        let len = 1 + next(9);
        let src: Vec<&str> = (0..len).map(|_| PIECES[next(PIECES.len())]).collect();
        let Ok(expr) = parse(&src.join(" ")) else {
            continue;
        };
        parsed += 1;
        let (x, y, b) = (NUMS[next(6)], NUMS[next(6)], next(2) == 0);
        let (qx, qy, qb) = (QUALS[next(3)], QUALS[next(3)], QUALS[next(3)]);
        let get = |name: &str| match name {
            "a.x" => Some(Sample { value: Value::Number(x), quality: qx }),
            "a.y" => Some(Sample { value: Value::Number(y), quality: qy }),
            "a.b" => Some(Sample { value: Value::Bool(b), quality: qb }),
            _ => None,
        };
        let got = expr.evaluate(&get);
        let worst = expr.refs().iter().map(|n| get(n).unwrap().quality).fold(Quality::Good, Quality::worst);
        assert!(got.quality >= worst, "{src:?}");
        match got.value {
            Some(Value::Number(n)) => assert!(n.is_finite(), "{src:?} gave {n}"),
            Some(Value::Bool(_)) => {}
            None => assert_eq!(got.quality, Quality::Bad, "{src:?}"),
        }
        evaluated += usize::from(got.value.is_some());
    }
    assert!(parsed > 1_000 && evaluated > 500, "generator too weak: {parsed}/{evaluated}");
}

/// REQ-ALM-005..008.
#[test]
fn alarm_detection() {
    use iron_domain::alarm::{AlarmDef, Condition, Detector};
    let doc = load("alarms.yaml");
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 9);
    let ms = |j: &J| (j.as_f64().unwrap_or(0.0) * 1000.0).round() as i64;
    for case in cases {
        let (name, a) = (label(case), &case["alarm"]);
        let condition = match a["state"].as_bool() {
            Some(state) => Condition::State(state),
            None => AlarmDef::level(a["name"].as_str().unwrap(), a["limit"].as_f64().unwrap()).unwrap(),
        };
        let def = AlarmDef {
            condition,
            deadband: a["deadband"].as_f64().unwrap_or(0.0),
            on_delay_ms: ms(&a["on_delay"]),
            off_delay_ms: ms(&a["off_delay"]),
        };
        let mut detector = Detector::default();
        for step in case["steps"].as_array().unwrap() {
            let t = ms(&step["t"]);
            let transitions = if step["poll"].as_bool() == Some(true) {
                [detector.poll(&def, t), None]
            } else {
                detector.update(&def, t, value(&step["value"]), quality(&step["quality"]))
            };
            assert_eq!(detector.active, step["active"].as_bool().unwrap(), "{name}: step {step}");
            if let Some(want) = step["transitions"].as_array() {
                let got: Vec<i64> = transitions.iter().flatten().map(|x| x.t_ms).collect();
                assert_eq!(got, want.iter().map(&ms).collect::<Vec<_>>(), "{name}: step {step}");
            }
        }
    }
}
