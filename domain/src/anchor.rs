use crate::{Source, Violation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConceptAnchor {
    pub concept: String,
    pub target: String,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AnchorKind {
    Type,
    Fn,
    Const,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorTarget {
    pub kind: AnchorKind,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAnchor {
    pub anchor: ConceptAnchor,
    pub target: Option<AnchorTarget>,
}

#[must_use]
pub fn anchor_violation(
    anchor: &ConceptAnchor,
    resolved: Option<&AnchorTarget>,
) -> Option<Violation> {
    if resolved.is_some() {
        return None;
    }
    Some(Violation::DanglingAnchor {
        concept: anchor.concept.clone(),
        target: anchor.target.clone(),
        spec_source: anchor.source.clone(),
    })
}

#[must_use]
pub const fn behavioral_exemption_applies(declared_behavioral: bool, has_substance: bool) -> bool {
    declared_behavioral && has_substance
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec_src(line: usize) -> Source {
        Source::Spec {
            path: PathBuf::from("specs/concepts/intake_validation.md"),
            line,
            context: None,
        }
    }

    fn code_src() -> Source {
        Source::Code {
            path: PathBuf::from("domain/src/intake.rs"),
            line: 7,
            provenance: crate::Provenance::empty(),
        }
    }

    fn anchor() -> ConceptAnchor {
        ConceptAnchor {
            concept: "ValidateIntakeFull".to_owned(),
            target: "validate_intake".to_owned(),
            source: spec_src(3),
        }
    }

    #[test]
    fn concept_anchor_round_trips_its_fields() {
        let a = anchor();
        assert_eq!(a.concept, "ValidateIntakeFull");
        assert_eq!(a.target, "validate_intake");
        assert_eq!(a.source, spec_src(3));
        assert_eq!(a.clone(), a);
    }

    #[test]
    fn resolved_anchor_yields_no_violation() {
        let target = AnchorTarget {
            kind: AnchorKind::Fn,
            source: code_src(),
        };
        assert_eq!(anchor_violation(&anchor(), Some(&target)), None);
    }

    #[test]
    fn unresolved_anchor_yields_dangling_anchor() {
        let v = anchor_violation(&anchor(), None).expect("dangling");
        match v {
            Violation::DanglingAnchor {
                concept,
                target,
                spec_source,
            } => {
                assert_eq!(concept, "ValidateIntakeFull");
                assert_eq!(target, "validate_intake");
                assert_eq!(spec_source, spec_src(3));
            }
            other => panic!("expected DanglingAnchor, got {other:?}"),
        }
    }

    #[test]
    fn anchor_kind_resolves_each_mvp_kind() {
        for kind in [AnchorKind::Type, AnchorKind::Fn, AnchorKind::Const] {
            let t = AnchorTarget {
                kind,
                source: code_src(),
            };
            assert_eq!(anchor_violation(&anchor(), Some(&t)), None);
        }
    }

    #[test]
    fn behavioral_exemption_requires_marker_and_substance() {
        assert!(behavioral_exemption_applies(true, true));
        assert!(!behavioral_exemption_applies(true, false));
        assert!(!behavioral_exemption_applies(false, true));
        assert!(!behavioral_exemption_applies(false, false));
    }
}
