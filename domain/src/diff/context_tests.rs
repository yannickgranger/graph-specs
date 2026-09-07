use crate::ConceptRef;
use crate::LocationKind;
use crate::Provenance;
use crate::{
    detect_import_cycle, CheckInput, ConceptNode, ContextDecl, ContextExport, ContextImport,
    ContextPattern, ContextViolation, Edge, EdgeKind, Graph, OwnedUnit, SignatureState, Source,
    VerbOwnership, Violation,
};
use std::path::PathBuf;

fn diff(spec: CheckInput, code: Graph) -> Vec<Violation> {
    crate::diff(spec, code, None).violations
}

fn code_node(name: &str, unit: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Code {
            language: crate::CodeLanguage::Rust,
            path: PathBuf::from(format!("./{unit}/src/lib.rs")),
            line: 1,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
        },
        SignatureState::Absent,
    )
}

fn code_edge(src: &str, kind: EdgeKind, target: &str) -> Edge {
    Edge {
        source_concept: ConceptRef::named(src.to_string()),
        kind,
        target: ConceptRef::named(target.to_string()),
        raw_target: target.to_string(),
        source: Source::Code {
            language: crate::CodeLanguage::Rust,
            path: PathBuf::from("./x/src/lib.rs"),
            line: 10,
            provenance: Provenance::empty(),
            location: LocationKind::Path,
        },
    }
}

fn spec_src() -> Source {
    Source::Spec {
        format: crate::SpecFormat::Markdown,
        path: PathBuf::from("specs/contexts/x.md"),
        line: 1,
        context: None,
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
    CheckInput::new(graph, contexts, VerbOwnership::default())
        .expect("the declarations under test declare one surface")
}

#[test]
fn empty_contexts_skip_context_pass() {
    let spec = Graph::new(vec![code_node("Foo", "domain")], vec![]);
    let code = Graph::new(vec![code_node("Foo", "domain")], vec![]);
    let v = diff(ci(spec, vec![]), code);
    assert!(
        v.iter().all(|v| !matches!(v, Violation::Context(_))),
        "no Context variants when contexts empty"
    );
}

#[test]
fn v03_regression_preserved_when_contexts_empty() {
    let spec_node = ConceptNode::new(
        "SpecOnly".into(),
        Source::Spec {
            format: crate::SpecFormat::Markdown,
            path: PathBuf::from("x.md"),
            line: 1,
            context: None,
        },
        SignatureState::Absent,
    );
    let spec = Graph::new(vec![spec_node], vec![]);
    let code = Graph::new(vec![code_node("CodeOnly", "domain")], vec![]);
    let v = diff(ci(spec, vec![]), code);
    assert!(v
        .iter()
        .any(|v| matches!(v, Violation::MissingInCode { .. })));
    assert!(v
        .iter()
        .any(|v| matches!(v, Violation::MissingInSpecs { .. })));
}

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
        !v.iter().any(|v| matches!(
            v,
            Violation::Context(ContextViolation::MembershipUnknown { .. })
        )),
        "no MembershipUnknown expected"
    );
}

