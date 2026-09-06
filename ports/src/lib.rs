mod lang;

pub use lang::{Extraction, LanguageBackend};

use domain::{AnchorTarget, ConceptNode, ContextDecl, Edge, Graph, PubFnDecl};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub trait Reader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}

pub trait VerbReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}

pub trait ContextReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError>;
}

pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
    fn relationships(&self, root: &Path) -> Result<Vec<Edge>, ReaderError>;
}

pub trait AnchorResolver {
    fn resolve(&self, qname: &str) -> Option<AnchorTarget>;
}

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("i/o failed on {path}: {cause}")]
    IoFailed { path: PathBuf, cause: String },

    #[error("parse failed at {path}:{line}: {message}")]
    ParseFailed {
        path: PathBuf,
        line: usize,
        message: String,
    },

    #[error("walk failed at {root}: {cause}")]
    WalkFailed { root: PathBuf, cause: String },
}
