//! Parser for `specs/contexts/<name>.md` files — v0.4 bounded-context
//! declarations.
//!
//! Deliberately NOT sharing `SectionState` with the concept parser — the
//! two formats don't overlap structurally (H1 name + four H2 sections
//! with flat list syntax vs H2/H3 + fenced rust + bullet-edge syntax).
//! Only the line-offset helpers are shared via [`crate::markdown_utils`].

use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{
    detect_import_cycle, ContextDecl, ContextExport, ContextImport, ContextPattern, OwnedUnit,
    Source,
};
use ports::ReaderError;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

/// Four top-level sections we recognise in a context file. Any other
/// `## Heading` is treated as prose and ignored (e.g. the `Concepts`
/// section — it exists for the human reader, not the parser).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Owns,
    Exports,
    Imports,
    Other,
}

struct State<'a> {
    path: &'a Path,
    line_starts: Vec<usize>,
    ctx_name: Option<(String, usize)>,
    owned_units: Vec<OwnedUnit>,
    exports: Vec<ContextExport>,
    imports: Vec<ContextImport>,
    section: Section,
    heading: Option<HeadingLevel>,
    heading_buf: String,
    item_depth: u32,
    in_top_item_at: Option<usize>,
    item_buf: String,
    error: Option<ReaderError>,
}

impl<'a> State<'a> {
    fn new(path: &'a Path, source: &str) -> Self {
        Self {
            path,
            line_starts: compute_line_starts(source),
            ctx_name: None,
            owned_units: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            section: Section::None,
            heading: None,
            heading_buf: String::new(),
            item_depth: 0,
            in_top_item_at: None,
            item_buf: String::new(),
            error: None,
        }
    }
}

/// Parse a single context-declaration markdown file.
///
/// # Errors
///
/// Returns [`ReaderError::ParseFailed`] if the file is missing an H1
/// name, contains multiple H1 headings, uses an unknown
/// [`ContextPattern`] token, or a bullet in a recognised section does
/// not match the expected shape (see module-level docs for the shapes).
pub fn parse_context_file(path: &Path, source: &str) -> Result<ContextDecl, ReaderError> {
    let mut st = State::new(path, source);
    let parser = Parser::new(source).into_offset_iter();
    for (event, range) in parser {
        handle_event(&mut st, event, range);
        if st.error.is_some() {
            break;
        }
    }
    if let Some(err) = st.error {
        return Err(err);
    }
    let Some((name, h1_line)) = st.ctx_name else {
        return Err(parse_err(
            path,
            1,
            "context file must open with a single `# ContextName` heading",
        ));
    };
    Ok(ContextDecl::new(
        name,
        st.owned_units,
        st.exports,
        st.imports,
        Source::Spec {
            path: path.to_path_buf(),
            line: h1_line,
        },
    ))
}

fn handle_event(st: &mut State, event: Event, range: std::ops::Range<usize>) {
    match event {
        Event::Start(Tag::Heading {
            level: HeadingLevel::H1,
            ..
        }) => {
            if st.ctx_name.is_some() {
                st.error = Some(parse_err(
                    st.path,
                    line_of_offset(&st.line_starts, range.start),
                    "context file must contain exactly one `# Heading` line",
                ));
                return;
            }
            st.heading = Some(HeadingLevel::H1);
            st.heading_buf.clear();
        }
        Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
            let name = st.heading_buf.trim().to_string();
            let line = line_of_offset(&st.line_starts, range.start);
            st.heading = None;
            if name.is_empty() {
                st.error = Some(parse_err(st.path, line, "H1 heading text is empty"));
                return;
            }
            st.ctx_name = Some((name, line));
        }
        Event::Start(Tag::Heading {
            level: HeadingLevel::H2,
            ..
        }) => {
            st.heading = Some(HeadingLevel::H2);
            st.heading_buf.clear();
        }
        Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
            st.heading = None;
            st.section = classify_section(&st.heading_buf);
        }
        Event::Start(Tag::Item) => {
            st.item_depth += 1;
            if st.item_depth == 1 {
                st.in_top_item_at = Some(line_of_offset(&st.line_starts, range.start));
                st.item_buf.clear();
            }
        }
        Event::End(TagEnd::Item) => {
            if st.item_depth == 1 {
                if let Some(line) = st.in_top_item_at.take() {
                    finish_item(st, line);
                }
            }
            st.item_depth = st.item_depth.saturating_sub(1);
        }
        Event::Text(s) | Event::Code(s) => {
            if st.heading.is_some() {
                st.heading_buf.push_str(&s);
            } else if st.item_depth == 1 {
                st.item_buf.push_str(&s);
            }
        }
        _ => {}
    }
}

