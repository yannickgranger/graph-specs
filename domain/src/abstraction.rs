#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbstractionLevel {
    Context,
    Concept,
    SubConcept,
    Member,
}

impl AbstractionLevel {
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
        assert_eq!(
            AbstractionLevel::from_heading_depth(0),
            AbstractionLevel::Context
        );
    }
}
