//! Bounded-context declaration vocabulary — the `Owns` / `Exports` /
//! `Imports` surfaces parsed from `specs/contexts/<name>.md` (RFC-001).

use crate::Source;

/// A crate, npm package, Go module, or equivalent — named deliberately to
/// keep the domain model language-agnostic across future adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedUnit(pub String);

/// Parsed from `specs/contexts/<name>.md`. `exports` and `imports` model
/// the DDD context-mapping patterns in [`ContextPattern`].
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
    /// Required constructor outside the defining crate — `#[non_exhaustive]`
    /// prevents the struct-literal form in external callers (markdown
    /// adapter, downstream consumers).
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

/// Export-centric framing (Evans Ch. 14): the supplying context is
/// authoritative about what it publishes. Asymmetric declarations fire
/// [`crate::ContextViolation::CrossEdgeUndeclared`].
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

/// A DDD context-mapping pattern. v0.4 ships four; Anti-Corruption Layer,
/// Separate Ways, and Open Host Service are deferred to v0.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContextPattern {
    SharedKernel,
    CustomerSupplier,
    Conformist,
    PublishedLanguage,
}

impl ContextPattern {
    /// Wire-form label used in violation messages and spec parsing.
    /// Stable across versions.
    #[must_use]
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::SharedKernel => "SharedKernel",
            Self::CustomerSupplier => "CustomerSupplier",
            Self::Conformist => "Conformist",
            Self::PublishedLanguage => "PublishedLanguage",
        }
    }

    /// Canonical iterator over v0.4 variants — the single source of truth
    /// for parsers and error-message enumeration. Adding a v0.5 variant
    /// only requires updating this list and `as_label`.
    #[must_use]
    pub const fn variants() -> &'static [Self] {
        &[
            Self::SharedKernel,
            Self::CustomerSupplier,
            Self::Conformist,
            Self::PublishedLanguage,
        ]
    }

    /// Returns `true` for patterns that doctrine-sanction cross-context
    /// appearances (no council escalation warranted). Per RFC-005 §3.3
    /// dry-run DDD-C: `PublishedLanguage` and `SharedKernel` are the two
    /// sanctioned patterns; `Conformist` and `CustomerSupplier` signal
    /// potential split-brain. Forward-compatible with `#[non_exhaustive]`
    /// — new variants must classify themselves by adding a match arm here.
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
                path: std::path::PathBuf::from("specs/concepts/reader.md"),
                line: 12,
            },
        };
        assert_eq!(decl.name, "equivalence");
        assert_eq!(decl.owned_units.len(), 2);
        assert_eq!(decl.exports[0].concept, "Graph");
    }
}
