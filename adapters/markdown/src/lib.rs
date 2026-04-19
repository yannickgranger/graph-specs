//! Markdown spec reader — concept-level.
//!
//! Walks a directory tree, parses each `*.md` file with `pulldown-cmark`,
//! and emits a [`ConceptNode`] for every `##` or `###` heading. Per the
//! dialect spec (`specs/dialect.md`), prose, tables, images, and links
//! are ignored — only `h2`/`h3` heading text, fenced `rust` blocks
//! (v0.2), and recognised bullet prefixes (v0.3) participate.
//!
//! Headings containing generic parameters are normalised: `## Graph<T>`
//! records the concept as `Graph`.
//!
//! ## v0.3 bullet edges
//!
//! Inside a concept section, bullet lines beginning with one of the
//! recognised relationship prefixes are collected as declared edges:
//!
//! - `- implements: <Target>` → [`EdgeKind::Implements`]
//! - `- depends on: <Target>` → [`EdgeKind::DependsOn`]
//! - `- returns: <Target>` → [`EdgeKind::Returns`]
//!
//! Prefix matching is case-sensitive. Bullets that do not match any
//! prefix are prose and are ignored. Empty targets (`- implements:`)
//! are also ignored.

mod contexts;
mod markdown_utils;

use crate::markdown_utils::{compute_line_starts, line_of_offset, path_under_dir};
use domain::{
    tokenise_target, ConceptNode, ContextDecl, Edge, EdgeKind, Graph, SignatureState, Source,
};
use ports::{ContextReader, Reader, ReaderError};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct MarkdownReader;

impl Reader for MarkdownReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // v0.4 layout: when the caller passes `specs/` (a root containing
        // both `concepts/` and `contexts/` subdirs), walk only
        // `concepts/`. This scopes the concept reader away from
        // `contexts/*.md` (different dialect) AND from prose sidecars
        // like `specs/dialect.md` or `specs/ndjson-output.md`. Absence
        // of a `concepts/` subdir preserves v0.3 behaviour — walk the
        // root directly.
        let concepts_subdir = root.join("concepts");
        let walk_root: &Path = if concepts_subdir.is_dir() {
            concepts_subdir.as_path()
        } else {
            root
        };

        for entry in WalkDir::new(walk_root) {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            // Defence in depth: even under the v0.3 fallback above, a
            // nested `contexts/` subtree is owned by the ContextReader.
            if path_under_dir(entry.path(), "contexts") {
                continue;
            }

            let path = entry.path();
            let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;

            extract_from_source(&source, path, &mut nodes, &mut edges);
        }

        Ok(Graph::new(nodes, edges))
    }
}

impl ContextReader for MarkdownReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        contexts::walk_contexts(root)
    }
}

/// Per-file extraction state. Grouping the state into a struct keeps
/// [`extract_from_source`] under the cognitive-complexity ceiling once
/// the v0.3 bullet-edge pass is woven in alongside the existing heading
/// and fenced-block handling.
struct SectionState<'a> {
    line_starts: Vec<usize>,
    path: &'a Path,
    // Heading collection.
    heading_buf: String,
    in_heading_at: Option<usize>,
    // Pending concept: held until the NEXT heading (or EOF) so the
    // accumulated rust blocks for the section can be attached.
    pending: Option<(String, usize)>,
    // Signature collection.
    rust_blocks: Vec<String>,
    in_rust_block: bool,
    block_buf: String,
    // Bullet collection (v0.3).
    in_bullet: Option<usize>,
    bullet_buf: String,
}

impl<'a> SectionState<'a> {
    fn new(source: &str, path: &'a Path) -> Self {
        Self {
            line_starts: compute_line_starts(source),
            path,
            heading_buf: String::new(),
            in_heading_at: None,
            pending: None,
            rust_blocks: Vec::new(),
            in_rust_block: false,
            block_buf: String::new(),
            in_bullet: None,
            bullet_buf: String::new(),
        }
    }

    fn current_concept(&self) -> Option<&str> {
        self.pending.as_ref().map(|(n, _)| n.as_str())
    }
}

fn extract_from_source(
    source: &str,
    path: &Path,
    nodes: &mut Vec<ConceptNode>,
    edges: &mut Vec<Edge>,
) {
    let mut st = SectionState::new(source, path);
    let parser = Parser::new(source).into_offset_iter();

    for (event, range) in parser {
        handle_event(&mut st, event, range, nodes, edges);
    }

    flush_pending(&mut st.pending, &st.rust_blocks, st.path, nodes);
}

