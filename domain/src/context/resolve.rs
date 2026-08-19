use crate::{ConceptNode, ContextDecl, Graph, Source};

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
