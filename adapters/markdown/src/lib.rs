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
    tokenise_target, ConceptNode, ContextDecl, Edge, EdgeKind, Graph, InvariantAnnotation,
    SignatureState, Source, TierKind, VerbAnchor,
};
use ports::{ContextReader, Reader, ReaderError};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;
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

            // Pre-authored draft specs declare concepts ahead of their
            // code; skip the file so its not-yet-implemented surface emits
            // no violation. See [`is_draft`].
            if is_draft(&source) {
                continue;
            }

            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            extract_from_source(
                &source,
                path,
                &mut nodes,
                &mut edges,
                &mut verb_anchors_scratch,
            );
        }

        Ok(Graph::new(nodes, edges))
    }
}

impl ContextReader for MarkdownReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        contexts::walk_contexts(root)
    }
}

impl MarkdownReader {
    /// Walk `root` and collect every `- verb: <ident>` anchor from concept
    /// spec files (`concepts/` subdir if present, else root). Skips
    /// `contexts/` files (different dialect).
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] or [`ReaderError::WalkFailed`] on
    /// I/O failures.
    pub fn extract_verb_anchors(&self, root: &Path) -> Result<Vec<VerbAnchor>, ReaderError> {
        let mut verb_anchors: Vec<VerbAnchor> = Vec::new();

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
            if path_under_dir(entry.path(), "contexts") {
                continue;
            }

            let path = entry.path();
            let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;

            // Draft specs are skipped wholesale — see [`is_draft`] and the
            // matching guard in `extract`.
            if is_draft(&source) {
                continue;
            }

            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            extract_from_source(
                &source,
                path,
                &mut nodes_scratch,
                &mut edges_scratch,
                &mut verb_anchors,
            );
        }

        Ok(verb_anchors)
    }

    /// Extract all `[enforced-by:]` / `[prose-only:]` bracketed annotations
    /// from `#### Operational invariants` sections in spec files under `root`.
    ///
    /// Per RFC-005 §3.2: uses a **fresh** `Parser::new(source).into_offset_iter()`
    /// per file — NOT shared with the concept walk. The existing `handle_event`
    /// (H2/H3-only) is NOT extended; this method's parser loop introduces its
    /// own `HeadingLevel::H4` arm.
    ///
    /// **Failure mode (Invariant 7):** a bullet that looks like an annotation
    /// (contains `[enforced-by:` or `[prose-only:`) but fails the bracket
    /// grammar emits `tracing::warn!` and is dropped from the returned `Vec`.
    /// `Err` is reserved for I/O / fundamental parse failures only.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] or [`ReaderError::WalkFailed`] on
    /// I/O failures. Grammar errors are tolerated per Invariant 7.
    pub fn extract_invariant_annotations(
        &self,
        root: &Path,
    ) -> Result<Vec<InvariantAnnotation>, ReaderError> {
        let mut result = Vec::new();

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
            if path_under_dir(entry.path(), "contexts") {
                continue;
            }

            let path = entry.path();
            let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;

            // Draft specs are skipped wholesale — see [`is_draft`] and the
            // matching guard in `extract`.
            if is_draft(&source) {
                continue;
            }

            extract_annotations_from_source(&source, path, &mut result);
        }

        Ok(result)
    }
}

/// Returns `true` when `source` opens with a YAML front-matter block
/// (delimited by lines containing only `---`) that declares
/// `status: draft`.
///
/// A draft spec is **pre-authored ahead of its code**: its concepts,
/// verb anchors, and invariant annotations intentionally have no
/// implementation yet, so every concept-walking reader skips the file
/// and it contributes no nodes, edges, or anchors to the spec graph —
/// no `missing in code` violation can arise from it. Removing or
/// changing the `status:` line re-arms the file: its declarations then
/// resolve against code like any other spec.
///
/// Only the leading front-matter block is consulted — a `status:` line
/// in the prose body has no effect. The value is matched
/// case-insensitively, with or without surrounding quotes, and any
/// trailing `#` comment is ignored. A front-matter block that closes
/// before any `status:` line, or a file with no front-matter at all,
/// is not draft.
fn is_draft(source: &str) -> bool {
    // The opening fence must be the first non-empty line.
    let mut lines = source.lines().skip_while(|l| l.trim().is_empty());
    if lines.next().map(str::trim) != Some("---") {
        return false;
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            return false;
        }
        if let Some(rest) = trimmed.strip_prefix("status:") {
            let value = rest
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            return value.eq_ignore_ascii_case("draft");
        }
    }
    false
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
    verb_anchors: &mut Vec<VerbAnchor>,
) {
    let mut st = SectionState::new(source, path);
    let parser = Parser::new(source).into_offset_iter();

    for (event, range) in parser {
        handle_event(&mut st, event, range, nodes, edges, verb_anchors);
    }

    flush_pending(&mut st.pending, &st.rust_blocks, st.path, nodes);
}

