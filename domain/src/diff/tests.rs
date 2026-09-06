use super::violation_key;
use crate::ConceptRef;
use crate::LocationKind;
use crate::{
    AnchorKind, AnchorTarget, CheckInput, CohesionViolation, ConceptAnchor, ConceptNode,
    ContextDecl, ContextViolation, Edge, EdgeKind, Graph, Marker, OwnedUnit, Polarity, Provenance,
    ResolvedAnchor, SignatureState, Source, VerbAnchor, VerbOwnership, Violation,
};
use std::path::PathBuf;

fn diff(specs: Graph, code: Graph) -> Vec<Violation> {
    super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        code,
        None,
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
            context: None,
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
            provenance: Provenance::empty(),
            location: LocationKind::Path,
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
            context: None,
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
            provenance: Provenance::empty(),
            location: LocationKind::Path,
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
            context: None,
        },
        SignatureState::Unparseable {
            raw: raw.to_string(),
            error: error.to_string(),
        },
    )
}

fn spec_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: ConceptRef::named(concept.to_string()),
        kind,
        target: ConceptRef::named(target.to_string()),
        raw_target: target.to_string(),
        source: Source::Spec {
            path: spec_path(),
            line: 10,
            context: None,
        },
    }
}
fn code_edge(concept: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: ConceptRef::named(concept.to_string()),
        kind,
        target: ConceptRef::named(target.to_string()),
        raw_target: target.to_string(),
        source: Source::Code {
            path: code_path(),
            line: 10,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
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

#[test]
fn concept_without_spec_edges_skips_edge_comparison() {
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

fn context_violation(name: &str) -> Violation {
    Violation::Context(ContextViolation::MembershipUnknown {
        concept: name.to_string(),
        owned_unit: OwnedUnit("some-crate".to_string()),
        code_source: Source::Code {
            path: code_path(),
            line: 1,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
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
            context: None,
        },
    };
    let b = context_violation("Foo");
    let (ka, da) = violation_key(&a);
    let (kb, db) = violation_key(&b);
    assert_eq!(ka, kb);
    assert!(da < db);
}

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
            context: None,
        },
    });
    let (key, rank) = violation_key(&mismatch);
    assert_eq!(key, "MarkdownReader");
    assert_eq!(rank, 12);
}

#[test]
fn violation_key_dangling_anchor_returns_rank_14() {
    let v = Violation::DanglingAnchor {
        concept: "ValidateIntakeFull".to_string(),
        target: "validate_intake".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 3,
            context: None,
        },
    };
    let (key, rank) = violation_key(&v);
    assert_eq!(key, "ValidateIntakeFull");
    assert_eq!(rank, 14);
}

#[test]
fn retired_slot_13_leaves_a_gap_between_cohesion_and_dangling_anchor() {
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
            context: None,
        },
    };
    let (ka, da) = violation_key(&cohesion);
    let (kb, db) = violation_key(&dangling);
    assert_eq!(ka, kb);
    assert_eq!((da, db), (12, 14), "slot 13 stays retired, not renumbered");
    assert!(da < db);
}

fn spec_marked(name: &str) -> ConceptNode {
    let mut n = spec(name);
    n.marker = Marker::Draft;
    n
}

