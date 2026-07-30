//! Equivalence diff — concept, signature, relationship, bounded-context, verb.
//!
//! Five passes over the spec and code graphs:
//!
//! 1. **Concept** — set-difference over concept names, plus the RFC-013
//!    spec-state-marker rows (see [`concept`]).
//! 2. **Signature** (v0.2) — per matched concept, compare signatures.
//! 3. **Edge** (v0.3) — per matched concept with ≥1 spec edge, compare edges.
//! 4. **Verb** (v0.5) — if `CheckInput.verb_ownership` has anchors, emit
//!    verb-level violations. Order-independent from passes 1–3.
//! 5. **Context** (v0.4) — if `CheckInput.contexts` is non-empty, emit
//!    [`crate::Violation::Context`] variants for membership + cross-context
//!    edges. Order-independent from passes 1–3 (RFC-001 §4 invariant 9).

mod cohesion;
mod concept;
mod context;
mod edge;
mod signature;
mod verb;

#[cfg(test)]
mod tests;

use crate::{
    anchor_violation, CheckInput, CheckOutcome, ConceptNode, ContextDecl, Graph, PendingRecord,
    RealizedRecord, Source, Violation,
};
use concept::{concept_pass, AnchorResolutions};
use std::collections::{HashMap, HashSet};

/// Snapshot each spec concept's declared owning context (its `concepts/` H1,
/// populated on [`ConceptNode::context`] by `run_check` from the R10-2 tree)
/// before `spec_nodes` is consumed by the concept loop. Empty when no
/// contexts are declared — `ConceptContextMismatch` is code-fact-gated
/// (RFC-010 §3.4), so the snapshot is wasted work without them.
fn snapshot_declared_contexts(
    spec_nodes: &[ConceptNode],
    spec_contexts: &[ContextDecl],
) -> Vec<(String, String, Source)> {
    if spec_contexts.is_empty() {
        return Vec::new();
    }
    spec_nodes
        .iter()
        .filter_map(|n| {
            n.context
                .as_ref()
                .map(|c| (n.name.clone(), c.clone(), n.source.clone()))
        })
        .collect()
}

#[must_use]
pub fn diff(spec: CheckInput, code: Graph) -> CheckOutcome {
    let CheckInput {
        graph: specs,
        contexts: spec_contexts,
        verb_ownership: spec_verb_ownership,
        spec_cohesion,
        concept_anchors,
    } = spec;
    let Graph {
        nodes: spec_nodes,
        edges: spec_edges,
    } = specs;
    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

    // Snapshots for passes 4 (context) and 5 (verb) — taken before
    // code_nodes is consumed into the name-indexed map.
    let code_for_context = if spec_contexts.is_empty() {
        Graph::default()
    } else {
        Graph::new(code_nodes.clone(), code_edges.clone())
    };
    let code_for_verb = if spec_verb_ownership.anchors.is_empty() {
        Graph::default()
    } else {
        Graph::new(code_nodes.clone(), Vec::new())
    };

    let declared_contexts = snapshot_declared_contexts(&spec_nodes, &spec_contexts);

    // Index code by name, consuming code_nodes — later lookups remove the
    // match so the remainder is "code-only" (missing in specs).
    let mut code_by_name: HashMap<String, ConceptNode> = code_nodes
        .into_iter()
        .map(|n| (n.name.clone(), n))
        .collect();

    // Name-sets are needed by the edge pass, which runs after spec_nodes
    // is consumed. Snapshot them before the concept/signature loop.
    let matched_concepts: HashSet<String> = spec_nodes
        .iter()
        .filter(|n| code_by_name.contains_key(&n.name))
        .map(|n| n.name.clone())
        .collect();
    let known_concepts: HashSet<String> = spec_nodes
        .iter()
        .map(|n| n.name.as_str())
        .chain(code_by_name.keys().map(String::as_str))
        .map(str::to_owned)
        .collect();

    let mut violations = Vec::new();
    let mut pending: Vec<PendingRecord> = Vec::new();
    let mut realized: Vec<RealizedRecord> = Vec::new();

    // RFC-012: anchored concepts redirect their equivalence target to a named
    // code item (§3.4) — emit a `DanglingAnchor` for every unresolved anchor
    // and collect each anchored concept's resolution verdict, which the
    // concept pass reads both as the `MissingInCode` exemption (RFC-012) and
    // as the pending-vs-realized fact (RFC-013 §3.4).
    //
    // The marked-name snapshot is taken here, before the concept loop
    // consumes `spec_nodes` — the same pre-snapshot pattern the matched- and
    // known-concept sets above use.
    let anchored_concepts = {
        // RFC-013 §3.4: a marked heading with no name-matching code item
        // whose `- impl:` target does not resolve is *pending*, not dangling
        // — the unresolved target is precisely the declared-ahead-of-code
        // state the marker announces, so the violation is suppressed.
        let marked_without_code: HashSet<&str> = spec_nodes
            .iter()
            .filter(|n| n.marked && !code_by_name.contains_key(&n.name))
            .map(|n| n.name.as_str())
            .collect();
        anchor_pass(concept_anchors, &marked_without_code, &mut violations)
    };

    concept_pass(
        spec_nodes,
        &mut code_by_name,
        &anchored_concepts,
        &mut violations,
        &mut pending,
        &mut realized,
    );

    orphan_pass(code_by_name, &mut violations);

    edge::edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &mut violations,
    );

    // RFC-013 §3.4 — the pending-side obligation skip, stated as one uniform
    // rule across sub-passes. The edge pass satisfies it by construction (its
    // matched-concept filter is built from code presence, and a pending
    // concept has none); the verb pass is told explicitly.
    let pending_concepts: HashSet<&str> = pending.iter().map(|p| p.concept.as_str()).collect();
    verb::verb_pass(
        spec_verb_ownership,
        &code_for_verb,
        &spec_contexts,
        &pending_concepts,
        &mut violations,
    );

    cohesion::cohesion_pass(
        spec_cohesion,
        declared_contexts,
        &code_for_context,
        &spec_contexts,
        &mut violations,
    );

    context::context_pass(spec_contexts, code_for_context, &mut violations);

    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });
    CheckOutcome::new(violations, pending, realized)
}

