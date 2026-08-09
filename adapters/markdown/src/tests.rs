use super::*;
use crate::bullets::parse_anchor_qname;
use domain::{EdgeKind, Marker, Polarity, SignatureState, Source};
use std::io::Write;
use tempfile::TempDir;

fn write(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("test");
    }
    let mut f = std::fs::File::create(&full).expect("test");
    f.write_all(content.as_bytes()).expect("test");
}

fn extract(dir: &Path) -> Vec<String> {
    let g = MarkdownReader.extract(dir).expect("test");
    let mut names: Vec<String> = g.nodes.into_iter().map(|n| n.name).collect();
    names.sort();
    names
}

fn extract_graph(dir: &Path) -> Graph {
    MarkdownReader.extract(dir).expect("test")
}

#[test]
fn captures_h2_and_h3_headings() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Foo\n### Bar\n");
    assert_eq!(extract(d.path()), vec!["Bar", "Foo"]);
}

#[test]
fn ignores_h1_and_deeper_than_h3() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "# Title\n## Keep\n#### Skip\n##### Deep\n",
    );
    assert_eq!(extract(d.path()), vec!["Keep"]);
}

#[test]
fn ignores_prose_and_nonmatching_bullets() {
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Foo\n\n```rust\npub struct Hidden;\n```\n\n```\npub struct AlsoHidden;\n```\n",
    );
    assert_eq!(extract(d.path()), vec!["Foo"]);
}

#[test]
fn strips_generics_from_heading() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Graph<T>\n");
    assert_eq!(extract(d.path()), vec!["Graph"]);
}

#[test]
fn handles_inline_backticks_in_heading() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## `Reader`\n");
    assert_eq!(extract(d.path()), vec!["Reader"]);
}

#[test]
fn records_correct_line_number() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "# Title\n\n## OnLine3\n");
    let g = MarkdownReader.extract(d.path()).expect("test");
    match &g.nodes[0].source {
        Source::Spec { line, .. } => assert_eq!(*line, 3),
        Source::Code { .. } => panic!("expected Spec source"),
    }
}

#[test]
fn non_md_files_are_ignored() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## FromMd\n");
    write(d.path(), "a.txt", "## FromTxt\n");
    assert_eq!(extract(d.path()), vec!["FromMd"]);
}

// --- v0.2 signature-level tests ---

fn extract_sig(dir: &Path, concept: &str) -> SignatureState {
    let g = MarkdownReader.extract(dir).expect("test");
    g.nodes
        .into_iter()
        .find(|n| n.name == concept)
        .unwrap_or_else(|| panic!("concept {concept} not found"))
        .signature
}

#[test]
fn concept_without_rust_block_has_absent_signature() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Foo\n\nJust prose.\n");
    assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
}

#[test]
fn concept_with_rust_block_has_normalized_signature() {
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Foo\n\n```python\npub struct Foo;\n```\n",
    );
    assert_eq!(extract_sig(d.path(), "Foo"), SignatureState::Absent);
}

#[test]
fn unparseable_rust_block_becomes_unparseable_signature() {
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "```rust\npub struct Orphan;\n```\n\n## Foo\n",
    );
    let g = MarkdownReader.extract(d.path()).expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "- implements: Ghost\n\n## Foo\n");
    let g = extract_graph(d.path());
    assert!(g.edges.is_empty());
}

#[test]
fn edges_are_scoped_to_current_concept_section() {
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Foo\n\n- implements: `Reader`\n");
    let g = extract_graph(d.path());
    let edges = find_edges_for(&g.edges, "Foo");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].target, "Reader");
}

#[test]
fn bullets_coexist_with_rust_block_in_same_section() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Reader\n\n- implements: Trait\n\n```rust\npub trait Reader { fn extract(&self); }\n```\n\n- depends on: Graph\n",
    );
    let g = extract_graph(d.path());
    let edges = find_edges_for(&g.edges, "Reader");
    assert_eq!(edges.len(), 2);
    let reader_node = g.nodes.iter().find(|n| n.name == "Reader").expect("test");
    assert!(matches!(
        reader_node.signature,
        SignatureState::Normalized(_)
    ));
}

