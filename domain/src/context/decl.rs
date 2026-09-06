use crate::Source;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedUnit(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContextDecl {
    pub name: String,
    pub owned_units: Vec<OwnedUnit>,
    pub exports: Vec<ContextExport>,
    pub imports: Vec<ContextImport>,
    pub source: Source,
}

impl ContextDecl {
    #[must_use]
    pub const fn new(
        name: String,
        owned_units: Vec<OwnedUnit>,
        exports: Vec<ContextExport>,
        imports: Vec<ContextImport>,
        source: Source,
    ) -> Self {
        Self {
            name,
            owned_units,
            exports,
            imports,
            source,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExport {
    pub concept: String,
    pub pattern: ContextPattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextImport {
    pub from_context: String,
    pub pattern: ContextPattern,
    pub concept: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextPattern {
    SharedKernel,
    CustomerSupplier,
    Conformist,
    PublishedLanguage,
}

impl ContextPattern {
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::SharedKernel => "SharedKernel",
            Self::CustomerSupplier => "CustomerSupplier",
            Self::Conformist => "Conformist",
            Self::PublishedLanguage => "PublishedLanguage",
        }
    }

    #[must_use]
    pub const fn variants() -> &'static [Self] {
        &[
            Self::SharedKernel,
            Self::CustomerSupplier,
            Self::Conformist,
            Self::PublishedLanguage,
        ]
    }

    #[must_use]
    pub const fn is_doctrine_sanctioned(self) -> bool {
        matches!(self, Self::PublishedLanguage | Self::SharedKernel)
    }
}

impl std::fmt::Display for ContextPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_unit_constructs_and_compares() {
        let a = OwnedUnit("domain".to_string());
        let b = OwnedUnit("domain".to_string());
        let c = OwnedUnit("ports".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn context_pattern_as_label_stable() {
        assert_eq!(ContextPattern::SharedKernel.as_label(), "SharedKernel");
        assert_eq!(
            ContextPattern::CustomerSupplier.as_label(),
            "CustomerSupplier"
        );
        assert_eq!(ContextPattern::Conformist.as_label(), "Conformist");
        assert_eq!(
            ContextPattern::PublishedLanguage.as_label(),
            "PublishedLanguage"
        );
    }

    #[test]
    fn context_pattern_display_matches_label() {
        assert_eq!(format!("{}", ContextPattern::SharedKernel), "SharedKernel");
    }

    #[test]
    fn context_decl_constructs_with_all_sections() {
        let decl = ContextDecl {
            name: "equivalence".to_string(),
            owned_units: vec![
                OwnedUnit("domain".to_string()),
                OwnedUnit("ports".to_string()),
            ],
            exports: vec![ContextExport {
                concept: "Graph".to_string(),
                pattern: ContextPattern::PublishedLanguage,
            }],
            imports: vec![],
            source: Source::Spec {
                format: crate::SpecFormat::Markdown,
                path: std::path::PathBuf::from("specs/concepts/reader.md"),
                line: 12,
                context: None,
            },
        };
        assert_eq!(decl.name, "equivalence");
        assert_eq!(decl.owned_units.len(), 2);
        assert_eq!(decl.exports[0].concept, "Graph");
    }
}
