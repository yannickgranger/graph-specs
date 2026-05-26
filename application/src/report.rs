//! Application-layer orchestration for `graph-specs report`.
//!
//! Thin glue between the readers (markdown + Rust), the pure
//! `report_verb_coverage` transformation in [`domain`], and the two
//! output emitters. Parallel to [`crate::run_check`] for the `check`
//! subcommand.

use adapter_markdown::MarkdownReader;
use adapter_rust::RustReader;
use domain::{report_verb_coverage, CheckInput, VerbOwnership};
use ports::{ContextReader, Reader, ReaderError, VerbReader};
use std::io;
use std::path::{Path, PathBuf};

/// Output format for `graph-specs report`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportFormat {
    Text,
    Ndjson,
}

/// Run the verb-coverage report subcommand.
///
/// Reads spec annotations and pub fn declarations from the given trees,
/// calls [`domain::report_verb_coverage`], then emits the result in the
/// requested format to stdout.
///
/// # Errors
///
/// Propagates any [`ReaderError`] from the underlying markdown or Rust
/// reader. I/O errors from writing to stdout are mapped to
/// [`ReaderError::IoFailed`].
pub fn run_report(specs: &Path, code: &Path, format: ReportFormat) -> Result<u8, ReaderError> {
    let pub_fns = RustReader.extract_pub_fns(code)?;
    let annotations = MarkdownReader.extract_invariant_annotations(specs)?;
    let specs_graph = MarkdownReader.extract(specs)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs)?;
    let check_input = CheckInput::new(specs_graph, spec_contexts, VerbOwnership::default());
    let report = report_verb_coverage(&check_input, &pub_fns, &annotations);

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let io_result = match format {
        ReportFormat::Text => crate::report_text::emit_text(&mut handle, &report),
        ReportFormat::Ndjson => crate::report_ndjson::emit_ndjson(&mut handle, &report),
    };
    io_result.map_err(|e| ReaderError::IoFailed {
        path: PathBuf::from("<stdout>"),
        cause: e.to_string(),
    })?;
    Ok(0_u8)
}
