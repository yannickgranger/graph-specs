//! v0.5 verb-anchoring pass — fifth pass in [`crate::diff::diff`].
//!
//! Opt-in per context: a context with zero `- verb:` anchors is not
//! inspected. Once a context is verb-anchored (at least one concept in
//! it carries a `- verb:` bullet), all `pub fn` declarations in that
//! context must be claimed by at least one anchor.
//!
//! Emits three [`crate::Violation`] variants and one
//! [`crate::ContextViolation`] variant:
//! - `VerbMissingInCode` — anchor references a qname absent from code
//! - `VerbTargetUnknown` — qname exists in code but no context claims it
//! - `Context(CrossVerbUnauthorized)` — qname is in a different context
//! - `VerbMissingInSpec` — pub fn in verb-anchored context has no anchor

use crate::{
    context::context_for_concept, ContextDecl, ContextViolation, Graph, VerbAnchor, VerbDecl,
    VerbOwnership, Violation,
};
use std::collections::{HashMap, HashSet};

pub(super) fn verb_pass(
    verb_ownership: VerbOwnership,
    code: &Graph,
    contexts: &[ContextDecl],
    unobliged_concepts: &HashSet<String>,
    out: &mut Vec<Violation>,
) {
    // Owned so `emit_missing_in_spec` can move each decl's `qname`/`source`
    // into the emitted violation instead of cloning per loop iteration.
    let VerbOwnership { decls, mut anchors } = verb_ownership;
    // The uniform obligation skip (RFC-013 §3.4 pending, RFC-014 §3.3
    // non-`declared`). A `- verb:` bullet under a concept that compels
    // nothing imposes nothing: there is no backing item to compare against,
    // and firing `VerbMissingInCode` would contradict the heading's own
    // meaning. Dropping the anchors (rather than post-filtering the
    // violations) also keeps a context whose only anchors sit on unobliged
    // concepts correctly *un*-opted-in, so no `VerbMissingInSpec` fires
    // against its pub fns either.
    if !unobliged_concepts.is_empty() {
        anchors.retain(|a| !unobliged_concepts.contains(a.concept.as_str()));
    }
    if anchors.is_empty() {
        return;
    }

    let unit_to_context = build_unit_context_map(contexts);
    let decl_by_qname = build_decl_map(&decls);

    let mut context_claimed_qnames: HashMap<&str, HashSet<&str>> = HashMap::new();

    for anchor in &anchors {
        let concept_ctx =
            context_for_concept(code, contexts, &anchor.concept).map(|c| c.name.as_str());

        check_anchor(anchor, &decl_by_qname, &unit_to_context, concept_ctx, out);

        if let Some(ctx) = concept_ctx {
            context_claimed_qnames
                .entry(ctx)
                .or_default()
                .insert(anchor.qname.as_str());
        }
    }

    // The map keys are concept names (anchor.concept). The impl-method branch of
    // emit_missing_in_spec checks `concepts_in_ctx.contains(type_root)` where
    // `type_root` is the Type portion of `Type::method`. This works iff concept
    // name == type name — the existing graph-specs dialect invariant.
    let mut opted_in_concepts_by_context: HashMap<&str, HashSet<&str>> = HashMap::new();
    for anchor in &anchors {
        if let Some(ctx) = context_for_concept(code, contexts, &anchor.concept) {
            opted_in_concepts_by_context
                .entry(ctx.name.as_str())
                .or_default()
                .insert(anchor.concept.as_str());
        }
    }

    // `decl_by_qname` (borrowing `decls`) is no longer used past the anchor
    // loop, so `decls` can be moved into the emitter.
    drop(decl_by_qname);
    emit_missing_in_spec(
        decls,
        &unit_to_context,
        &context_claimed_qnames,
        &opted_in_concepts_by_context,
        out,
    );
}

fn build_unit_context_map(contexts: &[ContextDecl]) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for ctx in contexts {
        for unit in &ctx.owned_units {
            map.insert(unit.0.as_str(), ctx.name.as_str());
        }
    }
    map
}

