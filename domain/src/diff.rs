//! Equivalence diff — concept, signature, relationship, bounded-context, verb.
//!
//! Five passes over the spec and code graphs:
//!
//! 1. **Concept** — set-difference over concept names.
//! 2. **Signature** (v0.2) — per matched concept, compare signatures.
//! 3. **Edge** (v0.3) — per matched concept with ≥1 spec edge, compare edges.
//! 4. **Verb** (v0.5) — if `CheckInput.verb_ownership` has anchors, emit
//!    verb-level violations. Order-independent from passes 1–3.
//! 5. **Context** (v0.4) — if `CheckInput.contexts` is non-empty, emit
//!    [`crate::Violation::Context`] variants for membership + cross-context
//!    edges. Order-independent from passes 1–3 (RFC-001 §4 invariant 9).

mod cohesion;
mod context;
mod edge;
mod signature;
mod verb;

#[cfg(test)]
mod tests;

use crate::{anchor_violation, CheckInput, ConceptNode, ContextDecl, Graph, Source, Violation};
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
pub fn diff(spec: CheckInput, code: Graph) -> Vec<Violation> {
    let CheckInput {
        graph: specs,
        contexts: spec_contexts,
        verb_ownership: spec_verb_ownership,
        draft_concepts,
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

    // RFC-012: anchored concepts redirect their equivalence target to a named
    // code item (§3.4) — emit a `DanglingAnchor` for every unresolved anchor
    // and collect the anchored concept names so the concept pass exempts them
    // from `MissingInCode` (their existence is governed by the anchor).
    let anchored_concepts = anchor_pass(concept_anchors, &mut violations);

    for spec_node in spec_nodes {
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            signature::compare_signatures(spec_node, code_node, &mut violations);
        } else if !anchored_concepts.contains(&spec_node.name) {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
        // else: anchored — existence governed by the anchor; a `DanglingAnchor`
        // was already emitted above if its target did not resolve.
    }
    let draft_by_name: HashMap<&str, &Source> = draft_concepts
        .iter()
        .map(|n| (n.name.as_str(), &n.source))
        .collect();
    for (_, code_node) in code_by_name {
        if let Some(draft_src) = draft_by_name.get(code_node.name.as_str()) {
            violations.push(Violation::ImplementsDraftConcept {
                name: code_node.name,
                draft_source: (**draft_src).clone(),
            });
        } else {
            violations.push(Violation::MissingInSpecs {
                name: code_node.name,
                code_source: code_node.source,
            });
        }
    }

    edge::edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &mut violations,
    );

    verb::verb_pass(
        spec_verb_ownership,
        &code_for_verb,
        &spec_contexts,
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

    violations
}

/// Process the resolved concept anchors (RFC-012 §3.4): push a
/// [`Violation::DanglingAnchor`] for every anchor whose target did not
/// resolve, and return the set of anchored concept names — the concept pass
/// exempts these from `MissingInCode` (an anchored concept's existence is
/// governed by its `- impl:` target, not a name-matched top-level `pub` type).
fn anchor_pass(
    concept_anchors: Vec<crate::ResolvedAnchor>,
    violations: &mut Vec<Violation>,
) -> HashSet<String> {
    let mut anchored = HashSet::new();
    for resolved in concept_anchors {
        anchored.insert(resolved.anchor.concept.clone());
        if let Some(v) = anchor_violation(&resolved.anchor, resolved.target.as_ref()) {
            violations.push(v);
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
        Violation::ImplementsDraftConcept { name, .. } => (name.as_str(), 13),
        Violation::DanglingAnchor { concept, .. } => (concept.as_str(), 14),
    }
}