#[test]
fn bullet_edge_source_line_is_recorded() {
    let d = TempDir::new().expect("test");
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
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Foo\n\n- depends on: domain::Graph\n");
    let g = extract_graph(d.path());
    let edges = find_edges_for(&g.edges, "Foo");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].target, "Graph");
    assert_eq!(edges[0].raw_target, "domain::Graph");
}

// --- RFC-005 invariant-annotation tests ---

#[test]
fn extract_annotations_empty_on_no_h4_section() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\nProse.\n\n- implements: Reader\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert!(anns.is_empty());
}

#[test]
fn extract_annotations_recognises_enforced_by_cypher() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- INV-001: desc [enforced-by: .cfdb/queries/rule.cypher; retire-when: never]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert_eq!(anns.len(), 1);
    assert_eq!(anns[0].inv_id, "INV-001: desc");
    assert_eq!(anns[0].tier, domain::TierKind::Cypher);
    assert_eq!(
        anns[0].artifact.as_deref(),
        Some(".cfdb/queries/rule.cypher")
    );
    assert_eq!(anns[0].retire_when.as_deref(), Some("never"));
    assert!(anns[0].prose_only_why.is_none());
}

#[test]
fn extract_annotations_recognises_enforced_by_script() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- [enforced-by: scripts/check.sh; retire-when: done]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert_eq!(anns.len(), 1);
    assert_eq!(anns[0].tier, domain::TierKind::ScriptFence);
}

#[test]
fn extract_annotations_recognises_prose_only() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- INV-waiver: [prose-only: no tooling available]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert_eq!(anns.len(), 1);
    assert_eq!(anns[0].tier, domain::TierKind::ProseOnly);
    assert_eq!(
        anns[0].prose_only_why.as_deref(),
        Some("no tooling available")
    );
}

#[test]
fn extract_annotations_malformed_skips_with_ok_return() {
    let d = TempDir::new().expect("test");
    // Bullet looks like annotation (has marker prefix) but bracket is malformed
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- [enforced-by: missing-close-bracket\n",
    );
    let result = MarkdownReader.extract_invariant_annotations(d.path());
    assert!(result.is_ok(), "tolerant-skip: malformed should not Err");
    // malformed annotation is dropped
    assert!(
        result.expect("asserted Ok above").is_empty(),
        "malformed annotation must be dropped"
    );
}

#[test]
fn extract_annotations_section_ends_at_next_h2() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- [enforced-by: rule.cypher; retire-when: done]\n\n## Other\n\n- [enforced-by: other.cypher]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    // Only annotation in the Operational invariants section should be found;
    // the bullet under ## Other is not in an h4 section.
    assert_eq!(anns.len(), 1);
}

#[test]
fn extract_annotations_multiple_in_section() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n#### Operational invariants\n\n- INV-1: [enforced-by: rule.cypher; retire-when: a]\n- INV-2: [prose-only: reason]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert_eq!(anns.len(), 2);
}

/// Cross-dogfood baseline: cfdb's specs/concepts/ currently has no
/// `#### Operational invariants` sections, so the result should be empty.
/// This test uses an empty directory to exercise the same empty-Vec path.
#[test]
fn extract_annotations_returns_ok_empty_on_no_annotations_present() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## ConceptA\n\nSome prose.\n\n## ConceptB\n\n- depends on: ConceptA\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert!(anns.is_empty());
}

// --- v0.5 verb-bullet tests ---

#[test]
fn parse_verb_bullet_returns_anchor_for_valid_ident() {
    let anchor = parse_verb_bullet("verb: diff").expect("should parse");
    assert_eq!(anchor.qname, "diff");
    assert_eq!(anchor.raw_target, "verb: diff");
    assert!(
        anchor.concept.is_empty(),
        "concept placeholder should be empty"
    );
}

#[test]
fn parse_verb_bullet_returns_none_for_non_verb_bullet() {
    assert!(parse_verb_bullet("implements: Reader").is_none());
    assert!(parse_verb_bullet("prose line").is_none());
    assert!(parse_verb_bullet("depends on: Foo").is_none());
}

