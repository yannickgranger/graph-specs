use super::violation_key;
use crate::{
    AnchorKind, AnchorTarget, CheckInput, CohesionViolation, ConceptAnchor, ConceptNode,
    ContextDecl, ContextViolation, Edge, EdgeKind, Graph, Marker, OwnedUnit, Polarity, Provenance,
    ResolvedAnchor, SignatureState, Source, VerbOwnership, Violation,
};
use std::path::PathBuf;

/// v0.3-style wrapper — local tests predate [`CheckInput`] and pass a
/// bare [`Graph`] for specs. The wrapper adds empty contexts and verb
/// ownership so context and verb passes are no-ops.
fn diff(specs: Graph, code: Graph) -> Vec<Violation> {
    super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        code,
    )
    .violations
}

fn spec_path() -> PathBuf {
    PathBuf::from("specs/concepts/core.md")
}
fn code_path() -> PathBuf {
    PathBuf::from("domain/src/lib.rs")
}

fn spec(name: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Spec {
            path: spec_path(),
            line: 1,
        },
        SignatureState::Absent,
    )
}
fn code(name: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Code {
            path: code_path(),
            line: 1,
        },
        SignatureState::Absent,
    )
}
fn spec_with_sig(name: &str, sig: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Spec {
            path: spec_path(),
            line: 1,
        },
        SignatureState::Normalized(sig.to_string()),
    )
}
fn code_with_sig(name: &str, sig: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Code {
            path: code_path(),
            line: 1,
        },
        SignatureState::Normalized(sig.to_string()),
    )
}
fn spec_unparseable(name: &str, raw: &str, error: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Spec {
            path: spec_path(),
            line: 1,
        },
        SignatureState::Unparseable {
            raw: raw.to_string(),
            error: error.to_string(),
        },
    )
}

fn spec_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: concept.to_string(),
        kind,
        target: target.to_string(),
        raw_target: target.to_string(),
        source: Source::Spec {
            path: spec_path(),
            line: 10,
        },
    }
}
fn code_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: concept.to_string(),
        kind,
        target: target.to_string(),
        raw_target: target.to_string(),
        source: Source::Code {
            path: code_path(),
            line: 10,
        },
    }
}

fn nodes(ns: Vec<ConceptNode>) -> Graph {
    Graph {
        nodes: ns,
        edges: vec![],
    }
}

#[test]
fn empty_graphs_yield_no_violations() {
    let v = diff(Graph::default(), Graph::default());
    assert!(v.is_empty());
}

#[test]
fn matching_graphs_yield_no_violations() {
    let specs = nodes(vec![spec("Graph"), spec("Reader")]);
    let code = nodes(vec![code("Graph"), code("Reader")]);
    assert!(diff(specs, code).is_empty());
}

#[test]
fn spec_only_concept_is_missing_in_code() {
    let specs = nodes(vec![spec("Graph"), spec("Orphan")]);
    let code = nodes(vec![code("Graph")]);
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(&v[0], Violation::MissingInCode { name, .. } if name == "Orphan"));
}

#[test]
fn code_only_concept_is_missing_in_specs() {
    let specs = nodes(vec![spec("Graph")]);
    let code = nodes(vec![code("Graph"), code("Undeclared")]);
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(&v[0], Violation::MissingInSpecs { name, .. } if name == "Undeclared"));
}

#[test]
fn violations_are_sorted_by_name_deterministically() {
    let specs = nodes(vec![spec("Zebra"), spec("Alpha")]);
    let code = Graph::default();
    let v = diff(specs, code);
    let names: Vec<&str> = v
        .iter()
        .filter_map(|vi| match vi {
            Violation::MissingInCode { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["Alpha", "Zebra"]);
}

// --- v0.2 signature-level tests ---

#[test]
fn matching_signatures_yield_no_violations() {
    let sig = "pub struct OrderId(pub Uuid);";
    let specs = nodes(vec![spec_with_sig("OrderId", sig)]);
    let code = nodes(vec![code_with_sig("OrderId", sig)]);
    assert!(diff(specs, code).is_empty());
}

#[test]
fn drifting_signatures_yield_signature_drift() {
    let specs = nodes(vec![spec_with_sig(
        "OrderId",
        "pub struct OrderId(pub Uuid);",
    )]);
    let code = nodes(vec![code_with_sig(
        "OrderId",
        "pub struct OrderId(pub u64);",
    )]);
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::SignatureDrift { name, spec_sig, code_sig, .. }
            if name == "OrderId"
                && spec_sig == "pub struct OrderId(pub Uuid);"
                && code_sig == "pub struct OrderId(pub u64);"
    ));
}