fn handle_event(
    st: &mut SectionState,
    event: Event,
    range: std::ops::Range<usize>,
    nodes: &mut Vec<ConceptNode>,
    edges: &mut Vec<Edge>,
    verb_anchors: &mut Vec<VerbAnchor>,
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
                finish_bullet(st, line, edges, verb_anchors);
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

fn finish_bullet(
    st: &mut SectionState,
    line: usize,
    edges: &mut Vec<Edge>,
    verb_anchors: &mut Vec<VerbAnchor>,
) {
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
    } else if let Some(mut anchor) = parse_verb_bullet(text.as_str()) {
        anchor.concept = concept;
        anchor.source = Source::Spec {
            path: st.path.to_path_buf(),
            line,
        };
        verb_anchors.push(anchor);
    }
}

const BULLET_PREFIXES: &[(&str, EdgeKind)] = &[
    ("implements:", EdgeKind::Implements),
    ("depends on:", EdgeKind::DependsOn),
    ("returns:", EdgeKind::Returns),
];

/// Compiled regex for validating verb-bullet qnames (v0.6).
///
/// Accepts bare identifiers (`foo`) and `Type::method` two-segment qnames.
/// Rejects multi-segment paths, leading or trailing `::`, and non-ident chars.
static VERB_QNAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$").expect("valid regex")
});

/// Parse a `- verb: <qname>` bullet into a [`VerbAnchor`] with placeholder fields.
///
/// The `concept` and `source` fields are left as placeholders; the caller
/// (`finish_bullet`) fills them in once the concept name and file location
/// are available.
///
/// Returns `None` for bullets that do not start with `verb:` or whose qname
/// does not satisfy `^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)?$`.
/// A non-empty, non-matching qname emits `tracing::warn!` before the skip
/// (tolerant-skip).
pub fn parse_verb_bullet(text: &str) -> Option<VerbAnchor> {
    let trimmed = text.trim();
    let rest = trimmed.strip_prefix("verb:")?;
    let qname = rest.trim();
    if qname.is_empty() {
        return None;
    }
    if !VERB_QNAME_RE.is_match(qname) {
        tracing::warn!("verb bullet has malformed qname — skipping: {qname:?}");
        return None;
    }
    Some(VerbAnchor {
        concept: String::new(),
        qname: qname.to_owned(),
        raw_target: trimmed.to_owned(),
        source: Source::Spec {
            path: std::path::PathBuf::new(),
            line: 0,
        },
    })
}

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

/// Parse a single spec file for `#### Operational invariants` sections,
/// extracting all well-formed bracketed annotations from bullet items.
/// Per RFC-005 §3.2: fresh parser per file, own H4 arm, new bracket grammar.
fn extract_annotations_from_source(source: &str, path: &Path, out: &mut Vec<InvariantAnnotation>) {
    let line_starts = compute_line_starts(source);
    let parser = Parser::new(source).into_offset_iter();

    let mut in_op_invariants = false;
    let mut in_h4 = false;
    let mut h4_buf = String::new();
    let mut in_bullet = false;
    let mut bullet_buf = String::new();
    let mut bullet_line = 0usize;

    for (event, range) in parser {
        match event {
            // Higher-level headings reset the invariants section.
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3,
                ..
            }) => {
                in_op_invariants = false;
            }
            Event::Start(Tag::Heading {
                level: HeadingLevel::H4,
                ..
            }) => {
                in_h4 = true;
                h4_buf.clear();
            }
            Event::End(TagEnd::Heading(HeadingLevel::H4)) => {
                if in_h4 {
                    in_op_invariants = h4_buf.trim() == "Operational invariants";
                    in_h4 = false;
                }
            }
            Event::Start(Tag::Item) if in_op_invariants => {
                in_bullet = true;
                bullet_line = line_of_offset(&line_starts, range.start);
                bullet_buf.clear();
            }
            Event::End(TagEnd::Item) if in_bullet => {
                if in_op_invariants {
                    if let Some(ann) = try_parse_annotation(&bullet_buf, path, bullet_line) {
                        out.push(ann);
                    }
                }
                in_bullet = false;
                bullet_buf.clear();
            }
            Event::Text(s) | Event::Code(s) => {
                if in_h4 {
                    h4_buf.push_str(&s);
                } else if in_bullet && in_op_invariants {
                    bullet_buf.push_str(&s);
                }
            }
            _ => {}
        }
    }
}