fn handle_event(
    st: &mut SectionState,
    event: Event,
    range: std::ops::Range<usize>,
    nodes: &mut Vec<ConceptNode>,
    edges: &mut Vec<Edge>,
) {
    match event {
        Event::Start(Tag::Heading {
            level: HeadingLevel::H2 | HeadingLevel::H3,
            ..
        }) => {
            flush_pending(&mut st.pending, &st.rust_blocks, st.path, nodes);
            st.rust_blocks.clear();
            st.heading_buf.clear();
            st.in_heading_at = Some(line_of_offset(&st.line_starts, range.start));
        }
        Event::End(TagEnd::Heading(HeadingLevel::H2 | HeadingLevel::H3)) => {
            if let Some(line) = st.in_heading_at.take() {
                let name = normalize_heading(&st.heading_buf);
                if !name.is_empty() {
                    st.pending = Some((name, line));
                }
            }
        }
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
            if st.pending.is_some() && lang.as_ref() == "rust" =>
        {
            st.in_rust_block = true;
            st.block_buf.clear();
        }
        Event::End(TagEnd::CodeBlock) if st.in_rust_block => {
            st.rust_blocks.push(std::mem::take(&mut st.block_buf));
            st.in_rust_block = false;
        }
        Event::Start(Tag::Item) if st.pending.is_some() => {
            st.in_bullet = Some(line_of_offset(&st.line_starts, range.start));
            st.bullet_buf.clear();
        }
        Event::End(TagEnd::Item) if st.in_bullet.is_some() => {
            if let Some(line) = st.in_bullet.take() {
                finish_bullet(st, line, edges);
            }
        }
        Event::Text(s) | Event::Code(s) => absorb_text(st, &s),
        _ => {}
    }
}

fn absorb_text(st: &mut SectionState, s: &str) {
    if st.in_heading_at.is_some() {
        st.heading_buf.push_str(s);
    } else if st.in_rust_block {
        st.block_buf.push_str(s);
    } else if st.in_bullet.is_some() {
        st.bullet_buf.push_str(s);
    }
}

fn finish_bullet(st: &mut SectionState, line: usize, edges: &mut Vec<Edge>) {
    let Some(concept) = st.current_concept().map(str::to_owned) else {
        st.bullet_buf.clear();
        return;
    };
    let text = std::mem::take(&mut st.bullet_buf);
    if let Some((kind, token, raw)) = parse_bullet_edge(text.as_str()) {
        edges.push(Edge {
            source_concept: concept,
            kind,
            target: token,
            raw_target: raw,
            source: Source::Spec {
                path: st.path.to_path_buf(),
                line,
            },
        });
    }
}

const BULLET_PREFIXES: &[(&str, EdgeKind)] = &[
    ("implements:", EdgeKind::Implements),
    ("depends on:", EdgeKind::DependsOn),
    ("returns:", EdgeKind::Returns),
];

/// Parse a bullet's accumulated text into an (`EdgeKind`, tokenised, raw)
/// triple, if it matches a recognised prefix. Returns `None` for prose
/// bullets and for recognised prefixes with an empty target.
fn parse_bullet_edge(text: &str) -> Option<(EdgeKind, String, String)> {
    let trimmed = text.trim();
    for (prefix, kind) in BULLET_PREFIXES {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let raw = rest.trim().to_string();
            if raw.is_empty() {
                return None;
            }
            let token = tokenise_target(&raw);
            return Some((*kind, token, raw));
        }
    }
    None
}

fn flush_pending(
    pending: &mut Option<(String, usize)>,
    rust_blocks: &[String],
    path: &Path,
    out: &mut Vec<ConceptNode>,
) {
    if let Some((name, line)) = pending.take() {
        out.push(ConceptNode {
            name,
            source: Source::Spec {
                path: path.to_path_buf(),
                line,
            },
            signature: signature_from_blocks(rust_blocks),
        });
    }
}

fn signature_from_blocks(blocks: &[String]) -> SignatureState {
    match blocks {
        [] => SignatureState::Absent,
        [only] => parse_single_block(only),
        many => {
            let count = many.len();
            SignatureState::Unparseable {
                raw: many.join("\n---\n"),
                error: format!(
                    "concept section contains {count} fenced rust blocks; at most one is allowed"
                ),
            }
        }
    }
}

fn parse_single_block(raw: &str) -> SignatureState {
    match syn::parse_str::<syn::Item>(raw) {
        Ok(item) => SignatureState::Normalized(adapter_rust::normalize(&item)),
        Err(e) => SignatureState::Unparseable {
            raw: raw.to_string(),
            error: e.to_string(),
        },
    }
}

/// Normalise a heading's collected text into a concept name.
/// Strips generics (`Foo<T>` → `Foo`) and trims whitespace.
fn normalize_heading(raw: &str) -> String {
    let trimmed = raw.trim();
    trimmed
        .find('<')
        .map_or_else(|| trimmed.to_string(), |i| trimmed[..i].trim().to_string())
}

#[cfg(test)]
mod tests;
