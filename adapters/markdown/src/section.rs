use crate::bullets::{malformed_anchor, parse_bullet_edge, parse_impl_bullet, parse_verb_bullet};
use crate::front_matter::blank_front_matter;
use crate::grounding::{read, DialectHeading};
use crate::markdown_utils::{compute_line_starts, line_of_offset};
use domain::{ConceptNode, Edge, SignatureState, Source};
use domain::{ConceptRef, Violation};
use ports::ReaderError;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::path::Path;

#[derive(Default)]
struct Collected {
    rust_blocks: Vec<(usize, String)>,
    bullets: Vec<(usize, String)>,
}

struct CollectorState {
    line_starts: Vec<usize>,
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
) -> Result<(), ReaderError> {
    let dialect = read(path, source)?;
    let cleaned = blank_front_matter(source);
    let collected = collect(cleaned.as_ref());

    for heading in dialect.concepts() {
        let (start, end) = dialect.extent(heading);
        let owned = |line: usize| line > start && line < end;

        let blocks: Vec<&str> = collected
            .rust_blocks
            .iter()
            .filter(|(line, _)| owned(*line))
            .map(|(_, block)| block.as_str())
            .collect();
        let mut node = ConceptNode::new(
            heading.name.clone(),
            spec_source(path, heading.line),
            signature_from_blocks(&blocks),
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
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) if lang.as_ref() == "rust" => {
            st.in_rust_block_at = Some(line_of_offset(&st.line_starts, range.start));
            st.block_buf.clear();
        }
        Event::End(TagEnd::CodeBlock) => {
            if let Some(line) = st.in_rust_block_at.take() {
                st.collected
                    .rust_blocks
                    .push((line, std::mem::take(&mut st.block_buf)));
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
    if st.in_rust_block_at.is_some() {
        st.block_buf.push_str(s);
    } else if st.in_bullet_at.is_some() {
        st.bullet_buf.push_str(s);
    }
}

fn signature_from_blocks(blocks: &[&str]) -> SignatureState {
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
        Ok(item) => SignatureState::Normalized(signature_norm::normalize(&item)),
        Err(e) => SignatureState::Unparseable {
            raw: raw.to_string(),
            error: e.to_string(),
        },
    }
}