fn build_decl_map(decls: &[VerbDecl]) -> HashMap<&str, Vec<&VerbDecl>> {
    let mut map: HashMap<&str, Vec<&VerbDecl>> = HashMap::new();
    for d in decls {
        map.entry(d.qname.as_str()).or_default().push(d);
    }
    map
}

fn check_anchor(
    anchor: &VerbAnchor,
    decl_by_qname: &HashMap<&str, Vec<&VerbDecl>>,
    unit_to_context: &HashMap<&str, &str>,
    concept_ctx: Option<&str>,
    out: &mut Vec<Violation>,
) {
    let Some(matching_decls) = decl_by_qname.get(anchor.qname.as_str()) else {
        out.push(Violation::VerbMissingInCode {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            spec_source: anchor.source.clone(),
        });
        return;
    };

    let decl_contexts: Vec<&str> = matching_decls
        .iter()
        .filter_map(|d| {
            d.owned_unit
                .as_deref()
                .and_then(|u| unit_to_context.get(u).copied())
        })
        .collect();

    if decl_contexts.is_empty() {
        out.push(Violation::VerbTargetUnknown {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            spec_source: anchor.source.clone(),
        });
        return;
    }

    let Some(concept_ctx_name) = concept_ctx else {
        return;
    };

    if decl_contexts.contains(&concept_ctx_name) {
        return;
    }

    let target_ctx = decl_contexts[0];
    out.push(Violation::Context(
        ContextViolation::CrossVerbUnauthorized {
            concept: anchor.concept.clone(),
            qname: anchor.qname.clone(),
            owning_context: concept_ctx_name.to_owned(),
            target_context: target_ctx.to_owned(),
            spec_source: anchor.source.clone(),
        },
    ));
}

