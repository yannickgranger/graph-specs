//! Cohesion pass — RFC-010 §3.5 / R10-3.
//!
//! The upward concept→context rung. Splits by **fact-dependency**
//! (§3.4 capability matrix):
//!
//! - [`CohesionViolation::ContextWithoutCohesionUnit`] and
//!   [`CohesionViolation::SubConceptOrphan`] are *spec-side* — the R10-2
//!   `TreeAssembler` detects them from the heading tree with zero code
//!   facts. The markdown adapter pre-computes them (it owns the tree); this
//!   pass only wraps them as [`Violation::Cohesion`].
//! - [`CohesionViolation::ConceptContextMismatch`] is *code-fact-gated*: it
//!   compares each concept's spec-side **declared** context (its `concepts/`
//!   H1) against the **code-resolved** context (the `specs/contexts/` Owns
//!   block that owns the crate the `pub` type lives in). It can only fire
//!   when `specs/contexts/` is present — without it there is no code-side
//!   context to disagree with.

use crate::context::context_for_concept;
use crate::{resolve_declared_context, CohesionViolation, ContextDecl, Graph, Source, Violation};

/// Append cohesion violations to `violations`.
///
/// `spec_cohesion` are the adapter-detected spec-side structural violations;
/// `declared` is each concept's `(name, declared_context, spec_source)`
/// snapshot; `code` is the code graph; `contexts` the declared bounded
/// contexts.
pub(super) fn cohesion_pass(
    spec_cohesion: Vec<CohesionViolation>,
    declared: Vec<(String, String, Source)>,
    code: &Graph,
    contexts: &[ContextDecl],
    violations: &mut Vec<Violation>,
) {
    // Spec-side structural cohesion — fires regardless of code facts.
    violations.extend(spec_cohesion.into_iter().map(Violation::Cohesion));

    // ConceptContextMismatch needs a code-side context (§3.4 matrix): only
    // resolvable when `specs/contexts/` Owns is declared.
    if contexts.is_empty() {
        return;
    }
    for (concept, h1_context, spec_source) in declared {
        // Canonical-upstream precedence: a matching `specs/contexts/` export
        // wins over the concept file's own H1 (RFC-010 §3.4 / R10-1).
        let upstream = contexts
            .iter()
            .find(|c| c.exports.iter().any(|e| e.concept == concept))
            .map(|c| c.name.as_str());
        let Some(declared_ctx) = resolve_declared_context(Some(h1_context.as_str()), upstream)
        else {
            continue;
        };
        let Some(code_ctx) = context_for_concept(code, contexts, &concept).map(|c| c.name.as_str())
        else {
            continue;
        };
        if declared_ctx != code_ctx {
            violations.push(Violation::Cohesion(
                CohesionViolation::ConceptContextMismatch {
                    declared: declared_ctx.to_owned(),
                    code_context: code_ctx.to_owned(),
                    concept,
                    spec_source,
                },
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConceptNode, ContextExport, ContextPattern, OwnedUnit, SignatureState};
    use std::path::PathBuf;

    fn spec_src(line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/reading.md"),
            line,
        }
    }

    fn code_node(name: &str, unit: &str) -> ConceptNode {
        ConceptNode::new(
            name.to_owned(),
            Source::Code {
                path: PathBuf::from(format!("{unit}/src/lib.rs")),
                line: 1,
            },
            SignatureState::Absent,
        )
        .with_provenance(Some(unit.to_owned()), Some(unit.to_owned()), None)
    }

    fn ctx(name: &str, unit: &str, exports: &[&str]) -> ContextDecl {
        ContextDecl::new(
            name.to_owned(),
            vec![OwnedUnit(unit.to_owned())],
            exports
                .iter()
                .map(|c| ContextExport {
                    concept: (*c).to_owned(),
                    pattern: ContextPattern::PublishedLanguage,
                })
                .collect(),
            Vec::new(),
            spec_src(1),
        )
    }

    #[test]
    fn spec_side_cohesion_fires_without_contexts() {
        let mut v = Vec::new();
        let specs = vec![CohesionViolation::ContextWithoutCohesionUnit {
            context: "lonely".to_owned(),
            file: PathBuf::from("specs/concepts/lonely.md"),
        }];
        cohesion_pass(specs, Vec::new(), &Graph::default(), &[], &mut v);
        assert_eq!(v.len(), 1);
        assert!(matches!(
            v[0],
            Violation::Cohesion(CohesionViolation::ContextWithoutCohesionUnit { .. })
        ));
    }

    #[test]
    fn mismatch_fires_when_declared_differs_from_code_resolved() {
        // `Widget` is documented under H1 `reading` but its code lives in
        // crate `domain`, owned by context `equivalence`.
        let code = Graph::new(vec![code_node("Widget", "domain")], Vec::new());
        let contexts = vec![ctx("equivalence", "domain", &[])];
        let declared = vec![("Widget".to_owned(), "reading".to_owned(), spec_src(7))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &contexts, &mut v);
        assert_eq!(v.len(), 1);
        match &v[0] {
            Violation::Cohesion(CohesionViolation::ConceptContextMismatch {
                concept,
                declared,
                code_context,
                ..
            }) => {
                assert_eq!(concept, "Widget");
                assert_eq!(declared, "reading");
                assert_eq!(code_context, "equivalence");
            }
            other => panic!("expected ConceptContextMismatch, got {other:?}"),
        }
    }

    #[test]
    fn no_mismatch_when_declared_matches_code() {
        let code = Graph::new(vec![code_node("Graph", "domain")], Vec::new());
        let contexts = vec![ctx("equivalence", "domain", &["Graph"])];
        let declared = vec![("Graph".to_owned(), "equivalence".to_owned(), spec_src(3))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &contexts, &mut v);
        assert!(v.is_empty(), "matching contexts must not fire: {v:?}");
    }

    #[test]
    fn mismatch_suppressed_without_specs_contexts() {
        // Data-dependency: with no contexts there is no code-side context,
        // so ConceptContextMismatch cannot fire (§3.4 matrix, source-walk
        // without specs/contexts/).
        let code = Graph::new(vec![code_node("Widget", "domain")], Vec::new());
        let declared = vec![("Widget".to_owned(), "reading".to_owned(), spec_src(7))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &[], &mut v);
        assert!(v.is_empty());
    }

    #[test]
    fn upstream_export_wins_over_h1_for_declared_context() {
        // `Graph` is exported by `equivalence` (canonical-upstream) but the
        // concept file H1 says `reading`. Code also resolves to equivalence,
        // so with upstream precedence there is NO mismatch.
        let code = Graph::new(vec![code_node("Graph", "domain")], Vec::new());
        let contexts = vec![ctx("equivalence", "domain", &["Graph"])];
        let declared = vec![("Graph".to_owned(), "reading".to_owned(), spec_src(3))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &contexts, &mut v);
        assert!(v.is_empty(), "upstream export should win: {v:?}");
    }
}
