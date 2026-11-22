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
    let run = |command: fn(&Args) -> Result<bool, String>, flags: &[&str]| match Args::parse(&args[1..], flags).and_then(|a| command(&a)) {
        Ok(passed) => ExitCode::from(u8::from(!passed)),
        Err(message) => {
            eprintln!("iron: {message}");
            ExitCode::from(2)
        }
    };
    match args.first().map(String::as_str) {
        Some("validate") => run(validate, &["--json"]),
        Some("test") => run(test, &["--json", "--sim"]),
        Some("version" | "--version" | "-V") => {
            println!("iron {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("iron: {}\n\n{USAGE}", other.map_or("no command given".into(), |c| format!("unknown command '{c}'")));
            ExitCode::from(2)
        }
    }
}

struct Args {
    flags: Vec<String>,
    dir: PathBuf,
}

impl Args {
    fn parse(args: &[String], known: &[&str]) -> Result<Args, String> {
        let (flags, dirs): (Vec<String>, Vec<String>) = args.iter().cloned().partition(|a| a.starts_with('-'));
        if let Some(unknown) = flags.iter().find(|f| !known.contains(&f.as_str())) {
            return Err(format!("unknown option '{unknown}'"));
        }
        match dirs.as_slice() {
            [] => Ok(Args { flags, dir: ".".into() }),
            [dir] => Ok(Args { flags, dir: dir.into() }),
            _ => Err("expected at most one directory".into()),
        }
    }

    fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    fn files(&self) -> Result<iron_core::Files, String> {
        iron_core::project::load(&self.dir).map_err(|e| e.to_string())
    }
}

fn validate(args: &Args) -> Result<bool, String> {
    let report = iron_core::validate(&args.files()?);
    print_report(&report, args.has("--json"));
    Ok(report.is_valid())
}

fn test(args: &Args) -> Result<bool, String> {
    if !args.has("--sim") {
        return Err("only `iron test --sim` is implemented".into());
    }
    let json = args.has("--json");
    let outcomes = match iron_core::sim::test(&args.files()?) {
        Ok(outcomes) => outcomes,
        Err(report) => {
            print_report(&report, json);
            if !json {
                println!("scenarios not run: the project is invalid");
            }
            return Ok(false);
        }
    };
    let failed = outcomes.iter().filter(|o| !o.passed).count();
    if json {
        let status = if failed == 0 { "ok" } else { "failed" };
        println!("{:#}", serde_json::json!({ "status": status, "scenarios": outcomes }));
        return Ok(failed == 0);
    }
    for o in &outcomes {
        println!("{} {}", if o.passed { "✅" } else { "❌" }, o.name);
        o.failures.iter().for_each(|f| println!("   {f}"));
    }
    match (outcomes.len(), failed) {
        (0, _) => println!("no scenarios in test/sim/scenarios/"),
        (n, 0) => println!("{n} passed"),
        (n, f) => println!("{} passed, {f} failed", n - f),
    }
    Ok(failed == 0)
}

fn print_report(report: &Report, json: bool) {
    if json {
        let mut doc = serde_json::to_value(report).unwrap_or_default();
        doc["status"] = if report.is_valid() { "ok" } else { "invalid" }.into();
        println!("{doc:#}");
        return;
    }
    for d in &report.diagnostics {
        let mark = if d.severity == Severity::Error { "❌" } else { "⚠️ " };
        let l = &d.location;
        let subject = if l.path.is_empty() { String::new() } else { format!("  {}", l.path) };
        println!("{mark} {}:{}:{}{subject}", l.file, l.line, l.col);
        let rule = d.requirement.map_or_else(|| d.code.to_string(), |r| format!("{} · {r}", d.code));
        println!("   {}  [{rule}]\n   hint: {}", d.message, d.hint);
    }
    let errors = report.errors();
    let warnings = report.diagnostics.len() - errors;
    let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let counts = format!("{}, {}", plural(report.tags, "tag"), plural(report.commands, "command"));
    match (errors, warnings) {
        (0, 0) => println!("✅ {counts} — valid"),
        (0, w) => println!("✅ {counts} — valid, {}", plural(w, "warning")),
        (e, w) => println!("{counts} — {}, {}", plural(e, "error"), plural(w, "warning")),
    }
}
