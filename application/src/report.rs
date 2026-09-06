use crate::SIGNATURE_NORMALIZERS;
use adapter_markdown::MarkdownReader;
use adapter_rust::{RustLoader, RustReader};
use domain::{report_verb_coverage, CheckInput, VerbOwnership};
use ports::{
    AnnotationReader, CodeLoader, ContextReader, ReaderError, SpecLoader, SpecReader, VerbReader,
};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportFormat {
    Text,
    Ndjson,
}

pub fn run_report(specs: &Path, code: &Path, format: ReportFormat) -> Result<u8, ReaderError> {
    let reader = MarkdownReader::new(SIGNATURE_NORMALIZERS);
    let spec_set = reader.load(specs)?;
    let cache = adapter_rust::parse(code, &RustLoader.load(code)?)?;
    let pub_fns = RustReader::new(cache).extract_pub_fns(code)?;
    let annotations = reader.extract_annotations(&spec_set)?;
    let specs_graph = reader.extract(&spec_set)?;
    let spec_contexts = reader.extract_contexts(&spec_set)?;
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

pub(crate) fn context_key(ctx: Option<&str>) -> (bool, &str) {
    ctx.map_or((true, ""), |s| (false, s))
}