#[test]
fn code_sig_without_spec_sig_is_not_a_v02_violation() {
    // v0.2 semantics: specs opt-in per-concept via a fenced `rust`
    // block. Absence on the spec side means "do not compare" — not
    // a violation.
    let specs = nodes(vec![spec("OrderId")]);
    let code = nodes(vec![code_with_sig(
        "OrderId",
        "pub struct OrderId(pub Uuid);",
    )]);
    assert!(diff(specs, code).is_empty());
}

#[test]
fn unparseable_spec_sig_yields_unparseable_violation() {
    let specs = nodes(vec![spec_unparseable(
        "OrderId",
        "pub struct OrderId(",
        "unexpected end of input, expected identifier",
    )]);
    let code = nodes(vec![code_with_sig(
        "OrderId",
        "pub struct OrderId(pub Uuid);",
    )]);
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::SignatureUnparseable { name, raw, .. }
            if name == "OrderId" && raw == "pub struct OrderId("
    ));
}

#[test]
fn absent_on_both_sides_still_passes_concept_check() {
    let specs = nodes(vec![spec("Foo")]);
    let code = nodes(vec![code("Foo")]);
    assert!(diff(specs, code).is_empty());
}

#[test]
fn duplicate_spec_names_collapse() {
    let specs = nodes(vec![spec("Graph"), spec("Graph")]);
    let code = Graph::default();
    let v = diff(specs, code);
    assert!(v
        .iter()
        .all(|vi| matches!(vi, Violation::MissingInCode { name, .. } if name == "Graph")));
}

// --- v0.3 edge-level tests ---

#[test]
fn concept_without_spec_edges_skips_edge_comparison() {
    // Reader is on both sides, but spec declares no bullet edges.
    // Code emits an IMPLEMENTS edge. Opt-in semantics: no comparison.
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("MarkdownReader")],
        edges: vec![],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("MarkdownReader")],
        edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    assert!(diff(specs, code).is_empty());
}

#[test]
fn matching_edges_yield_no_violations() {
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("MarkdownReader")],
        edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("MarkdownReader")],
        edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    assert!(diff(specs, code).is_empty());
}

#[test]
fn spec_edge_without_code_match_yields_edge_missing_in_code() {
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("MarkdownReader")],
        edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("MarkdownReader")],
        edges: vec![],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::EdgeMissingInCode { concept, edge_kind, target, .. }
            if concept == "MarkdownReader"
                && *edge_kind == EdgeKind::Implements
                && target == "Reader"
    ));
}

#[test]
fn code_edge_without_spec_match_yields_edge_missing_in_spec() {
    // MarkdownReader opts in via spec edge → code side's extra edge is
    // reported as missing in spec.
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("MarkdownReader"), spec("Parser")],
        edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("MarkdownReader"), code("Parser")],
        edges: vec![
            code_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
            code_edge("MarkdownReader", EdgeKind::DependsOn, "Parser"),
        ],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::EdgeMissingInSpec { concept, edge_kind, target, .. }
            if concept == "MarkdownReader"
                && *edge_kind == EdgeKind::DependsOn
                && target == "Parser"
    ));
}

