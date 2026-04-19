use super::{diff, violation_key};
use crate::{
    ConceptNode, ContextViolation, Edge, EdgeKind, Graph, OwnedUnit, SignatureState, Source,
    Violation,
};
use std::path::PathBuf;

fn spec_path() -> PathBuf {
    PathBuf::from("specs/concepts/core.md")
}
fn code_path() -> PathBuf {
    PathBuf::from("domain/src/lib.rs")
}

fn spec(name: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Spec {
            path: spec_path(),
            line: 1,
        },
        signature: SignatureState::Absent,
    }
}
fn code(name: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Code {
            path: code_path(),
            line: 1,
        },
        signature: SignatureState::Absent,
    }
}
fn spec_with_sig(name: &str, sig: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Spec {
            path: spec_path(),
            line: 1,
        },
        signature: SignatureState::Normalized(sig.to_string()),
    }
}
fn code_with_sig(name: &str, sig: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Code {
            path: code_path(),
            line: 1,
        },
        signature: SignatureState::Normalized(sig.to_string()),
    }
}
fn spec_unparseable(name: &str, raw: &str, error: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Spec {
            path: spec_path(),
            line: 1,
        },
        signature: SignatureState::Unparseable {
            raw: raw.to_string(),
            error: error.to_string(),
        },
    }
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
