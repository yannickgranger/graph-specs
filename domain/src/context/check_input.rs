use crate::{
    CohesionViolation, ContextDecl, DeclaredSurface, Graph, ResolvedAnchor, VerbOwnership,
    Violation,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckInput {
    pub graph: Graph,
    pub contexts: Vec<ContextDecl>,
    pub surface: DeclaredSurface,
    pub verb_ownership: VerbOwnership,
    pub spec_cohesion: Vec<CohesionViolation>,
    pub concept_anchors: Vec<ResolvedAnchor>,
    pub spec_findings: Vec<Violation>,
}

impl CheckInput {
    #[must_use]
    pub const fn new(
        graph: Graph,
        contexts: Vec<ContextDecl>,
        surface: DeclaredSurface,
        verb_ownership: VerbOwnership,
    ) -> Self {
        Self {
            graph,
            contexts,
            surface,
            verb_ownership,
            spec_cohesion: Vec::new(),
            concept_anchors: Vec::new(),
            spec_findings: Vec::new(),
        }
    }

    #[must_use]
    pub const fn with_graph_and_contexts(
        graph: Graph,
        contexts: Vec<ContextDecl>,
        surface: DeclaredSurface,
    ) -> Self {
        Self {
            graph,
            contexts,
            surface,
            verb_ownership: VerbOwnership {
                decls: Vec::new(),
                anchors: Vec::new(),
            },
            spec_cohesion: Vec::new(),
            concept_anchors: Vec::new(),
            spec_findings: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_spec_findings(self, spec_findings: Vec<Violation>) -> Self {
        Self {
            spec_findings,
            ..self
        }
    }

    #[must_use]
    pub fn with_spec_cohesion(self, spec_cohesion: Vec<CohesionViolation>) -> Self {
        Self {
            spec_cohesion,
            ..self
        }
    }

    #[must_use]
    pub fn with_concept_anchors(self, concept_anchors: Vec<ResolvedAnchor>) -> Self {
        Self {
            concept_anchors,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_input_default_is_empty() {
        let ci = CheckInput::default();
        assert!(ci.graph.nodes.is_empty());
        assert!(ci.graph.edges.is_empty());
        assert!(ci.contexts.is_empty());
        assert!(ci.verb_ownership.decls.is_empty());
        assert!(ci.verb_ownership.anchors.is_empty());
    }

    #[test]
    fn check_input_new_wraps_arguments() {
        use crate::Source;
        let g = Graph::empty();
        let ctxs = vec![ContextDecl {
            name: "x".to_string(),
            owned_units: vec![],
            exports: vec![],
            imports: vec![],
            source: Source::Spec {
                format: crate::SpecFormat::Markdown,
                path: std::path::PathBuf::from("specs/concepts/reader.md"),
                line: 12,
                context: None,
            },
        }];
        let ci = CheckInput::new(
            g,
            ctxs,
            crate::DeclaredSurface::default(),
            VerbOwnership::default(),
        );
        assert_eq!(ci.contexts.len(), 1);
        assert_eq!(ci.contexts[0].name, "x");
    }
}
