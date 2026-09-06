use super::decl::{ContextDecl, OwnedUnit};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredSurface {
    units: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipAmbiguity {
    pub outer: OwnedUnit,
    pub outer_context: String,
    pub inner: OwnedUnit,
    pub inner_context: String,
}

impl DeclaredSurface {
    pub fn from_contexts(contexts: &[ContextDecl]) -> Result<Self, OwnershipAmbiguity> {
        let mut declared: Vec<(String, &str)> = contexts
            .iter()
            .flat_map(|ctx| {
                ctx.owned_units
                    .iter()
                    .map(move |unit| (normalize(&unit.0), ctx.name.as_str()))
            })
            .filter(|(unit, _)| !unit.is_empty())
            .collect();
        declared.sort();
        if let Some(ambiguity) = nested_across_contexts(&declared) {
            return Err(ambiguity);
        }
        let mut units: Vec<(String, String)> = declared
            .into_iter()
            .map(|(unit, context)| (unit, context.to_owned()))
            .collect();
        units.dedup();
        Ok(Self { units })
    }

    #[must_use]
    pub fn admits(&self, qname: &str) -> bool {
        self.unit_of(qname).is_some()
    }

    #[must_use]
    pub fn unit_of(&self, qname: &str) -> Option<&str> {
        self.longest_covering(qname).map(|(unit, _)| unit.as_str())
    }

    #[must_use]
    pub fn context_of(&self, qname: &str) -> Option<&str> {
        self.longest_covering(qname)
            .map(|(_, context)| context.as_str())
    }

    fn longest_covering(&self, qname: &str) -> Option<&(String, String)> {
        let qname = normalize(qname);
        self.units
            .iter()
            .filter(|(unit, _)| covers(unit, &qname))
            .max_by_key(|(unit, _)| unit.len())
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

fn nested_across_contexts(declared: &[(String, &str)]) -> Option<OwnershipAmbiguity> {
    for (i, (outer, outer_context)) in declared.iter().enumerate() {
        for (inner, inner_context) in declared.iter().skip(i + 1) {
            if outer_context == inner_context {
                continue;
            }
            let (outer, inner, outer_context, inner_context) = if covers(outer, inner) {
                (outer, inner, outer_context, inner_context)
            } else if covers(inner, outer) {
                (inner, outer, inner_context, outer_context)
            } else {
                continue;
            };
            return Some(OwnershipAmbiguity {
                outer: OwnedUnit(outer.clone()),
                outer_context: (*outer_context).to_owned(),
                inner: OwnedUnit(inner.clone()),
                inner_context: (*inner_context).to_owned(),
            });
        }
    }
    None
}

fn normalize(raw: &str) -> String {
    let trimmed = raw.trim();
    let head = trimmed.strip_prefix('\\').unwrap_or(trimmed);
    head.strip_suffix('\\').unwrap_or(head).to_string()
}

fn covers(prefix: &str, qname: &str) -> bool {
    if qname.len() < prefix.len() {
        return false;
    }
    let (head, rest) = qname.split_at(prefix.len());
    if !head.eq_ignore_ascii_case(prefix) {
        return false;
    }
    !rest
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
#[path = "declared_surface_tests.rs"]
mod tests;
