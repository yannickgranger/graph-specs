use crate::bullets::{malformed_anchor, parse_bullet_edge, parse_impl_bullet, parse_verb_bullet};
use crate::front_matter::blank_front_matter;
use crate::grounding::{read, DialectHeading};
use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{ConceptNode, Edge, SignatureState, Source};
use domain::{ConceptRef, Violation};
use ports::{ReaderError, SignatureNormalizer};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::path::Path;

#[derive(Default)]
struct Collected {
    rust_blocks: Vec<(usize, String)>,
    fenced: Vec<(usize, String, String)>,
    bullets: Vec<(usize, String)>,
}

struct CollectorState {
    line_starts: Vec<usize>,
    in_fence: Option<(usize, String)>,
    in_rust_block_at: Option<usize>,
    block_buf: String,
    in_bullet_at: Option<usize>,
    bullet_buf: String,
    collected: Collected,
}

pub fn extract_from_source(
    source: &str,
    path: &Path,
    sink: &mut crate::BulletSink<'_>,
    normalizers: &[&dyn SignatureNormalizer],
) -> Result<(), ReaderError> {
    let dialect = read(path, source)?;
    let cleaned = blank_front_matter(source);
    let collected = collect(cleaned.as_ref());

    for heading in dialect.concepts() {
        let (start, end) = dialect.extent(heading);
        let owned = |line: usize| line > start && line < end;

        let blocks: Vec<(&str, &str)> = collected
            .fenced
            .iter()
            .filter(|(line, _, _)| owned(*line))
            .filter(|(_, tag, _)| normalizers.iter().any(|n| n.fence_tag() == tag))
            .map(|(_, tag, block)| (tag.as_str(), block.as_str()))
            .collect();
        let mut node = ConceptNode::new(
            heading.name.clone(),
            spec_source(path, heading.line),
            signature_from_fences(&blocks, normalizers),
        )
        .with_polarity(heading.polarity);
        node.marker = heading.marker;
        sink.nodes.push(node);

        for (line, text) in collected.bullets.iter().filter(|(line, _)| owned(*line)) {
            absorb_bullet(path, heading, *line, text, sink);
        }
    }

    Ok(())
}

fn spec_source(path: &Path, line: usize) -> Source {
    Source::Spec {
        format: domain::SpecFormat::Markdown,
        path: path.to_path_buf(),
        line,
        context: None,
    }
}

fn absorb_bullet(
    path: &Path,
    heading: &DialectHeading,
    line: usize,
    text: &str,
    sink: &mut crate::BulletSink<'_>,
) {
    if let Some((bullet, qname)) = malformed_anchor(text) {
        sink.malformed.push(Violation::MalformedAnchorBullet {
            concept: heading.name.clone(),
            bullet: bullet.to_owned(),
            qname,
            spec_source: spec_source(path, line),
        });
        return;
    }
    if let Some((kind, token, raw)) = parse_bullet_edge(text) {
        sink.edges.push(Edge {
            source_concept: ConceptRef::named(heading.name.clone()),
            kind,
            target: ConceptRef::named(token),
            raw_target: raw,
            source: spec_source(path, line),
        });
    } else if let Some(mut anchor) = parse_verb_bullet(text) {
        anchor.concept.clone_from(&heading.name);
        anchor.source = spec_source(path, line);
        sink.verb_anchors.push(anchor);
    } else if let Some(mut anchor) = parse_impl_bullet(text) {
        anchor.concept.clone_from(&heading.name);
        anchor.source = spec_source(path, line);
        sink.concept_anchors.push(anchor);
    }
}

fn collect(source: &str) -> Collected {
    let mut st = CollectorState {
        line_starts: compute_line_starts(source),
        in_fence: None,
        in_rust_block_at: None,
        block_buf: String::new(),
        in_bullet_at: None,
        bullet_buf: String::new(),
        collected: Collected::default(),
    };
    for (event, range) in Parser::new(source).into_offset_iter() {
        handle_event(&mut st, &event, range);
    }
    st.collected
}

fn handle_event(st: &mut CollectorState, event: &Event, range: std::ops::Range<usize>) {
    match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
            let line = line_of_offset(&st.line_starts, range.start);
            st.in_fence = Some((line, lang.as_ref().to_string()));
            if lang.as_ref() == "rust" {
                st.in_rust_block_at = Some(line);
            }
            st.block_buf.clear();
        }
        Event::End(TagEnd::CodeBlock) => {
            let block = std::mem::take(&mut st.block_buf);
            if let Some(line) = st.in_rust_block_at.take() {
                st.collected.rust_blocks.push((line, block.clone()));
            }
            if let Some((line, tag)) = st.in_fence.take() {
                st.collected.fenced.push((line, tag, block));
            }
        }
        Event::Start(Tag::Item) => {
            st.in_bullet_at = Some(line_of_offset(&st.line_starts, range.start));
            st.bullet_buf.clear();
        }
        Event::End(TagEnd::Item) => {
            if let Some(line) = st.in_bullet_at.take() {
                st.collected
                    .bullets
                    .push((line, std::mem::take(&mut st.bullet_buf)));
            }
        }
        Event::Text(s) | Event::Code(s) => absorb_text(st, s),
        _ => {}
    }
}

fn absorb_text(st: &mut CollectorState, s: &str) {
    if st.in_fence.is_some() {
        st.block_buf.push_str(s);
    } else if st.in_bullet_at.is_some() {
        st.bullet_buf.push_str(s);
    }
}

fn signature_from_fences(
    fences: &[(&str, &str)],
    normalizers: &[&dyn SignatureNormalizer],
) -> SignatureState {
    match fences {
        [] => SignatureState::Absent,
        [(tag, only)] => {
            normalizers
                .iter()
                .find(|n| n.fence_tag() == *tag)
                .map_or(SignatureState::Absent, |n| match n.normalize(only) {
                    Ok(target) => SignatureState::Normalized(target),
                    Err(error) => SignatureState::Unparseable {
                        raw: (*only).to_string(),
                        error: format!("{tag}: {error}"),
                    },
                })
        }
        many => {
            let count = many.len();
            let tags: Vec<&str> = many.iter().map(|(tag, _)| *tag).collect();
            SignatureState::Unparseable {
                raw: many
                    .iter()
                    .map(|(_, block)| *block)
                    .collect::<Vec<_>>()
                    .join("\n---\n"),
                error: format!(
                    "concept section contains {count} normalizable fenced blocks ({}); at most one is allowed",
                    tags.join(", ")
                ),
            }
        }
    }
}