#[test]
fn parse_verb_bullet_returns_none_for_empty_qname() {
    assert!(parse_verb_bullet("verb:").is_none());
    assert!(parse_verb_bullet("verb:   ").is_none());
}

#[test]
fn parse_verb_bullet_returns_none_for_multi_word_qname() {
    assert!(parse_verb_bullet("verb: fn_name extra").is_none());
    assert!(parse_verb_bullet("verb: two words").is_none());
}

#[test]
fn verb_bullet_does_not_yield_edge_in_graph() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n- verb: some_fn\n- implements: Reader\n",
    );
    let g = MarkdownReader.extract(d.path()).expect("test");
    let edges = find_edges_for(&g.edges, "Concept");
    assert_eq!(edges.len(), 1, "verb bullet must not become an edge");
    assert_eq!(edges[0].kind, EdgeKind::Implements);
}

#[test]
fn extract_verb_anchors_collects_anchors_from_spec() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Graph\n\n- verb: diff\n- verb: tokenise_target\n\n## Reader\n\n- verb: extract\n",
    );
    let anchors = MarkdownReader.extract_verb_anchors(d.path()).expect("test");
    assert_eq!(anchors.len(), 3);
    let qnames: Vec<&str> = anchors.iter().map(|a| a.qname.as_str()).collect();
    assert!(qnames.contains(&"diff"));
    assert!(qnames.contains(&"tokenise_target"));
    assert!(qnames.contains(&"extract"));
    let graph_anchor_count = anchors.iter().filter(|a| a.concept == "Graph").count();
    assert_eq!(graph_anchor_count, 2);
}

#[test]
fn extract_verb_anchors_returns_empty_when_no_verb_bullets() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Concept\n\n- implements: Reader\n- depends on: Graph\n",
    );
    let anchors = MarkdownReader.extract_verb_anchors(d.path()).expect("test");
    assert!(anchors.is_empty());
}

#[test]
fn extract_verb_anchors_records_correct_concept_and_source_line() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Graph\n\n- verb: diff\n");
    let anchors = MarkdownReader.extract_verb_anchors(d.path()).expect("test");
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].concept, "Graph");
    assert_eq!(anchors[0].qname, "diff");
    match &anchors[0].source {
        Source::Spec { line, .. } => assert!(*line >= 1),
        Source::Code { .. } => panic!("expected Spec source"),
    }
}

/// v0.4 scoping: when the reader is pointed at `specs/`, files under
/// `contexts/` are owned by the `ContextReader` impl and MUST NOT
/// contaminate the concept graph. Without this filter, every `## Owns`
/// heading in a context file would register a phantom concept.
#[test]
fn v04_ignores_files_under_contexts_subdir() {
    let d = TempDir::new().expect("test");
    write(d.path(), "concepts/a.md", "## Foo\n## Bar\n");
    write(
        d.path(),
        "contexts/equivalence.md",
        "# equivalence\n\n## Owns\n\n- domain\n",
    );
    assert_eq!(extract(d.path()), vec!["Bar", "Foo"]);
}

// --- v0.6 parse_verb_bullet qname validation ---

#[test]
fn parse_verb_bullet_accepts_type_method() {
    let anchor = parse_verb_bullet("verb: Foo::bar").expect("should parse");
    assert_eq!(anchor.qname, "Foo::bar");
}

#[test]
fn parse_verb_bullet_rejects_multi_segment() {
    assert!(
        parse_verb_bullet("verb: a::b::c").is_none(),
        "multi-segment path must be rejected"
    );
}

#[test]
fn parse_verb_bullet_rejects_leading_colons() {
    assert!(
        parse_verb_bullet("verb: ::foo").is_none(),
        "leading :: must be rejected"
    );
}

#[test]
fn parse_verb_bullet_accepts_bare_ident_unchanged() {
    let anchor = parse_verb_bullet("verb: foo").expect("should parse");
    assert_eq!(anchor.qname, "foo");
}

// --- RFC-013 §3.1/§3.3 — spec-state marker ---

