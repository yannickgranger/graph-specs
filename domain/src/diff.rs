//! Equivalence diff — concept, signature, relationship, bounded-context, verb.
//!
//! Five passes over the spec and code graphs:
//!
//! 1. **Concept** — set-difference over concept names, plus the
//!    spec-state-marker rows (see [`concept`]).
//! 2. **Signature** — per matched concept, compare signatures.
//! 3. **Edge** — per matched concept with ≥1 spec edge, compare edges.
//! 4. **Verb** — if `CheckInput.verb_ownership` has anchors, emit
//!    verb-level violations. Order-independent from passes 1–3.
//! 5. **Context** — if `CheckInput.contexts` is non-empty, emit
//!    [`crate::Violation::Context`] variants for membership + cross-context
//!    edges. Order-independent from passes 1–3.

mod cohesion;
mod concept;
mod context;
mod edge;
mod obligation;
mod signature;
mod verb;

#[cfg(test)]
mod tests;

use crate::context::context_for_code_node;
use crate::{
    anchor_violation, CheckInput, CheckOutcome, ConceptNode, ContextDecl, Graph, Polarity,
    Provenance, Source, Violation,
};
use concept::{concept_pass, AnchorResolutions, MarkerRecords};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Snapshot each spec concept's declared owning context (its `concepts/` H1,
/// populated on [`ConceptNode::context`])
/// before `spec_nodes` is consumed by the concept loop. Empty when no
/// contexts are declared — `ConceptContextMismatch` is code-fact-gated,
/// so the snapshot is wasted work without them.
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

    // Snapshot each code concept's containment
    // triple (resolving `context` through the same `specs/contexts/` Owns
    // lookup the cohesion pass uses) before code_nodes is consumed. The
    // NDJSON emitter reads this off the outcome; it cannot re-derive
    // `context` itself without a split-brain on the Owns resolution.
    let provenance = provenance_index(&code_nodes, &spec_contexts);

    // Index code by name, consuming code_nodes — later lookups remove the
    // match so the remainder is "code-only" (missing in specs).
    let mut code_by_name: HashMap<String, ConceptNode> = code_nodes
        .into_iter()
        .map(|n| (n.name.clone(), n))
        .collect();

    // RFC-014 §3.3 — the one shared polarity snapshot, read by the anchor,
    // edge and verb passes alike. Only non-`declared` headings are recorded,
    // so an ungrounded corpus (every heading `Declared`) yields an empty map
    // and every gate below is inert — RFC-014 §4 invariants 1 and 4.
    //
    // Keys are **owned**: `spec_nodes` is moved by the concept pass, while
    // the edge and verb passes run afterwards and key off owned `String`s of
    // their own, so a borrowed map could not survive to reach them. Polarity
    // is never put on `ConceptAnchor` — that would duplicate a fact
    // `ConceptNode` already owns and reintroduce the side-index split-brain
    // RFC-013 §3.3 retired.
    let unobliged_concepts: HashSet<String> = spec_nodes
        .iter()
        .filter(|n| n.polarity != Polarity::Declared)
        .map(|n| n.name.clone())
        .collect();

    let (matched_concepts, known_concepts) = snapshot_name_sets(&spec_nodes, &code_by_name);

    let mut violations = Vec::new();
    let mut records = MarkerRecords::default();

    // RFC-012: anchored concepts redirect their equivalence target to a named
    // code item (§3.4) — emit a `DanglingAnchor` for every unresolved anchor
    // and collect each anchored concept's resolution verdict, which the
    // concept pass reads both as the `MissingInCode` exemption (RFC-012) and
    // as the pending-vs-realized fact (RFC-013 §3.4).
    //
    // The marked-name snapshot is taken here, before the concept loop
    // consumes `spec_nodes` — the same pre-snapshot pattern the matched- and
    // known-concept sets above use.
    let anchored_concepts = run_anchor_pass(
        concept_anchors,
        &spec_nodes,
        &code_by_name,
        &unobliged_concepts,
        &mut violations,
    );

    // RFC-015 §3.4 — both obligation keys, taken before the concept pass
    // consumes `spec_nodes` and mutates `code_by_name`.
    let keys = obligation::obligation_keys(&spec_nodes, &code_by_name, &anchored_concepts);

    concept_pass(
        spec_nodes,
        &mut code_by_name,
        &anchored_concepts,
        &mut violations,
        &mut records,
    );

    orphan_pass(code_by_name, &mut violations);

    edge::edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &keys.unpointable,
        &mut violations,
    );

    // RFC-015 §3.4 — the source-side rule, now consumed from where it is
    // STATED rather than reassembled here from the record lists it happens
    // to produce. Same extension, one carrier.
    verb::verb_pass(
        spec_verb_ownership,
        &code_for_verb,
        &spec_contexts,
        &keys.unobliged,
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
    let mut outcome = CheckOutcome::new(
        violations,
        records.pending,
        records.realized,
        records.retirement_incomplete,
        records.retirement_complete,
    );
    outcome.provenance = provenance;
    outcome
}

