//! v0.4 bounded-context pass — fourth pass in [`crate::diff::diff`].
//!
//! Order-independent from passes 1–3 per RFC-001 §4 invariant 9: this pass
//! re-derives cross-context candidate edges from the spec and code graphs
//! rather than reading prior passes' violation output.
//!
//! Three violation variants emit here:
//! - `MembershipUnknown` — a pub type's owning unit isn't listed in any context's `Owns`
//! - `CrossEdgeUnauthorized` — an edge crosses contexts with no matching `Imports` entry
//! - `CrossEdgeUndeclared` — the importer names a concept the supplier does not `Exports`

use crate::{ContextDecl, ContextImport, ContextViolation, Graph, OwnedUnit, Source, Violation};
use std::collections::HashMap;

pub(super) fn context_pass(spec_contexts: &[ContextDecl], code: &Graph, out: &mut Vec<Violation>) {
    if spec_contexts.is_empty() {
        return;
    }
    let unit_to_context = build_unit_index(spec_contexts);
    let concept_to_context = build_concept_index(code, &unit_to_context);

    emit_membership_unknown(code, &unit_to_context, out);
    emit_cross_context_edge_violations(code, spec_contexts, &concept_to_context, out);
}

/// Map every declared [`OwnedUnit`] to its context name. Duplicates are
/// caller's problem — `walk_contexts` rejects cycles + duplicates before
/// the pass runs (RFC-001 §4 invariants 3, 4, 7).
fn build_unit_index(contexts: &[ContextDecl]) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for ctx in contexts {
        for unit in &ctx.owned_units {
            m.insert(unit.0.clone(), ctx.name.clone());
        }
    }
    m
}

/// Index code concepts by their owning context (derived from source path).
/// Concepts whose unit is not declared are left out of the index; the
/// [`emit_membership_unknown`] pass catches those separately.
fn build_concept_index(
    code: &Graph,
    unit_to_context: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for node in &code.nodes {
        if let Some(unit) = owning_unit(&node.source) {
            if let Some(ctx_name) = unit_to_context.get(unit.0.as_str()) {
                m.insert(node.name.clone(), ctx_name.clone());
            }
        }
    }
    m
}

fn emit_membership_unknown(
    code: &Graph,
    unit_to_context: &HashMap<String, String>,
    out: &mut Vec<Violation>,
) {
    for node in &code.nodes {
        let Some(unit) = owning_unit(&node.source) else {
            continue;
        };
        if unit_to_context.contains_key(unit.0.as_str()) {
            continue;
        }
        out.push(Violation::Context(ContextViolation::MembershipUnknown {
            concept: node.name.clone(),
            owned_unit: unit,
            code_source: node.source.clone(),
        }));
    }
}

fn emit_cross_context_edge_violations(
    code: &Graph,
    spec_contexts: &[ContextDecl],
    concept_to_context: &HashMap<String, String>,
    out: &mut Vec<Violation>,
) {
    let contexts_by_name: HashMap<&str, &ContextDecl> =
        spec_contexts.iter().map(|c| (c.name.as_str(), c)).collect();

    for edge in &code.edges {
        let Some(source_ctx) = concept_to_context.get(&edge.source_concept) else {
            continue;
        };
        let Some(target_ctx) = concept_to_context.get(&edge.target) else {
            continue;
        };
        if source_ctx == target_ctx {
            continue;
        }
        // Cross-context edge — check the importer's declarations.
        let Some(source_ctx_decl) = contexts_by_name.get(source_ctx.as_str()) else {
            continue;
        };
        let matching_import = find_import(
            &source_ctx_decl.imports,
            target_ctx.as_str(),
            edge.target.as_str(),
        );
        if matching_import.is_none() {
            out.push(Violation::Context(
                ContextViolation::CrossEdgeUnauthorized {
                    concept: edge.source_concept.clone(),
                    owning_context: source_ctx.clone(),
                    edge_kind: edge.kind,
                    target: edge.target.clone(),
                    target_context: target_ctx.clone(),
                    spec_source: source_ctx_decl.source.clone(),
                },
            ));
            continue;
        }
        // Import exists — verify supplier exports it.
        let Some(target_ctx_decl) = contexts_by_name.get(target_ctx.as_str()) else {
            continue;
        };
        let supplier_exports_it = target_ctx_decl
            .exports
            .iter()
            .any(|e| e.concept == edge.target);
        if !supplier_exports_it {
            out.push(Violation::Context(ContextViolation::CrossEdgeUndeclared {
                concept: edge.source_concept.clone(),
                owning_context: source_ctx.clone(),
                edge_kind: edge.kind,
                target: edge.target.clone(),
                target_context: target_ctx.clone(),
                spec_source: source_ctx_decl.source.clone(),
            }));
        }
    }
}

fn find_import<'a>(
    imports: &'a [ContextImport],
    target_ctx: &str,
    concept: &str,
) -> Option<&'a ContextImport> {
    imports
        .iter()
        .find(|i| i.from_context == target_ctx && i.concept == concept)
}

/// Extract the owning unit from a `Source::Code` path — everything before
/// `/src/` in the normalised path. Returns `None` for Spec sources.
///
/// Examples:
/// - `./domain/src/lib.rs` → `OwnedUnit("domain")`
/// - `./adapters/markdown/src/lib.rs` → `OwnedUnit("adapters/markdown")`
fn owning_unit(source: &Source) -> Option<OwnedUnit> {
    let path = match source {
        Source::Code { path, .. } => path,
        Source::Spec { .. } => return None,
    };
    let path_str = path.to_string_lossy();
    let trimmed = path_str.trim_start_matches("./");
    trimmed
        .split_once("/src/")
        .map(|(unit, _)| OwnedUnit(unit.to_string()))
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