/// `marked` flag per concept name, for the marker tests.
fn marks(dir: &Path) -> Vec<(String, bool)> {
    let g = MarkdownReader.extract(dir).expect("test");
    let mut out: Vec<(String, bool)> = g
        .nodes
        .into_iter()
        .map(|n| (n.name, n.marker.is_marked()))
        .collect();
    out.sort();
    out
}

#[test]
fn per_heading_marker_bullet_marks_exactly_that_heading() {
    // Rewritten from `extract_draft_concepts_returns_headings_of_draft_files_only`
    // (RFC-013 §7 Slice A) — the side index it exercised is retired; marker
    // state now rides on the node itself.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Widget\n\n- status: draft\n\n## Gadget\n\nPlain prose.\n",
    );
    assert_eq!(
        marks(d.path()),
        vec![("Gadget".to_owned(), false), ("Widget".to_owned(), true)]
    );
}

#[test]
fn marker_tolerates_the_upstream_citation_parenthetical() {
    // RFC-013 §3.1/§6: anything after the value is tolerated and ignored —
    // the parenthetical is an upstream authoring convention, never gate-parsed.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Widget\n\n- status: draft (per RFC-vocabulary.md §4)\n",
    );
    assert_eq!(marks(d.path()), vec![("Widget".to_owned(), true)]);
}

#[test]
fn marker_value_matches_case_insensitively() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Widget\n\n- status: Draft\n");
    assert_eq!(marks(d.path()), vec![("Widget".to_owned(), true)]);
}

#[test]
fn there_is_no_second_marker_value() {
    // RFC-013 §3.1: one legal value. Ratification is deletion of the line,
    // a presence flag — never a state machine. `- status: ratified` is an
    // unrecognised prefix and stays inert text.
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Widget\n\n- status: ratified\n");
    assert_eq!(marks(d.path()), vec![("Widget".to_owned(), false)]);
}

#[test]
fn misplaced_marker_is_inert_and_the_heading_reads_unmarked() {
    // RFC-013 §3.1 fail-loud: a marker that is not the first non-blank
    // content line binds nothing. The heading stays unmarked, so the
    // anti-invention check fires on it downstream — a visible violation,
    // never a silent suppression.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Widget\n\nSome prose first.\n\n- status: draft\n",
    );
    assert_eq!(marks(d.path()), vec![("Widget".to_owned(), false)]);
}

#[test]
fn marker_after_a_sibling_bullet_is_inert() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Widget\n\n- depends on: Gear\n- status: draft\n\n## Gear\n",
    );
    let seen = marks(d.path());
    assert_eq!(
        seen,
        vec![("Gear".to_owned(), false), ("Widget".to_owned(), false)]
    );
}

#[test]
fn a_marker_does_not_inherit_to_sub_concepts() {
    // RFC-013 §3.1: no subtree inheritance — the checker models H2 and H3 as
    // flat peers, and inheritance would make ratification non-local.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Widget\n\n- status: draft\n\n### Cog\n\nProse.\n",
    );
    assert_eq!(
        marks(d.path()),
        vec![("Cog".to_owned(), false), ("Widget".to_owned(), true)]
    );
}

#[test]
fn marker_bullet_is_not_an_edge_or_an_anchor() {
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Widget\n\n- status: draft\n");
    let g = extract_graph(d.path());
    assert!(g.edges.is_empty(), "marker must not become an edge");
    let anchors = MarkdownReader
        .extract_concept_anchors(d.path())
        .expect("test");
    assert!(anchors.is_empty(), "marker must not become an anchor");
}

