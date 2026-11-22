//! IRON's operations library — what the CLI, the MCP server, and a future
//! language server wrap (`docs/specs/agent-interface.md`).
//!
//! Implemented so far: reading project files and `validate`.

pub mod diag;
pub mod project;
pub mod validate;
pub mod yaml;

pub use diag::{Diagnostic, Report, Severity};
pub use validate::{validate, Files};
