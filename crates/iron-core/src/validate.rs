//! `iron validate`: every spec file checked against its invariants.

use crate::diag::{self, Diagnostic, Location, Report, Rule, Severity};
use crate::yaml::{self, Entry, Node, Span};
use iron_domain::expr::{self, ErrorKind, Expr, Type};
use iron_domain::source::{parse_modbus, SourceError, Table};
use iron_domain::TagType;
use std::collections::{BTreeMap, BTreeSet};

pub type Files = BTreeMap<String, String>;

const SCHEMES: [&str; 10] = [
    "modbus",
    "modbus+rtu",
    "opcua",
    "s7",
    "mqtt",
    "i2c",
    "gpio",
    "1wire",
    "spi",
    "sim",
];
const TAG_FIELDS: [&str; 11] = [
    "source",
    "derived",
    "type",
    "unit",
    "description",
    "scan_rate",
    "deadband",
    "range",
    "scaling",
    "signal",
    "alarms",
];
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
const ROLES: [&str; 4] = ["viewer", "operator", "engineer", "admin"];
const LEVELS: [&str; 4] = ["low_low", "low", "high", "high_high"];

struct Pending {
    file: String,
    path: String,
    span: Span,
    expr: Expr,
    want: Type,
}

struct Validator<'a> {
    files: &'a Files,
    diags: Vec<Diagnostic>,
    types: BTreeMap<String, Type>,
    derived: Vec<Pending>,
    conditions: Vec<Pending>,
    feedback: Vec<(String, String, Span, String)>,
    commands: BTreeSet<String>,
}

#[must_use]
pub fn validate(files: &Files) -> Report {
    let mut v = Validator {
        files,
        diags: Vec::new(),
        types: BTreeMap::new(),
        derived: Vec::new(),
        conditions: Vec::new(),
        feedback: Vec::new(),
        commands: BTreeSet::new(),
    };
    for (file, src) in files {
        if !(file.ends_with(".yaml") || file.ends_with(".yml")) {
            continue;
        }
        let in_dir = |dir: &str| file.starts_with(dir);
        if in_dir("config/tags/") {
            if let Some(root) = v.read(file, src) {
                v.tag_file(file, &root);
            }
        } else if in_dir("config/commands/") {
            if let Some(root) = v.read(file, src) {
                v.command_file(file, &root);
            }
        } else if file == "config/connections.yaml" {
            if let Some(root) = v.read(file, src) {
                v.connections(file, &root);
            }
        } else if in_dir("config/") {
            v.emit(
                diag::NOT_VALIDATED,
                Severity::Warning,
                file,
                "",
                Span::default(),
                "this file is not validated yet".into(),
                "The current build checks tags, commands, and connections only.".into(),
            );
        }
    }
    v.check_expressions();
    v.check_cycles();
    v.check_feedback();
    let key = |d: &Diagnostic| (d.location.file.clone(), d.location.line, d.location.col);
    v.diags.sort_by_key(key);
    Report {
        tags: v.types.len(),
        commands: v.commands.len(),
        diagnostics: v.diags,
    }
}

