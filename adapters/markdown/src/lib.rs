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

use domain::{tokenise_target, ConceptNode, Edge, EdgeKind, Graph, SignatureState, Source};
use ports::{Reader, ReaderError};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct MarkdownReader;

impl Reader for MarkdownReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for entry in WalkDir::new(root) {
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

            let path = entry.path();
            let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;

            extract_from_source(&source, path, &mut nodes, &mut edges);
        }

        Ok(Graph { nodes, edges })
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

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn line_of_offset(starts: &[usize], offset: usize) -> usize {
    match starts.binary_search(&offset) {
        Ok(i) => i + 1,
        Err(i) => i.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let full = dir.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&full).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn extract(dir: &Path) -> Vec<String> {
        let g = MarkdownReader.extract(dir).unwrap();
        let mut names: Vec<String> = g.nodes.into_iter().map(|n| n.name).collect();
        names.sort();
        names
    }

    fn extract_graph(dir: &Path) -> Graph {
        MarkdownReader.extract(dir).unwrap()
    }

    #[test]
    fn captures_h2_and_h3_headings() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n### Bar\n");
        assert_eq!(extract(d.path()), vec!["Bar", "Foo"]);
    }

    #[test]
    fn ignores_h1_and_deeper_than_h3() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "# Title\n## Keep\n#### Skip\n##### Deep\n",
        );
        assert_eq!(extract(d.path()), vec!["Keep"]);
    }

    #[test]
    fn ignores_prose_and_nonmatching_bullets() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Concept\n\nSome prose about Concept.\n\n- a bullet mentioning Other\n- another\n\nMore prose.\n",
        );
        let g = extract_graph(d.path());
        assert_eq!(
            g.nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(),
            vec!["Concept"]
        );
        assert!(g.edges.is_empty(), "prose bullets must not yield edges");
    }

    #[test]
    fn ignores_fenced_code_blocks_for_concept_names() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Hidden;\n```\n\n```\npub struct AlsoHidden;\n```\n",
        );
        assert_eq!(extract(d.path()), vec!["Foo"]);
    }

    #[test]
    fn strips_generics_from_heading() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Graph<T>\n");
        assert_eq!(extract(d.path()), vec!["Graph"]);
    }

    #[test]
    fn handles_inline_backticks_in_heading() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## `Reader`\n");
        assert_eq!(extract(d.path()), vec!["Reader"]);
    }

    #[test]
    fn records_correct_line_number() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "# Title\n\n## OnLine3\n");
        let g = MarkdownReader.extract(d.path()).unwrap();
        match &g.nodes[0].source {
            Source::Spec { line, .. } => assert_eq!(*line, 3),
            Source::Code { .. } => panic!("expected Spec source"),
        }
    }

    #[test]
    fn non_md_files_are_ignored() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## FromMd\n");
        write(d.path(), "a.txt", "## FromTxt\n");
        assert_eq!(extract(d.path()), vec!["FromMd"]);
    }

    // --- v0.2 signature-level tests ---

    fn extract_sig(dir: &Path, concept: &str) -> SignatureState {
        let g = MarkdownReader.extract(dir).unwrap();
        g.nodes
            .into_iter()
            .find(|n| n.name == concept)
            .unwrap_or_else(|| panic!("concept {concept} not found"))
            .signature
    }

    #[test]
    fn concept_without_rust_block_has_absent_signature() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n\nJust prose.\n");
        assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
    }

    #[test]
    fn concept_with_rust_block_has_normalized_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\nProse.\n\n```rust\npub struct Foo(pub u32);\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Normalized(s) => {
                assert!(s.contains("pub struct Foo"));
                assert!(s.contains("u32"));
            }
            other => panic!("expected Normalized, got {other:?}"),
        }
    }

    #[test]
    fn non_rust_fenced_blocks_do_not_populate_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```python\npub struct Foo;\n```\n",
        );
        assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
    }

    #[test]
    fn unparseable_rust_block_becomes_unparseable_signature() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo(\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Unparseable { raw, error } => {
                assert!(raw.contains("pub struct Foo("));
                assert!(!error.is_empty());
            }
            other => panic!("expected Unparseable, got {other:?}"),
        }
    }

    #[test]
    fn multiple_rust_blocks_in_one_concept_is_unparseable() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo;\n```\n\n```rust\npub struct Bar;\n```\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Unparseable { error, .. } => {
                assert!(error.to_lowercase().contains("multiple") || error.contains('2'));
            }
            other => panic!("expected Unparseable (multiple rust blocks), got {other:?}"),
        }
    }

    #[test]
    fn rust_block_scoped_to_current_concept_only() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n```rust\npub struct Foo;\n```\n\n## Bar\n\nNo block here.\n",
        );
        match extract_sig(d.path(), "Foo") {
            SignatureState::Normalized(_) => {}
            other => panic!("Foo should have Normalized sig, got {other:?}"),
        }
        assert_eq!(extract_sig(d.path(), "Bar"), SignatureState::Absent);
    }

    #[test]
    fn rust_block_before_first_heading_is_ignored() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "```rust\npub struct Orphan;\n```\n\n## Foo\n",
        );
        let g = MarkdownReader.extract(d.path()).unwrap();
        let names: Vec<&str> = g.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["Foo"]);
        assert_eq!(g.nodes[0].signature, SignatureState::Absent);
    }

    // --- v0.3 bullet-edge tests ---

    fn find_edges_for<'a>(edges: &'a [Edge], concept: &str) -> Vec<&'a Edge> {
        edges
            .iter()
            .filter(|e| e.source_concept == concept)
            .collect()
    }

    #[test]
    fn implements_bullet_yields_edge() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## MarkdownReader\n\n- implements: Reader\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "MarkdownReader");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Implements);
        assert_eq!(edges[0].target, "Reader");
        assert_eq!(edges[0].raw_target, "Reader");
    }

    #[test]
    fn depends_on_bullet_yields_edge() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## MarkdownReader\n\n- depends on: pulldown_cmark\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "MarkdownReader");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::DependsOn);
        assert_eq!(edges[0].target, "pulldown_cmark");
    }

    #[test]
    fn returns_bullet_yields_edge_and_tokenises_target() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Reader\n\n- returns: Result<Graph, ReaderError>\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "Reader");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Returns);
        assert_eq!(edges[0].target, "Result");
        assert_eq!(edges[0].raw_target, "Result<Graph, ReaderError>");
    }

    #[test]
    fn multiple_bullets_yield_multiple_edges() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## MarkdownReader\n\n- implements: Reader\n- depends on: pulldown_cmark\n- depends on: walkdir\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "MarkdownReader");
        assert_eq!(edges.len(), 3);
        let kinds: Vec<EdgeKind> = edges.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&EdgeKind::Implements));
        assert_eq!(
            kinds.iter().filter(|k| **k == EdgeKind::DependsOn).count(),
            2
        );
    }

    #[test]
    fn bullet_without_matching_prefix_is_prose() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## MarkdownReader\n\n- some narrative bullet\n- another prose line\n",
        );
        let g = extract_graph(d.path());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn empty_bullet_target_is_ignored() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## MarkdownReader\n\n- implements:\n- depends on:    \n",
        );
        let g = extract_graph(d.path());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn prefix_match_is_case_sensitive() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Thing\n\n- Implements: Foo\n- DEPENDS ON: Bar\n",
        );
        let g = extract_graph(d.path());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn bullet_before_any_heading_is_ignored() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "- implements: Ghost\n\n## Foo\n");
        let g = extract_graph(d.path());
        assert!(g.edges.is_empty());
    }

    #[test]
    fn edges_are_scoped_to_current_concept_section() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\n- implements: X\n\n## Bar\n\n- depends on: Y\n",
        );
        let g = extract_graph(d.path());
        let foo = find_edges_for(&g.edges, "Foo");
        let bar = find_edges_for(&g.edges, "Bar");
        assert_eq!(foo.len(), 1);
        assert_eq!(foo[0].kind, EdgeKind::Implements);
        assert_eq!(foo[0].target, "X");
        assert_eq!(bar.len(), 1);
        assert_eq!(bar[0].kind, EdgeKind::DependsOn);
        assert_eq!(bar[0].target, "Y");
    }

    #[test]
    fn bullet_with_inline_backticks_yields_edge() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n\n- implements: `Reader`\n");
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "Foo");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, "Reader");
    }

    #[test]
    fn bullets_coexist_with_rust_block_in_same_section() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Reader\n\n- implements: Trait\n\n```rust\npub trait Reader { fn extract(&self); }\n```\n\n- depends on: Graph\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "Reader");
        assert_eq!(edges.len(), 2);
        let reader_node = g.nodes.iter().find(|n| n.name == "Reader").unwrap();
        assert!(matches!(
            reader_node.signature,
            SignatureState::Normalized(_)
        ));
    }

    #[test]
    fn bullet_edge_source_line_is_recorded() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "a.md",
            "## Foo\n\nProse line.\n\n- implements: Reader\n",
        );
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "Foo");
        assert_eq!(edges.len(), 1);
        match &edges[0].source {
            Source::Spec { line, .. } => assert!(
                *line >= 3,
                "bullet line should point somewhere past the heading, got {line}"
            ),
            Source::Code { .. } => panic!("expected Spec source"),
        }
    }

    #[test]
    fn module_path_target_is_tokenised() {
        let d = TempDir::new().unwrap();
        write(d.path(), "a.md", "## Foo\n\n- depends on: domain::Graph\n");
        let g = extract_graph(d.path());
        let edges = find_edges_for(&g.edges, "Foo");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, "Graph");
        assert_eq!(edges[0].raw_target, "domain::Graph");
    }
}
