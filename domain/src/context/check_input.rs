//! Input to the v0.4+ diff on the spec side — concept graph plus declared
//! bounded-context map plus verb-ownership aggregate.

use crate::{CohesionViolation, ConceptNode, ContextDecl, Graph, ResolvedAnchor, VerbOwnership};

/// Input to the v0.4+ diff on the spec side — concept graph plus
/// declared bounded-context map plus verb-ownership aggregate.
///
/// Keeps [`Graph`] focused on concepts + edges; contexts and
/// `verb_ownership` are carried alongside per SOLID lens RC-1.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckInput {
    pub graph: Graph,
    pub contexts: Vec<ContextDecl>,
    pub verb_ownership: VerbOwnership,
    /// Spec-side headings parsed from `status: draft` files. Empty
    /// until the markdown reader populates it (Slice B). Used by the
    /// orphan pass to distinguish `ImplementsDraftConcept` from
    /// `MissingInSpecs`.
    pub draft_concepts: Vec<ConceptNode>,
    /// Spec-side structural cohesion violations detected by the R10-2
    /// `TreeAssembler` (`ContextWithoutCohesionUnit` / `SubConceptOrphan`),
    /// pre-computed by the markdown adapter because they need the heading
    /// tree (an adapter artifact). The diff's cohesion pass wraps them as
    /// [`crate::Violation::Cohesion`] and folds them into the sorted output
    /// (RFC-010 §3.5, fact-dependency split).
    pub spec_cohesion: Vec<CohesionViolation>,
    /// Concept anchors (`- impl: <qname>`) paired with their code-side
    /// resolution verdict (RFC-012 §3.4). Built by the application — which
    /// resolves each target through the `AnchorResolver` port — so the diff
    /// stays pure. An anchored concept is exempt from `MissingInCode`; an
    /// unresolved anchor becomes [`crate::Violation::DanglingAnchor`].
    pub concept_anchors: Vec<ResolvedAnchor>,
}

impl CheckInput {
    /// Full constructor — carries all three spec-side inputs.
    #[must_use]
    pub const fn new(
        graph: Graph,
        contexts: Vec<ContextDecl>,
        verb_ownership: VerbOwnership,
    ) -> Self {
        Self {
            graph,
            contexts,
            verb_ownership,
            draft_concepts: Vec::new(),
            spec_cohesion: Vec::new(),
            concept_anchors: Vec::new(),
        }
    }

    /// Convenience constructor for callers that do not populate
    /// `verb_ownership`. Defaults `verb_ownership` to empty vecs.
    #[must_use]
    pub const fn with_graph_and_contexts(graph: Graph, contexts: Vec<ContextDecl>) -> Self {
        Self {
            graph,
            contexts,
            verb_ownership: VerbOwnership {
                decls: Vec::new(),
                anchors: Vec::new(),
            },
            draft_concepts: Vec::new(),
            spec_cohesion: Vec::new(),
            concept_anchors: Vec::new(),
        }
    }

    /// Builder: attach draft-spec concept headings parsed from
    /// `status: draft` files. Wired by the markdown reader in Slice B;
    /// the unit test in this slice uses it directly.
    #[must_use]
    pub fn with_draft_concepts(self, draft_concepts: Vec<ConceptNode>) -> Self {
        Self {
            draft_concepts,
            ..self
        }
    }

    /// Builder: attach the spec-side structural cohesion violations the
    /// markdown adapter pre-computed from the heading tree (RFC-010 R10-3).
    #[must_use]
    pub fn with_spec_cohesion(self, spec_cohesion: Vec<CohesionViolation>) -> Self {
        Self {
            spec_cohesion,
            ..self
        }
    }

    /// Builder: attach the resolved concept anchors (`- impl: <qname>` with
    /// their code-side resolution verdict, RFC-012 §3.4). Wired by the
    /// application after resolving each target through the `AnchorResolver`
    /// port.
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
                path: std::path::PathBuf::from("specs/concepts/reader.md"),
                line: 12,
            },
        }];
        let ci = CheckInput::new(g, ctxs, VerbOwnership::default());
        assert_eq!(ci.contexts.len(), 1);
        assert_eq!(ci.contexts[0].name, "x");
    }
}