#[test]
fn parse_status_marker_grammar() {
    use crate::bullets::parse_status_marker;
    // RFC-015 §3.1 — two legal values, both ASCII-case-insensitive, both
    // tolerating the upstream trailing parenthetical. Everything else is an
    // unrecognised prefix and stays inert text.
    assert_eq!(parse_status_marker("status: draft"), Some(Marker::Draft));
    assert_eq!(parse_status_marker("status:draft"), Some(Marker::Draft));
    assert_eq!(
        parse_status_marker("status: DRAFT (per x.md §1)"),
        Some(Marker::Draft)
    );
    assert_eq!(
        parse_status_marker("status: retired"),
        Some(Marker::Retired)
    );
    assert_eq!(parse_status_marker("status:retired"), Some(Marker::Retired));
    assert_eq!(
        parse_status_marker("status: ReTiReD (per 015.md §3.1)"),
        Some(Marker::Retired)
    );
    assert_eq!(parse_status_marker("status: drafty"), None);
    assert_eq!(parse_status_marker("status: retiring"), None);
    assert_eq!(parse_status_marker("status: ratified"), None);
    assert_eq!(parse_status_marker("status:"), None);
    assert_eq!(parse_status_marker("depends on: draft"), None);
}

// --- draft front-matter suppression ---

#[test]
fn is_draft_recognises_status_draft_front_matter() {
    let src = "---\nstatus: draft\nauthor_council: council/x.md\n---\n\n## Foo\n";
    assert!(is_draft(src));
}

#[test]
fn is_draft_accepts_quoted_and_mixed_case_value() {
    assert!(is_draft("---\nstatus: \"Draft\"\n---\n"));
    assert!(is_draft("---\nstatus: 'draft' # pre-authored\n---\n"));
}

#[test]
fn is_draft_false_for_ratified_status() {
    assert!(!is_draft("---\nstatus: ratified\n---\n\n## Foo\n"));
}

#[test]
fn is_draft_false_without_front_matter() {
    assert!(!is_draft("## Foo\n\nstatus: draft mentioned in prose\n"));
}

#[test]
fn is_draft_false_when_front_matter_closes_before_status() {
    assert!(!is_draft("---\nauthor: x\n---\nstatus: draft\n"));
}

#[test]
fn draft_front_matter_marks_every_heading_in_the_file() {
    // RFC-013 §3.1/§3.3 file scope — the file is **parsed, not skipped**, and
    // every concept heading in it is marked. (Rewritten from
    // `extract_skips_draft_spec_keeps_siblings`.)
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "draft.md",
        "---\nstatus: draft\n---\n\n## Reconciler\n\n```rust\npub trait Reconciler {}\n```\n\n## Ledger\n",
    );
    write(d.path(), "live.md", "## Live\n");
    assert_eq!(
        marks(d.path()),
        vec![
            ("Ledger".to_owned(), true),
            ("Live".to_owned(), false),
            ("Reconciler".to_owned(), true)
        ]
    );
}

#[test]
fn draft_file_signatures_are_still_parsed() {
    // Marking relaxes the code-existence obligation, not the equivalence
    // check that follows once the concept is realized (§3.2 row 4).
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "draft.md",
        "---\nstatus: draft\n---\n\n## Reconciler\n\n```rust\npub trait Reconciler {}\n```\n",
    );
    assert!(matches!(
        extract_sig(d.path(), "Reconciler"),
        SignatureState::Normalized(_)
    ));
}

#[test]
fn extract_verb_anchors_reads_draft_specs() {
    // RFC-013 §3.3: anchors under a marked heading are extracted as normal;
    // the obligation skip is the diff's concern, not the reader's.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "draft.md",
        "---\nstatus: draft\n---\n\n## Reconciler\n\n- verb: reconcile\n",
    );
    let anchors = MarkdownReader.extract_verb_anchors(d.path()).expect("test");
    assert_eq!(anchors.len(), 1, "draft verb anchors are extracted");
    assert_eq!(anchors[0].qname, "reconcile");
}

#[test]
fn extract_invariant_annotations_skips_draft_spec() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "draft.md",
        "---\nstatus: draft\n---\n\n## Reconciler\n\n#### Operational invariants\n\n- INV-001: x [enforced-by: .cfdb/queries/r.cypher; retire-when: never]\n",
    );
    let anns = MarkdownReader
        .extract_invariant_annotations(d.path())
        .expect("test");
    assert!(
        anns.is_empty(),
        "draft invariant annotations must be skipped"
    );
}

// --- RFC-012 §3.2 / R12-2 — `- impl:` concept anchors + shared grammar ---

