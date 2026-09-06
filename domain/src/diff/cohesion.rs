use crate::context::context_for_concept;
use crate::{resolve_declared_context, CohesionViolation, ContextDecl, Graph, Source, Violation};

pub(super) fn cohesion_pass(
    spec_cohesion: Vec<CohesionViolation>,
    declared: Vec<(String, String, Source)>,
    code: &Graph,
    contexts: &[ContextDecl],
    violations: &mut Vec<Violation>,
) {
    violations.extend(spec_cohesion.into_iter().map(Violation::Cohesion));

    if contexts.is_empty() {
        return;
    }
    for (concept, h1_context, spec_source) in declared {
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
            format: crate::SpecFormat::Markdown,
            path: PathBuf::from("specs/concepts/reading.md"),
            line,
            context: None,
        }
    }

    fn code_node(name: &str, unit: &str) -> ConceptNode {
        ConceptNode::new(
            name.to_owned(),
            Source::Code {
                language: crate::CodeLanguage::Rust,
                path: PathBuf::from(format!("{unit}/src/lib.rs")),
                line: 1,
                provenance: crate::Provenance::empty(),
                location: crate::LocationKind::Path,
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
        let code = Graph::new(vec![code_node("Widget", "domain")], Vec::new());
        let declared = vec![("Widget".to_owned(), "reading".to_owned(), spec_src(7))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &[], &mut v);
        assert!(v.is_empty());
    }

    #[test]
    fn upstream_export_wins_over_h1_for_declared_context() {
        let code = Graph::new(vec![code_node("Graph", "domain")], Vec::new());
        let contexts = vec![ctx("equivalence", "domain", &["Graph"])];
        let declared = vec![("Graph".to_owned(), "reading".to_owned(), spec_src(3))];
        let mut v = Vec::new();
        cohesion_pass(Vec::new(), declared, &code, &contexts, &mut v);
        assert!(v.is_empty(), "upstream export should win: {v:?}");
    }
}
