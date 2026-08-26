use crate::front_matter::blank_front_matter;
use crate::grounding::DialectRead;
use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{InvariantAnnotation, Source, TierKind};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::path::Path;

const OPERATIONAL_INVARIANTS: &str = "Operational invariants";

struct AnnotationState<'a> {
    line_starts: Vec<usize>,
    path: &'a Path,
    spans: Vec<(usize, usize)>,
    in_bullet: bool,
    bullet_buf: String,
    bullet_line: usize,
}

impl<'a> AnnotationState<'a> {
    fn new(source: &str, path: &'a Path, spans: Vec<(usize, usize)>) -> Self {
        Self {
            line_starts: compute_line_starts(source),
            path,
            spans,
            in_bullet: false,
            bullet_buf: String::new(),
            bullet_line: 0,
        }
    }

    fn published(&self, line: usize) -> bool {
        self.spans
            .iter()
            .any(|(start, end)| line > *start && line < *end)
    }
}

pub fn extract_annotations_from_source(
    source: &str,
    path: &Path,
    dialect: &DialectRead,
    out: &mut Vec<InvariantAnnotation>,
) {
    let spans: Vec<(usize, usize)> = dialect
        .ladder
        .iter()
        .filter(|rung| rung.level >= 4 && rung.name == OPERATIONAL_INVARIANTS)
        .map(|rung| dialect.extent(rung))
        .collect();
    if spans.is_empty() {
        return;
    }
    let cleaned = blank_front_matter(source);
    let source = cleaned.as_ref();
    let mut st = AnnotationState::new(source, path, spans);

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
        Event::Start(Tag::Item) => {
            let line = line_of_offset(&st.line_starts, range.start);
            if st.published(line) {
                st.in_bullet = true;
                st.bullet_line = line;
                st.bullet_buf.clear();
            }
        }
        Event::End(TagEnd::Item) if st.in_bullet => finish_annotation_bullet(st, out),
        Event::Text(s) | Event::Code(s) => absorb_annotation_text(st, &s),
        _ => {}
    }
}

fn finish_annotation_bullet(st: &mut AnnotationState, out: &mut Vec<InvariantAnnotation>) {
    if let Some(ann) = try_parse_annotation(&st.bullet_buf, st.path, st.bullet_line) {
        out.push(ann);
    }
    st.in_bullet = false;
    st.bullet_buf.clear();
}

fn absorb_annotation_text(st: &mut AnnotationState, s: &str) {
    if st.in_bullet {
        st.bullet_buf.push_str(s);
    }
}

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

type AnnotationFields = (
    String,
    TierKind,
    Option<String>,
    Option<String>,
    Option<String>,
);

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
