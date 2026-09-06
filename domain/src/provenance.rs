#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    pub module_path: Option<String>,
    pub unit: Option<String>,
    pub context: Option<String>,
}

impl Provenance {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            module_path: None,
            unit: None,
            context: None,
        }
    }
}
