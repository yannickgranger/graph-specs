use super::*;
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
    assert!(result.unwrap().is_empty());
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
