//! The obligation rule of RFC-015 §3.4 — stated once, cited by both
//! consumers.
//!
//! Two questions that look like one and are not:
//!
//! - **`unobliged`** — *what does this heading oblige?* It compels no code
//!   item to exist. Governs the **source side**: an `unobliged` heading
//!   imposes no code-existence demand through its own declarations. The verb
//!   pass and the anchor pass consume it.
//! - **`unpointable`** — *can this edge exist?* The heading offers no
//!   legitimate code item to point at, and its own declared state accounts
//!   for that. Governs the **target side**: no heading bears a
//!   code-existence demand made of it by *another* heading's declarations.
//!   The edge pass consumes it. **This is what RFC-015 adds.**
//!
//! A third predicate, **`unbound`** (*does this heading describe a code
//! item?* — member: `illustrative`, alone), is named by §3.4 and is
//! deliberately **not** implemented here: it has no consumer in this RFC and
//! is known under-enforced (§6, issue #187). Adding it without its consumer
//! would ship an unread predicate.
//!
//! # Why two predicates and not one with a caveat
//!
//! The member sets nest — `unpointable ⊂ unobliged` — while the predicates
//! do not, and the witness is **`illustrative` with its item present**: it
//! compels nothing, so it is `unobliged`, but an edge into it is perfectly
//! satisfiable and the remedy is to add the field. Keying the target side on
//! `unobliged` suppresses that finding and **parks a real spec↔code
//! divergence**, which the whole marker design forbids.
//!
//! Set inclusion does not license clause subordination: a subordinate clause
//! quantifies over its main clause's subject, so hanging either predicate
//! off "compels no code item" asserts it of the whole `unobliged` extension
//! rather than of the subset. `forbidden` is the standing witness on the
//! other containment — it is `unobliged` **and bound**, which is why
//! `ForbiddenConceptReintroduced` survives.

use super::concept::AnchorResolutions;
use crate::{ConceptNode, Polarity};
use std::collections::{HashMap, HashSet};

/// `unobliged` — this heading compels no code item to exist (RFC-015 §3.4).
///
/// Members: a heading marked with **either** value whose item is absent,
/// `forbidden`, and `illustrative`.
///
/// Unchanged in extension by RFC-015 apart from `retired` + absent joining
/// it; what changes is that the rule is now *stated* rather than assembled
/// downstream from the record lists it happens to produce.
pub(super) fn is_unobliged(node: &ConceptNode, item_present: bool) -> bool {
    node.polarity != Polarity::Declared || (node.marker.is_marked() && !item_present)
}

/// `unpointable` — this heading offers no legitimate code item to point at,
/// and its own declared state accounts for that (RFC-015 §3.4).
///
/// The members are normative and are checked here in the order §3.4 states
/// them: marked-with-either-value + absent; `illustrative` + absent;
/// `forbidden` + absent; `forbidden` + present.
///
/// **The accounting clause is load-bearing and is not a restatement of the
/// member list.** Keying on item *absence alone* would admit an unmarked
/// `declared` heading whose item is absent — which is matrix row 1, where
/// nothing accounts for the absence because that absence *is* the finding.
/// Suppressing there silently moves an existing row and breaks invariant 2.
///
/// `illustrative` + **present** is deliberately excluded: the checker
/// already accepts such an item as a legitimate edge target and reports it
/// as `MissingInSpecs` — unspecced, not illegitimate. Contrast `forbidden`,
/// where the name is genuinely expelled and the item's existence is itself
/// the violation.
pub(super) const fn is_unpointable(node: &ConceptNode, item_present: bool) -> bool {
    match node.polarity {
        Polarity::Forbidden => true,
        Polarity::Illustrative => !item_present,
        Polarity::Declared => node.marker.is_marked() && !item_present,
    }
}

