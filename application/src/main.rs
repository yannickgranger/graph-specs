use application::report::ReportFormat;
use clap::{Parser, Subcommand, ValueEnum};
use domain::{CheckOutcome, Violation};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "graph-specs",
    version,
    about = "Graph-based equivalence checker between markdown specifications and source code",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Ndjson,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Run the concept-level equivalence check between specs and code")]
    Check {
        #[arg(
            long,
            help = "Directory walked for markdown specs (e.g., `specs/concepts/`)"
        )]
        specs: PathBuf,
        #[arg(long, help = "Directory walked for Rust source (e.g., `.`)")]
        code: PathBuf,
        #[arg(
            long,
            help = "cfdb keyspace JSON read as the code input instead of walking Rust source"
        )]
        keyspace: Option<PathBuf>,
        #[arg(
            long,
            value_enum,
            default_value_t = Format::Text,
            help = "Output format. Defaults to `text`"
        )]
        format: Format,
    },
    #[command(about = "Generate a verb-coverage (and related) report across specs and code")]
    Report {
        #[arg(
            long,
            help = "Emit the verb-coverage report (pub fn × spec citation matrix)"
        )]
        verb_coverage: bool,
        #[arg(long, help = "Directory walked for markdown specs (e.g., `specs/`)")]
        specs: PathBuf,
        #[arg(long, help = "Directory walked for Rust source (e.g., `.`)")]
        code: PathBuf,
        #[arg(
            long,
            value_enum,
            default_value_t = ReportFormat::Text,
            help = "Output format. Defaults to `text`"
        )]
        format: ReportFormat,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            specs,
            code,
            keyspace,
            format,
        } => run_check_command(&specs, &code, keyspace.as_deref(), format),
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

fn run_check_command(
    specs: &std::path::Path,
    code: &std::path::Path,
    keyspace: Option<&std::path::Path>,
    format: Format,
) -> ExitCode {
    eprintln!(
        "graph-specs: code input read as {}",
        keyspace.map_or_else(
            || format!("source-walk — {}", code.display()),
            |k| format!("keyspace — {}", k.display())
        )
    );
    match application::run_check(specs, code, keyspace) {
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
