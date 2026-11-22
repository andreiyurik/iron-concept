//! IRON's operations library: what the CLI and MCP wrap.

pub mod diag;
pub mod model;
pub mod project;
pub mod sim;
pub mod validate;
pub mod yaml;

pub use diag::{Diagnostic, Report, Severity};
pub use validate::{check, validate, Checked, Files};
