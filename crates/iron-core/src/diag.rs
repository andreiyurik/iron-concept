//! Diagnostics: the one format IRON reports problems in.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    pub file: String,
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<&'static str>,
    pub docs: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub code: &'static str,
    pub requirement: Option<&'static str>,
    pub docs: &'static str,
}

impl Rule {
    /// The code carries the severity: `IRON-W…` is a warning.
    #[must_use]
    pub fn severity(&self) -> Severity {
        if self.code.starts_with("IRON-W") {
            Severity::Warning
        } else {
            Severity::Error
        }
    }
}

macro_rules! rules {
    ($($name:ident = $code:literal, $req:expr, $docs:literal;)*) => {
        $(pub const $name: Rule = Rule { code: $code, requirement: $req, docs: $docs };)*
    };
}

rules! {
    YAML_SYNTAX      = "IRON-E001", None, "specs/config-lifecycle.md";
    YAML_SUBSET      = "IRON-E002", Some("REQ-CFG-005"), "specs/config-lifecycle.md";
    UNKNOWN_FIELD    = "IRON-E003", None, "specs/tag-model.md";
    BAD_VALUE        = "IRON-E004", None, "specs/tag-model.md";
    SOURCE_XOR       = "IRON-E010", Some("REQ-TAG-002"), "specs/tag-model.md";
    UNKNOWN_SCHEME   = "IRON-E011", Some("REQ-TAG-005"), "specs/tag-model.md";
    BAD_ADDRESS      = "IRON-E012", Some("REQ-TAG-005"), "specs/protocol-drivers.md";
    DATASHEET_ADDR   = "IRON-E013", Some("REQ-DRV-009"), "specs/protocol-drivers.md";
    DUPLICATE_TAG    = "IRON-E014", Some("REQ-TAG-006"), "specs/tag-model.md";
    RESERVED_NAME    = "IRON-E015", Some("REQ-TAG-007"), "specs/tag-model.md";
    TARGET_IN_TAG    = "IRON-E016", Some("REQ-RWS-002"), "specs/read-write-separation.md";
    MISSING_RANGE    = "IRON-E017", None, "specs/tag-model.md";
    EXPR_SYNTAX      = "IRON-E020", None, "specs/expressions.md";
    EXPR_TYPE        = "IRON-E021", Some("REQ-EXP-002"), "specs/expressions.md";
    EXPR_IF          = "IRON-E022", Some("REQ-EXP-001"), "specs/expressions.md";
    EXPR_REFERENCE   = "IRON-E023", Some("REQ-EXP-005"), "specs/expressions.md";
    EXPR_CYCLE       = "IRON-E024", Some("REQ-EXP-003"), "specs/expressions.md";
    ALARM_ORDER      = "IRON-E040", Some("REQ-TAG-004"), "specs/alarm-engine.md";
    ALARM_FIELDS     = "IRON-E041", None, "specs/alarm-engine.md";
    ALARM_RANGE      = "IRON-E042", Some("REQ-TAG-003"), "specs/tag-model.md";
    ALARM_ACTION     = "IRON-E043", Some("REQ-ALM-001"), "specs/alarm-engine.md";
    ALARM_NAME       = "IRON-E044", Some("REQ-ALM-004"), "specs/alarm-engine.md";
    COMMAND          = "IRON-E050", None, "specs/command-path.md";
    CONNECTION       = "IRON-E060", None, "specs/protocol-drivers.md";
    NOT_IMPLEMENTED  = "IRON-E900", None, "business/roadmap.md";
    MISSING_UNIT     = "IRON-W001", None, "specs/tag-model.md";
    NOT_VALIDATED    = "IRON-W900", None, "business/roadmap.md";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub tags: usize,
    pub commands: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    #[must_use]
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors() == 0
    }
}