#[test]
fn multi_level_unit_is_matched_by_path() {
    let code = Graph::new(
        vec![code_node("MarkdownReader", "adapters/markdown")],
        vec![],
    );
    let contexts = vec![ctx("reading", &["adapters/markdown"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(
        !v.iter().any(|v| matches!(
            v,
            Violation::Context(ContextViolation::MembershipUnknown { .. })
        )),
        "multi-segment unit should be matched"
    );
}

#[test]
fn intra_context_edge_is_not_cross_context() {
    let code = Graph::new(
        vec![code_node("A", "domain"), code_node("B", "domain")],
        vec![code_edge("A", EdgeKind::DependsOn, "B")],
    );
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(v.iter().all(|v| !matches!(
        v,
        Violation::Context(ContextViolation::CrossEdgeUnauthorized { .. })
    )));
}

#[test]
fn cross_context_edge_unauthorized_without_matching_import() {
    let code = Graph::new(
        vec![
            code_node("Reader", "ports"),
            code_node("MR", "adapters/markdown"),
        ],
        vec![code_edge("MR", EdgeKind::Implements, "Reader")],
    );
    let contexts = vec![
        ctx("eq", &["ports"], vec![], vec![]),
        ctx("reading", &["adapters/markdown"], vec![], vec![]),
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
        vec![
            code_node("Reader", "ports"),
            code_node("MR", "adapters/markdown"),
        ],
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
        v.iter().all(|v| !matches!(v, Violation::Context(_))),
        "authorized cross-context edge should produce no Context violations, got {v:?}"
    );
}

#[test]
fn cross_context_edge_undeclared_when_supplier_does_not_export() {
    let code = Graph::new(
        vec![
            code_node("Secret", "ports"),
            code_node("MR", "adapters/markdown"),
        ],
        vec![code_edge("MR", EdgeKind::DependsOn, "Secret")],
    );
    let contexts = vec![
        ctx("eq", &["ports"], vec![], vec![]),
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
            vec![im("b", ContextPattern::Conformist, "BB")],
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
    let contexts = vec![ctx(
        "a",
        &["a"],
        vec![],
        vec![im("nonexistent", ContextPattern::Conformist, "Q")],
    )];
    assert!(detect_import_cycle(&contexts).is_none());
}

#[test]
fn context_violations_sort_after_edge_variants() {
    let code = Graph::new(vec![code_node("X", "stray")], vec![]);
    let contexts = vec![ctx("eq", &["domain"], vec![], vec![])];
    let v = diff(ci(Graph::default(), contexts), code);
    assert!(!v.is_empty());
}

fn code_node_with_provenance(name: &str, unit: &str) -> ConceptNode {
    code_node(name, unit).with_provenance(Some(unit.to_owned()), Some(unit.to_owned()), None)
}

fn spec_node_in(name: &str, context: &str) -> ConceptNode {
    ConceptNode::new(
        name.to_string(),
        Source::Spec {
            format: crate::SpecFormat::Markdown,
            path: PathBuf::from(format!("specs/concepts/{context}.md")),
            line: 1,
            context: Some(context.to_string()),
        },
        SignatureState::Absent,
    )
}

fn mismatches(violations: &[Violation]) -> Vec<(String, String, Option<String>)> {
    violations
        .iter()
        .filter_map(|v| match v {
            Violation::Cohesion(crate::CohesionViolation::ConceptContextMismatch {
                concept,
                code_context,
                code_source,
                ..
            }) => Some((
                concept.clone(),
                code_context.clone(),
                code_source
                    .as_ref()
                    .and_then(|s| s.unit().map(str::to_owned)),
            )),
            _ => None,
        })
        .collect()
}

fn leftovers(violations: &[Violation]) -> Vec<String> {
    violations
        .iter()
        .filter_map(|v| match v {
            Violation::MissingInSpecs { code_source, .. } => code_source.unit().map(str::to_owned),
            _ => None,
        })
        .collect()
}

#[test]
fn same_named_items_under_two_units_each_bind_their_own_heading() {
    let spec = Graph::new(
        vec![
            spec_node_in("Clock", "scheduling"),
            spec_node_in("Clock", "privacy"),
        ],
        vec![],
    );
    let code = Graph::new(
        vec![
            code_node_with_provenance("Clock", "scheduling"),
            code_node_with_provenance("Clock", "privacy"),
        ],
        vec![],
    );
    let contexts = vec![
        ctx("scheduling", &["scheduling"], vec![], vec![]),
        ctx("privacy", &["privacy"], vec![], vec![]),
    ];

    let violations = diff(ci(spec, contexts), code);

    assert_eq!(
        mismatches(&violations),
        Vec::new(),
        "each heading binds the item of its own unit, so neither reports a mismatch"
    );
    assert_eq!(
        leftovers(&violations),
        Vec::<String>::new(),
        "and both items are bound: {violations:?}"
    );
}

#[test]
fn one_heading_among_two_items_binds_the_one_of_its_own_context() {
    let spec = Graph::new(vec![spec_node_in("Clock", "privacy")], vec![]);
    let code = Graph::new(
        vec![
            code_node_with_provenance("Clock", "enrolment"),
            code_node_with_provenance("Clock", "privacy"),
        ],
        vec![],
    );
    let contexts = vec![
        ctx("enrolment", &["enrolment"], vec![], vec![]),
        ctx("privacy", &["privacy"], vec![], vec![]),
    ];

    let violations = diff(ci(spec, contexts), code);

    assert_eq!(
        leftovers(&violations),
        vec!["enrolment".to_string()],
        "the privacy heading binds the privacy item, so the enrolment item is what is left \
         undescribed — a binder that took the first item of the name would leave privacy \
         instead: {violations:?}"
    );
    assert!(
        !violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { .. })),
        "and the heading itself binds: {violations:?}"
    );
}

#[test]
fn a_heading_in_a_context_owning_no_item_of_its_name_binds_nothing_at_all() {
    let spec = Graph::new(vec![spec_node_in("Clock", "reading")], vec![]);
    let code = Graph::new(
        vec![
            code_node_with_provenance("Clock", "enrolment"),
            code_node_with_provenance("Clock", "privacy"),
        ],
        vec![],
    );
    let contexts = vec![
        ctx("reading", &["reading"], vec![], vec![]),
        ctx("enrolment", &["enrolment"], vec![], vec![]),
        ctx("privacy", &["privacy"], vec![], vec![]),
    ];

    let violations = diff(ci(spec, contexts), code);

    assert!(
        violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "Clock")),
        "the heading's context owns no Clock, so it binds nothing: {violations:?}"
    );
    let mut left = leftovers(&violations);
    left.sort();
    assert_eq!(
        left,
        vec!["enrolment".to_string(), "privacy".to_string()],
        "and both items stay undescribed — a binder falling back to the first item of the name \
         would bind one of them and report neither it nor the heading: {violations:?}"
    );
}

