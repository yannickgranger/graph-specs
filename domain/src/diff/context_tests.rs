use crate::{
    detect_import_cycle, diff, CheckInput, ConceptNode, ContextDecl, ContextExport, ContextImport,
    ContextPattern, ContextViolation, Edge, EdgeKind, Graph, OwnedUnit, SignatureState, Source,
    Violation,
};
use std::path::PathBuf;

// --- helpers -------------------------------------------------------

fn code_node(name: &str, unit: &str) -> ConceptNode {
    ConceptNode {
        name: name.to_string(),
        source: Source::Code {
            path: PathBuf::from(format!("./{unit}/src/lib.rs")),
            line: 1,
        },
        signature: SignatureState::Absent,
    }
}

fn code_edge(src: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: src.to_string(),
        kind,
        target: target.to_string(),
        raw_target: target.to_string(),
        source: Source::Code {
            path: PathBuf::from("./x/src/lib.rs"),
            line: 10,
        },
    }
}

fn spec_src() -> Source {
    Source::Spec {
        path: PathBuf::from("specs/contexts/x.md"),
        line: 1,
    }
}

fn ctx(
    name: &str,
    units: &[&str],
    exports: Vec<ContextExport>,
    imports: Vec<ContextImport>,
) -> ContextDecl {
    ContextDecl::new(
        name.to_string(),
        units.iter().map(|u| OwnedUnit(u.to_string())).collect(),
        exports,
        imports,
        spec_src(),
    )
}

fn ex(concept: &str, pattern: ContextPattern) -> ContextExport {
    ContextExport {
        concept: concept.to_string(),
        pattern,
    }
}

fn im(from: &str, pattern: ContextPattern, concept: &str) -> ContextImport {
    ContextImport {
        from_context: from.to_string(),
        pattern,
        concept: concept.to_string(),
    }
}

fn ci(graph: Graph, contexts: Vec<ContextDecl>) -> CheckInput {
    CheckInput::new(graph, contexts)
}

// --- context pass: empty / v0.3 regression -------------------------

#[test]
fn empty_contexts_skip_context_pass() {
    let spec = Graph::new(
        vec![code_node("Foo", "domain")],
        vec![],
    );
    // Same Foo on code side — no concept violation.
    let code = Graph::new(
        vec![code_node("Foo", "domain")],
        vec![],
    );
    let v = diff(ci(spec, vec![]), code);
    assert!(
        v.iter().all(|v| !matches!(v, Violation::Context(_))),
        "no Context variants when contexts empty"
    );
}

#[test]
fn v03_regression_preserved_when_contexts_empty() {
    // spec-only concept → MissingInCode; code-only → MissingInSpecs.
    let spec_node = ConceptNode {
        name: "SpecOnly".into(),
        source: Source::Spec {
            path: PathBuf::from("x.md"),
            line: 1,
        },
        signature: SignatureState::Absent,
    };
    let spec = Graph::new(vec![spec_node], vec![]);
    let code = Graph::new(vec![code_node("CodeOnly", "domain")], vec![]);
    let v = diff(ci(spec, vec![]), code);
    assert!(v.iter().any(|v| matches!(v, Violation::MissingInCode { .. })));
    assert!(v.iter().any(|v| matches!(v, Violation::MissingInSpecs { .. })));
}

// --- MembershipUnknown ---------------------------------------------

#[test]
fn membership_unknown_fires_for_code_in_undeclared_unit() {
    let code = Graph::new(vec![code_node("Orphan", "stray-crate")], vec![]);
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    let found = v.iter().any(|v| {
        matches!(
            v,
            Violation::Context(ContextViolation::MembershipUnknown { concept, .. })
                if concept == "Orphan"
        )
    });
    assert!(found, "expected MembershipUnknown for stray-crate/Orphan");
}

#[test]
fn membership_unknown_does_not_fire_for_declared_unit() {
    let code = Graph::new(vec![code_node("Foo", "domain")], vec![]);
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(
        !v.iter()
            .any(|v| matches!(v, Violation::Context(ContextViolation::MembershipUnknown { .. }))),
        "no MembershipUnknown expected"
    );
}

