use crate::ReaderError;
use domain::{ConceptNode, Edge};
use std::path::Path;

#[derive(Debug, Default)]
pub struct Extraction {
    pub concepts: Vec<ConceptNode>,
    pub raw_edges: Vec<Edge>,
}

pub trait LanguageBackend {
    #[must_use]
    fn detect(&self, code_root: &Path) -> bool;

    fn extract(&self, code_root: &Path) -> Result<Extraction, ReaderError>;
}