#[test]
fn parse_anchor_qname_accepts_bare_and_two_segment() {
    assert_eq!(
        parse_anchor_qname(" validate_intake "),
        Some("validate_intake")
    );
    assert_eq!(parse_anchor_qname("Type::method"), Some("Type::method"));
}

#[test]
fn parse_anchor_qname_rejects_multi_segment_and_empty() {
    assert_eq!(parse_anchor_qname("a::b::c"), None);
    assert_eq!(parse_anchor_qname("::lead"), None);
    assert_eq!(parse_anchor_qname("trail::"), None);
    assert_eq!(parse_anchor_qname("   "), None);
    assert_eq!(parse_anchor_qname("has space"), None);
}

#[test]
fn parse_impl_bullet_yields_concept_anchor() {
    let a = parse_impl_bullet("impl: validate_intake").expect("anchor");
    assert_eq!(a.target, "validate_intake");
    // concept/source are placeholders filled by finish_bullet.
    assert!(a.concept.is_empty());
}

#[test]
fn parse_impl_bullet_does_not_collide_with_implements_edge() {
    // `implements:` is an edge bullet, not an `impl:` anchor — strip_prefix
    // is literal, so no false match.
    assert!(parse_impl_bullet("implements: Foo").is_none());
}

#[test]
fn parse_impl_bullet_rejects_malformed_qname() {
    assert!(parse_impl_bullet("impl: a::b::c").is_none());
    assert!(parse_impl_bullet("impl:").is_none());
}

#[test]
fn impl_and_verb_share_one_qname_grammar() {
    // §4 I7: the same qname routes identically through both prefixes.
    let v = parse_verb_bullet("verb: Foo::bar").expect("verb");
    let i = parse_impl_bullet("impl: Foo::bar").expect("impl");
    assert_eq!(v.qname, i.target);
    // ...and both reject the same malformed qname.
    assert!(parse_verb_bullet("verb: a::b::c").is_none());
    assert!(parse_impl_bullet("impl: a::b::c").is_none());
}

#[test]
fn extract_concept_anchors_collects_impl_bullet() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "intake.md",
        "## ValidateIntakeFull\n\n- impl: validate_intake\n",
    );
    let anchors = MarkdownReader
        .extract_concept_anchors(d.path())
        .expect("test");
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].concept, "ValidateIntakeFull");
    assert_eq!(anchors[0].target, "validate_intake");
}

#[test]
fn extract_concept_anchors_is_empty_without_impl_and_reads_draft_files() {
    // RFC-013 §3.3: draft files are parsed. A bullet-free non-draft file
    // still contributes nothing.
    let d = TempDir::new().expect("test");
    write(d.path(), "plain.md", "## Foo\n\n- depends on: Bar\n");
    write(
        d.path(),
        "draft.md",
        "---\nstatus: draft\n---\n\n## Drafted\n\n- impl: drafted_fn\n",
    );
    let anchors = MarkdownReader
        .extract_concept_anchors(d.path())
        .expect("test");
    assert_eq!(anchors.len(), 1, "the draft file's anchor is extracted");
    assert_eq!(anchors[0].target, "drafted_fn");
}

#[test]
fn impl_bullet_is_not_an_edge() {
    // An `- impl:` bullet must not be parsed as a relationship edge.
    let d = TempDir::new().expect("test");
    write(d.path(), "a.md", "## Foo\n\n- impl: foo_fn\n");
    let g = extract_graph(d.path());
    assert!(g.edges.is_empty(), "impl bullet must not become an edge");
}

// --- RFC-012 §3.3 / R12-2 — `cohesion: behavioral` front-matter ---

#[test]
fn is_behavioral_context_detects_marker() {
    assert!(is_behavioral_context(
        "---\ncohesion: behavioral\n---\n\n# secrets\n"
    ));
    assert!(is_behavioral_context(
        "---\ncohesion: \"behavioral\" # doctrine\n---\n# x\n"
    ));
}

