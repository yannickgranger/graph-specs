use crate::Source;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CohesionViolation {
    ContextWithoutCohesionUnit {
        context: String,
        file: PathBuf,
    },
    SubConceptOrphan {
        sub_concept: String,
        file: PathBuf,
    },
    ConceptContextMismatch {
        concept: String,
        declared: String,
        code_context: String,
        spec_source: Source,
        code_source: Option<Source>,
    },
}

impl CohesionViolation {
    #[must_use]
    pub const fn key(&self) -> &str {
        match self {
            Self::ContextWithoutCohesionUnit { context, .. } => context.as_str(),
            Self::SubConceptOrphan { sub_concept, .. } => sub_concept.as_str(),
            Self::ConceptContextMismatch { concept, .. } => concept.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec_src() -> Source {
        Source::Spec {
            format: crate::SpecFormat::Markdown,
            path: PathBuf::from("specs/concepts/equivalence.md"),
            line: 42,
            context: None,
        }
    }

    #[test]
    fn context_without_cohesion_unit_key_is_context() {
        let v = CohesionViolation::ContextWithoutCohesionUnit {
            context: "equivalence".to_owned(),
            file: PathBuf::from("specs/concepts/equivalence.md"),
        };
        assert_eq!(v.key(), "equivalence");
    }

    #[test]
    fn sub_concept_orphan_key_is_sub_concept() {
        let v = CohesionViolation::SubConceptOrphan {
            sub_concept: "InnerThing".to_owned(),
            file: PathBuf::from("specs/concepts/equivalence.md"),
        };
        assert_eq!(v.key(), "InnerThing");
    }

    #[test]
    fn concept_context_mismatch_key_is_concept() {
        let v = CohesionViolation::ConceptContextMismatch {
            concept: "MarkdownReader".to_owned(),
            declared: "reading".to_owned(),
            code_context: "equivalence".to_owned(),
            spec_source: spec_src(),
            code_source: None,
        };
        assert_eq!(v.key(), "MarkdownReader");
    }
}
