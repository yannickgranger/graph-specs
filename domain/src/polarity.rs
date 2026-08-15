//! Grounding polarity — which direction a spec heading's obligation points.
//!
//! **Imported, not defined here.** `polarity` is owned upstream: defined in
//! agentry, realized as `cascade::Polarity`. graph-specs is a **Conformist** —
//! it tracks that definition and does not fork it. The three values and
//! their meanings are cited from cascade's `resolve_polarity`, never
//! re-derived.
//!
//! "Conformist" here is prose, not a wired relationship: this is *not*
//! [`crate::ContextPattern::Conformist`], which is a formal enum scoped to
//! this repo's own bounded contexts.

/// Which direction a concept heading's obligation points.
///
/// **Disambiguation.** This is the *concept-grounding* sense of the word —
/// which way a heading's obligation points. It is not the vocabulary
/// system's word-polarity; cascade itself keeps the two apart
/// (`WordPolarity`, "Distinct from `Polarity`").
///
/// `#[non_exhaustive]` because: if cascade adds a value, this enum gains it,
/// and downstream consumers should not have been matching exhaustively.
///
/// Data only — **no predicate methods**. The branch table lives at its one
/// call site in the diff. This matches upstream exactly: cascade's own
/// `Polarity` has zero methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Polarity {
    /// The ordinary obligation: the concept must exist in code. The default
    /// for every heading — absent comment, absent key, or unreadable value
    /// all land here, so a typo leaves the obligation **armed**.
    #[default]
    Declared,
    /// The name is expelled — code must **not** bear it. Absence is clean;
    /// presence is [`crate::Violation::ForbiddenConceptReintroduced`].
    Forbidden,
    /// An example. The heading neither compels a code item nor satisfies
    /// one, so a matching `pub` item falls through to the orphan sweep as
    /// `MissingInSpecs` — the marker cannot launder unspecced public
    /// surface past the gate.
    Illustrative,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_declared() {
        // The fallback direction is the point.
        assert_eq!(Polarity::default(), Polarity::Declared);
    }
}
