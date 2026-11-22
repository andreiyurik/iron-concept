//! `iron validate`: every spec file checked against its invariants.

use crate::diag::{self, Diagnostic, Location, Report, Rule};
use crate::model::{Project, Tag};
use crate::yaml::{self, Entry, Node, Span};
use iron_domain::alarm::{AlarmDef, Condition, LEVELS};
use iron_domain::decode::ByteOrder;
use iron_domain::expr::{self, ErrorKind, Expr, Type};
use iron_domain::source::{parse_modbus, SourceError, Table};
use iron_domain::TagType;
use std::collections::{BTreeMap, BTreeSet};

/// Project files: path relative to the project root → content.
pub type Files = BTreeMap<String, String>;

const SCHEMES: [&str; 10] = ["modbus", "modbus+rtu", "opcua", "s7", "mqtt", "i2c", "gpio", "1wire", "spi", "sim"];
const TAG_FIELDS: [&str; 11] =
    ["source", "derived", "type", "unit", "description", "scan_rate", "deadband", "range", "scaling", "signal", "alarms"];
const ALARM_FIELDS: [&str; 12] = [
    "limit",
    "state",
    "priority",
    "message",
    "deadband",
    "on_delay",
    "off_delay",
    "suppressed_when",
    "adjustable",
    "cause",
    "consequence",
    "operator_action",
];
const COMMAND_FIELDS: [&str; 11] = [
    "target",
    "type",
    "unit",
    "description",
    "requires_role",
    "requires_confirm",
    "cooldown",
    "limits",
    "max_step",
    "feedback_tag",
    "feedback_timeout",
];
const CONNECTION_FIELDS: [&str; 10] =
    ["host", "port", "unit", "order", "timeout", "retries", "max_block", "max_gap", "concurrency", "serial"];
const ROLES: [&str; 4] = ["viewer", "operator", "engineer", "admin"];

struct Pending {
    file: String,
    /// Where a problem is reported: `<tag>.derived` or `<tag>.alarms.<a>.suppressed_when`.
    path: String,
    span: Span,
    expr: Expr,
    want: Type,
    /// The tag this expression defines; absent for a suppression condition.
    tag: Option<String>,
}

struct Validator<'a> {
    files: &'a Files,
    diags: Vec<Diagnostic>,
    project: Project,
    pending: Vec<Pending>,
    feedback: Vec<(String, String, Span, String)>,
    commands: BTreeSet<String>,
}

pub struct Checked {
    pub report: Report,
    /// Meaningful only when the report has no errors.
    pub project: Project,
}

#[must_use]
pub fn validate(files: &Files) -> Report {
    check(files).report
}

#[must_use]
pub fn check<'a>(files: &'a Files) -> Checked {
    let mut v = Validator {
        files,
        diags: Vec::new(),
        project: Project::default(),
        pending: Vec::new(),
        feedback: Vec::new(),
        commands: BTreeSet::new(),
    };
    for (file, src) in files.iter().filter(|(f, _)| f.ends_with(".yaml") || f.ends_with(".yml")) {
        let handler: fn(&mut Validator<'a>, &str, &Node) = match file.as_str() {
            f if f.starts_with("config/tags/") => Validator::tag_file,
            f if f.starts_with("config/commands/") => Validator::command_file,
            "config/connections.yaml" => Validator::connections,
            f if f.starts_with("config/") => {
                let hint = "The current build checks tags, commands, and connections only.";
                v.report(diag::NOT_VALIDATED, file, "", Span::default(), "this file is not validated yet", hint);
                continue;
            }
            _ => continue,
        };
        if let Some(root) = v.read(file, src) {
            handler(&mut v, file, &root);
        }
    }
    v.check_expressions();
    v.order_derived();
    v.check_feedback();
    v.diags.sort_by_key(|d| (d.location.file.clone(), d.location.line, d.location.col));
    let report = Report { tags: v.project.tags.len(), commands: v.commands.len(), diagnostics: v.diags };
    Checked { report, project: v.project }
}

