//! Verb-coverage pass: per-`pub fn` context resolution + spec citation.

use super::types::{PubFnDecl, VerbCoverageRecord};
use std::collections::{HashMap, HashSet};

pub(super) fn build_verb_coverage(
    pub_fns: &[PubFnDecl],
    unit_to_context: &HashMap<&str, &str>,
    spec_names: &HashSet<&str>,
) -> Vec<VerbCoverageRecord> {
    pub_fns
        .iter()
        .map(|f| {
            let context = f
                .owned_unit
                .as_deref()
                .and_then(|u| unit_to_context.get(u).copied())
                .map(str::to_owned);
            let cited = spec_names.contains(f.name.as_str());
            VerbCoverageRecord {
                context,
                pub_fn: f.clone(),
                cited,
            }
        })
        .collect()
}
