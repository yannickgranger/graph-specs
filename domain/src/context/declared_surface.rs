use super::decl::ContextDecl;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeclaredSurface {
    prefixes: Vec<String>,
}

impl DeclaredSurface {
    #[must_use]
    pub fn from_contexts(contexts: &[ContextDecl]) -> Self {
        let mut prefixes: Vec<String> = contexts
            .iter()
            .flat_map(|ctx| ctx.owned_units.iter())
            .map(|unit| normalize(&unit.0))
            .filter(|unit| !unit.is_empty())
            .collect();
        prefixes.sort();
        prefixes.dedup();
        Self { prefixes }
    }

    #[must_use]
    pub fn admits(&self, qname: &str) -> bool {
        self.unit_of(qname).is_some()
    }

    #[must_use]
    pub fn unit_of(&self, qname: &str) -> Option<&str> {
        let qname = normalize(qname);
        self.prefixes
            .iter()
            .filter(|prefix| covers(prefix, &qname))
            .max_by_key(|prefix| prefix.len())
            .map(String::as_str)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.prefixes.is_empty()
    }
}

fn normalize(raw: &str) -> String {
    raw.trim().trim_start_matches('\\').to_string()
}

fn covers(prefix: &str, qname: &str) -> bool {
    if qname == prefix {
        return true;
    }
    qname.strip_prefix(prefix).is_some_and(|rest| {
        rest.chars()
            .next()
            .is_some_and(|c| !c.is_alphanumeric() && c != '_')
    })
}

#[cfg(test)]
#[path = "declared_surface_tests.rs"]
mod tests;