#[test]
fn is_behavioral_context_rejects_absent_or_other_shapes() {
    assert!(!is_behavioral_context("# secrets\n")); // no front-matter
    assert!(!is_behavioral_context("---\nstatus: draft\n---\n# x\n")); // different key
    assert!(!is_behavioral_context(
        "---\ncohesion: load-bearing\n---\n# x\n"
    )); // other value
        // key only in the prose body, not the leading block:
    assert!(!is_behavioral_context("# x\n\ncohesion: behavioral\n"));
}

// --- RFC-012 §3.3.1 / R12-4 — behavioral substance detection ---

#[test]
fn has_behavioral_substance_detects_each_marker() {
    assert!(has_behavioral_substance("- impl: foo\n"));
    assert!(has_behavioral_substance("- verb: bar\n"));
    assert!(has_behavioral_substance(
        "- INV-x: y [enforced-by: q.cypher; retire-when: never]\n"
    ));
    assert!(has_behavioral_substance(
        "- INV-x: y [prose-only: doctrine]\n"
    ));
    assert!(has_behavioral_substance("  * verb: indented\n")); // other markers + indent
}

#[test]
fn has_behavioral_substance_rejects_non_substance() {
    assert!(!has_behavioral_substance("# ctx\n\nJust prose.\n"));
    // `- implements:` is an edge bullet, NOT an `impl:` anchor — not substance.
    assert!(!has_behavioral_substance(
        "- implements: Foo\n- depends on: Bar\n"
    ));
}

// --- RFC-014 §3.2 — grounding polarity binds to its heading ---

/// `polarity` per concept name.
fn polarities(dir: &Path) -> Vec<(String, Polarity)> {
    let g = MarkdownReader.extract(dir).expect("test");
    let mut out: Vec<(String, Polarity)> =
        g.nodes.into_iter().map(|n| (n.name, n.polarity)).collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn a_grounding_comment_binds_to_the_heading_it_opens() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Member\n<!-- parent:spec:Unit polarity:forbidden -->\n\n## Unit\n\nProse.\n",
    );
    assert_eq!(
        polarities(d.path()),
        vec![
            ("Member".to_owned(), Polarity::Forbidden),
            ("Unit".to_owned(), Polarity::Declared),
        ],
        "polarity binds to one heading — no inheritance to the next"
    );
}

#[test]
fn a_blank_line_between_heading_and_comment_still_binds() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Member\n\n<!-- polarity:illustrative -->\n",
    );
    assert_eq!(
        polarities(d.path()),
        vec![("Member".to_owned(), Polarity::Illustrative)]
    );
}

#[test]
fn a_comment_that_is_not_the_first_content_line_is_inert() {
    // Same adjacency rule as the `- status: draft` marker, deliberately —
    // one primitive, so the two cannot drift apart.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Member\n\nSome prose first.\n\n<!-- polarity:forbidden -->\n",
    );
    assert_eq!(
        polarities(d.path()),
        vec![("Member".to_owned(), Polarity::Declared)],
        "a comment further down the section binds nothing"
    );
}

#[test]
fn a_comment_above_the_first_heading_binds_nothing() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "<!-- polarity:forbidden -->\n\n## Member\n\nProse.\n",
    );
    assert_eq!(
        polarities(d.path()),
        vec![("Member".to_owned(), Polarity::Declared)]
    );
}

#[test]
fn a_quoted_decoy_in_a_real_grounding_block_is_not_read() {
    // End-to-end through the reader, not just the token scanner: upstream
    // makes `anchor:"…"` mandatory, so this shape is the common case.
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Member\n<!-- parent:rfc:RFC-9 anchor:\"why polarity:forbidden exists\" -->\n",
    );
    assert_eq!(
        polarities(d.path()),
        vec![("Member".to_owned(), Polarity::Declared)],
        "prose inside the mandatory quoted anchor is not the key"
    );
}

#[test]
fn a_grounding_comment_is_not_an_edge_or_an_anchor() {
    let d = TempDir::new().expect("test");
    write(
        d.path(),
        "a.md",
        "## Member\n<!-- parent:spec:Unit polarity:forbidden -->\n",
    );
    let g = extract_graph(d.path());
    assert!(g.edges.is_empty());
    assert!(MarkdownReader
        .extract_concept_anchors(d.path())
        .expect("test")
        .is_empty());
}
