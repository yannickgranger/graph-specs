//! Context-lookup helpers — resolving a concept's owning bounded context
//! from either the code-side graph or the spec-side declaration chain.

use crate::{ConceptNode, ContextDecl, Graph, Source};

/// Two-hop context lookup: find the concept named `concept_name` in
/// `graph.nodes`, extract its source path, then return the
/// [`ContextDecl`] whose `owned_units` prefix matches that path.
///
/// Returns `None` when the concept is absent from the graph or when no
/// declared context owns its path.
#[must_use]
pub fn context_for_concept<'a>(
    graph: &Graph,
    contexts: &'a [ContextDecl],
    concept_name: &str,
) -> Option<&'a ContextDecl> {
    let node = graph.nodes.iter().find(|n| n.name == concept_name)?;
    match &node.source {
        Source::Code { .. } => context_for_code_node(node, contexts),
        Source::Spec { path, .. } => {
            let path_str = path.to_string_lossy();
            let trimmed = path_str.trim_start_matches("./");
            contexts.iter().find(|ctx| {
                ctx.owned_units
                    .iter()
                    .any(|u| trimmed.starts_with(u.0.as_str()))
            })
        }
    }
}

/// The code-side arm of [`context_for_concept`], on an already-located
/// node: return the [`ContextDecl`] whose `owned_units` contains the
/// node's owning unit. "Which context owns this code item" has exactly one
/// resolution site.
///
/// Prefers the adapter-populated `unit` (relative to the code root);
/// falls back to deriving it from the path for nodes without provenance.
/// The fallback's `split_once("/src/")` keeps the full absolute prefix on
/// an absolute `--code` path, so it may mismatch `owned_units` — routing
/// through the relative `unit` avoids this.
pub fn context_for_code_node<'a>(
    node: &ConceptNode,
    contexts: &'a [ContextDecl],
) -> Option<&'a ContextDecl> {
    let derived = || match &node.source {
        Source::Code { path, .. } => {
            let path_str = path.to_string_lossy();
            let trimmed = path_str.trim_start_matches("./").to_owned();
            trimmed.split_once("/src/").map(|(u, _)| u.to_owned())
        }
        Source::Spec { .. } => None,
    };
    let unit = node.unit.clone().or_else(derived)?;
    contexts
        .iter()
        .find(|ctx| ctx.owned_units.iter().any(|u| u.0 == unit))
}

/// Resolve a concept's **spec-side declared** owning context.
///
/// Applies the canonical-upstream precedence rule: a `specs/contexts/`
/// declaration wins over the concept file's own `H1` when both name a
/// context. Returns `None` only when neither source names a context.
///
/// This is deliberately a *separate question* from the code-side resolution
/// computed by [`context_for_concept`]: when the spec-side declaration and
/// the code-side resolution disagree, a cohesion mismatch is detected.
#[must_use]
pub fn resolve_declared_context<'a>(
    h1_context: Option<&'a str>,
    contexts_upstream: Option<&'a str>,
) -> Option<&'a str> {
    contexts_upstream.or(h1_context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_context_prefers_specs_contexts_upstream() {
        // Both the concept H1 and the canonical specs/contexts/ declaration
        // name a context — the canonical-upstream one wins.
        let resolved = resolve_declared_context(Some("reading"), Some("equivalence"));
        assert_eq!(resolved, Some("equivalence"));
    }

    #[test]
    fn declared_context_falls_back_to_h1_when_no_upstream() {
        let resolved = resolve_declared_context(Some("reading"), None);
        assert_eq!(resolved, Some("reading"));
    }

    #[test]
    fn declared_context_uses_upstream_when_no_h1() {
        let resolved = resolve_declared_context(None, Some("equivalence"));
        assert_eq!(resolved, Some("equivalence"));
    }

    #[test]
    fn declared_context_is_none_when_neither_source_names_one() {
        assert_eq!(resolve_declared_context(None, None), None);
    }
}