/// Attempt to parse a bracket annotation from bullet text.
///
/// Silently returns `None` when the bullet has no annotation marker.
/// Emits `tracing::warn!` and returns `None` when the bullet LOOKS like
/// an annotation but fails the bracket grammar (Invariant 7).
fn try_parse_annotation(text: &str, path: &Path, line: usize) -> Option<InvariantAnnotation> {
    let has_enforced = text.contains("[enforced-by:");
    let has_prose = text.contains("[prose-only:");

    if !has_enforced && !has_prose {
        return None;
    }

    if let Some((inv_id, tier, artifact, retire_when, prose_only_why)) =
        parse_annotation_grammar(text)
    {
        Some(InvariantAnnotation {
            inv_id,
            tier,
            artifact,
            retire_when,
            prose_only_why,
            source: Source::Spec {
                path: path.to_path_buf(),
                line,
            },
        })
    } else {
        tracing::warn!(
            "malformed invariant annotation at {}:{} — skipping: {:?}",
            path.display(),
            line,
            text
        );
        None
    }
}

/// `(inv_id, tier, artifact, retire_when, prose_only_why)` — return type of
/// [`parse_annotation_grammar`]. Aliased to keep the type below clippy's
/// `type_complexity` threshold.
type AnnotationFields = (
    String,
    TierKind,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Bracket-grammar parser — per RFC-005 §3.2 dry-run rust-systems-E:
/// new infrastructure distinct from prefix-matching `parse_bullet_edge`.
///
/// Recognises:
/// - `[enforced-by: <artifact>; retire-when: <predicate>]`
/// - `[prose-only: <why>]`
///
/// Returns `None` when the bracket block cannot be parsed.
fn parse_annotation_grammar(text: &str) -> Option<AnnotationFields> {
    let bracket_start = text.find('[')?;
    let bracket_end = text.rfind(']')?;
    if bracket_end <= bracket_start {
        return None;
    }
    let inv_id = text[..bracket_start].trim().to_owned();
    let inside = text[bracket_start + 1..bracket_end].trim();

    if let Some(rest) = inside.strip_prefix("enforced-by:") {
        let mut artifact: Option<String> = None;
        let mut retire_when: Option<String> = None;
        for part in rest.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some(rw) = part.strip_prefix("retire-when:") {
                retire_when = Some(rw.trim().to_owned());
            } else if artifact.is_none() {
                artifact = Some(part.to_owned());
            }
        }
        let tier = derive_tier(artifact.as_deref().unwrap_or(""));
        return Some((inv_id, tier, artifact, retire_when, None));
    }

    if let Some(why) = inside.strip_prefix("prose-only:") {
        return Some((
            inv_id,
            TierKind::ProseOnly,
            None,
            None,
            Some(why.trim().to_owned()),
        ));
    }

    None
}

/// Derive `TierKind` from an artifact path string per RFC-005 §3.2.
fn derive_tier(artifact: &str) -> TierKind {
    let a = artifact.trim();
    if a.ends_with(".cypher") {
        TierKind::Cypher
    } else if std::path::Path::new(a)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sh"))
    {
        TierKind::ScriptFence
    } else if a.is_empty() {
        TierKind::ProseOnly
    } else {
        TierKind::Tier0
    }
}

#[cfg(test)]
mod tests;
