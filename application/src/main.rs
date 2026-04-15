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
            if violations
                .iter()
                .any(|v| matches!(v, Violation::SignatureUnparseable { .. }))
            {
                ExitCode::from(2)
            } else {
                ExitCode::from(1)
            }
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
        Violation::SignatureDrift {
            name,
            spec_sig,
            code_sig,
            spec_source,
            code_source,
        } => {
            let (spec_path, spec_line) = source_pair(spec_source);
            let (code_path, code_line) = source_pair(code_source);
            println!(
                "signature drift: {name}\n  spec ({}:{spec_line}): {spec_sig}\n  code ({}:{code_line}): {code_sig}",
                spec_path.display(),
                code_path.display()
            );
        }
        Violation::SignatureMissingInSpec {
            name,
            code_sig,
            code_source,
        } => {
            let (path, line) = source_pair(code_source);
            println!(
                "signature missing in spec: {name} ({}:{line})\n  code: {code_sig}",
                path.display()
            );
        }
        Violation::SignatureUnparseable {
            name,
            raw,
            error,
            source,
        } => {
            let (path, line) = source_pair(source);
            println!(
                "signature unparseable: {name} ({}:{line})\n  raw: {raw}\n  error: {error}",
                path.display()
            );
        }
        Violation::EdgeMissingInCode {
            concept,
            edge_kind,
            target,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            println!(
                "edge missing in code: {concept} --{edge_kind}--> {target} ({}:{line})",
                path.display()
            );
        }
        Violation::EdgeMissingInSpec {
            concept,
            edge_kind,
            target,
            code_source,
        } => {
            let (path, line) = source_pair(code_source);
            println!(
                "edge missing in spec: {concept} --{edge_kind}--> {target} ({}:{line})",
                path.display()
            );
        }
        Violation::EdgeTargetUnknown {
            concept,
            edge_kind,
            target,
            spec_source,
        } => {
            let (path, line) = source_pair(spec_source);
            println!(
                "edge target unknown: {concept} --{edge_kind}--> {target} (not a concept in either graph) ({}:{line})",
                path.display()
            );
        }
    }
}

fn source_pair(s: &Source) -> (&std::path::Path, usize) {
    match s {
        Source::Spec { path, line } | Source::Code { path, line } => (path.as_path(), *line),
    }
}
