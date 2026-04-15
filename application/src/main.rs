//! graph-specs CLI entry point — scaffolding stub.
//!
//! `--help` produces usage so issue #1 AC (`cargo run -p application --
//! --help` exits 0) is satisfied. Subcommand dispatch and reader wiring
//! land in follow-up issues.

use clap::{Parser, Subcommand};

/// Graph-based equivalence checker between markdown specifications
/// and source code.
#[derive(Debug, Parser)]
#[command(name = "graph-specs", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the four-level equivalence check (not yet implemented).
    Check,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("graph-specs: scaffolding only — no logic implemented yet");
            println!("Run with --help for usage.");
        }
        Some(Command::Check) => {
            eprintln!("error: `check` not yet implemented (scaffolding stage)");
            std::process::exit(1);
        }
    }
}