/// Whatever is left in `code_by_name` after the concept pass has no spec
/// heading at all (matrix row 5, unchanged). A marked heading satisfies its
/// pub item — its name was removed by the concept pass — so realized pairs
/// never surface here.
fn orphan_pass(code_by_name: HashMap<String, ConceptNode>, violations: &mut Vec<Violation>) {
    for (_, code_node) in code_by_name {
        violations.push(Violation::MissingInSpecs {
            name: code_node.name,
            code_source: code_node.source,
        });
    }
}

/// Process the resolved concept anchors (RFC-012 §3.4): push a
/// [`Violation::DanglingAnchor`] for every anchor whose target did not
/// resolve, and return each anchored concept's verdict — `true` when every
/// anchor it declares resolved.
///
/// The concept pass reads that map twice over. As a key set it is the
/// `MissingInCode` exemption (an anchored concept's existence is governed by
/// its `- impl:` target, not a name-matched top-level `pub` type). As a
/// value it is the row-3-vs-row-4 fact for a marked heading: anchor
/// resolution and "backing item present" are the same fact, not two
/// (RFC-013 §3.4).
///
/// `marked_without_code` names the concepts whose `DanglingAnchor` is
/// suppressed — see the call site.
fn anchor_pass(
    concept_anchors: Vec<crate::ResolvedAnchor>,
    marked_without_code: &HashSet<&str>,
    violations: &mut Vec<Violation>,
) -> AnchorResolutions {
    let mut anchored = AnchorResolutions::new();
    for resolved in concept_anchors {
        let violation = anchor_violation(&resolved.anchor, resolved.target.as_ref());
        let this_resolved = violation.is_none();
        // A concept may declare several anchors; it is resolved only when
        // every one of them is.
        anchored
            .entry(resolved.anchor.concept.clone())
            .and_modify(|all_resolved| *all_resolved &= this_resolved)
            .or_insert(this_resolved);
        if let Some(v) = violation {
            if !marked_without_code.contains(resolved.anchor.concept.as_str()) {
                violations.push(v);
            }
        }
    }
    anchored
}

// The `Cohesion` arm delegates to `CohesionViolation::key`. RFC-010
// §3.5/§12-D speculated this would force `violation_key` non-`const`
// (heterogeneous variant fields); in practice `str::as_str` is `const`
// on this toolchain, so the whole match stays `const fn` —
// `clippy::nursery` (`missing_const_for_fn`) requires it.
const fn violation_key(v: &Violation) -> (&str, u8) {
    match v {
        Violation::MissingInCode { name, .. } => (name.as_str(), 0),
        Violation::MissingInSpecs { name, .. } => (name.as_str(), 1),
        Violation::SignatureDrift { name, .. } => (name.as_str(), 2),
        Violation::SignatureMissingInSpec { name, .. } => (name.as_str(), 3),
        Violation::SignatureUnparseable { name, .. } => (name.as_str(), 4),
        Violation::EdgeMissingInCode { concept, .. } => (concept.as_str(), 5),
        Violation::EdgeMissingInSpec { concept, .. } => (concept.as_str(), 6),
        Violation::EdgeTargetUnknown { concept, .. } => (concept.as_str(), 7),
        Violation::Context(ctx) => (ctx.concept(), 8),
        Violation::VerbMissingInCode { concept, .. } => (concept.as_str(), 9),
        Violation::VerbMissingInSpec { qname, .. } => (qname.as_str(), 10),
        Violation::VerbTargetUnknown { concept, .. } => (concept.as_str(), 11),
        Violation::Cohesion(c) => (c.key(), 12),
        // Slot 13 is retired with `ImplementsDraftConcept` (RFC-013 §3.4) —
        // not reused; existing slots are not renumbered.
        Violation::DanglingAnchor { concept, .. } => (concept.as_str(), 14),
    }
}