#[test]
fn spec_edge_with_unknown_target_yields_edge_target_unknown() {
    let specs = Graph {
        nodes: vec![spec("MarkdownReader")],
        edges: vec![spec_edge(
            "MarkdownReader",
            EdgeKind::Implements,
            "NotAConcept",
        )],
    };
    let code = Graph {
        nodes: vec![code("MarkdownReader")],
        edges: vec![],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::EdgeTargetUnknown { concept, target, .. }
            if concept == "MarkdownReader" && target == "NotAConcept"
    ));
}

#[test]
fn edge_target_unknown_suppresses_missing_in_code() {
    // When the target is unknown we emit EdgeTargetUnknown only —
    // reporting both EdgeTargetUnknown and EdgeMissingInCode for the
    // same bullet would double-count.
    let specs = Graph {
        nodes: vec![spec("Reader")],
        edges: vec![spec_edge("Reader", EdgeKind::Implements, "Iterator")],
    };
    let code = Graph {
        nodes: vec![code("Reader")],
        edges: vec![],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(&v[0], Violation::EdgeTargetUnknown { .. }));
}

#[test]
fn edges_on_spec_only_concept_do_not_double_count() {
    // MarkdownReader is spec-only (MissingInCode already fires).
    // Its bullet edges must not also produce EdgeMissingInCode noise.
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("MarkdownReader")],
        edges: vec![spec_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    let code = Graph {
        nodes: vec![code("Reader")],
        edges: vec![],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::MissingInCode { name, .. } if name == "MarkdownReader"
    ));
}

#[test]
fn edges_on_code_only_concept_are_ignored() {
    // MarkdownReader is code-only. Its emitted edges have no spec side
    // to compare against → no EdgeMissingInSpec. Only MissingInSpecs.
    let specs = Graph {
        nodes: vec![spec("Reader")],
        edges: vec![],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("MarkdownReader")],
        edges: vec![code_edge("MarkdownReader", EdgeKind::Implements, "Reader")],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 1);
    assert!(matches!(
        &v[0],
        Violation::MissingInSpecs { name, .. } if name == "MarkdownReader"
    ));
}

#[test]
fn multi_edge_per_concept_is_diffed_independently() {
    // MarkdownReader specs: implements: Reader + depends on: Parser.
    // Code: implements: Reader + depends on: Tokenizer (different).
    let specs = Graph {
        nodes: vec![
            spec("Reader"),
            spec("Parser"),
            spec("Tokenizer"),
            spec("MarkdownReader"),
        ],
        edges: vec![
            spec_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
            spec_edge("MarkdownReader", EdgeKind::DependsOn, "Parser"),
        ],
    };
    let code = Graph {
        nodes: vec![
            code("Reader"),
            code("Parser"),
            code("Tokenizer"),
            code("MarkdownReader"),
        ],
        edges: vec![
            code_edge("MarkdownReader", EdgeKind::Implements, "Reader"),
            code_edge("MarkdownReader", EdgeKind::DependsOn, "Tokenizer"),
        ],
    };
    let v = diff(specs, code);
    assert_eq!(v.len(), 2);
    // Deterministic ordering: concept name "MarkdownReader" tied,
    // variant rank 5 (EdgeMissingInCode) before 6 (EdgeMissingInSpec).
    assert!(matches!(
        &v[0],
        Violation::EdgeMissingInCode { target, .. } if target == "Parser"
    ));
    assert!(matches!(
        &v[1],
        Violation::EdgeMissingInSpec { target, .. } if target == "Tokenizer"
    ));
}

#[test]
fn same_kind_different_targets_treated_as_separate_edges() {
    let specs = Graph {
        nodes: vec![spec("Reader"), spec("Parser"), spec("Thing")],
        edges: vec![
            spec_edge("Thing", EdgeKind::DependsOn, "Reader"),
            spec_edge("Thing", EdgeKind::DependsOn, "Parser"),
        ],
    };
    let code = Graph {
        nodes: vec![code("Reader"), code("Parser"), code("Thing")],
        edges: vec![
            code_edge("Thing", EdgeKind::DependsOn, "Reader"),
            code_edge("Thing", EdgeKind::DependsOn, "Parser"),
        ],
    };
    assert!(diff(specs, code).is_empty());
}