impl Validator<'_> {
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        rule: Rule,
        severity: Severity,
        file: &str,
        path: &str,
        span: Span,
        message: String,
        hint: String,
    ) {
        let src = self.files.get(file).map_or("", String::as_str);
        let (line, col) = yaml::line_col(src, span.start);
        let (end_line, end_col) = yaml::line_col(src, span.end.max(span.start));
        self.diags.push(Diagnostic {
            code: rule.code,
            severity,
            location: Location {
                file: file.to_string(),
                path: path.to_string(),
                line,
                col,
                end_line,
                end_col,
            },
            message,
            hint,
            requirement: rule.requirement,
            docs: rule.docs,
        });
    }

    fn error(&mut self, rule: Rule, file: &str, path: &str, span: Span, message: &str, hint: &str) {
        self.emit(
            rule,
            Severity::Error,
            file,
            path,
            span,
            message.into(),
            hint.into(),
        );
    }

    fn read(&mut self, file: &str, src: &str) -> Option<Node> {
        let (root, problems) = yaml::parse(src);
        let clean = problems.is_empty();
        for p in problems {
            if p.syntax {
                self.error(
                    diag::YAML_SYNTAX,
                    file,
                    &p.path,
                    p.span,
                    &p.message,
                    "Fix the YAML syntax; nothing else in this file was checked.",
                );
            } else {
                self.error(
                    diag::YAML_SUBSET,
                    file,
                    &p.path,
                    p.span,
                    &p.message,
                    "IRON reads a subset of YAML that programs can edit without losing comments. Write the value out in full; use a device type for repetition.",
                );
            }
        }
        root.filter(|_| clean)
    }

    fn tag_file(&mut self, file: &str, root: &Node) {
        for area in root.entries().unwrap_or_default() {
            if area.key == "instances" {
                self.error(
                    diag::NOT_IMPLEMENTED,
                    file,
                    "instances",
                    area.key_span,
                    "device types are not implemented yet",
                    "Write the tags out for now; device-type expansion is on the roadmap.",
                );
                continue;
            }
            if area.key.starts_with('_') {
                self.error(
                    diag::RESERVED_NAME,
                    file,
                    &area.key,
                    area.key_span,
                    "names beginning with '_' are reserved for IRON's system tags",
                    "Rename the area.",
                );
                continue;
            }
            let Some(tags) = area.value.entries() else {
                self.error(
                    diag::BAD_VALUE,
                    file,
                    &area.key,
                    area.key_span,
                    "an area must be a mapping of tags",
                    "Write `area:` followed by indented tag definitions.",
                );
                continue;
            };
            for tag in tags {
                let path = format!("{}.{}", area.key, tag.key);
                if tag.key.starts_with('_') {
                    self.error(
                        diag::RESERVED_NAME,
                        file,
                        &path,
                        tag.key_span,
                        "names beginning with '_' are reserved for IRON's system tags",
                        "Rename the tag.",
                    );
                } else {
                    self.tag(file, &path, tag);
                }
            }
        }
    }

    fn tag(&mut self, file: &str, path: &str, entry: &Entry) {
        let at = |field: &str| format!("{path}.{field}");
        let Some(fields) = entry.value.entries() else {
            self.error(
                diag::BAD_VALUE,
                file,
                path,
                entry.key_span,
                "a tag must be a mapping",
                "Give the tag at least `source` and `type`.",
            );
            return;
        };
        for f in fields {
            if f.key == "target" {
                self.error(
                    diag::TARGET_IN_TAG,
                    file,
                    &at("target"),
                    f.key_span,
                    "a tag cannot have a writable target",
                    "Tags are read-only. Declare the write as a command in config/commands/.",
                );
            } else if !TAG_FIELDS.contains(&f.key.as_str()) {
                let hint = format!("Known tag fields: {}.", TAG_FIELDS.join(", "));
                self.error(
                    diag::UNKNOWN_FIELD,
                    file,
                    &at(&f.key),
                    f.key_span,
                    &format!("unknown field '{}'", f.key),
                    &hint,
                );
            }
        }
        let get = |name: &str| entry.value.get(name);

        let ty = match get("type") {
            Some(n) => match n.as_str().and_then(TagType::parse) {
                Some(t) => Some(t),
                None => {
                    self.error(
                        diag::BAD_VALUE,
                        file,
                        &at("type"),
                        n.span,
                        "unknown type",
                        "Use bool, int16, int32, uint16, uint32, float32, or float64.",
                    );
                    None
                }
            },
            None => {
                self.error(
                    diag::BAD_VALUE,
                    file,
                    path,
                    entry.key_span,
                    "missing `type`",
                    "Add `type:` — bool, int16, int32, uint16, uint32, float32, or float64.",
                );
                None
            }
        };
        let numeric = ty.is_some_and(|t| t != TagType::Bool);

        if let Some(t) = ty {
            let declared = if t == TagType::Bool {
                Type::Bool
            } else {
                Type::Number
            };
            if self.types.insert(path.to_string(), declared).is_some() {
                self.error(
                    diag::DUPLICATE_TAG,
                    file,
                    path,
                    entry.key_span,
                    "this tag is already defined",
                    "A tag path must be unique across the project. Rename one of them.",
                );
                return;
            }
        }

        match (get("source"), get("derived")) {
            (Some(_), Some(_)) => self.error(diag::SOURCE_XOR, file, path, entry.key_span, "a tag has both `source` and `derived`", "Keep one: `source` for a signal read from a device, `derived` for a value computed from other tags."),
            (None, None) => self.error(diag::SOURCE_XOR, file, path, entry.key_span, "a tag needs `source` or `derived`", "Add `source:` with the signal address, or `derived:` with an expression."),
            (Some(source), None) => self.source(file, &at("source"), source, ty, false),
            (None, Some(derived)) => {
                if let Some(rate) = get("scan_rate") {
                    self.error(diag::BAD_VALUE, file, &at("scan_rate"), rate.span, "`scan_rate` is not allowed on a derived tag", "A derived tag is evaluated whenever one of its inputs changes. Remove the field.");
                }
                if let (Some(expr), Some(t)) = (self.expression(file, &at("derived"), derived), ty) {
                    let want = if t == TagType::Bool { Type::Bool } else { Type::Number };
                    self.derived.push(Pending { file: file.into(), path: path.into(), span: derived.span, expr, want });
                }
            }
        }

        if let Some(n) = get("scan_rate") {
            self.duration(file, &at("scan_rate"), n);
        }
        if let Some(n) = get("deadband") {
            if !n.as_f64().is_some_and(|d| d >= 0.0) {
                self.error(
                    diag::BAD_VALUE,
                    file,
                    &at("deadband"),
                    n.span,
                    "`deadband` must be a number, zero or greater",
                    "Give the minimum change that should be published, in engineering units.",
                );
            }
        }
        let range = get("range").and_then(|n| self.pair(file, &at("range"), n, true));
        if numeric && get("range").is_none() {
            self.error(diag::MISSING_RANGE, file, path, entry.key_span, "a numeric tag needs `range`", "Add `range: [min, max]` in engineering units — it bounds alarm limits and flags implausible values.");
        }
        if numeric && get("unit").is_none() {
            self.emit(
                diag::MISSING_UNIT,
                Severity::Warning,
                file,
                path,
                entry.key_span,
                "no `unit`".into(),
                "Add `unit:` so operators see what the number means.".into(),
            );
        }
        if let Some(s) = get("scaling") {
            for side in ["raw", "engineering"] {
                match s.get(side) {
                    Some(n) => {
                        self.pair(file, &at(&format!("scaling.{side}")), n, false);
                    }
                    None => self.error(
                        diag::BAD_VALUE,
                        file,
                        &at("scaling"),
                        s.span,
                        &format!("`scaling` needs `{side}`"),
                        "Write `scaling: { raw: [a, b], engineering: [c, d] }`.",
                    ),
                }
            }
        }
        if let Some(alarms) = get("alarms") {
            self.alarms(file, path, alarms, ty, range);
        }
    }

    fn source(&mut self, file: &str, path: &str, node: &Node, ty: Option<TagType>, writable: bool) {
        let (rule, what) = if writable {
            (diag::COMMAND, "target")
        } else {
            (diag::UNKNOWN_SCHEME, "source")
        };
        let Some(uri) = node.as_str() else {
            self.error(
                diag::BAD_VALUE,
                file,
                path,
                node.span,
                &format!("`{what}` must be a URI"),
                "Example: modbus://plc-01/holding/0x1000",
            );
            return;
        };
        let scheme = uri.split_once("://").map(|(s, _)| s);
        if !scheme.is_some_and(|s| SCHEMES.contains(&s)) {
            let hint = format!("Known schemes: {}.", SCHEMES.join(", "));
            self.error(
                rule,
                file,
                path,
                node.span,
                &format!("unknown {what} scheme"),
                &hint,
            );
            return;
        }
        if scheme != Some("modbus") {
            return;
        }
        match parse_modbus(uri) {
            Ok(m) => {
                if m.bit.is_some() && ty.is_some_and(|t| t != TagType::Bool) {
                    self.error(
                        diag::BAD_ADDRESS,
                        file,
                        path,
                        node.span,
                        "`bit` applies to bool tags only",
                        "Remove `?bit=` or make the tag `type: bool`.",
                    );
                }
                if writable && !matches!(m.table, Table::Coil | Table::Holding) {
                    self.error(
                        diag::COMMAND,
                        file,
                        path,
                        node.span,
                        "this Modbus table is read-only",
                        "A command can write a `coil` or a `holding` register.",
                    );
                }
            }
            Err(SourceError::DatasheetNotation { suggested }) => {
                let hint = format!("Source addresses are zero-based protocol addresses. This one is most likely {suggested} (0x{suggested:04X}). If you mean the literal address, write it in hexadecimal.");
                self.error(
                    diag::DATASHEET_ADDR,
                    file,
                    path,
                    node.span,
                    "datasheet address notation is not accepted",
                    &hint,
                );
            }
            Err(SourceError::Syntax(message)) => self.error(
                diag::BAD_ADDRESS,
                file,
                path,
                node.span,
                &message,
                "Expected modbus://<host>/<table>/<address>[?bit=<n>][&order=<order>].",
            ),
            Err(SourceError::Scheme) => {}
        }
    }

    fn alarms(
        &mut self,
        file: &str,
        tag: &str,
        node: &Node,
        ty: Option<TagType>,
        range: Option<(f64, f64)>,
    ) {
        let base = format!("{tag}.alarms");
        let Some(alarms) = node.entries() else {
            self.error(
                diag::ALARM_FIELDS,
                file,
                &base,
                node.span,
                "`alarms` must be a mapping",
                "Write `alarms:` followed by named alarms.",
            );
            return;
        };
        let is_bool = ty == Some(TagType::Bool);
        let mut limits: BTreeMap<&str, f64> = BTreeMap::new();
        for a in alarms {
            let path = format!("{base}.{}", a.key);
            let at = |field: &str| format!("{path}.{field}");
            for f in a.value.entries().unwrap_or_default() {
                if f.key == "action" {
                    self.error(diag::ALARM_ACTION, file, &at("action"), f.key_span, "an alarm cannot act", "Alarms notify people. Put protective logic in the PLC, and let an operator send the command.");
                } else if !ALARM_FIELDS.contains(&f.key.as_str()) {
                    let hint = format!("Known alarm fields: {}.", ALARM_FIELDS.join(", "));
                    self.error(
                        diag::UNKNOWN_FIELD,
                        file,
                        &at(&f.key),
                        f.key_span,
                        &format!("unknown field '{}'", f.key),
                        &hint,
                    );
                }
            }
            let get = |name: &str| a.value.get(name);
            match (get("limit"), get("state")) {
                (Some(l), None) if !is_bool => match l.as_f64() {
                    Some(limit) => {
                        if let Some((min, max)) = range {
                            if limit < min || limit > max {
                                let message = format!(
                                    "alarm {} ({limit}) lies outside range [{min}, {max}]",
                                    a.key
                                );
                                self.error(diag::ALARM_RANGE, file, &at("limit"), l.span, &message, "Alarm limits MUST lie inside range: widen the range if the sensor supports it, or move the limit.");
                            }
                        }
                        if LEVELS.contains(&a.key.as_str()) {
                            limits.insert(
                                LEVELS[LEVELS.iter().position(|n| *n == a.key).unwrap_or(0)],
                                limit,
                            );
                        }
                    }
                    None => self.error(
                        diag::ALARM_FIELDS,
                        file,
                        &at("limit"),
                        l.span,
                        "`limit` must be a number",
                        "Give the threshold in engineering units.",
                    ),
                },
                (None, Some(s)) if is_bool => {
                    if s.as_bool().is_none() {
                        self.error(
                            diag::ALARM_FIELDS,
                            file,
                            &at("state"),
                            s.span,
                            "`state` must be true or false",
                            "Name the state that is the alarm.",
                        );
                    }
                }
                _ if ty.is_some() => {
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
                    self.error(diag::ALARM_FIELDS, file, &path, a.key_span, message, hint);
                }
                _ => {}
            }
            match get("priority").map(|p| (p, p.as_f64())) {
                Some((_, Some(p))) if p.fract() == 0.0 && (1.0..=4.0).contains(&p) => {}
                Some((p, _)) => self.error(
                    diag::ALARM_FIELDS,
                    file,
                    &at("priority"),
                    p.span,
                    "`priority` must be 1, 2, 3, or 4",
                    "1 is critical, 4 is low.",
                ),
                None => self.error(
                    diag::ALARM_FIELDS,
                    file,
                    &path,
                    a.key_span,
                    "missing `priority`",
                    "Add `priority:` from 1 (critical) to 4 (low).",
                ),
            }
            if get("message")
                .and_then(Node::as_str)
                .is_none_or(str::is_empty)
            {
                self.error(
                    diag::ALARM_FIELDS,
                    file,
                    &path,
                    a.key_span,
                    "missing `message`",
                    "Add `message:` — what the operator sees.",
                );
            }
            for d in ["on_delay", "off_delay"] {
                if let Some(n) = get(d) {
                    self.duration(file, &at(d), n);
                }
            }
            if let Some(c) = get("suppressed_when") {
                if let Some(expr) = self.expression(file, &at("suppressed_when"), c) {
                    self.conditions.push(Pending {
                        file: file.into(),
                        path: at("suppressed_when"),
                        span: c.span,
                        expr,
                        want: Type::Bool,
                    });
                }
            }
        }
        let ordered: Vec<f64> = LEVELS
            .iter()
            .filter_map(|l| limits.get(l).copied())
            .collect();
        if ordered.windows(2).any(|w| w[0] >= w[1]) {
            self.error(
                diag::ALARM_ORDER,
                file,
                &base,
                node.span,
                "alarm limits are out of order",
                "Limits MUST satisfy low_low < low < high < high_high.",
            );
        }
    }

    fn command_file(&mut self, file: &str, root: &Node) {
        for area in root.entries().unwrap_or_default() {
            for cmd in area.value.entries().unwrap_or_default() {
                let path = format!("{}.{}", area.key, cmd.key);
                let at = |field: &str| format!("{path}.{field}");
                if !self.commands.insert(path.clone()) {
                    self.error(
                        diag::COMMAND,
                        file,
                        &path,
                        cmd.key_span,
                        "this command is already defined",
                        "A command path must be unique across the project.",
                    );
                    continue;
                }
                for f in cmd.value.entries().unwrap_or_default() {
                    if !COMMAND_FIELDS.contains(&f.key.as_str()) {
                        let hint = format!("Known command fields: {}.", COMMAND_FIELDS.join(", "));
                        self.error(
                            diag::UNKNOWN_FIELD,
                            file,
                            &at(&f.key),
                            f.key_span,
                            &format!("unknown field '{}'", f.key),
                            &hint,
                        );
                    }
                }
                let get = |name: &str| cmd.value.get(name);
                let ty = get("type").and_then(Node::as_str).and_then(TagType::parse);
                if ty.is_none() {
                    self.error(
                        diag::COMMAND,
                        file,
                        &path,
                        cmd.key_span,
                        "a command needs a valid `type`",
                        "Use bool, int16, int32, uint16, uint32, float32, or float64.",
                    );
                }
                match get("target") {
                    Some(t) => self.source(file, &at("target"), t, ty, true),
                    None => self.error(
                        diag::COMMAND,
                        file,
                        &path,
                        cmd.key_span,
                        "a command needs `target`",
                        "Add `target:` with the writable address.",
                    ),
                }
                if let Some(r) = get("requires_role") {
                    if !r.as_str().is_some_and(|s| ROLES.contains(&s)) {
                        self.error(
                            diag::COMMAND,
                            file,
                            &at("requires_role"),
                            r.span,
                            "unknown role",
                            "Use viewer, operator, engineer, or admin.",
                        );
                    }
                }
                if let Some(c) = get("requires_confirm") {
                    if c.as_bool().is_none() {
                        self.error(
                            diag::COMMAND,
                            file,
                            &at("requires_confirm"),
                            c.span,
                            "`requires_confirm` must be true or false",
                            "Write true or false.",
                        );
                    }
                }
                for d in ["cooldown", "feedback_timeout"] {
                    if let Some(n) = get(d) {
                        self.duration(file, &at(d), n);
                    }
                }
                if let Some(l) = get("limits") {
                    self.pair(file, &at("limits"), l, true);
                }
                if let Some(s) = get("max_step") {
                    if !s.as_f64().is_some_and(|v| v > 0.0) {
                        self.error(
                            diag::COMMAND,
                            file,
                            &at("max_step"),
                            s.span,
                            "`max_step` must be a positive number",
                            "Give the largest change allowed in one write.",
                        );
                    }
                }
                if let Some(f) = get("feedback_tag") {
                    match f.as_str() {
                        Some(tag) => self.feedback.push((
                            file.into(),
                            at("feedback_tag"),
                            f.span,
                            tag.into(),
                        )),
                        None => self.error(
                            diag::COMMAND,
                            file,
                            &at("feedback_tag"),
                            f.span,
                            "`feedback_tag` must be a tag path",
                            "Example: pump_01.running",
                        ),
                    }
                }
            }
        }
    }

    fn connections(&mut self, file: &str, root: &Node) {
        for c in root.entries().unwrap_or_default() {
            if let Some(o) = c.value.get("order") {
                if !o
                    .as_str()
                    .is_some_and(|s| iron_domain::decode::ByteOrder::parse(s).is_some())
                {
                    self.error(
                        diag::BAD_VALUE,
                        file,
                        &format!("{}.order", c.key),
                        o.span,
                        "unknown byte order",
                        "Use ABCD, CDAB, BADC, or DCBA.",
                    );
                }
            }
            if let Some(n) = c.value.get("timeout") {
                self.duration(file, &format!("{}.timeout", c.key), n);
            }
        }
    }

    fn duration(&mut self, file: &str, path: &str, node: &Node) {
        if node.as_str().and_then(parse_duration_ms).is_none() {
            self.error(
                diag::BAD_VALUE,
                file,
                path,
                node.span,
                "not a duration",
                "Write a number with a unit: 500ms, 5s, 2m, 1h.",
            );
        }
    }

    fn pair(&mut self, file: &str, path: &str, node: &Node, ordered: bool) -> Option<(f64, f64)> {
        let pair = match node.items() {
            Some([a, b]) => a.as_f64().zip(b.as_f64()),
            _ => None,
        };
        match pair {
            Some((a, b)) if a != b && (!ordered || a < b) => Some((a, b)),
            _ => {
                let need = if ordered {
                    "two numbers [min, max] with min < max"
                } else {
                    "two different numbers"
                };
                self.error(
                    diag::BAD_VALUE,
                    file,
                    path,
                    node.span,
                    &format!("expected {need}"),
                    "Example: [0, 200]",
                );
                None
            }
        }
    }

    fn expression(&mut self, file: &str, path: &str, node: &Node) -> Option<Expr> {
        let Some(text) = node.as_str() else {
            self.error(
                diag::EXPR_SYNTAX,
                file,
                path,
                node.span,
                "an expression must be text",
                "Quote the expression.",
            );
            return None;
        };
        match expr::parse(text) {
            Ok(e) => Some(e),
            Err(e) => {
                self.error(
                    diag::EXPR_SYNTAX,
                    file,
                    path,
                    node.span,
                    &e.message,
                    "See docs/specs/expressions.md for the grammar and the list of functions.",
                );
                None
            }
        }
    }

    fn check_expressions(&mut self) {
        let pending: Vec<Pending> = self
            .derived
            .drain(..)
            .chain(self.conditions.drain(..))
            .collect();
        for p in &pending {
            let types = &self.types;
            let outcome = p.expr.check(&|name| types.get(name).copied());
            let (rule, message, hint): (Rule, String, &str) = match outcome {
                Ok(t) if t == p.want => continue,
                Ok(_) => {
                    let want = if p.want == Type::Bool {
                        "a boolean"
                    } else {
                        "a number"
                    };
                    (
                        diag::EXPR_TYPE,
                        format!("the expression must produce {want}"),
                        "Check the tag's `type` against what the expression computes.",
                    )
                }
                Err(e) => match e.kind {
                    ErrorKind::UnknownReference => (
                        diag::EXPR_REFERENCE,
                        e.message,
                        "Reference a tag by its full path, for example zone_1.air_temperature.",
                    ),
                    ErrorKind::IfBranches => (
                        diag::EXPR_IF,
                        e.message,
                        "Both results of if(cond, a, b) must be numbers, or both booleans.",
                    ),
                    _ => (
                        diag::EXPR_TYPE,
                        e.message,
                        "Numbers and booleans do not mix: compare first, then combine.",
                    ),
                },
            };
            let path = if p.path.ends_with(".suppressed_when") {
                p.path.clone()
            } else {
                format!("{}.derived", p.path)
            };
            self.error(rule, &p.file, &path, p.span, &message, hint);
        }
        self.derived = pending
            .into_iter()
            .filter(|p| !p.path.ends_with(".suppressed_when"))
            .collect();
    }

    // REQ-EXP-003: one report per cycle, at its first member.
    fn check_cycles(&mut self) {
        let graph: BTreeMap<&str, Vec<&str>> = self
            .derived
            .iter()
            .map(|p| (p.path.as_str(), p.expr.refs()))
            .collect();
        let mut reported: BTreeSet<Vec<&str>> = BTreeSet::new();
        for start in graph.keys().copied() {
            let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
            while let Some((node, next)) = stack.last().copied() {
                let deps = graph.get(node).map_or(&[][..], Vec::as_slice);
                let Some(dep) = deps.get(next).copied() else {
                    stack.pop();
                    continue;
                };
                if let Some(top) = stack.last_mut() {
                    top.1 += 1;
                }
                if let Some(at) = stack.iter().position(|(n, _)| *n == dep) {
                    let mut cycle: Vec<&str> = stack[at..].iter().map(|(n, _)| *n).collect();
                    cycle.sort_unstable();
                    reported.insert(cycle);
                } else if graph.contains_key(dep) && stack.len() <= graph.len() {
                    stack.push((dep, 0));
                }
            }
        }
        let found: Vec<(String, String)> = reported
            .iter()
            .map(|cycle| (cycle[0].to_string(), cycle.join(" → ")))
            .collect();
        for (first, members) in found {
            let Some(p) = self.derived.iter().find(|p| p.path == first) else {
                continue;
            };
            let (file, span) = (p.file.clone(), p.span);
            self.error(diag::EXPR_CYCLE, &file, &format!("{first}.derived"), span, &format!("derived tags depend on each other: {members}"), "Break the cycle: at least one of these tags must be computed from measured tags only.");
        }
    }

    fn check_feedback(&mut self) {
        let missing: Vec<_> = self
            .feedback
            .iter()
            .filter(|(.., tag)| !self.types.contains_key(tag))
            .cloned()
            .collect();
        for (file, path, span, tag) in missing {
            self.error(
                diag::COMMAND,
                &file,
                &path,
                span,
                &format!("feedback tag '{tag}' does not exist"),
                "Name an existing tag by its full path.",
            );
        }
    }
}

#[must_use]
pub fn parse_duration_ms(text: &str) -> Option<u64> {
    let split = text.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let (number, unit) = text.split_at(split);
    let value: f64 = number.parse().ok()?;
    let scale = match unit {
        "ms" => 1.0,
        "s" => 1_000.0,
        "m" => 60_000.0,
        "h" => 3_600_000.0,
        _ => return None,
    };
    (value.is_finite() && value >= 0.0).then_some((value * scale) as u64)
}