/// The two name-sets the edge pass needs, snapshotted before the concept
/// loop consumes `spec_nodes`.
///
/// A non-`declared` concept is excluded from `matched_concepts`, which is
/// how the edge pass inherits RFC-014 §3.3's uniform obligation skip: the
/// pass groups BOTH sides by that set, so neither its spec edges nor its
/// code edges impose anything. `known_concepts` is deliberately NOT
/// filtered — the name is still declared in specs, so another concept
/// pointing at it is not aiming at a mirage.
fn snapshot_name_sets(
    spec_nodes: &[ConceptNode],
    code_by_name: &HashMap<String, ConceptNode>,
) -> (HashSet<String>, HashSet<String>) {
    let matched = spec_nodes
        .iter()
        .filter(|n| n.polarity == Polarity::Declared && code_by_name.contains_key(&n.name))
        .map(|n| n.name.clone())
        .collect();
    let known = spec_nodes
        .iter()
        .map(|n| n.name.as_str())
        .chain(code_by_name.keys().map(String::as_str))
        .map(str::to_owned)
        .collect();
    (matched, known)
}

/// Run the RFC-012 anchor pass behind the suppression set it needs.
///
/// Two sources, one rule: a heading that compels nothing cannot have a
/// *missing* anchor target.
///
/// - RFC-013 §3.4 (widened by RFC-015 §3.3) — a **marked** heading with no
///   name-matching code item whose `- impl:` target does not resolve is a
///   marker record, not dangling: the unresolved target is precisely the
///   state the marker announces. True under either marker value, which is
///   why the filter asks `is_marked` rather than dispatching on the value.
/// - RFC-014 §3.3 (OQ-4, ruled) — a non-`declared` heading compels nothing
///   at all, so a dangling anchor under one fires nothing.
///
/// Called before the concept loop consumes `spec_nodes` — the same
/// pre-snapshot pattern the matched- and known-concept sets use.
fn run_anchor_pass(
    concept_anchors: Vec<crate::ResolvedAnchor>,
    spec_nodes: &[ConceptNode],
    code_by_name: &HashMap<String, ConceptNode>,
    non_declared: &HashSet<String>,
    violations: &mut Vec<Violation>,
) -> AnchorResolutions {
    let mut suppressed: HashSet<&str> = spec_nodes
        .iter()
        .filter(|n| n.marker.is_marked() && !code_by_name.contains_key(&n.name))
        .map(|n| n.name.as_str())
        .collect();
    suppressed.extend(non_declared.iter().map(String::as_str));
    anchor_pass(concept_anchors, &suppressed, violations)
}

/// Snapshot the containment triple of every code concept that has one
/// (RFC-010 §3.6 / #136), keyed by concept name. `module_path` / `unit`
/// come straight off the node (adapter-populated, RFC-010 §3.3);
/// `context` is resolved through [`context_for_code_node`] — the single
/// code-side Owns resolution site. Nodes with no facts at all are
/// skipped, so a provenance-free corpus yields an empty index.
fn provenance_index(
    code_nodes: &[ConceptNode],
    contexts: &[ContextDecl],
) -> BTreeMap<String, Provenance> {
    code_nodes
        .iter()
        .filter_map(|n| {
            let context = context_for_code_node(n, contexts).map(|c| c.name.clone());
            if n.module_path.is_none() && n.unit.is_none() && context.is_none() {
                return None;
            }
            Some((
                n.name.clone(),
                Provenance {
                    module_path: n.module_path.clone(),
                    unit: n.unit.clone(),
                    context,
                },
            ))
        })
        .collect()
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
        // RFC-014 §3.4 — appended after `DanglingAnchor`. Append-only:
        // existing slots are never renumbered, and slot 13 stays retired.
        Violation::ForbiddenConceptReintroduced { name, .. } => (name.as_str(), 15),
    }
}
