//! graph-specs CLI entry point.
//!
//! Thin shell over [`application::run_check`]. Parses flags, delegates,
//! prints violations one per line, emits a terse summary and exit code.
//!
//! Exit codes:
//! - `0` — zero violations (specs and code agree)
//! - `1` — one or more violations found
//! - `2` — reader error (I/O, parse, or walk failure)

use clap::{Parser, Subcommand};
use domain::{Source, Violation};
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
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { specs, code } => run_check_command(&specs, &code),
    }
}

fn run_check_command(specs: &std::path::Path, code: &std::path::Path) -> ExitCode {
    match application::run_check(specs, code) {
        Ok(violations) if violations.is_empty() => {
            println!("0 violations.");
            ExitCode::SUCCESS
        }
        Ok(violations) => {
            for v in &violations {
                print_violation(v);
            }
            println!("{} violation(s) found.", violations.len());
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("reader error: {e}");
            ExitCode::from(2)
        }
    }
}

fn print_violation(v: &Violation) {
    match v {
        Violation::MissingInCode { name, spec_source } => {
            let (path, line) = source_pair(spec_source);
            println!("missing in code: {name} ({}:{line})", path.display());
        }
        Violation::MissingInSpecs { name, code_source } => {
            let (path, line) = source_pair(code_source);
            println!("missing in specs: {name} ({}:{line})", path.display());
        }
    }
}

fn source_pair(s: &Source) -> (&std::path::Path, usize) {
    match s {
        Source::Spec { path, line } | Source::Code { path, line } => (path.as_path(), *line),
    }
}