// --- v0.4 violation_key ordering tests (#22) ---

fn context_violation(name: &str) -> Violation {
    Violation::Context(ContextViolation::MembershipUnknown {
        concept: name.to_string(),
        owned_unit: OwnedUnit("some-crate".to_string()),
        code_source: Source::Code {
            path: code_path(),
            line: 1,
        },
    })
}

#[test]
fn violation_key_context_returns_rank_8() {
    let v = context_violation("Foo");
    let (concept, rank) = violation_key(&v);
    assert_eq!(concept, "Foo");
    assert_eq!(rank, 8);
}

#[test]
fn violation_key_context_sorts_after_edge_target_unknown() {
    let a = Violation::EdgeTargetUnknown {
        concept: "Foo".to_string(),
        edge_kind: EdgeKind::Implements,
        target: "X".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 1,
        },
    };
    let b = context_violation("Foo");
    // Same concept name ("Foo") tied — rank determines order. Edge
    // variants are rank 5-7, context is rank 8 — context sorts after.
    let (ka, da) = violation_key(&a);
    let (kb, db) = violation_key(&b);
    assert_eq!(ka, kb);
    assert!(da < db);
}

// --- v0.6 cohesion violation_key ranks (RFC-010 §3.5 / #125) ---

#[test]
fn violation_key_cohesion_returns_rank_12() {
    let v = Violation::Cohesion(CohesionViolation::ContextWithoutCohesionUnit {
        context: "equivalence".to_string(),
        file: PathBuf::from("specs/concepts/equivalence.md"),
    });
    let (key, rank) = violation_key(&v);
    assert_eq!(key, "equivalence");
    assert_eq!(rank, 12);
}

#[test]
fn violation_key_cohesion_uses_each_variant_key() {
    let mismatch = Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
        concept: "MarkdownReader".to_string(),
        declared: "reading".to_string(),
        code_context: "equivalence".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 1,
        },
    });
    let (key, rank) = violation_key(&mismatch);
    assert_eq!(key, "MarkdownReader");
    assert_eq!(rank, 12);
}

// --- v0.7 dangling-anchor violation_key rank (RFC-012 §3.5 / #146) ---

#[test]
fn violation_key_dangling_anchor_returns_rank_14() {
    // DanglingAnchor is a top-level arm (NOT nested in Cohesion) at rank 14.
    // Slot 13 held `ImplementsDraftConcept` until RFC-013 §3.4 retired it;
    // the slot is retired, not reused, so this rank does NOT move down.
    let v = Violation::DanglingAnchor {
        concept: "ValidateIntakeFull".to_string(),
        target: "validate_intake".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 3,
        },
    };
    let (key, rank) = violation_key(&v);
    assert_eq!(key, "ValidateIntakeFull");
    assert_eq!(rank, 14);
}

#[test]
fn retired_slot_13_leaves_a_gap_between_cohesion_and_dangling_anchor() {
    // Rewritten from `violation_key_dangling_anchor_sorts_after_implements_
    // draft_concept` (RFC-013 §7 Slice A: rewritten, not deleted). The
    // ordering it protected — the neighbouring ranks staying apart on a tied
    // key — must survive the retirement, with slot 13 left empty.
    let cohesion = Violation::Cohesion(CohesionViolation::ContextWithoutCohesionUnit {
        context: "Foo".to_string(),
        file: PathBuf::from("specs/concepts/foo.md"),
    });
    let dangling = Violation::DanglingAnchor {
        concept: "Foo".to_string(),
        target: "foo_impl".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 2,
        },
    };
    let (ka, da) = violation_key(&cohesion);
    let (kb, db) = violation_key(&dangling);
    assert_eq!(ka, kb);
    assert_eq!((da, db), (12, 14), "slot 13 stays retired, not renumbered");
    assert!(da < db);
}

// --- RFC-013 §3.2 — spec-state marker matrix rows 3 and 4 ---

