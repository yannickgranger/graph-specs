#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Polarity {
    #[default]
    Declared,
    Forbidden,
    Illustrative,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_declared() {
        assert_eq!(Polarity::default(), Polarity::Declared);
    }
}
