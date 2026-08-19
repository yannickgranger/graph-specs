use super::concept::AnchorResolutions;
use crate::{ConceptNode, Polarity};
use std::collections::{HashMap, HashSet};

pub(super) fn is_unobliged(node: &ConceptNode, item_present: bool) -> bool {
    node.polarity != Polarity::Declared || (node.marker.is_marked() && !item_present)
}

pub(super) const fn is_unpointable(node: &ConceptNode, item_present: bool) -> bool {
    match node.polarity {
        Polarity::Forbidden => true,
        Polarity::Illustrative => !item_present,
        Polarity::Declared => node.marker.is_marked() && !item_present,
    }
}

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

fn backing_item_present(
    node: &ConceptNode,
    code_by_name: &HashMap<String, ConceptNode>,
    anchored: &AnchorResolutions,
) -> bool {
    code_by_name.contains_key(&node.name) || anchored.get(&node.name).copied().unwrap_or(false)
}

pub(super) struct ObligationKeys {
    pub unobliged: HashSet<String>,
    pub unpointable: HashSet<String>,
}

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
        assert!(!is_unpointable(
            &node(Polarity::Declared, Marker::Unmarked),
            false
        ));
    }

    #[test]
    fn illustrative_with_an_item_is_unobliged_but_pointable() {
        let n = node(Polarity::Illustrative, Marker::Unmarked);
        assert!(is_unobliged(&n, true), "it compels nothing");
        assert!(
            !is_unpointable(&n, true),
            "yet an edge into it is satisfiable"
        );
    }

    #[test]
    fn forbidden_is_unobliged_and_unpointable_under_either_presence() {
        for present in [false, true] {
            let n = node(Polarity::Forbidden, Marker::Unmarked);
            assert!(is_unobliged(&n, present));
            assert!(is_unpointable(&n, present));
        }
    }
}