/// Convert the per-**heading** [`is_unpointable`] predicate into the
/// per-**name** key the edge pass needs, conservatively.
///
/// **A name is `unpointable` only if EVERY heading carrying it is.** Two
/// headings may share a name across files, and the edge pass keys on an
/// edge's target, which is a name rather than a heading. The permissive
/// direction — any heading suffices — parks a real divergence: a heading in
/// one context illustrating a type really declared in another is the
/// *canonical* use of `illustrative`, and under a permissive key an edge
/// into that name is suppressed while the declared heading's own item sits
/// there satisfiable, with no other violation co-firing to keep the gate
/// red.
///
/// Only `unpointable` is ruled conservative. [`is_unobliged`]'s per-name
/// conversion is left exactly as it was — §3.4 rules this direction for the
/// target side, and widening the source side here would be an invention.
fn unpointable_names(
    spec_nodes: &[ConceptNode],
    code_by_name: &HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
) -> HashSet<String> {
    let mut by_name: HashMap<&str, bool> = HashMap::new();
    for node in spec_nodes {
        let verdict = is_unpointable(node, backing_item_present(node, code_by_name, anchored));
        by_name
            .entry(node.name.as_str())
            .and_modify(|all| *all &= verdict)
            .or_insert(verdict);
    }
    by_name
        .into_iter()
        .filter(|&(_, all)| all)
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// The names whose headings are all [`is_unobliged`], for the source-side
/// consumers.
///
/// Permissive per-name, unchanged from the pre-RFC-015 construction: a name
/// enters the set if any heading carrying it is `unobliged`. See
/// [`unpointable_names`] for why the target side is not.
fn unobliged_names(
    spec_nodes: &[ConceptNode],
    code_by_name: &HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
) -> HashSet<String> {
    spec_nodes
        .iter()
        .filter(|n| is_unobliged(n, backing_item_present(n, code_by_name, anchored)))
        .map(|n| n.name.clone())
        .collect()
}

/// "Backing item present" — one fact with two spellings (RFC-012 §3.4): a
/// name-matched code item, or a resolved `- impl:` anchor. Both predicates
/// read the same fact, exactly as the concept pass's row selection does.
fn backing_item_present(
    node: &ConceptNode,
    code_by_name: &HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
) -> bool {
    code_by_name.contains_key(&node.name) || anchored.get(&node.name).copied().unwrap_or(false)
}

/// Both obligation keys, from one walk of the spec nodes.
///
/// Grouped so [`super::diff`] carries one binding: the two sets are read by
/// different passes, but they are derived from the same three inputs at the
/// same moment — before the concept pass consumes `spec_nodes` and mutates
/// `code_by_name`.
pub(super) struct ObligationKeys {
    /// Source side — consumed by the verb pass.
    pub unobliged: HashSet<String>,
    /// Target side — consumed by the edge pass. RFC-015's addition.
    pub unpointable: HashSet<String>,
}

/// Compute both keys. See [`is_unobliged`] and [`is_unpointable`] for the
/// rules, and [`unpointable_names`] for why only one of them is
/// conservative per name.
pub(super) fn obligation_keys(
    spec_nodes: &[ConceptNode],
    code_by_name: &HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
) -> ObligationKeys {
    ObligationKeys {
        unobliged: unobliged_names(spec_nodes, code_by_name, anchored),
        unpointable: unpointable_names(spec_nodes, code_by_name, anchored),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Marker, SignatureState, Source};
    use std::path::PathBuf;

    fn node(polarity: Polarity, marker: Marker) -> ConceptNode {
        let mut n = ConceptNode::new(
            "T".to_owned(),
            Source::Spec {
                path: PathBuf::from("specs/concepts/a.md"),
                line: 1,
            },
            SignatureState::Absent,
        )
        .with_polarity(polarity);
        n.marker = marker;
        n
    }

    /// The §3.4 derivation table, cell by cell. Spot-checking the motivating
    /// cell is how two previous revisions passed review, so every cell is
    /// asserted with its ground.
    #[test]
    fn the_unpointable_derivation_table_holds_cell_by_cell() {
        let cells = [
            (Polarity::Declared, Marker::Unmarked, false, false),
            (Polarity::Declared, Marker::Unmarked, true, false),
            (Polarity::Declared, Marker::Draft, false, true),
            (Polarity::Declared, Marker::Retired, false, true),
            (Polarity::Declared, Marker::Draft, true, false),
            (Polarity::Declared, Marker::Retired, true, false),
            (Polarity::Illustrative, Marker::Unmarked, false, true),
            (Polarity::Illustrative, Marker::Unmarked, true, false),
            (Polarity::Forbidden, Marker::Unmarked, false, true),
            (Polarity::Forbidden, Marker::Unmarked, true, true),
        ];
        for (polarity, marker, present, want) in cells {
            assert_eq!(
                is_unpointable(&node(polarity, marker), present),
                want,
                "{polarity:?} + {marker:?} + present={present}"
            );
        }
    }

    #[test]
    fn row_1_is_not_unpointable_because_nothing_accounts_for_its_absence() {
        // The accounting clause, isolated. An absence-only key would answer
        // `true` here and silently move matrix row 1.
        assert!(!is_unpointable(
            &node(Polarity::Declared, Marker::Unmarked),
            false
        ));
    }

    #[test]
    fn illustrative_with_an_item_is_unobliged_but_pointable() {
        // The witness that the two predicates come apart — the cell that
        // makes `unpointable` a separate predicate rather than a synonym.
        let n = node(Polarity::Illustrative, Marker::Unmarked);
        assert!(is_unobliged(&n, true), "it compels nothing");
        assert!(
            !is_unpointable(&n, true),
            "yet an edge into it is satisfiable"
        );
    }

    #[test]
    fn forbidden_is_unobliged_and_unpointable_under_either_presence() {
        // The other containment's witness. `forbidden` is `unobliged` and
        // BOUND — which is why the binding predicate is not implemented as a
        // subordinate clause of this one.
        for present in [false, true] {
            let n = node(Polarity::Forbidden, Marker::Unmarked);
            assert!(is_unobliged(&n, present));
            assert!(is_unpointable(&n, present));
        }
    }
}