/// A spec node carrying the `- status: draft` marker (RFC-013 §3.3).
fn spec_marked(name: &str) -> ConceptNode {
    let mut n = spec(name);
    n.marker = Marker::Draft;
    n
}

#[test]
fn marked_heading_without_code_is_pending_not_missing_in_code() {
    // Matrix row 3. Rewritten from the retired `ImplementsDraftConcept`
    // suite (RFC-013 §7 Slice A).
    let input = CheckInput::new(
        nodes(vec![spec_marked("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, Graph::default());
    assert!(
        outcome.violations.is_empty(),
        "a marked heading imposes no code-existence obligation: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
    assert_eq!(outcome.pending[0].concept, "Widget");
    assert!(outcome.realized.is_empty());
    assert!(outcome.is_clean(), "pending never fails the gate");
}

#[test]
fn marked_heading_with_code_is_realized_not_a_violation() {
    // Matrix row 4 — the polarity flip of RFC-009. What
    // `ImplementsDraftConcept` reported as a violation is now the
    // ratification signal.
    let input = CheckInput::new(
        nodes(vec![spec_marked("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, nodes(vec![code("Widget")]));
    assert!(
        outcome.violations.is_empty(),
        "code backing a marked heading is the normal mid-arc state: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.realized.len(), 1);
    assert_eq!(outcome.realized[0].concept, "Widget");
    assert!(outcome.pending.is_empty());
}

#[test]
fn a_marker_never_parks_a_divergence() {
    // RFC-013 §4 invariant 1 / §3.2 escalation-on-contradiction: a marked
    // heading whose backing item exists but whose signature drifted still
    // produces the ordinary violation — alongside the realized record.
    let specs = {
        let mut n = spec_with_sig("Widget", "pub struct Widget;");
        n.marker = Marker::Draft;
        nodes(vec![n])
    };
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(
        input,
        nodes(vec![code_with_sig("Widget", "pub enum Widget {}")]),
    );
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::SignatureDrift { name, .. } if name == "Widget")),
        "drift under a marker must still fire: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.realized.len(), 1, "and the record rides alongside");
    assert!(!outcome.is_clean(), "the gate stays red");
}

#[test]
fn unmarked_trees_produce_no_marker_records() {
    // RFC-013 §4 invariant 2 — a tree with no markers is semantically
    // identical to today's behavior.
    let specs = nodes(vec![spec("Present"), spec("Absent")]);
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(input, nodes(vec![code("Present"), code("Orphan")]));
    assert!(outcome.pending.is_empty());
    assert!(outcome.realized.is_empty());
    let ranks: Vec<_> = outcome.violations.iter().map(violation_key).collect();
    assert_eq!(
        ranks,
        vec![("Absent", 0u8), ("Orphan", 1u8)],
        "rows 1, 2 and 5 are byte-for-byte today's behavior: {:?}",
        outcome.violations
    );
}

#[test]
fn pending_concepts_edge_bullets_impose_no_obligation() {
    // RFC-013 §3.4 uniform obligation skip — the edge pass satisfies it by
    // construction (its matched-concept filter is built from code presence),
    // and this pins that it stays satisfied.
    let specs = Graph::new(
        vec![spec_marked("Widget"), spec("Gear")],
        vec![Edge {
            source_concept: "Widget".to_string(),
            kind: EdgeKind::DependsOn,
            target: "Gear".to_string(),
            raw_target: "Gear".to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 9,
            },
        }],
    );
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(input, nodes(vec![code("Gear")]));
    assert!(
        !outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::EdgeMissingInCode { .. })),
        "a pending concept's edges impose nothing: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1);
}

#[test]
fn orphan_without_a_heading_is_missing_in_specs() {
    // Matrix row 5, unchanged. (Formerly
    // `orphan_without_draft_match_is_missing_in_specs`.)
    let input = CheckInput::new(Graph::default(), Vec::new(), VerbOwnership::default());
    let v = super::diff(input, nodes(vec![code("Gadget")])).violations;
    assert_eq!(v.len(), 1);
    assert!(
        matches!(&v[0], Violation::MissingInSpecs { name, .. } if name == "Gadget"),
        "expected MissingInSpecs, got: {:?}",
        v[0]
    );
}

// --- RFC-012 §3.4 / R12-3 — anchored-concept exemption + DanglingAnchor ---

fn resolved_anchor(concept: &str, target: &str, resolves: bool) -> ResolvedAnchor {
    ResolvedAnchor {
        anchor: ConceptAnchor {
            concept: concept.to_string(),
            target: target.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 3,
            },
        },
        target: resolves.then(|| AnchorTarget {
            kind: AnchorKind::Fn,
            source: Source::Code {
                path: code_path(),
                line: 7,
            },
        }),
    }
}

#[test]
fn anchored_concept_with_resolved_target_is_not_missing_in_code() {
    // `## ValidateIntakeFull` has no name-matched pub type, but its `- impl:`
    // target resolves at any visibility → the concept is satisfied.
    let specs = Graph::new(vec![spec("ValidateIntakeFull")], Vec::new());
    let v = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()).with_concept_anchors(vec![
            resolved_anchor("ValidateIntakeFull", "validate_intake", true),
        ]),
        Graph::default(),
    )
    .violations;
    assert!(
        v.is_empty(),
        "resolved anchor must satisfy the concept: {v:?}"
    );
}

#[test]
fn anchored_concept_with_unresolved_target_is_dangling_not_missing() {
    let specs = Graph::new(vec![spec("ValidateIntakeFull")], Vec::new());
    let v = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default())
            .with_concept_anchors(vec![resolved_anchor("ValidateIntakeFull", "gone", false)]),
        Graph::default(),
    )
    .violations;
    assert_eq!(v.len(), 1, "exactly one violation: {v:?}");
    match &v[0] {
        Violation::DanglingAnchor {
            concept, target, ..
        } => {
            assert_eq!(concept, "ValidateIntakeFull");
            assert_eq!(target, "gone");
        }
        other => panic!("expected DanglingAnchor, got {other:?}"),
    }
    assert!(
        !v.iter()
            .any(|x| matches!(x, Violation::MissingInCode { .. })),
        "an anchored concept must not also be MissingInCode"
    );
}

#[test]
fn unanchored_missing_concept_still_missing_in_code() {
    // Regression (§4 I1): a concept with no anchor and no code match still fires.
    let specs = Graph::new(vec![spec("Orphan")], Vec::new());
    let v = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        Graph::default(),
    )
    .violations;
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::MissingInCode { .. })),
        "unanchored missing concept must still be MissingInCode: {v:?}"
    );
}

