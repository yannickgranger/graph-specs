use crate::{behavioral_exemption_applies, AbstractionLevel, CohesionViolation};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadingNode {
    pub level: AbstractionLevel,
    pub text: String,
    pub id: Option<String>,
    pub line: usize,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecTree {
    pub file: PathBuf,
    pub nodes: Vec<HeadingNode>,
    pub behavioral: bool,
    pub has_substance: bool,
}

impl SpecTree {
    #[must_use]
    pub fn context_id(&self) -> Option<&str> {
        self.nodes
            .iter()
            .find(|n| n.level == AbstractionLevel::Context)
            .and_then(|n| n.id.as_deref())
    }

    #[must_use]
    pub fn concept_declarations(&self) -> Vec<(&str, &str)> {
        let Some(ctx) = self.context_id() else {
            return Vec::new();
        };
        self.nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.level,
                    AbstractionLevel::Concept | AbstractionLevel::SubConcept
                )
            })
            .map(|n| (n.text.as_str(), ctx))
            .collect()
    }

    #[must_use]
    pub fn cohesion_violations(&self) -> Vec<CohesionViolation> {
        let context_exempt = behavioral_exemption_applies(self.behavioral, self.has_substance);
        let mut out = Vec::new();
        for (idx, node) in self.nodes.iter().enumerate() {
            match node.level {
                AbstractionLevel::Context => {
                    if !self.has_cohesion_unit(idx) && !context_exempt {
                        out.push(CohesionViolation::ContextWithoutCohesionUnit {
                            context: node.id.clone().unwrap_or_else(|| node.text.clone()),
                            file: self.file.clone(),
                        });
                    }
                }
                AbstractionLevel::SubConcept if node.parent.is_none() => {
                    out.push(CohesionViolation::SubConceptOrphan {
                        sub_concept: node.text.clone(),
                        file: self.file.clone(),
                    });
                }
                _ => {}
            }
        }
        out
    }

    fn has_cohesion_unit(&self, ctx_idx: usize) -> bool {
        self.nodes[ctx_idx + 1..]
            .iter()
            .take_while(|n| n.level != AbstractionLevel::Context)
            .any(|n| {
                matches!(
                    n.level,
                    AbstractionLevel::Concept | AbstractionLevel::SubConcept
                )
            })
    }
}