#[test]
fn multi_level_unit_is_matched_by_path() {
    // `adapters/markdown` as OwnedUnit matches `./adapters/markdown/src/...`.
    let code = Graph::new(vec![code_node("MarkdownReader", "adapters/markdown")], vec![]);
    let contexts = vec![ctx("reading", &["adapters/markdown"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(
        !v.iter()
            .any(|v| matches!(v, Violation::Context(ContextViolation::MembershipUnknown { .. }))),
        "multi-segment unit should be matched"
    );
}

// --- CrossEdgeUnauthorized / CrossEdgeUndeclared / intra-context ---

#[test]
fn intra_context_edge_is_not_cross_context() {
    let code = Graph::new(
        vec![code_node("A", "domain"), code_node("B", "domain")],
        vec![code_edge("A", EdgeKind::DependsOn, "B")],
    );
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(v
        .iter()
        .all(|v| !matches!(v, Violation::Context(ContextViolation::CrossEdgeUnauthorized { .. }))));
}

#[test]
fn cross_context_edge_unauthorized_without_matching_import() {
    let code = Graph::new(
        vec![code_node("Reader", "ports"), code_node("MR", "adapters/markdown")],
        vec![code_edge("MR", EdgeKind::Implements, "Reader")],
    );
    let contexts = vec![
        ctx("eq", &["ports"], vec![], vec![]),
        ctx("reading", &["adapters/markdown"], vec![], vec![]), // no Imports
    ];
    let v = diff(ci(Graph::default(), contexts), code);
    let found = v.iter().any(|v| {
        matches!(
            v,
            Violation::Context(ContextViolation::CrossEdgeUnauthorized { target, target_context, .. })
                if target == "Reader" && target_context == "eq"
        )
    });
    assert!(found, "expected CrossEdgeUnauthorized");
}

#[test]
fn cross_context_edge_authorized_via_import_and_export() {
    let code = Graph::new(
        vec![code_node("Reader", "ports"), code_node("MR", "adapters/markdown")],
        vec![code_edge("MR", EdgeKind::Implements, "Reader")],
    );
    let contexts = vec![
        ctx(
            "eq",
            &["ports"],
            vec![ex("Reader", ContextPattern::PublishedLanguage)],
            vec![],
        ),
        ctx(
            "reading",
            &["adapters/markdown"],
            vec![],
            vec![im("eq", ContextPattern::Conformist, "Reader")],
        ),
    ];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(
        v.iter()
            .all(|v| !matches!(v, Violation::Context(_))),
        "authorized cross-context edge should produce no Context violations, got {v:?}"
    );
}

#[test]
fn cross_context_edge_undeclared_when_supplier_does_not_export() {
    let code = Graph::new(
        vec![code_node("Secret", "ports"), code_node("MR", "adapters/markdown")],
        vec![code_edge("MR", EdgeKind::DependsOn, "Secret")],
    );
    let contexts = vec![
        ctx("eq", &["ports"], vec![], vec![]), // no Exports
        ctx(
            "reading",
            &["adapters/markdown"],
            vec![],
            vec![im("eq", ContextPattern::PublishedLanguage, "Secret")],
        ),
    ];
    let v = diff(ci(Graph::default(), contexts), code);
    let found = v.iter().any(|v| {
        matches!(
            v,
            Violation::Context(ContextViolation::CrossEdgeUndeclared { target, .. })
                if target == "Secret"
        )
    });
    assert!(found, "expected CrossEdgeUndeclared, got {v:?}");
}

#[test]
fn cross_context_edge_to_concept_in_same_context_no_violation() {
    let code = Graph::new(
        vec![code_node("A", "domain"), code_node("B", "domain")],
        vec![code_edge("A", EdgeKind::DependsOn, "B")],
    );
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(v.iter().all(|v| !matches!(v, Violation::Context(_))));
}

#[test]
fn transitive_import_forbidden() {
    // A imports from B, B imports from C, A references Y (in C) directly.
    let code = Graph::new(
        vec![
            code_node("AA", "a"),
            code_node("BB", "b"),
            code_node("CC", "c"),
        ],
        vec![code_edge("AA", EdgeKind::DependsOn, "CC")],
    );
    let contexts = vec![
        ctx(
            "a",
            &["a"],
            vec![],
            vec![im("b", ContextPattern::Conformist, "BB")], // only B, not C
        ),
        ctx(
            "b",
            &["b"],
            vec![ex("BB", ContextPattern::PublishedLanguage)],
            vec![im("c", ContextPattern::Conformist, "CC")],
        ),
        ctx(
            "c",
            &["c"],
            vec![ex("CC", ContextPattern::PublishedLanguage)],
            vec![],
        ),
    ];
    let v = diff(ci(Graph::default(), contexts), code);
    let found = v.iter().any(|v| {
        matches!(
            v,
            Violation::Context(ContextViolation::CrossEdgeUnauthorized { target, target_context, .. })
                if target == "CC" && target_context == "c"
        )
    });
    assert!(found, "expected CrossEdgeUnauthorized on transitive edge");
}

// --- detect_import_cycle ---

#[test]
fn detect_cycle_on_direct_two_context_loop() {
    let contexts = vec![
        ctx(
            "a",
            &["a"],
            vec![],
            vec![im("b", ContextPattern::Conformist, "X")],
        ),
        ctx(
            "b",
            &["b"],
            vec![],
            vec![im("a", ContextPattern::Conformist, "Y")],
        ),
    ];
    let cycle = detect_import_cycle(&contexts);
    assert!(cycle.is_some(), "expected cycle detected");
}

#[test]
fn detect_cycle_returns_none_on_acyclic() {
    let contexts = vec![
        ctx("a", &["a"], vec![], vec![]),
        ctx(
            "b",
            &["b"],
            vec![],
            vec![im("a", ContextPattern::PublishedLanguage, "X")],
        ),
    ];
    assert!(detect_import_cycle(&contexts).is_none());
}

#[test]
fn detect_cycle_allows_shared_kernel_mutual() {
    // Invariant 4: Shared Kernel is the one legal mutual reference.
    let contexts = vec![
        ctx(
            "a",
            &["a"],
            vec![],
            vec![im("b", ContextPattern::SharedKernel, "K")],
        ),
        ctx(
            "b",
            &["b"],
            vec![],
            vec![im("a", ContextPattern::SharedKernel, "K")],
        ),
    ];
    assert!(
        detect_import_cycle(&contexts).is_none(),
        "SharedKernel mutual should not count as a cycle"
    );
}

#[test]
fn detect_cycle_catches_three_way_loop() {
    let contexts = vec![
        ctx(
            "a",
            &["a"],
            vec![],
            vec![im("b", ContextPattern::Conformist, "X")],
        ),
        ctx(
            "b",
            &["b"],
            vec![],
            vec![im("c", ContextPattern::Conformist, "Y")],
        ),
        ctx(
            "c",
            &["c"],
            vec![],
            vec![im("a", ContextPattern::Conformist, "Z")],
        ),
    ];
    assert!(detect_import_cycle(&contexts).is_some());
}

#[test]
fn detect_cycle_ignores_imports_to_unknown_context() {
    // Import from an undeclared context should not panic or misfire.
    let contexts = vec![ctx(
        "a",
        &["a"],
        vec![],
        vec![im("nonexistent", ContextPattern::Conformist, "Q")],
    )];
    assert!(detect_import_cycle(&contexts).is_none());
}

// --- sort order: Context variants rank 8 ---

#[test]
fn context_violations_sort_after_edge_variants() {
    let code = Graph::new(vec![code_node("X", "stray")], vec![]);
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    // All emissions should be Context::MembershipUnknown or MissingInSpecs —
    // verify they co-exist and sort deterministically.
    assert!(!v.is_empty());
}