#[test]
fn marked_heading_without_code_is_pending_not_missing_in_code() {
    let input = CheckInput::new(
        nodes(vec![spec_marked("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, Graph::default(), None);
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
    let input = CheckInput::new(
        nodes(vec![spec_marked("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, nodes(vec![code("Widget")]), None);
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
    let specs = {
        let mut n = spec_with_sig("Widget", "pub struct Widget;");
        n.marker = Marker::Draft;
        nodes(vec![n])
    };
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(
        input,
        nodes(vec![code_with_sig("Widget", "pub enum Widget {}")]),
        None,
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
    let specs = nodes(vec![spec("Present"), spec("Absent")]);
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(input, nodes(vec![code("Present"), code("Orphan")]), None);
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
    let specs = Graph::new(
        vec![spec_marked("Widget"), spec("Gear")],
        vec![Edge {
            source_concept: ConceptRef::named("Widget".to_string()),
            kind: EdgeKind::DependsOn,
            target: ConceptRef::named("Gear".to_string()),
            raw_target: "Gear".to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 9,
                context: None,
            },
        }],
    );
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(input, nodes(vec![code("Gear")]), None);
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
    let input = CheckInput::new(Graph::default(), Vec::new(), VerbOwnership::default());
    let v = super::diff(input, nodes(vec![code("Gadget")]), None).violations;
    assert_eq!(v.len(), 1);
    assert!(
        matches!(&v[0], Violation::MissingInSpecs { name, .. } if name == "Gadget"),
        "expected MissingInSpecs, got: {:?}",
        v[0]
    );
}

fn resolved_anchor(concept: &str, target: &str, resolves: bool) -> ResolvedAnchor {
    ResolvedAnchor {
        anchor: ConceptAnchor {
            concept: concept.to_string(),
            target: target.to_string(),
            source: Source::Spec {
                path: spec_path(),
                line: 3,
                context: None,
            },
        },
        target: resolves.then(|| AnchorTarget {
            kind: AnchorKind::Fn,
            source: Source::Code {
                path: code_path(),
                line: 7,
                provenance: Provenance::empty(),
                location: LocationKind::Path,
            },
        }),
    }
}

#[test]
fn anchored_concept_with_resolved_target_is_not_missing_in_code() {
    let specs = Graph::new(vec![spec("ValidateIntakeFull")], Vec::new());
    let v = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()).with_concept_anchors(vec![
            resolved_anchor("ValidateIntakeFull", "validate_intake", true),
        ]),
        Graph::default(),
        None,
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
        None,
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
    let specs = Graph::new(vec![spec("Orphan")], Vec::new());
    let v = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        Graph::default(),
        None,
    )
    .violations;
    assert!(
        v.iter()
            .any(|x| matches!(x, Violation::MissingInCode { .. })),
        "unanchored missing concept must still be MissingInCode: {v:?}"
    );
}

fn spec_with_polarity(name: &str, polarity: Polarity) -> ConceptNode {
    spec(name).with_polarity(polarity)
}

#[test]
fn polarity_presence_matrix() {
    let cases: &[(Polarity, bool, &[u8])] = &[
        (Polarity::Declared, false, &[0]),
        (Polarity::Declared, true, &[]),
        (Polarity::Forbidden, false, &[]),
        (Polarity::Forbidden, true, &[15]),
        (Polarity::Illustrative, false, &[]),
        (Polarity::Illustrative, true, &[1]),
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
            None,
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
    let specs = nodes(vec![spec_with_polarity("Member", Polarity::Illustrative)]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Member")]),
        None,
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
        None,
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
                None,
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
    let mut node = spec_with_polarity("Widget", Polarity::Declared);
    node.marker = Marker::Draft;
    let outcome = super::diff(
        CheckInput::new(nodes(vec![node]), Vec::new(), VerbOwnership::default()),
        Graph::default(),
        None,
    );
    assert_eq!(outcome.pending.len(), 1);
}

#[test]
fn a_non_declared_concepts_edge_bullets_impose_no_obligation() {
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        let specs = Graph::new(
            vec![spec_with_polarity("Member", polarity), spec("Gear")],
            vec![Edge {
                source_concept: ConceptRef::named("Member".to_string()),
                kind: EdgeKind::DependsOn,
                target: ConceptRef::named("Gear".to_string()),
                raw_target: "Gear".to_string(),
                source: Source::Spec {
                    path: spec_path(),
                    line: 9,
                    context: None,
                },
            }],
        );
        let outcome = super::diff(
            CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
            nodes(vec![code("Member"), code("Gear")]),
            None,
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
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        let specs = nodes(vec![spec_with_polarity("Member", polarity)]);
        let outcome = super::diff(
            CheckInput::new(specs, Vec::new(), VerbOwnership::default())
                .with_concept_anchors(vec![resolved_anchor("Member", "gone", false)]),
            Graph::default(),
            None,
        );
        assert!(
            outcome.is_clean(),
            "{polarity:?} heading compels nothing, so its anchor cannot dangle: {:?}",
            outcome.violations
        );
    }

    let specs = nodes(vec![spec("Member")]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default())
            .with_concept_anchors(vec![resolved_anchor("Member", "gone", false)]),
        Graph::default(),
        None,
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
    let v = Violation::ForbiddenConceptReintroduced {
        name: "Member".to_string(),
        spec_source: Source::Spec {
            path: spec_path(),
            line: 5,
            context: None,
        },
        code_source: Source::Code {
            path: code_path(),
            line: 12,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
        },
    };
    let (key, rank) = violation_key(&v);
    assert_eq!(key, "Member");
    assert_eq!(rank, 15);
}

#[test]
fn an_ungrounded_corpus_is_byte_identical() {
    let specs = nodes(vec![spec("Present"), spec("Absent")]);
    let outcome = super::diff(
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Present"), code("Orphan")]),
        None,
    );
    let ranks: Vec<_> = outcome.violations.iter().map(violation_key).collect();
    assert_eq!(ranks, vec![("Absent", 0u8), ("Orphan", 1u8)]);
}

#[test]
fn outcome_provenance_snapshots_the_code_triple_with_resolved_context() {
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
            context: None,
        },
    )];
    let outcome = super::diff(
        CheckInput::new(Graph::default(), contexts, VerbOwnership::default()),
        nodes(vec![widget]),
        None,
    );
    let code_source = outcome
        .violations
        .iter()
        .find_map(|v| match v {
            Violation::MissingInSpecs { name, code_source } if name == "Widget" => {
                Some(code_source.clone())
            }
            _ => None,
        })
        .expect("Widget unmatched by any heading");
    assert_eq!(
        code_source,
        Source::Code {
            path: PathBuf::from("domain/src/lib.rs"),
            line: 1,
            provenance: Provenance {
                module_path: Some("domain".to_owned()),
                unit: Some("domain".to_owned()),
                context: Some("equivalence".to_owned()),
            },
            location: LocationKind::Path,
        }
    );
}

#[test]
fn outcome_provenance_skips_nodes_with_no_facts() {
    let outcome = super::diff(
        CheckInput::new(Graph::default(), Vec::new(), VerbOwnership::default()),
        nodes(vec![code("Bare")]),
        None,
    );
    let code_source = outcome
        .violations
        .iter()
        .find_map(|v| match v {
            Violation::MissingInSpecs { code_source, .. } => Some(code_source.clone()),
            _ => None,
        })
        .expect("Bare unmatched by any heading");
    assert_eq!(code_source.module_path(), None);
    assert_eq!(code_source.unit(), None);
    assert_eq!(code_source.context(), None);
}

fn spec_retired(name: &str) -> ConceptNode {
    let mut n = spec(name);
    n.marker = Marker::Retired;
    n
}

#[test]
fn retired_heading_with_code_is_retirement_incomplete() {
    let input = CheckInput::new(
        nodes(vec![spec_retired("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, nodes(vec![code("Widget")]), None);
    assert!(
        outcome.violations.is_empty(),
        "marker/code co-presence is not itself the contradiction: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.retirement_incomplete.len(), 1);
    assert_eq!(outcome.retirement_incomplete[0].concept, "Widget");
    assert!(outcome.retirement_complete.is_empty());
    assert!(
        outcome.realized.is_empty(),
        "row 7 is not row 4 — `realized — ratify` would invite deleting a \
         marker line that is never deleted"
    );
    assert!(outcome.is_clean(), "row 7 never fails the gate");
}

#[test]
fn retired_heading_without_code_is_retirement_complete_not_missing_in_code() {
    let input = CheckInput::new(
        nodes(vec![spec_retired("Widget")]),
        Vec::new(),
        VerbOwnership::default(),
    );
    let outcome = super::diff(input, Graph::default(), None);
    assert!(
        outcome.violations.is_empty(),
        "a retired heading imposes no code-existence obligation: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.retirement_complete.len(), 1);
    assert_eq!(outcome.retirement_complete[0].concept, "Widget");
    assert!(outcome.retirement_incomplete.is_empty());
    assert!(
        outcome.pending.is_empty(),
        "row 8 is not row 3 — nothing here is a transcription worklist item"
    );
    assert!(outcome.is_clean());
}

#[test]
fn rows_7_and_8_are_selected_by_the_same_backing_item_fact_as_rows_3_and_4() {
    for (marker, spelling_by_name) in [
        (Marker::Draft, true),
        (Marker::Draft, false),
        (Marker::Retired, true),
        (Marker::Retired, false),
    ] {
        for backed in [false, true] {
            let mut node = spec("Widget");
            node.marker = marker;
            let mut input =
                CheckInput::new(nodes(vec![node]), Vec::new(), VerbOwnership::default());
            let code_graph = if spelling_by_name && backed {
                nodes(vec![code("Widget")])
            } else {
                Graph::default()
            };
            if !spelling_by_name {
                input = input.with_concept_anchors(vec![resolved_anchor(
                    "Widget",
                    "build_widget",
                    backed,
                )]);
            }
            let outcome = super::diff(input, code_graph, None);
            let backed_len = match marker {
                Marker::Draft => outcome.realized.len(),
                _ => outcome.retirement_incomplete.len(),
            };
            let unbacked_len = match marker {
                Marker::Draft => outcome.pending.len(),
                _ => outcome.retirement_complete.len(),
            };
            let (want_backed, want_unbacked) = if backed { (1, 0) } else { (0, 1) };
            assert_eq!(
                (backed_len, unbacked_len),
                (want_backed, want_unbacked),
                "{marker:?}, by_name={spelling_by_name}, backed={backed}"
            );
            assert!(
                !outcome
                    .violations
                    .iter()
                    .any(|v| matches!(v, Violation::DanglingAnchor { .. })),
                "an unresolved anchor under EITHER marker is the state the \
                 marker announces, not a dangling anchor: {:?}",
                outcome.violations
            );
        }
    }
}

#[test]
fn a_retired_marker_never_parks_a_divergence() {
    let specs = {
        let mut n = spec_with_sig("Widget", "pub struct Widget;");
        n.marker = Marker::Retired;
        nodes(vec![n])
    };
    let input = CheckInput::new(specs, Vec::new(), VerbOwnership::default());
    let outcome = super::diff(
        input,
        nodes(vec![code_with_sig("Widget", "pub enum Widget {}")]),
        None,
    );
    assert!(
        outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::SignatureDrift { name, .. } if name == "Widget")),
        "drift under a retired heading must still fire: {:?}",
        outcome.violations
    );
    assert_eq!(
        outcome.retirement_incomplete.len(),
        1,
        "and the record rides alongside"
    );
    assert!(!outcome.is_clean(), "the gate stays red");
}

#[test]
fn non_declared_polarity_is_terminal_over_the_retired_value_too() {
    for polarity in [Polarity::Forbidden, Polarity::Illustrative] {
        for code_present in [false, true] {
            let mut node = spec_with_polarity("Member", polarity);
            node.marker = Marker::Retired;
            let code_graph = if code_present {
                nodes(vec![code("Member")])
            } else {
                Graph::default()
            };
            let outcome = super::diff(
                CheckInput::new(nodes(vec![node]), Vec::new(), VerbOwnership::default()),
                code_graph,
                None,
            );
            assert!(
                outcome.retirement_incomplete.is_empty() && outcome.retirement_complete.is_empty(),
                "retired+{polarity:?} must emit no marker record at all \
                 (code_present={code_present})"
            );
        }
    }
}

#[test]
fn row_8_verb_anchors_impose_no_obligation() {
    let ctx = ContextDecl::new(
        "eq".to_owned(),
        vec![OwnedUnit("domain".to_owned())],
        vec![],
        vec![],
        Source::Spec {
            path: spec_path(),
            line: 1,
            context: None,
        },
    );
    let anchor = VerbAnchor {
        concept: "Widget".to_owned(),
        qname: "build_widget".to_owned(),
        raw_target: "verb: build_widget".to_owned(),
        source: Source::Spec {
            path: spec_path(),
            line: 3,
            context: None,
        },
    };
    let neighbour_code = ConceptNode::new(
        "Anchorage".to_owned(),
        Source::Code {
            path: PathBuf::from("domain/src/lib.rs"),
            line: 7,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
        },
        SignatureState::Absent,
    );
    let verbs_of = |marker: Marker| {
        let mut node = spec("Widget");
        node.marker = marker;
        super::diff(
            CheckInput::new(
                nodes(vec![node, spec("Anchorage")]),
                vec![ctx.clone()],
                VerbOwnership {
                    decls: vec![],
                    anchors: vec![anchor.clone()],
                },
            ),
            Graph::new(vec![neighbour_code.clone()], vec![]),
            None,
        )
        .violations
        .into_iter()
        .filter(|v| matches!(v, Violation::VerbMissingInCode { .. }))
        .count()
    };
    assert_eq!(
        verbs_of(Marker::Unmarked),
        1,
        "control: an unmarked concept's dangling verb anchor still fires"
    );
    assert_eq!(
        verbs_of(Marker::Draft),
        0,
        "row 3 skips — the rule this one is claimed to mirror"
    );
    assert_eq!(
        verbs_of(Marker::Retired),
        0,
        "row 8 skips identically, and it is listed rather than inherited"
    );
}

fn edge_into(target_state: ConceptNode, target_has_code: bool) -> (CheckInput, Graph) {
    let specs = Graph::new(
        vec![spec("Source"), target_state],
        vec![spec_edge("Source", EdgeKind::DependsOn, "Target")],
    );
    let mut code_nodes = vec![code("Source")];
    if target_has_code {
        code_nodes.push(code("Target"));
    }
    (
        CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
        Graph::new(code_nodes, Vec::new()),
    )
}

fn target(marker: Marker, polarity: Polarity) -> ConceptNode {
    let mut n = spec_with_polarity("Target", polarity);
    n.marker = marker;
    n
}

fn fires_edge_missing_in_code(input: CheckInput, code: Graph) -> bool {
    super::diff(input, code, None)
        .violations
        .iter()
        .any(|v| matches!(v, Violation::EdgeMissingInCode { target, .. } if target == "Target"))
}

#[test]
fn the_marker_matrix_suppresses_in_exactly_the_two_marked_and_absent_cells() {
    for marker in [Marker::Unmarked, Marker::Draft, Marker::Retired] {
        for present in [false, true] {
            let (input, code) = edge_into(target(marker, Polarity::Declared), present);
            let want = !marker.is_marked() || present;
            assert_eq!(
                fires_edge_missing_in_code(input, code),
                want,
                "{marker:?} + present={present}"
            );
        }
    }
}

#[test]
fn an_anchored_target_counts_as_present_and_is_not_suppressed() {
    let (input, code) = edge_into(target(Marker::Draft, Polarity::Declared), false);
    let input = input.with_concept_anchors(vec![resolved_anchor("Target", "build_target", true)]);
    assert!(
        fires_edge_missing_in_code(input, code),
        "a resolved anchor is a backing item — the marker has nothing to account for"
    );
}

#[test]
fn the_polarity_matrix_fires_only_for_illustrative_with_an_item() {
    for (polarity, present, want_fire) in [
        (Polarity::Forbidden, false, false),
        (Polarity::Forbidden, true, false),
        (Polarity::Illustrative, false, false),
        (Polarity::Illustrative, true, true),
    ] {
        let (input, code) = edge_into(target(Marker::Unmarked, polarity), present);
        assert_eq!(
            fires_edge_missing_in_code(input, code),
            want_fire,
            "{polarity:?} + present={present}"
        );
    }
}

#[test]
fn adding_the_field_clears_the_illustrative_present_finding() {
    let armed = super::diff(
        edge_into(target(Marker::Unmarked, Polarity::Illustrative), true).0,
        Graph::new(vec![code("Source"), code("Target")], Vec::new()),
        None,
    );
    let cleared = super::diff(
        edge_into(target(Marker::Unmarked, Polarity::Illustrative), true).0,
        Graph::new(
            vec![code("Source"), code("Target")],
            vec![code_edge("Source", EdgeKind::DependsOn, "Target")],
        ),
        None,
    );
    assert_eq!(armed.violations.len(), 2, "{:?}", armed.violations);
    assert_eq!(cleared.violations.len(), 1, "{:?}", cleared.violations);
    assert!(
        cleared
            .violations
            .iter()
            .all(|v| matches!(v, Violation::MissingInSpecs { .. })),
        "only the unspecced-item finding survives: {:?}",
        cleared.violations
    );
}

#[test]
fn edge_missing_in_spec_fires_in_every_cell_of_both_matrices() {
    let states = [
        (Marker::Unmarked, Polarity::Declared),
        (Marker::Draft, Polarity::Declared),
        (Marker::Retired, Polarity::Declared),
        (Marker::Unmarked, Polarity::Forbidden),
        (Marker::Unmarked, Polarity::Illustrative),
    ];
    for (marker, polarity) in states {
        for present in [false, true] {
            let specs = Graph::new(
                vec![spec("Source"), target(marker, polarity)],
                vec![spec_edge("Source", EdgeKind::DependsOn, "Target")],
            );
            let mut code_nodes = vec![code("Source")];
            if present {
                code_nodes.push(code("Target"));
            }
            let outcome = super::diff(
                CheckInput::new(specs, Vec::new(), VerbOwnership::default()),
                Graph::new(
                    code_nodes,
                    vec![code_edge("Source", EdgeKind::Implements, "Target")],
                ),
                None,
            );
            assert!(
                outcome
                    .violations
                    .iter()
                    .any(|v| matches!(v, Violation::EdgeMissingInSpec { .. })),
                "{marker:?} + {polarity:?} + present={present}: {:?}",
                outcome.violations
            );
        }
    }
}

#[test]
fn a_suppressed_target_yields_no_edge_target_unknown_and_a_mirage_still_does() {
    let (input, code_graph) = edge_into(target(Marker::Retired, Polarity::Declared), false);
    let suppressed = super::diff(input, code_graph, None);
    assert!(
        !suppressed
            .violations
            .iter()
            .any(|v| matches!(v, Violation::EdgeTargetUnknown { .. })),
        "a declared-but-unpointable name is not a mirage: {:?}",
        suppressed.violations
    );

    let mirage = super::diff(
        CheckInput::new(
            Graph::new(
                vec![spec("Source")],
                vec![spec_edge("Source", EdgeKind::DependsOn, "Ghost")],
            ),
            Vec::new(),
            VerbOwnership::default(),
        ),
        nodes(vec![code("Source")]),
        None,
    );
    assert!(
        mirage
            .violations
            .iter()
            .any(|v| matches!(v, Violation::EdgeTargetUnknown { .. })),
        "a genuinely unknown target still does: {:?}",
        mirage.violations
    );
}

#[test]
fn the_target_side_mirror_of_the_source_side_marker_skip() {
    let (input, code) = edge_into(target(Marker::Draft, Polarity::Declared), false);
    let outcome = super::diff(input, code, None);
    assert!(
        outcome.is_clean(),
        "a draft-marked absent target bears no code-existence demand: {:?}",
        outcome.violations
    );
    assert_eq!(outcome.pending.len(), 1, "and it is still pending");
}

#[test]
fn the_source_side_per_name_conversion_stays_permissive() {
    let ctx = ContextDecl::new(
        "eq".to_owned(),
        vec![OwnedUnit("domain".to_owned())],
        vec![],
        vec![],
        Source::Spec {
            path: spec_path(),
            line: 1,
            context: None,
        },
    );
    let anchor = VerbAnchor {
        concept: "T".to_owned(),
        qname: "build_t".to_owned(),
        raw_target: "verb: build_t".to_owned(),
        source: Source::Spec {
            path: spec_path(),
            line: 3,
            context: None,
        },
    };
    let specs = Graph::new(
        vec![spec_with_polarity("T", Polarity::Illustrative), spec("T")],
        Vec::new(),
    );
    let outcome = super::diff(
        CheckInput::new(
            specs,
            vec![ctx],
            VerbOwnership {
                decls: vec![],
                anchors: vec![anchor],
            },
        ),
        Graph::new(
            vec![ConceptNode::new(
                "T".to_owned(),
                Source::Code {
                    path: PathBuf::from("domain/src/lib.rs"),
                    line: 7,
                    provenance: Provenance::empty(),
                    location: LocationKind::Path,
                },
                SignatureState::Absent,
            )],
            Vec::new(),
        ),
        None,
    );
    assert!(
        !outcome
            .violations
            .iter()
            .any(|v| matches!(v, Violation::VerbMissingInCode { .. })),
        "one unobliged heading carries the whole name on the source side: {:?}",
        outcome.violations
    );
}