// --- RFC-014 §3.3 — grounding polarity ---

fn spec_with_polarity(name: &str, polarity: Polarity) -> ConceptNode {
    spec(name).with_polarity(polarity)
}

/// The §3.3 table, asserted directly as a table.
#[test]
fn polarity_presence_matrix() {
    // (polarity, code present) -> the violation ranks the diff must emit.
    let cases: &[(Polarity, bool, &[u8])] = &[
        (Polarity::Declared, false, &[0]),    // missing_in_code
        (Polarity::Declared, true, &[]),      // satisfied
        (Polarity::Forbidden, false, &[]),    // clean
        (Polarity::Forbidden, true, &[15]),   // forbidden_concept_reintroduced
        (Polarity::Illustrative, false, &[]), // clean
        (Polarity::Illustrative, true, &[1]), // missing_in_specs, via the orphan sweep
    ];
    for (polarity, code_present, expected) in cases {
        let specs = nodes(vec![spec_with_polarity("Member", *polarity)]);
        let code = if *code_present {
            nodes(vec![code("Member")])
        } else {
            Graph::default()
        };
        let outcome = super::diff(
            CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
            code,
        );
        let ranks: Vec<u8> = outcome
            .violations
            .iter()
            .map(|v| violation_key(v).1)
            .collect();
        assert_eq!(
            ranks, *expected,
            "polarity={polarity:?} code_present={code_present} produced {:?}",
            outcome.violations
        );
        assert!(outcome.pending.is_empty() && outcome.realized.is_empty());
    }
}

