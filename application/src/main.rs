//! graph-specs CLI entry point.
//!
//! Thin shell over [`application::run_check`]. Parses flags, delegates,
//! prints violations one per line, emits a terse summary and exit code.
//!
//! Exit codes:
//! - `0` — zero violations (specs and code agree)
//! - `1` — one or more violations found (drift, missing-in-code, missing-in-specs)
//! - `2` — reader error OR any spec-side `SignatureUnparseable`. Both
//!   mean "input can't be parsed" — the author must fix the input before
//!   any equivalence check is meaningful.

use clap::{Parser, Subcommand, ValueEnum};
use domain::Violation;
use std::path::PathBuf;
use std::process::ExitCode;

/// Graph-based equivalence checker between markdown specifications
/// and source code.
#[derive(Debug, Parser)]
#[command(name = "graph-specs", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Output format for `check`. `text` is the human-readable default; `ndjson`
/// emits one JSON object per violation — see `specs/ndjson-output.md` for
/// the schema.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Ndjson,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the concept-level equivalence check between specs and code.
    Check {
        /// Directory walked for markdown specs (e.g., `specs/concepts/`).
        #[arg(long)]
        specs: PathBuf,
        /// Directory walked for Rust source (e.g., `.`).
        #[arg(long)]
        code: PathBuf,
        /// Output format. Defaults to `text`.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            specs,
            code,
            format,
        } => run_check_command(&specs, &code, format),
    }
}

fn run_check_command(specs: &std::path::Path, code: &std::path::Path, format: Format) -> ExitCode {
    match application::run_check(specs, code) {
        Ok(violations) => emit(&violations, format),
        Err(e) => {
            eprintln!("reader error: {e}");
            ExitCode::from(2)
        }
    }
}

fn emit(violations: &[Violation], format: Format) -> ExitCode {
    match format {
        Format::Text => emit_text(violations),
        Format::Ndjson => emit_ndjson(violations),
    }
}

fn emit_text(violations: &[Violation]) -> ExitCode {
    if violations.is_empty() {
        println!("0 violations.");
        return ExitCode::SUCCESS;
    }
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for v in violations {
        if let Err(e) = application::text::format_violation(v, &mut handle) {
            eprintln!("text write error: {e}");
            return ExitCode::from(2);
        }
    }
    drop(handle);
    println!("{} violation(s) found.", violations.len());
    exit_code_for(violations)
}

fn emit_ndjson(violations: &[Violation]) -> ExitCode {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = application::ndjson::write_ndjson(violations, &mut handle) {
        eprintln!("ndjson write error: {e}");
        return ExitCode::from(2);
    }
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        exit_code_for(violations)
    }
}

fn exit_code_for(violations: &[Violation]) -> ExitCode {
    if violations
        .iter()
        .any(|v| matches!(v, Violation::SignatureUnparseable { .. }))
    {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    }
}
