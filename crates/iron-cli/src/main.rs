//! The `iron` command: a thin wrapper over iron-core. Exit codes 0 / 1 / 2.

use iron_core::{Report, Severity};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
iron — industrial automation as text in Git

Usage:
  iron validate [DIR] [--json]   check every spec against its invariants
  iron test --sim [DIR] [--json] run every simulation scenario
  iron version                   print the version
  iron help                      print this text

DIR defaults to the current directory. Other commands in docs/specs/cli.md
are not implemented yet.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") => validate(&args[1..]),
        Some("test") => test(&args[1..]),
        Some("version" | "--version" | "-V") => {
            println!("iron {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => usage_error(&format!("unknown command '{other}'")),
        None => usage_error("no command given"),
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("iron: {message}\n\n{USAGE}");
    ExitCode::from(2)
}

fn validate(args: &[String]) -> ExitCode {
    let mut json = false;
    let mut dir: Option<PathBuf> = None;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            flag if flag.starts_with('-') => {
                return usage_error(&format!("unknown option '{flag}'"))
            }
            path if dir.is_none() => dir = Some(PathBuf::from(path)),
            _ => return usage_error("validate takes one directory"),
        }
    }
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    let files = match iron_core::project::load(&dir) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("iron: {e}");
            return ExitCode::from(2);
        }
    };
    let report = iron_core::validate(&files);
    if json {
        let mut doc = serde_json::to_value(&report).unwrap_or_default();
        doc["status"] = if report.is_valid() { "ok" } else { "invalid" }.into();
        println!("{doc:#}");
    } else {
        print_human(&report);
    }
    if report.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn print_human(report: &Report) {
    for d in &report.diagnostics {
        let mark = if d.severity == Severity::Error {
            "❌"
        } else {
            "⚠️ "
        };
        let l = &d.location;
        let subject = if l.path.is_empty() {
            String::new()
        } else {
            format!("  {}", l.path)
        };
        println!("{mark} {}:{}:{}{subject}", l.file, l.line, l.col);
        let rule = d
            .requirement
            .map_or_else(|| d.code.to_string(), |r| format!("{} · {r}", d.code));
        println!("   {}  [{rule}]", d.message);
        println!("   hint: {}", d.hint);
    }
    let errors = report.errors();
    let warnings = report.diagnostics.len() - errors;
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let counts = format!(
        "{}, {}",
        plural(report.tags, "tag"),
        plural(report.commands, "command")
    );
    if errors == 0 && warnings == 0 {
        println!("✅ {counts} — valid");
    } else if errors == 0 {
        println!("✅ {counts} — valid, {}", plural(warnings, "warning"));
    } else {
        println!(
            "{counts} — {}, {}",
            plural(errors, "error"),
            plural(warnings, "warning")
        );
    }
}

fn test(args: &[String]) -> ExitCode {
    let (mut json, mut sim) = (false, false);
    let mut dir: Option<PathBuf> = None;
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--sim" => sim = true,
            flag if flag.starts_with('-') => {
                return usage_error(&format!("unknown option '{flag}'"))
            }
            path if dir.is_none() => dir = Some(PathBuf::from(path)),
            _ => return usage_error("test takes one directory"),
        }
    }
    if !sim {
        return usage_error("only `iron test --sim` is implemented");
    }
    let files = match iron_core::project::load(&dir.unwrap_or_else(|| PathBuf::from("."))) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("iron: {e}");
            return ExitCode::from(2);
        }
    };
    let report = iron_core::validate(&files);
    if !report.is_valid() {
        if json {
            let mut doc = serde_json::to_value(&report).unwrap_or_default();
            doc["status"] = "invalid".into();
            println!("{doc:#}");
        } else {
            print_human(&report);
            println!("scenarios not run: the project is invalid");
        }
        return ExitCode::from(1);
    }
    let outcomes = iron_core::sim::run_all(&files, &iron_core::model::build(&files));
    let failed = outcomes.iter().filter(|o| !o.passed).count();
    if json {
        let status = if failed == 0 { "ok" } else { "failed" };
        println!(
            "{:#}",
            serde_json::json!({ "status": status, "scenarios": outcomes })
        );
    } else {
        for o in &outcomes {
            println!("{} {}", if o.passed { "✅" } else { "❌" }, o.name);
            for f in &o.failures {
                println!("   {f}");
            }
        }
        match (outcomes.len(), failed) {
            (0, _) => println!("no scenarios in test/sim/scenarios/"),
            (n, 0) => println!("{n} passed"),
            (n, f) => println!("{} passed, {f} failed", n - f),
        }
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