fn finish_item(st: &mut State, line: usize) {
    let text = std::mem::take(&mut st.item_buf);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    match st.section {
        Section::Owns => st.owned_units.push(OwnedUnit(trimmed.to_string())),
        Section::Exports => match parse_export(trimmed, st.path, line) {
            Ok(e) => st.exports.push(e),
            Err(err) => st.error = Some(err),
        },
        Section::Imports => match parse_import(trimmed, st.path, line) {
            Ok(i) => st.imports.push(i),
            Err(err) => st.error = Some(err),
        },
        Section::None | Section::Other => {}
    }
}

fn classify_section(heading: &str) -> Section {
    // Headings may have annotation like "Exports (Published Language — …)".
    // Match on the first word, case-sensitive per RFC.
    let first = heading.split_whitespace().next().unwrap_or("");
    match first {
        "Owns" => Section::Owns,
        "Exports" => Section::Exports,
        "Imports" => Section::Imports,
        _ => Section::Other,
    }
}

/// Export bullet: `<Concept> (<Pattern>)` — e.g. `Graph (PublishedLanguage)`.
fn parse_export(text: &str, path: &Path, line: usize) -> Result<ContextExport, ReaderError> {
    let (concept, pattern_raw) = split_paren(text)
        .ok_or_else(|| parse_err(path, line, "Exports bullet must be `<Concept> (<Pattern>)`"))?;
    let pattern = parse_pattern(pattern_raw, path, line)?;
    Ok(ContextExport {
        concept: concept.to_string(),
        pattern,
    })
}

/// Import bullet: `<Concept> from <Context> (<Pattern>)`.
fn parse_import(text: &str, path: &Path, line: usize) -> Result<ContextImport, ReaderError> {
    let (prefix, pattern_raw) = split_paren(text).ok_or_else(|| {
        parse_err(
            path,
            line,
            "Imports bullet must be `<Concept> from <Context> (<Pattern>)`",
        )
    })?;
    let mut parts = prefix.splitn(3, ' ');
    let concept = parts.next().unwrap_or("").trim();
    let from_kw = parts.next().unwrap_or("").trim();
    let from_context = parts.next().unwrap_or("").trim();
    if concept.is_empty() || from_kw != "from" || from_context.is_empty() {
        return Err(parse_err(
            path,
            line,
            "Imports bullet must be `<Concept> from <Context> (<Pattern>)`",
        ));
    }
    let pattern = parse_pattern(pattern_raw, path, line)?;
    Ok(ContextImport {
        from_context: from_context.to_string(),
        pattern,
        concept: concept.to_string(),
    })
}

fn parse_pattern(raw: &str, path: &Path, line: usize) -> Result<ContextPattern, ReaderError> {
    let trimmed = raw.trim();
    for v in ContextPattern::variants() {
        if v.as_label() == trimmed {
            return Ok(*v);
        }
    }
    let known: Vec<&str> = ContextPattern::variants()
        .iter()
        .map(|v| v.as_label())
        .collect();
    Err(parse_err(
        path,
        line,
        &format!(
            "unknown ContextPattern `{trimmed}` — expected one of {}",
            known.join(", ")
        ),
    ))
}

/// Split `"Foo (Bar)"` into `("Foo", "Bar")`.
fn split_paren(text: &str) -> Option<(&str, &str)> {
    let open = text.rfind('(')?;
    let close = text.rfind(')')?;
    if close <= open {
        return None;
    }
    let prefix = text[..open].trim();
    let inner = text[open + 1..close].trim();
    if prefix.is_empty() || inner.is_empty() {
        return None;
    }
    Some((prefix, inner))
}

fn parse_err(path: &Path, line: usize, message: &str) -> ReaderError {
    ReaderError::ParseFailed {
        path: path.to_path_buf(),
        line,
        message: message.to_string(),
    }
}

/// Walk `root` for `*.md` files and parse each as a context declaration.
/// Output is sorted by path for deterministic downstream ordering. Missing
/// root yields `Ok(Vec::new())` — v0.3 spec trees have no `specs/contexts/`.
pub fn walk_contexts(root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(root).sort_by_file_name();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e)
                if e.io_error()
                    .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound) =>
            {
                return Ok(out);
            }
            Err(e) => {
                return Err(ReaderError::WalkFailed {
                    root: root.to_path_buf(),
                    cause: e.to_string(),
                })
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let p = entry.path();
        if p.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let source = std::fs::read_to_string(p).map_err(|e| ReaderError::IoFailed {
            path: p.to_path_buf(),
            cause: e.to_string(),
        })?;
        out.push(parse_context_file(p, &source)?);
    }
    // Invariant 7: cyclic import declarations are a reader error
    // (SharedKernel is the one legal form of mutual reference).
    if let Some(cycle) = detect_import_cycle(&out) {
        return Err(ReaderError::ParseFailed {
            path: root.to_path_buf(),
            line: 0,
            message: format!("cyclic import declarations detected: {}", cycle.join(" → ")),
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "contexts_tests.rs"]
mod tests;
