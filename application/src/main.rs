//! graph-specs CLI entry point.
//!
//! Thin shell over [`application::run_check`]. Parses flags, delegates,
//! prints violations one per line, emits a terse summary and exit code.
//!
//! The summary carries the `pending` and `realized` marker-record counts,
//! and each record is enumerated one per line. The exit code is a function
//! of **violations alone** — a tree whose only findings are marker records
//! exits 0.
//!
//! Exit codes:
//! - `0` — zero violations (specs and code agree)
//! - `1` — one or more violations found (drift, missing-in-code, missing-in-specs)
//! - `2` — reader error OR any spec-side `SignatureUnparseable`. Both
//!   mean "input can't be parsed" — the author must fix the input before
//!   any equivalence check is meaningful.

use application::report::ReportFormat;
use clap::{Parser, Subcommand, ValueEnum};
use domain::{CheckOutcome, Violation};
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
    /// Generate a verb-coverage (and related) report across specs and code.
    Report {
        /// Emit the verb-coverage report (pub fn × spec citation matrix).
        #[arg(long)]
        verb_coverage: bool,
        /// Directory walked for markdown specs (e.g., `specs/`).
        #[arg(long)]
        specs: PathBuf,
        /// Directory walked for Rust source (e.g., `.`).
        #[arg(long)]
        code: PathBuf,
        /// Output format. Defaults to `text`.
        #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
        format: ReportFormat,
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
        Command::Report {
            verb_coverage,
            specs,
            code,
            format,
        } => {
            if !verb_coverage {
                eprintln!(
                    "error: at least one report type must be specified (e.g. --verb-coverage)"
                );
                return ExitCode::from(2);
            }
            match application::report::run_report(&specs, &code, format) {
                Ok(code) => ExitCode::from(code),
                Err(e) => {
                    eprintln!("reader error: {e}");
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn run_check_command(specs: &std::path::Path, code: &std::path::Path, format: Format) -> ExitCode {
    match application::run_check(specs, code) {
        Ok(outcome) => emit(&outcome, format),
        Err(e) => {
            eprintln!("reader error: {e}");
            ExitCode::from(2)
        }
    }
}

fn emit(outcome: &CheckOutcome, format: Format) -> ExitCode {
    match format {
        Format::Text => emit_text(outcome),
        Format::Ndjson => emit_ndjson(outcome),
    }
}

fn emit_text(outcome: &CheckOutcome) -> ExitCode {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = write_text(outcome, &mut handle) {
        eprintln!("text write error: {e}");
        return ExitCode::from(2);
    }
    drop(handle);
    exit_code_for(outcome)
}

fn write_text(outcome: &CheckOutcome, out: &mut impl std::io::Write) -> std::io::Result<()> {
    for v in &outcome.violations {
        application::text::format_violation(v, out)?;
    }
    for p in &outcome.pending {
        application::text::format_pending(p, out)?;
    }
    for r in &outcome.realized {
        application::text::format_realized(r, out)?;
    }
    for r in &outcome.retirement_incomplete {
        application::text::format_retirement_incomplete(r, out)?;
    }
    for r in &outcome.retirement_complete {
        application::text::format_retirement_complete(r, out)?;
    }
    application::text::format_summary(outcome, out)
}

fn emit_ndjson(outcome: &CheckOutcome) -> ExitCode {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if let Err(e) = application::ndjson::write_ndjson(outcome, &mut handle) {
        eprintln!("ndjson write error: {e}");
        return ExitCode::from(2);
    }
    drop(handle);
    exit_code_for(outcome)
}

/// Computed from `violations` only. Marker records never move it.
fn exit_code_for(outcome: &CheckOutcome) -> ExitCode {
    if outcome.violations.is_empty() {
        return ExitCode::SUCCESS;
    }
    if outcome
        .violations
        .iter()
        .any(|v| matches!(v, Violation::SignatureUnparseable { .. }))
    {
        ExitCode::from(2)
    } else {
        ExitCode::from(1)
    }
}
