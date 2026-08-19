#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    pub module_path: Option<String>,
    pub unit: Option<String>,
    pub context: Option<String>,
}
