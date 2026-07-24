//! `#### Operational invariants` bracket-annotation extraction (RFC-005
//! §3.2) — `[enforced-by: <artifact>; retire-when: <predicate>]` and
//! `[prose-only: <why>]` bullets.
//!
//! Uses a **fresh** `Parser::new(source).into_offset_iter()` per file — NOT
//! shared with [`crate::section`]'s concept walk. The H4-only heading arm
//! and the bracket grammar are this module's own infrastructure, distinct
//! from the prefix-matching bullet grammars in [`crate::bullets`].

use crate::front_matter::blank_front_matter;
use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{InvariantAnnotation, Source, TierKind};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;

/// Per-file annotation-walk state.
struct AnnotationState<'a> {
    line_starts: Vec<usize>,
    path: &'a Path,
    in_op_invariants: bool,
    in_h4: bool,
    h4_buf: String,
    in_bullet: bool,
    bullet_buf: String,
    bullet_line: usize,
}

impl<'a> AnnotationState<'a> {
    fn new(source: &str, path: &'a Path) -> Self {
        Self {
            line_starts: compute_line_starts(source),
            path,
            in_op_invariants: false,
            in_h4: false,
            h4_buf: String::new(),
            in_bullet: false,
            bullet_buf: String::new(),
            bullet_line: 0,
        }
    }
}

/// Parse a single spec file for `#### Operational invariants` sections,
/// extracting all well-formed bracketed annotations from bullet items.
/// Per RFC-005 §3.2: fresh parser per file, own H4 arm, new bracket grammar.
pub(crate) fn extract_annotations_from_source(
    source: &str,
    path: &Path,
    out: &mut Vec<InvariantAnnotation>,
) {
    // Blank leading front-matter so a `cohesion: behavioral` block does not
    // perturb the H4-invariant parse (RFC-012 §3.3); line numbers preserved.
    let cleaned = blank_front_matter(source);
    let source = cleaned.as_ref();
    let mut st = AnnotationState::new(source, path);

    for (event, range) in Parser::new(source).into_offset_iter() {
        handle_annotation_event(&mut st, event, range, out);
    }
}

fn handle_annotation_event(
    st: &mut AnnotationState,
    event: Event,
    range: std::ops::Range<usize>,
    out: &mut Vec<InvariantAnnotation>,
) {
    match event {
        // Higher-level headings reset the invariants section.
        Event::Start(Tag::Heading {
            level: HeadingLevel::H1 | HeadingLevel::H2 | HeadingLevel::H3,
            ..
        }) => {
            st.in_op_invariants = false;
        }
        Event::Start(Tag::Heading {
            level: HeadingLevel::H4,
            ..
        }) => {
            st.in_h4 = true;
            st.h4_buf.clear();
        }
        Event::End(TagEnd::Heading(HeadingLevel::H4)) => {
            if st.in_h4 {
                st.in_op_invariants = st.h4_buf.trim() == "Operational invariants";
                st.in_h4 = false;
            }
        }
        Event::Start(Tag::Item) if st.in_op_invariants => {
            st.in_bullet = true;
            st.bullet_line = line_of_offset(&st.line_starts, range.start);
            st.bullet_buf.clear();
        }
        Event::End(TagEnd::Item) if st.in_bullet => finish_annotation_bullet(st, out),
        Event::Text(s) | Event::Code(s) => absorb_annotation_text(st, &s),
        _ => {}
    }
}

fn finish_annotation_bullet(st: &mut AnnotationState, out: &mut Vec<InvariantAnnotation>) {
    if st.in_op_invariants {
        if let Some(ann) = try_parse_annotation(&st.bullet_buf, st.path, st.bullet_line) {
            out.push(ann);
        }
    }
    st.in_bullet = false;
    st.bullet_buf.clear();
}

fn absorb_annotation_text(st: &mut AnnotationState, s: &str) {
    if st.in_h4 {
        st.h4_buf.push_str(s);
    } else if st.in_bullet && st.in_op_invariants {
        st.bullet_buf.push_str(s);
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
        return Some(parse_enforced_by(inv_id, rest));
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

/// Parse the `enforced-by: <artifact>; retire-when: <predicate>` clause body
/// (the text after the `enforced-by:` prefix has already been stripped) into
/// its fields. Split out of [`parse_annotation_grammar`] to keep that
/// function's branching under the cognitive-complexity ceiling.
fn parse_enforced_by(inv_id: String, rest: &str) -> AnnotationFields {
    let mut artifact: Option<String> = None;
    let mut retire_when: Option<String> = None;
    for part in rest.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        if let Some(rw) = part.strip_prefix("retire-when:") {
            retire_when = Some(rw.trim().to_owned());
        } else if artifact.is_none() {
            artifact = Some(part.to_owned());
        }
    }
    let tier = derive_tier(artifact.as_deref().unwrap_or(""));
    (inv_id, tier, artifact, retire_when, None)
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