#[test]
fn two_foreign_items_of_one_name_are_two_records_told_apart_by_unit() {
    let spec = Graph::new(vec![spec_node_in("Clock", "reading")], vec![]);
    let code = Graph::new(
        vec![
            code_node_with_provenance("Clock", "domain/enrolment"),
            code_node_with_provenance("Clock", "domain/privacy"),
        ],
        vec![],
    );
    let contexts = vec![
        ctx("reading", &["reading"], vec![], vec![]),
        ctx(
            "modelling",
            &["domain/enrolment", "domain/privacy"],
            vec![],
            vec![],
        ),
    ];

    let mut found = mismatches(&diff(ci(spec, contexts), code));
    found.sort();

    assert_eq!(
        found,
        vec![
            (
                "Clock".to_string(),
                "modelling".to_string(),
                Some("domain/enrolment".to_string())
            ),
            (
                "Clock".to_string(),
                "modelling".to_string(),
                Some("domain/privacy".to_string())
            ),
        ],
        "two items under two units of one foreign context are two records, told apart by unit"
    );
}

#[test]
fn a_unit_below_a_declared_prefix_resolves_to_the_declaring_context() {
    let node = code_node_with_provenance("Clock", "domain/enrolment/deep");
    let contexts = vec![ctx("modelling", &["domain"], vec![], vec![])];

    let resolved = crate::context::context_for_code_node(&node, &contexts);
    assert_eq!(
        resolved.map(|c| c.name.as_str()),
        Some("modelling"),
        "the Owns prefix `domain` covers the unit `domain/enrolment/deep`, so the item resolves \
         to modelling — an equality match against the declared string resolves to nothing"
    );

    let spec = Graph::new(vec![spec_node_in("Clock", "modelling")], vec![]);
    let code = Graph::new(vec![node], vec![]);
    let violations = diff(ci(spec, contexts), code);

    assert!(
        !violations.iter().any(|v| matches!(
            v,
            Violation::MissingInCode { .. } | Violation::MissingInSpecs { .. }
        )),
        "and the heading binds it, so neither side is reported missing — under an equality match \
         the item resolves to no context and the heading binds nothing: {violations:?}"
    );
    assert_eq!(mismatches(&violations), Vec::new());
}

