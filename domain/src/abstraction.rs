//! The abstraction ladder.
//!
//! Heading depth maps to a typed abstraction level. Depth is
//! *authoritative*: in a conformant spec a heading's role **is** its
//! depth, and the tool enforces the mapping rather than inferring intent.
//! A `pub` type written at H4 is drift the gate surfaces.
//!
//! `H1 = Context`, `H2 = Concept` (the diff unit), `H3 = SubConcept`
//! (a nested pub type, diffed at L2), `H4+ = Member` (emitted, not diffed).

/// One rung of the four-level abstraction ladder.
///
/// Adapters call [`AbstractionLevel::from_heading_depth`] rather than
/// `match`-ing the enum, so `#[non_exhaustive]` never forces a dead
/// wildcard arm in adapter crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbstractionLevel {
    /// `H1` — a bounded context identifier.
    Context,
    /// `H2` — a concept (the diff unit; one `pub` type).
    Concept,
    /// `H3` — a sub-concept (a nested `pub` type, diffed at L2).
    SubConcept,
    /// `H4+` — a member (field / variant / param; emitted, not diffed).
    Member,
}

impl AbstractionLevel {
    /// Map a markdown heading depth (`#` = 1, `##` = 2, …) to its rung.
    ///
    /// `0 | 1 → Context`, `2 → Concept`, `3 → SubConcept`, `4+ → Member`.
    /// Total over all `u8` inputs so callers never need a fallback arm.
    #[must_use]
    pub const fn from_heading_depth(depth: u8) -> Self {
        match depth {
            0 | 1 => Self::Context,
            2 => Self::Concept,
            3 => Self::SubConcept,
            _ => Self::Member,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_depth_maps_to_each_rung() {
        assert_eq!(
            AbstractionLevel::from_heading_depth(1),
            AbstractionLevel::Context
        );
        assert_eq!(
            AbstractionLevel::from_heading_depth(2),
            AbstractionLevel::Concept
        );
        assert_eq!(
            AbstractionLevel::from_heading_depth(3),
            AbstractionLevel::SubConcept
        );
        assert_eq!(
            AbstractionLevel::from_heading_depth(4),
            AbstractionLevel::Member
        );
    }

    #[test]
    fn depth_beyond_four_saturates_to_member() {
        assert_eq!(
            AbstractionLevel::from_heading_depth(5),
            AbstractionLevel::Member
        );
        assert_eq!(
            AbstractionLevel::from_heading_depth(u8::MAX),
            AbstractionLevel::Member
        );
    }

    #[test]
    fn depth_zero_folds_to_context() {
        // Depth 0 should not occur for real headings; folding to Context
        // keeps the constructor total without a panic path.
        assert_eq!(
            AbstractionLevel::from_heading_depth(0),
            AbstractionLevel::Context
        );
    }
}
