//! The concept walk — one `pulldown-cmark` pass per spec file collecting
//! `##`/`###` heading concepts, their single fenced `rust` block signature,
//! and the bullet grammars declared inside each section
//! ([`crate::bullets`]).
//!
//! A **separate pass** from the heading-tree assembler ([`crate::tree`]):
//! this module's `SectionState` is shaped for H2/H3 + fenced-rust +
//! bullet-edge dispatch, not the tree's full-depth abstraction ladder.

use crate::bullets::{
    parse_bullet_edge, parse_impl_bullet, parse_status_marker, parse_verb_bullet,
};
use crate::front_matter::{blank_front_matter, file_marker};
use crate::grounding::polarity_from_comment;
use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{
    ConceptAnchor, ConceptNode, Edge, Marker, Polarity, SignatureState, Source, VerbAnchor,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

/// Per-file extraction state. Grouping the state into a struct keeps
/// [`extract_from_source`] under the cognitive-complexity ceiling once
/// the v0.3 bullet-edge pass is woven in alongside the existing heading
/// and fenced-block handling.
struct SectionState<'a> {
    line_starts: Vec<usize>,
    /// The front-matter-blanked file body. Held so the marker rule can ask
    /// the one question `pulldown-cmark` events cannot answer directly: is
    /// this bullet the **first non-blank content line** below its heading?
    /// (see [`is_first_content_line`].)
    source: &'a str,
    path: &'a Path,
    // Heading collection.
    heading_buf: String,
    in_heading_at: Option<usize>,
    // Pending concept: held until the NEXT heading (or EOF) so the
    // accumulated rust blocks for the section can be attached.
    pending: Option<(String, usize)>,
    /// Which spec-state marker the pending
    /// concept carries, if any. Cleared with every new heading — a marker
    /// binds only to the heading whose block it opens; a marked H2 does not
    /// mark its H3s (no subtree inheritance).
    pending_marker: Option<Marker>,
    /// A `status:` front-matter value marks
    /// **every** concept heading in the file, so a per-heading bullet inside
    /// one is redundant, inert text — which is why the file value wins the
    /// combination in [`SectionState::effective_marker`] rather than the
    /// heading's.
    file_marker: Option<Marker>,
    /// The grounding polarity of the pending concept. Reset
    /// with every new heading — like the spec-state marker, a grounding
    /// comment binds only to the heading whose block it opens.
    pending_polarity: Polarity,
    /// Grounding-comment collection. `Some(line)` while inside an HTML
    /// block that opened under a concept heading.
    in_html_at: Option<usize>,
    html_buf: String,
    // Signature collection.
    rust_blocks: Vec<String>,
    in_rust_block: bool,
    block_buf: String,
    // Bullet collection (v0.3).
    in_bullet: Option<usize>,
    bullet_buf: String,
    // `- impl:` concept anchors collected during the walk.
    // Held on the state (not a threaded out-param like `verb_anchors`) so
    // `finish_bullet` / `handle_event` signatures stay unchanged; drained
    // by `extract_from_source` after the walk.
    concept_anchors: Vec<ConceptAnchor>,
}

impl<'a> SectionState<'a> {
    fn new(source: &'a str, path: &'a Path, file_marker: Option<Marker>) -> Self {
        Self {
            line_starts: compute_line_starts(source),
            source,
            path,
            heading_buf: String::new(),
            in_heading_at: None,
            pending: None,
            pending_marker: None,
            file_marker,
            pending_polarity: Polarity::Declared,
            in_html_at: None,
            html_buf: String::new(),
            rust_blocks: Vec::new(),
            in_rust_block: false,
            block_buf: String::new(),
            in_bullet: None,
            bullet_buf: String::new(),
            concept_anchors: Vec::new(),
        }
    }

    /// The marker that binds to the pending concept: the file-scope value
    /// when the file declares one, else the heading's own bullet.
    ///
    /// File scope wins because a per-heading bullet inside a marked file is
    /// redundant, inert text (RFC-013 §3.1 file scope). Under one value the
    /// two were indistinguishable and this was an `||`; under two
    /// (RFC-015 §3.1) the precedence has to be stated.
    fn effective_marker(&self) -> Option<Marker> {
        self.file_marker.or(self.pending_marker)
    }

    fn current_concept(&self) -> Option<&str> {
        self.pending.as_ref().map(|(n, _)| n.as_str())
    }
}