fn emit_missing_in_spec(
    decls: Vec<VerbDecl>,
    unit_to_context: &HashMap<&str, &str>,
    context_claimed_qnames: &HashMap<&str, HashSet<&str>>,
    opted_in_concepts_by_context: &HashMap<&str, HashSet<&str>>,
    out: &mut Vec<Violation>,
) {
    for decl in decls {
        let Some(unit) = decl.owned_unit.as_deref() else {
            continue;
        };
        let Some(&decl_ctx) = unit_to_context.get(unit) else {
            continue;
        };

        // Fast-path exit if already claimed in this context.
        if context_claimed_qnames
            .get(decl_ctx)
            .is_some_and(|claimed| claimed.contains(decl.qname.as_str()))
        {
            continue;
        }

        // Decide whether this decl is missing an anchor. Impl-method qnames
        // (`Type::method`) opt in per-concept (context-scoped); bare-ident
        // free fns opt in per-context; a leading `::` is malformed → skip.
        let fires = match decl.qname.split_once("::") {
            Some(("", _)) => false,
            Some((type_root, _method)) => opted_in_concepts_by_context
                .get(decl_ctx)
                .is_some_and(|concepts_in_ctx| concepts_in_ctx.contains(type_root)),
            None => opted_in_concepts_by_context.contains_key(decl_ctx),
        };

        if fires {
            // All borrows of `decl` have ended; move its fields out — no clone.
            out.push(Violation::VerbMissingInSpec {
                qname: decl.qname,
                code_source: decl.source,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConceptNode, OwnedUnit, Source};
    use std::path::PathBuf;

    fn code_src(path: &str) -> Source {
        Source::Code {
            path: PathBuf::from(path),
            line: 1,
        }
    }

    fn spec_src() -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/core.md"),
            line: 10,
        }
    }

    fn make_ctx(name: &str, units: &[&str]) -> ContextDecl {
        ContextDecl::new(
            name.to_owned(),
            units.iter().map(|u| OwnedUnit((*u).to_owned())).collect(),
            vec![],
            vec![],
            spec_src(),
        )
    }

    fn make_code_node(name: &str, unit: &str) -> ConceptNode {
        ConceptNode::new(
            name.to_owned(),
            code_src(&format!("{unit}/src/lib.rs")),
            crate::SignatureState::Absent,
        )
    }

    fn make_decl(qname: &str, unit: &str) -> VerbDecl {
        VerbDecl {
            qname: qname.to_owned(),
            owned_unit: Some(unit.to_owned()),
            source: code_src(&format!("{unit}/src/lib.rs")),
        }
    }

    fn make_anchor(concept: &str, qname: &str) -> VerbAnchor {
        VerbAnchor {
            concept: concept.to_owned(),
            qname: qname.to_owned(),
            raw_target: format!("verb: {qname}"),
            source: spec_src(),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn run(
        anchors: Vec<VerbAnchor>,
        decls: Vec<VerbDecl>,
        code_nodes: Vec<ConceptNode>,
        contexts: Vec<ContextDecl>,
    ) -> Vec<Violation> {
        let code = Graph::new(code_nodes, vec![]);
        let verb_ownership = VerbOwnership { decls, anchors };
        let mut out = Vec::new();
        verb_pass(verb_ownership, &code, &contexts, &HashSet::new(), &mut out);
        out
    }

    /// As [`run`], but with `concept` pending (RFC-013 §3.4).
    #[allow(clippy::needless_pass_by_value)]
    fn run_with_pending(
        pending: &str,
        anchors: Vec<VerbAnchor>,
        decls: Vec<VerbDecl>,
        code_nodes: Vec<ConceptNode>,
        contexts: Vec<ContextDecl>,
    ) -> Vec<Violation> {
        let code = Graph::new(code_nodes, vec![]);
        let verb_ownership = VerbOwnership { decls, anchors };
        let mut out = Vec::new();
        let pending_concepts: HashSet<String> = std::iter::once(pending.to_owned()).collect();
        verb_pass(
            verb_ownership,
            &code,
            &contexts,
            &pending_concepts,
            &mut out,
        );
        out
    }

    #[test]
    fn pending_concepts_verb_anchors_impose_no_obligation() {
        // RFC-013 §3.4 uniform obligation skip: a `- verb:` bullet under a
        // pending concept would otherwise fire `VerbMissingInCode` and make
        // matrix row 3 unreachable.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Reconciler", "reconcile");
        let node = make_code_node("Reconciler", "domain");
        let armed = run(
            vec![anchor.clone()],
            vec![],
            vec![node.clone()],
            vec![ctx.clone()],
        );
        assert!(
            armed
                .iter()
                .any(|v| matches!(v, Violation::VerbMissingInCode { .. })),
            "control: an unmarked concept's dangling verb anchor still fires: {armed:?}"
        );

        let pending = run_with_pending("Reconciler", vec![anchor], vec![], vec![node], vec![ctx]);
        assert!(
            pending.is_empty(),
            "pending concept's verb anchors must impose nothing: {pending:?}"
        );
    }

    #[test]
    fn pending_concept_does_not_opt_its_context_into_verb_anchoring() {
        // Dropping the anchor (rather than post-filtering violations) keeps a
        // context whose only anchors sit on pending concepts un-opted-in, so
        // its pub fns raise no `VerbMissingInSpec` either.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Reconciler", "reconcile");
        let decl = make_decl("unrelated_fn", "domain");
        let node = make_code_node("Reconciler", "domain");
        let out = run_with_pending(
            "Reconciler",
            vec![anchor],
            vec![decl],
            vec![node],
            vec![ctx],
        );
        assert!(
            !out.iter()
                .any(|v| matches!(v, Violation::VerbMissingInSpec { .. })),
            "pending-only context must not be verb-anchored: {out:?}"
        );
    }

    #[test]
    fn empty_anchors_is_noop() {
        let ctx = make_ctx("eq", &["domain"]);
        let decl = make_decl("diff", "domain");
        let node = make_code_node("Graph", "domain");
        let violations = run(vec![], vec![decl], vec![node], vec![ctx]);
        assert!(violations.is_empty());
    }

    #[test]
    fn verb_missing_in_code_when_no_decl_matches() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "nonexistent_fn");
        let node = make_code_node("Graph", "domain");
        let violations = run(vec![anchor], vec![], vec![node], vec![ctx]);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            Violation::VerbMissingInCode { qname, .. } if qname == "nonexistent_fn"
        ));
    }

    #[test]
    fn verb_target_unknown_when_decl_has_no_context() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "orphan_fn");
        let node = make_code_node("Graph", "domain");
        let decl = VerbDecl {
            qname: "orphan_fn".to_owned(),
            owned_unit: Some("unknown_crate".to_owned()),
            source: code_src("unknown_crate/src/lib.rs"),
        };
        let violations = run(vec![anchor], vec![decl], vec![node], vec![ctx]);
        assert_eq!(violations.len(), 1);
        assert!(matches!(
            &violations[0],
            Violation::VerbTargetUnknown { qname, .. } if qname == "orphan_fn"
        ));
    }

    #[test]
    fn cross_verb_unauthorized_when_decl_in_different_context() {
        let ctx_a = make_ctx("ctx_a", &["crate_a"]);
        let ctx_b = make_ctx("ctx_b", &["crate_b"]);
        let anchor = make_anchor("ConceptA", "fn_in_b");
        let node_a = make_code_node("ConceptA", "crate_a");
        let decl = make_decl("fn_in_b", "crate_b");
        let violations = run(vec![anchor], vec![decl], vec![node_a], vec![ctx_a, ctx_b]);
        assert_eq!(violations.len(), 1);
        assert!(
            matches!(
                &violations[0],
                Violation::Context(ContextViolation::CrossVerbUnauthorized {
                    qname, owning_context, target_context, ..
                }) if qname == "fn_in_b"
                    && owning_context == "ctx_a"
                    && target_context == "ctx_b"
            ),
            "expected CrossVerbUnauthorized, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn same_context_verb_anchor_is_valid() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl = make_decl("diff", "domain");
        let violations = run(vec![anchor], vec![decl], vec![node], vec![ctx]);
        assert!(
            violations.is_empty(),
            "expected no violations, got: {violations:?}"
        );
    }

    #[test]
    fn verb_missing_in_spec_when_unclaimed_fn_in_anchored_context() {
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl_diff = make_decl("diff", "domain");
        let decl_unclaimed = make_decl("unclaimed_fn", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_diff, decl_unclaimed],
            vec![node],
            vec![ctx],
        );
        assert_eq!(violations.len(), 1);
        assert!(
            matches!(&violations[0], Violation::VerbMissingInSpec { qname, .. } if qname == "unclaimed_fn"),
            "expected VerbMissingInSpec for unclaimed_fn, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn context_with_no_anchors_not_inspected_for_missing_in_spec() {
        let ctx_anchored = make_ctx("eq", &["domain"]);
        let ctx_not_anchored = make_ctx("other", &["other_crate"]);
        let anchor = make_anchor("Graph", "diff");
        let node = make_code_node("Graph", "domain");
        let decl_diff = make_decl("diff", "domain");
        let decl_other = make_decl("other_fn", "other_crate");
        let violations = run(
            vec![anchor],
            vec![decl_diff, decl_other],
            vec![node],
            vec![ctx_anchored, ctx_not_anchored],
        );
        assert!(
            violations.is_empty(),
            "other_fn is in a non-anchored context and must not fire VerbMissingInSpec: {violations:?}"
        );
    }

    #[test]
    fn impl_method_in_anchored_concept_fires_missing_in_spec() {
        // Foo has anchor Foo::baz (claimed). Foo::bar is in the same context
        // but not claimed. Because Foo is opted-in, Foo::bar fires VerbMissingInSpec.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Foo", "Foo::baz");
        let node = make_code_node("Foo", "domain");
        let decl_claimed = make_decl("Foo::baz", "domain");
        let decl_unclaimed = make_decl("Foo::bar", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_claimed, decl_unclaimed],
            vec![node],
            vec![ctx],
        );
        assert_eq!(
            violations.len(),
            1,
            "expected exactly one violation: {violations:?}"
        );
        assert!(
            matches!(&violations[0], Violation::VerbMissingInSpec { qname, .. } if qname == "Foo::bar"),
            "expected VerbMissingInSpec for Foo::bar, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn impl_method_in_non_anchored_concept_does_not_fire() {
        // Bar is opted-in (has anchor bar_fn). Foo has no anchors.
        // Foo::bar must not fire because Foo is not in opted_in_concepts.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Bar", "bar_fn");
        let node_foo = make_code_node("Foo", "domain");
        let node_bar = make_code_node("Bar", "domain");
        let decl_bar_fn = make_decl("bar_fn", "domain");
        let decl_foo_bar = make_decl("Foo::bar", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_bar_fn, decl_foo_bar],
            vec![node_foo, node_bar],
            vec![ctx],
        );
        assert!(
            violations.is_empty(),
            "Foo has no anchors so Foo::bar must not fire VerbMissingInSpec: {violations:?}"
        );
    }

    #[test]
    fn impl_method_anchored_concept_in_different_context_does_not_fire() {
        // Foo (in ctx_a) is opted-in. Foo::bar decl is in ctx_b.
        // The concept-scoped lookup must not cross context boundaries.
        let ctx_a = make_ctx("ctx_a", &["crate_a"]);
        let ctx_b = make_ctx("ctx_b", &["crate_b"]);
        let anchor = make_anchor("Foo", "Foo::baz");
        let node_foo = make_code_node("Foo", "crate_a");
        let decl_in_a = make_decl("Foo::baz", "crate_a");
        let decl_in_b = make_decl("Foo::bar", "crate_b");
        let violations = run(
            vec![anchor],
            vec![decl_in_a, decl_in_b],
            vec![node_foo],
            vec![ctx_a, ctx_b],
        );
        assert!(
            violations.is_empty(),
            "Foo::bar is in ctx_b where Foo is not opted-in; must not fire: {violations:?}"
        );
    }

    #[test]
    fn free_fn_fires_missing_in_spec_under_per_context_activation() {
        // SomeConcept is opted-in in ctx. A bare-ident unclaimed_fn in the
        // same context must fire VerbMissingInSpec via the free-fn branch.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("SomeConcept", "some_fn");
        let node = make_code_node("SomeConcept", "domain");
        let decl_claimed = make_decl("some_fn", "domain");
        let decl_unclaimed = make_decl("unclaimed_fn", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_claimed, decl_unclaimed],
            vec![node],
            vec![ctx],
        );
        assert_eq!(
            violations.len(),
            1,
            "expected exactly one violation: {violations:?}"
        );
        assert!(
            matches!(&violations[0], Violation::VerbMissingInSpec { qname, .. } if qname == "unclaimed_fn"),
            "expected VerbMissingInSpec for unclaimed_fn, got: {:?}",
            violations[0]
        );
    }

    #[test]
    fn malformed_leading_colons_does_not_panic_or_fire() {
        // A decl with qname "::orphan" matches Some(("", _)) and is silently skipped.
        let ctx = make_ctx("eq", &["domain"]);
        let anchor = make_anchor("Foo", "Foo::baz");
        let node = make_code_node("Foo", "domain");
        let decl_baz = make_decl("Foo::baz", "domain");
        let decl_malformed = make_decl("::orphan", "domain");
        let violations = run(
            vec![anchor],
            vec![decl_baz, decl_malformed],
            vec![node],
            vec![ctx],
        );
        assert!(
            violations.is_empty(),
            "::orphan must be silently skipped with no panic or violation: {violations:?}"
        );
    }
}