#[test]
fn a_unit_below_a_declared_prefix_is_a_member_of_that_context() {
    let spec = Graph::new(vec![spec_node_in("Clock", "modelling")], vec![]);
    let code = Graph::new(
        vec![code_node_with_provenance("Clock", "domain/enrolment/deep")],
        vec![],
    );
    let contexts = vec![ctx("modelling", &["domain"], vec![], vec![])];

    let violations = diff(ci(spec, contexts), code);

    assert!(
        !violations.iter().any(|v| matches!(
            v,
            Violation::Context(ContextViolation::MembershipUnknown { .. })
        )),
        "the Owns prefix `domain` covers the unit `domain/enrolment/deep`, so the item is a \
         member of modelling — an equality match against the declared string makes it an orphan: \
         {violations:?}"
    );
    assert!(
        violations.is_empty(),
        "and nothing else is reported either: {violations:?}"
    );
}

#[test]
fn a_unit_no_declared_prefix_covers_is_still_an_orphan() {
    let spec = Graph::new(vec![spec_node_in("Clock", "modelling")], vec![]);
    let code = Graph::new(
        vec![code_node_with_provenance("Clock", "vendor/elsewhere")],
        vec![],
    );
    let contexts = vec![ctx("modelling", &["domain"], vec![], vec![])];

    let violations = diff(ci(spec, contexts), code);

    assert!(
        violations.iter().any(|v| matches!(
            v,
            Violation::Context(ContextViolation::MembershipUnknown { owned_unit, .. })
                if owned_unit.0 == "vendor/elsewhere"
        )),
        "no declared prefix covers `vendor/elsewhere`, so the item is off the surface and the \
         prefix rule must not swallow it: {violations:?}"
    );
}

#[test]
fn nested_prefixes_cannot_build_a_check_input() {
    let spec = Graph::new(vec![spec_node_in("Clock", "outer")], vec![]);
    let contexts = vec![
        ctx("outer", &["domain"], vec![], vec![]),
        ctx("inner", &["domain/enrolment"], vec![], vec![]),
    ];

    let refused = CheckInput::new(spec, contexts, VerbOwnership::default());

    let ambiguity = refused.expect_err(
        "two contexts nesting their prefixes declare no surface, so the input the diff reads \
         cannot be built from them",
    );
    assert_eq!(ambiguity.outer.0, "domain");
    assert_eq!(ambiguity.outer_context, "outer");
    assert_eq!(ambiguity.inner.0, "domain/enrolment");
    assert_eq!(ambiguity.inner_context, "inner");
}

#[test]
fn a_heading_whose_item_is_absent_reads_missing_not_a_mismatch() {
    let spec = Graph::new(
        vec![
            spec_node_in("UnknownCourse", "scheduling"),
            spec_node_in("UnknownCourse", "enrolment"),
        ],
        vec![],
    );
    let code = Graph::new(
        vec![code_node_with_provenance("UnknownCourse", "enrolment")],
        vec![],
    );
    let contexts = vec![
        ctx("scheduling", &["scheduling"], vec![], vec![]),
        ctx("enrolment", &["enrolment"], vec![], vec![]),
    ];

    let violations = diff(ci(spec, contexts), code);

    assert_eq!(
        mismatches(&violations),
        Vec::new(),
        "enrolment's own heading binds the enrolment item, so scheduling's heading is not \
         compared with it: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|v| matches!(v, Violation::MissingInCode { name, .. } if name == "UnknownCourse")),
        "scheduling's heading reads missing in code, which is what an absent item is: \
         {violations:?}"
    );
}