impl Validator<'_> {
    fn report(&mut self, rule: Rule, file: &str, path: &str, span: Span, message: &str, hint: &str) {
        let src = self.files.get(file).map_or("", String::as_str);
        let (line, col) = yaml::line_col(src, span.start);
        let (end_line, end_col) = yaml::line_col(src, span.end.max(span.start));
        self.diags.push(Diagnostic {
            code: rule.code,
            severity: rule.severity(),
            location: Location { file: file.into(), path: path.into(), line, col, end_line, end_col },
            message: message.into(),
            hint: hint.into(),
            requirement: rule.requirement,
            docs: rule.docs,
        });
    }

    /// A file with a syntax error or a construct outside the subset is
    /// reported and not interpreted further.
    fn read(&mut self, file: &str, src: &str) -> Option<Node> {
        let (root, problems) = yaml::parse(src);
        let clean = problems.is_empty();
        for p in problems {
            let (rule, hint) = if p.syntax {
                (diag::YAML_SYNTAX, "Fix the YAML syntax; nothing else in this file was checked.")
            } else {
                (diag::YAML_SUBSET, "IRON reads a subset of YAML that programs can edit without losing comments. Write the value out in full; use a device type for repetition.")
            };
            self.report(rule, file, &p.path, p.span, &p.message, hint);
        }
        root.filter(|_| clean)
    }

    fn section<'n>(&mut self, file: &str, path: &str, entry: &'n Entry, what: &str) -> &'n [Entry] {
        if entry.key.starts_with('_') {
            let message = "names beginning with '_' are reserved for IRON's system tags";
            self.report(diag::RESERVED_NAME, file, path, entry.key_span, message, "Choose another name.");
            return &[];
        }
        entry.value.entries().unwrap_or_else(|| {
            let hint = "Write the name, a colon, and its fields indented below.";
            self.report(diag::BAD_VALUE, file, path, entry.key_span, &format!("{what} must be a mapping"), hint);
            &[]
        })
    }

    /// Reports fields outside `known`. `forbidden` names one field that gets
    /// its own rule instead of the generic "unknown field".
    fn fields(&mut self, file: &str, path: &str, fields: &[Entry], known: &[&str], forbidden: Option<(&str, Rule, &str, &str)>) {
        for f in fields {
            let at = format!("{path}.{}", f.key);
            match forbidden {
                Some((name, rule, message, hint)) if f.key == name => self.report(rule, file, &at, f.key_span, message, hint),
                _ if !known.contains(&f.key.as_str()) => {
                    let hint = format!("Known fields: {}.", known.join(", "));
                    self.report(diag::UNKNOWN_FIELD, file, &at, f.key_span, &format!("unknown field '{}'", f.key), &hint);
                }
                _ => {}
            }
        }
    }

    fn durations(&mut self, file: &str, base: &str, node: &Node, keys: &[&str]) {
        for key in keys {
            if let Some(n) = node.get(key).filter(|n| n.as_ms().is_none()) {
                let hint = "Write a number with a unit: 500ms, 5s, 2m, 1h.";
                self.report(diag::BAD_VALUE, file, &format!("{base}.{key}"), n.span, "not a duration", hint);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn numbers(&mut self, rule: Rule, file: &str, base: &str, node: &Node, keys: &[&str], (min, max): (f64, f64), whole: bool) {
        for key in keys {
            let Some(n) = node.get(key) else { continue };
            if !n.as_f64().is_some_and(|v| v >= min && v <= max && (!whole || v.fract() == 0.0)) {
                let kind = if whole { "a whole number" } else { "a number" };
                let range = if max.is_finite() { format!("from {min} to {max}") } else { format!("{min} or greater") };
                let message = format!("`{key}` must be {kind}, {range}");
                self.report(rule, file, &format!("{base}.{key}"), n.span, &message, "Correct the value.");
            }
        }
    }

    fn pair(&mut self, file: &str, path: &str, node: &Node, ordered: bool) -> Option<(f64, f64)> {
        let pair = match node.items() {
            Some([a, b]) => a.as_f64().zip(b.as_f64()),
            _ => None,
        };
        let valid = pair.filter(|(a, b)| a != b && (!ordered || a < b));
        if valid.is_none() {
            let need = if ordered { "two numbers [min, max] with min < max" } else { "two different numbers" };
            self.report(diag::BAD_VALUE, file, path, node.span, &format!("expected {need}"), "Example: [0, 200]");
        }
        valid
    }

    fn tag_type(&mut self, rule: Rule, file: &str, path: &str, entry: &Entry) -> Option<TagType> {
        let ty = entry.value.get("type").and_then(Node::as_str).and_then(TagType::parse);
        if ty.is_none() {
            let span = entry.value.get("type").map_or(entry.key_span, |n| n.span);
            self.report(rule, file, path, span, "missing or unknown `type`", &format!("Use {}.", TagType::NAMES));
        }
        ty
    }

    fn tag_file(&mut self, file: &str, root: &Node) {
        for area in root.entries().unwrap_or_default() {
            if area.key == "instances" {
                let hint = "Write the tags out for now; device-type expansion is on the roadmap.";
                self.report(diag::NOT_IMPLEMENTED, file, "instances", area.key_span, "device types are not implemented yet", hint);
                continue;
            }
            for tag in self.section(file, &area.key, area, "an area") {
                let path = format!("{}.{}", area.key, tag.key);
                if !self.section(file, &path, tag, "a tag").is_empty() {
                    self.tag(file, &path, tag);
                }
            }
        }
    }

    fn tag(&mut self, file: &str, path: &str, entry: &Entry) {
        let at = |field: &str| format!("{path}.{field}");
        let node = &entry.value;
        let target = (
            "target",
            diag::TARGET_IN_TAG,
            "a tag cannot have a writable target",
            "Tags are read-only. Declare the write as a command in config/commands/.",
        );
        self.fields(file, path, node.entries().unwrap_or_default(), &TAG_FIELDS, Some(target));

        let ty = self.tag_type(diag::BAD_VALUE, file, path, entry);
        if ty.is_some() && self.project.tags.contains_key(path) {
            let hint = "A tag path must be unique across the project. Rename one of them.";
            self.report(diag::DUPLICATE_TAG, file, path, entry.key_span, "this tag is already defined", hint);
            return;
        }
        let numeric = ty.is_some_and(|t| t != TagType::Bool);
        let mut derived = None;
        match (node.get("source"), node.get("derived")) {
            (Some(_), Some(_)) => {
                let hint = "Keep one: `source` for a signal read from a device, `derived` for a value computed from other tags.";
                self.report(diag::SOURCE_XOR, file, path, entry.key_span, "a tag has both `source` and `derived`", hint);
            }
            (None, None) => {
                let hint = "Add `source:` with the signal address, or `derived:` with an expression.";
                self.report(diag::SOURCE_XOR, file, path, entry.key_span, "a tag needs `source` or `derived`", hint);
            }
            (Some(source), None) => {
                self.source(file, &at("source"), source, ty, false);
                self.durations(file, path, node, &["scan_rate"]);
            }
            (None, Some(d)) => {
                if let Some(rate) = node.get("scan_rate") {
                    let hint = "A derived tag is evaluated whenever one of its inputs changes. Remove the field.";
                    self.report(diag::BAD_VALUE, file, &at("scan_rate"), rate.span, "`scan_rate` is not allowed on a derived tag", hint);
                }
                derived = self.expression(file, &at("derived"), d);
                if let (Some(expr), Some(t)) = (&derived, ty) {
                    let (expr, want, tag) = (expr.clone(), t.expr_type(), Some(path.into()));
                    self.pending.push(Pending { file: file.into(), path: at("derived"), span: d.span, expr, want, tag });
                }
            }
        }

        self.numbers(diag::BAD_VALUE, file, path, node, &["deadband"], (0.0, f64::INFINITY), false);
        let range = node.get("range").and_then(|n| self.pair(file, &at("range"), n, true));
        if numeric && node.get("range").is_none() {
            let hint = "Add `range: [min, max]` in engineering units — it bounds alarm limits and flags implausible values.";
            self.report(diag::MISSING_RANGE, file, path, entry.key_span, "a numeric tag needs `range`", hint);
        }
        if numeric && node.get("unit").is_none() {
            let hint = "Add `unit:` so operators see what the number means.";
            self.report(diag::MISSING_UNIT, file, path, entry.key_span, "no `unit`", hint);
        }
        if let Some(s) = node.get("scaling") {
            for side in ["raw", "engineering"] {
                match s.get(side) {
                    Some(n) => drop(self.pair(file, &at(&format!("scaling.{side}")), n, false)),
                    None => {
                        let hint = "Write `scaling: { raw: [a, b], engineering: [c, d] }`.";
                        self.report(diag::BAD_VALUE, file, &at("scaling"), s.span, &format!("`scaling` needs `{side}`"), hint);
                    }
                }
            }
        }
        let alarms = node.get("alarms").map_or_else(Vec::new, |a| self.alarms(file, path, a, ty, range));
        if let Some(ty) = ty {
            self.project.tags.insert(path.into(), Tag { ty, derived, alarms });
        }
    }

    fn source(&mut self, file: &str, path: &str, node: &Node, ty: Option<TagType>, writable: bool) {
        let (rule, what) = if writable { (diag::COMMAND, "target") } else { (diag::UNKNOWN_SCHEME, "source") };
        let scheme = node.as_str().and_then(|uri| uri.split_once("://")).map(|(s, _)| s);
        if !scheme.is_some_and(|s| SCHEMES.contains(&s)) {
            let hint = format!("Write a URI such as modbus://plc-01/holding/0x1000. Known schemes: {}.", SCHEMES.join(", "));
            self.report(rule, file, path, node.span, &format!("unknown {what} scheme"), &hint);
            return;
        }
        if scheme != Some("modbus") {
            return;
        }
        match parse_modbus(node.as_str().unwrap_or_default()) {
            Ok(m) => {
                if m.bit.is_some() && ty.is_some_and(|t| t != TagType::Bool) {
                    let hint = "Remove `?bit=` or make the tag `type: bool`.";
                    self.report(diag::BAD_ADDRESS, file, path, node.span, "`bit` applies to bool tags only", hint);
                }
                if writable && !matches!(m.table, Table::Coil | Table::Holding) {
                    let hint = "A command can write a `coil` or a `holding` register.";
                    self.report(diag::COMMAND, file, path, node.span, "this Modbus table is read-only", hint);
                }
            }
            Err(SourceError::DatasheetNotation { suggested }) => {
                let hint = format!("Source addresses are zero-based protocol addresses. This one is most likely {suggested} (0x{suggested:04X}). If you mean the literal address, write it in hexadecimal.");
                self.report(diag::DATASHEET_ADDR, file, path, node.span, "datasheet address notation is not accepted", &hint);
            }
            Err(SourceError::Syntax(message)) => {
                let hint = "Expected modbus://<host>/<table>/<address>[?bit=<n>][&order=<order>].";
                self.report(diag::BAD_ADDRESS, file, path, node.span, &message, hint);
            }
            Err(SourceError::Scheme) => {}
        }
    }

    fn alarms(&mut self, file: &str, tag: &str, node: &Node, ty: Option<TagType>, range: Option<(f64, f64)>) -> Vec<(String, AlarmDef)> {
        let base = format!("{tag}.alarms");
        let is_bool = ty == Some(TagType::Bool);
        let mut defs = Vec::new();
        let mut limits: BTreeMap<usize, f64> = BTreeMap::new();
        let Some(alarms) = node.entries() else {
            self.report(
                diag::ALARM_FIELDS,
                file,
                &base,
                node.span,
                "`alarms` must be a mapping",
                "Write `alarms:` followed by named alarms.",
            );
            return defs;
        };
        for a in alarms {
            let path = format!("{base}.{}", a.key);
            let at = |field: &str| format!("{path}.{field}");
            let level = LEVELS.iter().position(|l| *l == a.key);
            if ty.is_some() && !is_bool && level.is_none() {
                let message = "an alarm on a numeric tag must be named low_low, low, high, or high_high";
                let hint = "The name gives the direction: high alarms fire at or above the limit, low alarms at or below it.";
                self.report(diag::ALARM_NAME, file, &path, a.key_span, message, hint);
                continue;
            }
            let action = (
                "action",
                diag::ALARM_ACTION,
                "an alarm cannot act",
                "Alarms notify people. Put protective logic in the PLC, and let an operator send the command.",
            );
            self.fields(file, &path, a.value.entries().unwrap_or_default(), &ALARM_FIELDS, Some(action));
            let get = |name: &str| a.value.get(name);

            let condition = match (get("limit"), get("state")) {
                (Some(l), None) if !is_bool => {
                    let limit = l.as_f64();
                    match (limit, range) {
                        (Some(v), Some((min, max))) if v < min || v > max => {
                            let message = format!("alarm {} ({v}) lies outside range [{min}, {max}]", a.key);
                            let hint = "Alarm limits MUST lie inside range: widen the range if the sensor supports it, or move the limit.";
                            self.report(diag::ALARM_RANGE, file, &at("limit"), l.span, &message, hint);
                        }
                        (None, _) => {
                            let hint = "Give the threshold in engineering units.";
                            self.report(diag::ALARM_FIELDS, file, &at("limit"), l.span, "`limit` must be a number", hint);
                        }
                        _ => {}
                    }
                    limit.zip(level).and_then(|(v, i)| {
                        limits.insert(i, v);
                        AlarmDef::level(&a.key, v)
                    })
                }
                (None, Some(s)) if is_bool => {
                    if s.as_bool().is_none() {
                        let hint = "Name the state that is the alarm.";
                        self.report(diag::ALARM_FIELDS, file, &at("state"), s.span, "`state` must be true or false", hint);
                    }
                    s.as_bool().map(Condition::State)
                }
                _ => {
                    if ty.is_some() {
                        let (message, hint) = if is_bool {
                            (
                                "an alarm on a bool tag needs `state` and no `limit`",
                                "Write `state: true` or `state: false` — the alarming state.",
                            )
                        } else {
                            (
                                "an alarm on a numeric tag needs `limit` and no `state`",
                                "Write `limit:` with the threshold in engineering units.",
                            )
                        };
                        self.report(diag::ALARM_FIELDS, file, &path, a.key_span, message, hint);
                    }
                    None
                }
            };
            if get("priority").is_none() {
                let hint = "Add `priority:` from 1 (critical) to 4 (low).";
                self.report(diag::ALARM_FIELDS, file, &path, a.key_span, "missing `priority`", hint);
            }
            self.numbers(diag::ALARM_FIELDS, file, &path, &a.value, &["priority"], (1.0, 4.0), true);
            self.numbers(diag::ALARM_FIELDS, file, &path, &a.value, &["deadband"], (0.0, f64::INFINITY), false);
            if get("message").and_then(Node::as_str).is_none_or(str::is_empty) {
                let hint = "Add `message:` — what the operator sees.";
                self.report(diag::ALARM_FIELDS, file, &path, a.key_span, "missing `message`", hint);
            }
            if let Some(n) = get("adjustable").filter(|n| n.as_bool().is_none()) {
                self.report(
                    diag::ALARM_FIELDS,
                    file,
                    &at("adjustable"),
                    n.span,
                    "`adjustable` must be true or false",
                    "Write true or false.",
                );
            }
            self.durations(file, &path, &a.value, &["on_delay", "off_delay"]);
            if let Some(c) = get("suppressed_when") {
                if let Some(expr) = self.expression(file, &at("suppressed_when"), c) {
                    let path = at("suppressed_when");
                    self.pending.push(Pending { file: file.into(), path, span: c.span, expr, want: Type::Bool, tag: None });
                }
            }
            if let Some(condition) = condition {
                let ms = |key: &str| get(key).and_then(Node::as_ms).unwrap_or(0);
                let deadband = get("deadband").and_then(Node::as_f64).unwrap_or(0.0);
                let def = AlarmDef { condition, deadband, on_delay_ms: ms("on_delay"), off_delay_ms: ms("off_delay") };
                defs.push((a.key.clone(), def));
            }
        }
        let ordered: Vec<f64> = limits.into_values().collect();
        if ordered.windows(2).any(|w| w[0] >= w[1]) {
            let hint = "Limits MUST satisfy low_low < low < high < high_high.";
            self.report(diag::ALARM_ORDER, file, &base, node.span, "alarm limits are out of order", hint);
        }
        defs
    }

    fn command_file(&mut self, file: &str, root: &Node) {
        for area in root.entries().unwrap_or_default() {
            for cmd in self.section(file, &area.key, area, "an area") {
                let path = format!("{}.{}", area.key, cmd.key);
                let at = |field: &str| format!("{path}.{field}");
                let fields = self.section(file, &path, cmd, "a command");
                if fields.is_empty() {
                    continue;
                }
                if !self.commands.insert(path.clone()) {
                    let hint = "A command path must be unique across the project.";
                    self.report(diag::COMMAND, file, &path, cmd.key_span, "this command is already defined", hint);
                    continue;
                }
                self.fields(file, &path, fields, &COMMAND_FIELDS, None);
                let get = |name: &str| cmd.value.get(name);
                let ty = self.tag_type(diag::COMMAND, file, &path, cmd);
                match get("target") {
                    Some(t) => self.source(file, &at("target"), t, ty, true),
                    None => {
                        let hint = "Add `target:` with the writable address.";
                        self.report(diag::COMMAND, file, &path, cmd.key_span, "a command needs `target`", hint);
                    }
                }
                if let Some(r) = get("requires_role").filter(|r| !r.as_str().is_some_and(|s| ROLES.contains(&s))) {
                    let hint = "Use viewer, operator, engineer, or admin.";
                    self.report(diag::COMMAND, file, &at("requires_role"), r.span, "unknown role", hint);
                }
                if let Some(c) = get("requires_confirm").filter(|c| c.as_bool().is_none()) {
                    let message = "`requires_confirm` must be true or false";
                    self.report(diag::COMMAND, file, &at("requires_confirm"), c.span, message, "Write true or false.");
                }
                self.durations(file, &path, &cmd.value, &["cooldown", "feedback_timeout"]);
                if let Some(l) = get("limits") {
                    self.pair(file, &at("limits"), l, true);
                }
                self.numbers(diag::COMMAND, file, &path, &cmd.value, &["max_step"], (f64::MIN_POSITIVE, f64::INFINITY), false);
                if let Some(f) = get("feedback_tag") {
                    self.feedback.push((file.into(), at("feedback_tag"), f.span, f.as_str().unwrap_or_default().into()));
                }
            }
        }
    }

    fn connections(&mut self, file: &str, root: &Node) {
        const WHOLE: [(&str, (f64, f64)); 6] = [
            ("port", (1.0, 65_535.0)),
            ("unit", (0.0, 255.0)),
            ("retries", (0.0, 100.0)),
            ("max_block", (1.0, 125.0)),
            ("max_gap", (0.0, 125.0)),
            ("concurrency", (1.0, 64.0)),
        ];
        for c in root.entries().unwrap_or_default() {
            let fields = self.section(file, &c.key, c, "a connection");
            self.fields(file, &c.key, fields, &CONNECTION_FIELDS, None);
            if let Some(o) = c.value.get("order").filter(|o| o.as_str().and_then(ByteOrder::parse).is_none()) {
                let hint = format!("Use {}.", ByteOrder::NAMES);
                self.report(diag::CONNECTION, file, &format!("{}.order", c.key), o.span, "unknown byte order", &hint);
            }
            self.durations(file, &c.key, &c.value, &["timeout"]);
            for (key, range) in WHOLE {
                self.numbers(diag::CONNECTION, file, &c.key, &c.value, &[key], range, true);
            }
        }
    }

    fn expression(&mut self, file: &str, path: &str, node: &Node) -> Option<Expr> {
        let parsed = node.as_str().ok_or_else(|| "an expression must be text".to_string());
        match parsed.and_then(|text| expr::parse(text).map_err(|e| e.message)) {
            Ok(e) => Some(e),
            Err(message) => {
                let hint = "See docs/specs/expressions.md for the grammar and the list of functions.";
                self.report(diag::EXPR_SYNTAX, file, path, node.span, &message, hint);
                None
            }
        }
    }

    fn check_expressions(&mut self) {
        let mut found = Vec::new();
        for p in &self.pending {
            let tags = &self.project.tags;
            let checked = p.expr.check(&|name| tags.get(name).map(|t| t.ty.expr_type()));
            let (rule, message, hint): (Rule, String, &str) = match checked {
                Ok(t) if t == p.want => continue,
                Ok(_) => {
                    let want = if p.want == Type::Bool { "a boolean" } else { "a number" };
                    let hint = "Check the tag's `type` against what the expression computes.";
                    (diag::EXPR_TYPE, format!("the expression must produce {want}"), hint)
                }
                Err(e) => match e.kind {
                    ErrorKind::UnknownReference => {
                        (diag::EXPR_REFERENCE, e.message, "Reference a tag by its full path, for example zone_1.air_temperature.")
                    }
                    ErrorKind::IfBranches => {
                        (diag::EXPR_IF, e.message, "Both results of if(cond, a, b) must be numbers, or both booleans.")
                    }
                    _ => (diag::EXPR_TYPE, e.message, "Numbers and booleans do not mix: compare first, then combine."),
                },
            };
            found.push((rule, p.file.clone(), p.path.clone(), p.span, message, hint));
        }
        for (rule, file, path, span, message, hint) in found {
            self.report(rule, &file, &path, span, &message, hint);
        }
    }

    /// One depth-first pass gives both the evaluation order of derived tags
    /// and every dependency cycle among them (REQ-EXP-003).
    fn order_derived(&mut self) {
        let graph: BTreeMap<&str, (Vec<&str>, &Pending)> =
            self.pending.iter().filter_map(|p| Some((p.tag.as_deref()?, (p.expr.refs(), p)))).collect();
        let mut finished: BTreeMap<&str, bool> = BTreeMap::new(); // false while on the stack
        let mut order = Vec::new();
        let mut cycles: BTreeMap<Vec<&str>, (&Pending, String)> = BTreeMap::new();
        for start in graph.keys().copied() {
            if finished.contains_key(start) {
                continue;
            }
            finished.insert(start, false);
            let mut stack = vec![(start, 0)];
            while let Some((node, next)) = stack.pop() {
                let Some(dep) = graph[node].0.get(next).copied() else {
                    finished.insert(node, true);
                    order.push(node.to_string());
                    continue;
                };
                stack.push((node, next + 1));
                match finished.get(dep) {
                    Some(true) => {}
                    Some(false) => {
                        let at = stack.iter().position(|(n, _)| *n == dep).unwrap_or(0);
                        let mut members: Vec<&str> = stack[at..].iter().map(|(n, _)| *n).collect();
                        let first = members.iter().enumerate().min_by_key(|(_, n)| **n).map_or(0, |(i, _)| i);
                        members.rotate_left(first);
                        let chain = format!("{} → {}", members.join(" → "), members[0]);
                        let mut key = members.clone();
                        key.sort_unstable();
                        cycles.entry(key).or_insert((graph[members[0]].1, chain));
                    }
                    None if graph.contains_key(dep) => {
                        finished.insert(dep, false);
                        stack.push((dep, 0));
                    }
                    None => {}
                }
            }
        }
        let found: Vec<_> = cycles.into_values().map(|(p, chain)| (p.file.clone(), p.path.clone(), p.span, chain)).collect();
        for (file, path, span, chain) in found {
            let hint = "Break the cycle: at least one of these tags must be computed from measured tags only.";
            self.report(diag::EXPR_CYCLE, &file, &path, span, &format!("derived tags depend on each other: {chain}"), hint);
        }
        self.project.derived_order = order;
    }

    fn check_feedback(&mut self) {
        let missing: Vec<_> = self.feedback.iter().filter(|(.., tag)| !self.project.tags.contains_key(tag)).cloned().collect();
        for (file, path, span, tag) in missing {
            let hint = "Name an existing tag by its full path.";
            self.report(diag::COMMAND, &file, &path, span, &format!("feedback tag '{tag}' does not exist"), hint);
        }
    }
}
