//! Concept-level pass — the first pass of [`crate::diff`].
//!
//! Set-difference over concept names, widened by RFC-013 §3.2 into the
//! four spec-side rows of the enforcement matrix:
//!
//! | Heading | Backing item | Result |
//! |---|---|---|
//! | unmarked | present | signature comparison (unchanged) |
//! | unmarked | absent | [`Violation::MissingInCode`] (unchanged) |
//! | marked | present | signature comparison **and** a [`RealizedRecord`] |
//! | marked | absent | a [`PendingRecord`], **no** `MissingInCode` |
//!
//! "Backing item present" is one fact with two spellings: a name-matched
//! code node, or a resolved `- impl:` anchor (RFC-012 §3.4). Row 3 vs row 4
//! and the anchor's resolution outcome are the same question, not two
//! (RFC-013 §3.4).

use super::signature;
use crate::{ConceptNode, PendingRecord, RealizedRecord, Violation};
use std::collections::HashMap;

/// Whether each anchored concept's `- impl:` targets all resolved — the
/// output of [`super::anchor_pass`]. Absence from the map means the concept
/// declares no anchor at all.
pub(super) type AnchorResolutions = HashMap<String, bool>;

/// Run the concept pass, consuming `spec_nodes` and removing each matched
/// name from `code_by_name` (the remainder is the code-only orphan set).
pub(super) fn concept_pass(
    spec_nodes: Vec<ConceptNode>,
    code_by_name: &mut HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
    violations: &mut Vec<Violation>,
    pending: &mut Vec<PendingRecord>,
    realized: &mut Vec<RealizedRecord>,
) {
    for spec_node in spec_nodes {
        let marked = spec_node.marked;
        if let Some(code_node) = code_by_name.remove(&spec_node.name) {
            // Row 4 when marked: the record rides *alongside* full
            // equivalence enforcement — a marker never parks a divergence.
            if marked {
                realized.push(RealizedRecord {
                    concept: spec_node.name.clone(),
                    spec_source: spec_node.source.clone(),
                });
            }
            signature::compare_signatures(spec_node, code_node, violations);
        } else if let Some(&resolved) = anchored.get(&spec_node.name) {
            // Anchored: existence is governed by the anchor, so the anchor's
            // resolution IS the row-3/row-4 fact. Unmarked anchored concepts
            // are unchanged — a `DanglingAnchor` was already emitted above if
            // the target did not resolve.
            if marked {
                push_marker(resolved, spec_node, pending, realized);
            }
        } else if marked {
            pending.push(PendingRecord {
                concept: spec_node.name,
                spec_source: spec_node.source,
            });
        } else {
            violations.push(Violation::MissingInCode {
                name: spec_node.name,
                spec_source: spec_node.source,
            });
        }
    }
}

/// Route a marked, anchored concept to the record its anchor resolution
/// selects. Split out so [`concept_pass`] stays under the cognitive-
/// complexity ceiling.
fn push_marker(
    resolved: bool,
    spec_node: ConceptNode,
    pending: &mut Vec<PendingRecord>,
    realized: &mut Vec<RealizedRecord>,
) {
    if resolved {
        realized.push(RealizedRecord {
            concept: spec_node.name,
            spec_source: spec_node.source,
        });
    } else {
        pending.push(PendingRecord {
            concept: spec_node.name,
            spec_source: spec_node.source,
        });
    }
}
