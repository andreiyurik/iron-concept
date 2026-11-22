//! Diagnostics: the one format IRON reports problems in
//! (`docs/specs/agent-interface.md`, "Diagnostics teach").

use serde::Serialize;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The project is invalid.
    Error,
    /// Worth attention; the project is still valid.
    Warning,
}

/// Where a diagnostic points. Lines and columns are 1-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Location {
    /// File, relative to the project root.
    pub file: String,
    /// Dotted path of the offending element.
    pub path: String,
    /// First line.
    pub line: usize,
    /// First column.
    pub col: usize,
    /// Last line.
    pub end_line: usize,
    /// Column after the last character.
    pub end_col: usize,
}

/// One problem, written to be read by a person or pasted to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable code. Never reused for a different condition.
    pub code: &'static str,
    /// Error or warning.
    pub severity: Severity,
    /// Where.
    pub location: Location,
    /// What is wrong.
    pub message: String,
    /// What to do about it.
    pub hint: String,
    /// The requirement this diagnostic enforces, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<&'static str>,
    /// The spec that defines the rule.
    pub docs: &'static str,
}

/// A rule: its code, the requirement behind it, and its spec. Codes are
/// assigned here once and then frozen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// Stable code.
    pub code: &'static str,
    /// Requirement identifier from `docs/requirements.md`.
    pub requirement: Option<&'static str>,
    /// Spec file.
    pub docs: &'static str,
}

macro_rules! rules {
    ($($name:ident = $code:literal, $req:expr, $docs:literal;)*) => {
        $(#[allow(missing_docs)] pub const $name: Rule = Rule { code: $code, requirement: $req, docs: $docs };)*
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
    COMMAND          = "IRON-E050", None, "specs/command-path.md";
    NOT_IMPLEMENTED  = "IRON-E900", None, "business/roadmap.md";
    MISSING_UNIT     = "IRON-W001", None, "specs/tag-model.md";
    NOT_VALIDATED    = "IRON-W900", None, "business/roadmap.md";
}

/// The outcome of validating a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// How many tags the project defines.
    pub tags: usize,
    /// How many commands the project defines.
    pub commands: usize,
    /// Everything found, ordered by file and position.
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// Number of errors.
    #[must_use]
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// True when the project has no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors() == 0
    }
}
