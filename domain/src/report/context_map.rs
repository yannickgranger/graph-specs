//! Owned-unit → bounded-context name lookup, shared by the coverage and
//! homonym passes.

use crate::ContextDecl;
use std::collections::HashMap;

pub(super) fn build_unit_context_map(contexts: &[ContextDecl]) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for ctx in contexts {
        for unit in &ctx.owned_units {
            map.insert(unit.0.as_str(), ctx.name.as_str());
        }
    }
    map
}
