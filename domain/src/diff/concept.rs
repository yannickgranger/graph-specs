use super::signature;
use crate::{
    ConceptNode, Marker, PendingRecord, Polarity, RealizedRecord, RetirementCompleteRecord,
    RetirementIncompleteRecord, Source, Violation,
};
use std::collections::HashMap;

pub(super) type AnchorResolutions = HashMap<String, bool>;

#[derive(Default)]
pub(super) struct MarkerRecords {
    pub pending: Vec<PendingRecord>,
    pub realized: Vec<RealizedRecord>,
    pub retirement_incomplete: Vec<RetirementIncompleteRecord>,
    pub retirement_complete: Vec<RetirementCompleteRecord>,
}

impl MarkerRecords {
    fn push_backed(&mut self, marker: Marker, name: &str, source: &Source) {
        match marker {
            Marker::Unmarked => {}
            Marker::Draft => self.realized.push(RealizedRecord {
                concept: name.to_owned(),
                spec_source: source.clone(),
            }),
            Marker::Retired => self.retirement_incomplete.push(RetirementIncompleteRecord {
                concept: name.to_owned(),
                spec_source: source.clone(),
            }),
        }
    }

    fn push_unbacked(&mut self, marker: Marker, spec_node: ConceptNode) {
        match marker {
            Marker::Unmarked => {}
            Marker::Draft => self.pending.push(PendingRecord {
                concept: spec_node.name,
                spec_source: spec_node.source,
            }),
            Marker::Retired => self.retirement_complete.push(RetirementCompleteRecord {
                concept: spec_node.name,
                spec_source: spec_node.source,
            }),
        }
    }

    fn push_anchored(&mut self, resolved: bool, marker: Marker, spec_node: ConceptNode) {
        if resolved {
            self.push_backed(marker, &spec_node.name, &spec_node.source);
        } else {
            self.push_unbacked(marker, spec_node);
        }
    }
}

pub(super) fn concept_pass(
    spec_nodes: Vec<ConceptNode>,
    code_by_name: &mut HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
    violations: &mut Vec<Violation>,
    sinks: &mut MarkerRecords,
) {
    for spec_node in spec_nodes {
        let Some(spec_node) = polarity_guard(spec_node, code_by_name, violations) else {
            continue;
        };
        let marker = spec_node.marker;
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            sinks.push_backed(marker, &spec_node.name, &spec_node.source);
            signature::compare_signatures(spec_node, code_node, violations);
        } else if let Some(&resolved) = anchored.get(&spec_node.name) {
            sinks.push_anchored(resolved, marker, spec_node);
        } else if marker.is_marked() {
            sinks.push_unbacked(marker, spec_node);
        } else {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
    }
}

fn polarity_guard(
    spec_node: ConceptNode,
    code_by_name: &mut HashMap<String, ConceptNode>,
    violations: &mut Vec<Violation>,
) -> Option<ConceptNode> {
    match spec_node.polarity {
        Polarity::Declared => Some(spec_node),
        Polarity::Forbidden => {
            if let Some(code_node) = code_by_name.remove(&spec_node.name) {
                violations.push(Violation::ForbiddenConceptReintroduced {
                    name: spec_node.name,
                    spec_source: spec_node.source,
                    code_source: code_node.source,
                });
            }
            None
        }
        Polarity::Illustrative => None,
    }
}