#[test]
fn illustrative_does_not_consume_the_code_node() {
    // The match-attempt gate, asserted directly rather than through its
    // symptom: an illustrative heading must leave the code node unconsumed
    // so it falls through to the orphan sweep. Removing it and re-emitting
    // `MissingInSpecs` would be a different thing wearing the same output —
    // and would take the concept out of the orphan set for everything else.
    let specs = nodes(vec![spec_with_polarity("Member", Polarity::Illustrative)]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Member")]),
    );
    assert!(
        matches!(
            outcome.violations.as_slice(),
            [Violation::MissingInSpecs { name, .. }] if name == "Member"
        ),
        "expected the code node to reach the orphan sweep, got: {:?}",
        outcome.violations
    );
}

#[test]
fn forbidden_reintroduction_names_both_sites() {
    let specs = nodes(vec![spec_with_polarity("Member", Polarity::Forbidden)]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Member")]),
    );
    match outcome.violations.as_slice() {
        [Violation::ForbiddenConceptReintroduced {
            name,
            spec_source,
            code_source,
        }] => {
            assert_eq!(name, "Member");
            assert!(
                matches!(spec_source, Source::Spec { .. }),
                "expelled-by site"
            );
            assert!(
                matches!(code_source, Source::Code { .. }),
                "reintroduced-at site"
            );
        }
        other => panic!("expected ForbiddenConceptReintroduced, got {other:?}"),
    }
    assert!(
        !outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInSpecs { .. })),
        "the heading documents the name — as banned — so no orphan fires too"
    );
}

/// RFC-014 §3.3 precedence: `polarity != declared` is terminal, so a marked
/// heading emits no marker record either way.
#[test]
fn non_declared_polarity_is_terminal_over_the_spec_state_marker() {
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        for code_present in [false, true] {
            let mut node = spec_with_polarity("Member", polarity);
            node.marker = Marker::Draft;
            let code_graph = if code_present {
                nodes(vec![code("Member")])
            } else {
                Graph::default()
            };
            let outcome = super::diff(
                CheckInput::new(nodes(vec![node]), Vec::new(), VerbOwnership::default()),
                code_graph,
            );
            assert!(
                outcome.pending.is_empty(),
                "marked+{polarity:?} must emit no Pending (code_present={code_present})"
            );
            assert!(
                outcome.realized.is_empty(),
                "marked+{polarity:?} must emit no Realized (code_present={code_present}) — \
                 `realized — ratify` on an expelled name would be an actively wrong instruction"
            );
            if !code_present {
                assert!(
                    outcome.is_clean(),
                    "marked+{polarity:?}+absent is clean: {:?}",
                    outcome.violations
                );
            }
        }
    }
}

#[test]
fn declared_polarity_leaves_the_marker_dispatch_intact() {
    // The other side of terminality — `declared` must reach RFC-013's rows.
    let mut node = spec_with_polarity("Widget", Polarity::Declared);
    node.marker = Marker::Draft;
    let outcome = super::diff(
        CheckInput::new(nodes(vec![node]), Vec::new(), VerbOwnership::default()),
        Graph::default(),
    );
    assert_eq!(outcome.pending.len(), 1);
}

#[test]
fn a_non_declared_concepts_edge_bullets_impose_no_obligation() {
    // RFC-014 §3.3 uniform obligation-skip. A heading that compels nothing
    // cannot be missing anything.
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        let specs = Graph::new(
            vec![spec_with_polarity("Member", polarity), spec("Gear")],
            vec![Edge {
                source_concept: "Member".to_string(),
                kind: EdgeKind::DependsOn,
                target: "Gear".to_string(),
                raw_target: "Gear".to_string(),
                source: Source::Spec {
                    path: spec_path(),
                    line: 9,
                },
            }],
        );
        let outcome = super::diff(
            CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
            nodes(vec![code("Member"), code("Gear")]),
        );
        assert!(
            !outcome
                .violations
                .iter()
                .any(|v| matches!(v, Violation::EdgeMissingInCode { .. })),
            "{polarity:?} concept's edges impose nothing: {:?}",
            outcome.violations
        );
    }
}