pub fn extract_from_source(
    source: &str,
    path: &Path,
    nodes: &mut Vec<ConceptNode>,
    edges: &mut Vec<Edge>,
    verb_anchors: &mut Vec<VerbAnchor>,
    concept_anchors: &mut Vec<ConceptAnchor>,
) {
    // RFC-013 §3.1 file scope. Read from the RAW source — the blanking pass
    // below erases the front matter this consults. Draft files are no longer
    // skipped; they are parsed, and every heading in them is marked.
    let file_marker = file_marker(source);
    // Blank any leading front-matter so a `cohesion: behavioral` block is not
    // mis-parsed as a setext heading (RFC-012 §3.3). Line numbers preserved.
    let cleaned = blank_front_matter(source);
    let source = cleaned.as_ref();
    let mut st = SectionState::new(source, path, file_marker);
    let parser = Parser::new(source).into_offset_iter();

    for (event, range) in parser {
        handle_event(&mut st, event, range, nodes, edges, verb_anchors);
    }

    let marker = st.effective_marker();
    flush_pending(
        &mut st.pending,
        marker,
        st.pending_polarity,
        &st.rust_blocks,
        st.path,
        nodes,
    );
    concept_anchors.append(&mut st.concept_anchors);
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
            let marker = st.effective_marker();
            flush_pending(
                &mut st.pending,
                marker,
                st.pending_polarity,
                &st.rust_blocks,
                st.path,
                nodes,
            );
            st.pending_marker = None;
            st.pending_polarity = Polarity::Declared;
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
        // RFC-014 §3.2 — the grounding comment. Collected only under a
        // concept heading; the adjacency test in `finish_html` decides
        // whether it actually binds.
        Event::Start(Tag::HtmlBlock) if st.pending.is_some() => {
            st.in_html_at = Some(line_of_offset(&st.line_starts, range.start));
            st.html_buf.clear();
        }
        Event::Html(ref html) if st.in_html_at.is_some() => st.html_buf.push_str(html),
        Event::End(TagEnd::HtmlBlock) => {
            if let Some(line) = st.in_html_at.take() {
                finish_html(st, line);
            }
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

/// Bind a collected HTML block's `polarity:` to the pending concept, if the
/// block is where a grounding comment has to be.
///
/// Adjacency reuses [`is_first_content_line`] — RFC-014 §3.2 names this the
/// same primitive RFC-013's `- status: draft` rule needs, deliberately, so
/// the two cannot drift into subtly different "immediately below" semantics.
/// A comment further down the section is inert.
fn finish_html(st: &mut SectionState, line: usize) {
    let html = std::mem::take(&mut st.html_buf);
    let Some((_, heading_line)) = st.pending.as_ref() else {
        return;
    };
    if is_first_content_line(st.source, *heading_line, line) {
        st.pending_polarity = polarity_from_comment(&html);
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
    // RFC-013 §3.1: the spec-state marker. Checked first — it shares no
    // prefix with any other bullet grammar, so this is ordering for
    // legibility, not for disambiguation.
    if let Some(marker) = parse_status_marker(text.as_str()) {
        if let Some((_, heading_line)) = st.pending.as_ref() {
            if is_first_content_line(st.source, *heading_line, line) {
                st.pending_marker = Some(marker);
            }
        }
        return;
    }
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
    } else if let Some(mut anchor) = parse_impl_bullet(text.as_str()) {
        anchor.concept = concept;
        anchor.source = Source::Spec {
            path: st.path.to_path_buf(),
            line,
        };
        st.concept_anchors.push(anchor);
    }
}

fn flush_pending(
    pending: &mut Option<(String, usize)>,
    marker: Option<Marker>,
    polarity: Polarity,
    rust_blocks: &[String],
    path: &Path,
    out: &mut Vec<ConceptNode>,
) {
    if let Some((name, line)) = pending.take() {
        // Spec-side nodes carry no containment provenance (RFC-010 §3.3).
        let mut node = ConceptNode::new(
            name,
            Source::Spec {
                path: path.to_path_buf(),
                line,
            },
            signature_from_blocks(rust_blocks),
        )
        .with_polarity(polarity);
        node.marker = marker.unwrap_or_default();
        out.push(node);
    }
}

/// Is the line at `bullet_line` the first non-blank line below the heading
/// at `heading_line`? — the placement half of the RFC-013 §3.1 marker rule.
///
/// Asked of the source text rather than the event stream because "first
/// non-blank **content line**" is a line-level fact: `pulldown-cmark` would
/// answer "first block", which differs for a bullet nested under a
/// paragraph or a loose list.
///
/// Mis-placement fails **loud, not silent**: a marker bullet that is not the
/// first content line is inert, the heading reads unmarked, and the
/// anti-invention check (`MissingInCode`) fires if its code is absent.
///
/// Both line numbers are 1-indexed, as [`line_of_offset`] produces.
fn is_first_content_line(source: &str, heading_line: usize, bullet_line: usize) -> bool {
    if bullet_line <= heading_line {
        return false;
    }
    source
        .lines()
        .skip(heading_line)
        .take(bullet_line - heading_line - 1)
        .all(|l| l.trim().is_empty())
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
