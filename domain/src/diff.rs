//! Equivalence diff — concept, signature, and relationship levels.
//!
//! Three passes over the spec and code graphs:
//!
//! 1. **Concept** — set-difference over concept names. A concept present
//!    in specs but not in code yields [`Violation::MissingInCode`]; the
//!    inverse yields [`Violation::MissingInSpecs`]. Duplicates within a
//!    side are collapsed by name — the first occurrence carries the
//!    source location.
//! 2. **Signature** — per matched concept, the v0.2 signature payload is
//!    compared via [`signature::compare_signatures`]. Absent on the spec
//!    side is *opt-out* (no comparison). `SignatureUnparseable`
//!    short-circuits a concept's comparison.
//! 3. **Edge** (v0.3) — per matched concept that has ≥1 spec-side edge,
//!    edge sets are compared. Opt-in per concept: no spec edges means no
//!    edge check for that concept.

mod edge;
mod signature;

#[cfg(test)]
mod tests;

use crate::{ConceptNode, Graph, Violation};
use std::collections::{HashMap, HashSet};

#[must_use]
pub fn diff(specs: Graph, code: Graph) -> Vec<Violation> {
    let Graph {
        nodes: spec_nodes,
        edges: spec_edges,
    } = specs;
    let Graph {
        nodes: code_nodes,
        edges: code_edges,
    } = code;

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

    for spec_node in spec_nodes {
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            signature::compare_signatures(spec_node, code_node, &mut violations);
        } else {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
    }
    for (_, code_node) in code_by_name {
        violations.push(Violation::MissingInSpecs {
            name: code_node.name,
            code_source: code_node.source,
        });
    }

    edge::edge_diff(
        spec_edges,
        code_edges,
        &known_concepts,
        &matched_concepts,
        &mut violations,
    );

    violations.sort_by(|a, b| {
        let (ka, da) = violation_key(a);
        let (kb, db) = violation_key(b);
        ka.cmp(kb).then(da.cmp(&db))
    });

    violations
}

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
    }
}