#[test]
fn a_dangling_anchor_under_a_non_declared_heading_fires_nothing() {
    // RFC-014 §3.3, OQ-4 ruled. Polarity is read off the shared snapshot,
    // never off `ConceptAnchor` — that would duplicate a fact `ConceptNode`
    // already owns.
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        let specs = nodes(vec![spec_with_polarity("Member", polarity)]);
        let outcome = super::diff(
            CheckInput::new(specs, Vec::new(), VerbOwnership::default())
                .with_concept_anchors(vec![resolved_anchor("Member", "gone", false)]),
            Graph::default(),
        );
        assert!(
            outcome.is_clean(),
            "{polarity:?} heading compels nothing, so its anchor cannot dangle: {:?}",
            outcome.violations
        );
    }

    // Control: the same anchor under a `declared` heading still fires.
    let specs = nodes(vec![spec("Member")]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default())
            .with_concept_anchors(vec![resolved_anchor("Member", "gone", false)]),
        Graph::default(),
    );
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::DanglingAnchor { .. })),
        "control: a declared heading's dangling anchor still fires"
    );
}

#[test]
fn violation_key_forbidden_reintroduced_returns_rank_15() {
    // Append-only tripwire: slot 15 sits after DanglingAnchor (14), and the
    // gap at 13 that RFC-013 retired stays a gap.
    let v = Violation::ForbiddenConceptReintroduced {
        name: "Member".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 5,
        },
        code_source: Source::Code {
            path: code_path(),
            line: 12,
        },
    };
    let (key, rank) = violation_key(&v);
    assert_eq!(key, "Member");
    assert_eq!(rank, 15);
}

#[test]
fn an_ungrounded_corpus_is_byte_identical() {
    // RFC-014 §4 invariant 1 — every heading `Declared` means every gate is
    // inert and the outcome matches pre-RFC behaviour exactly.
    let specs = nodes(vec![spec("Present"), spec("Absent")]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Present"), code("Orphan")]),
    );
    let ranks: Vec<_> = outcome.violations.iter().map(violation_key).collect();
    assert_eq!(ranks, vec![("Absent", 0u8), ("Orphan", 1u8)]);
}

// --- RFC-010 §3.6 / #136 — provenance index on the outcome ---

#[test]
fn outcome_provenance_snapshots_the_code_triple_with_resolved_context() {
    // `context` is resolved through the same `specs/contexts/` Owns lookup
    // the cohesion pass uses — carried on the outcome, not re-derived by
    // the emitter.
    let widget =
        code("Widget").with_provenance(Some("domain".to_owned()), Some("domain".to_owned()), None);
    let contexts = vec![ContextDecl::new(
        "equivalence".to_owned(),
        vec![OwnedUnit("domain".to_owned())],
        Vec::new(),
        Vec::new(),
        Source::Spec {
            path: spec_path(),
            line: 1,
        },
    )];
    let outcome = super::diff(
        CheckInput::new(
            nodes(vec![spec("Widget")]),
            contexts,
            VerbOwnership::default(),
        ),
        nodes(vec![widget]),
    );
    assert_eq!(
        outcome.provenance.get("Widget"),
        Some(&Provenance {
            module_path: Some("domain".to_owned()),
            unit: Some("domain".to_owned()),
            context: Some("equivalence".to_owned()),
        })
    );
}

#[test]
fn outcome_provenance_skips_nodes_with_no_facts() {
    // A provenance-free corpus (no adapter facts, no declared contexts)
    // yields an empty index — the emitter then renders plain source objects.
    let outcome = super::diff(
        CheckInput::new(Graph::default(), Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Bare")]),
    );
    assert!(
        outcome.provenance.is_empty(),
        "no facts must index nothing: {:?}",
        outcome.provenance
    );
}
