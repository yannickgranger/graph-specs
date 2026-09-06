mod lang;

pub use lang::{Extraction, LanguageBackend};

use domain::{
    AnchorTarget, ConceptAnchor, ConceptNode, ContextDecl, Edge, EdgeKind, Graph,
    InvariantAnnotation, PubFnDecl, SpecTree, VerbAnchor, Violation,
};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub trait SignatureNormalizer {
    fn fence_tag(&self) -> &'static str;
    fn normalize(&self, block: &str) -> Result<String, String>;
}

pub struct LoadedFile {
    pub path: PathBuf,
    pub text: String,
}

pub struct SpecFileSet {
    files: Vec<LoadedFile>,
}

impl SpecFileSet {
    #[must_use]
    pub fn new(mut files: Vec<LoadedFile>) -> Self {
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Self { files }
    }

    #[must_use]
    pub fn files(&self) -> &[LoadedFile] {
        &self.files
    }
}

pub struct CodeFileSet {
    files: Vec<LoadedFile>,
}

impl CodeFileSet {
    #[must_use]
    pub fn new(mut files: Vec<LoadedFile>) -> Self {
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Self { files }
    }

    #[must_use]
    pub fn files(&self) -> &[LoadedFile] {
        &self.files
    }
}

pub trait SpecLoader {
    fn load(&self, root: &Path) -> Result<SpecFileSet, ReaderError>;
}

pub trait CodeLoader {
    fn load(&self, root: &Path) -> Result<CodeFileSet, ReaderError>;
}

pub trait SpecReader {
    fn extract(&self, files: &SpecFileSet) -> Result<Graph, ReaderError>;
}

pub trait CodeReader {
    fn extract(&self, files: &CodeFileSet) -> Result<Graph, ReaderError>;
}

pub trait VerbReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}

pub trait ContextReader {
    fn extract_contexts(&self, files: &SpecFileSet) -> Result<Vec<ContextDecl>, ReaderError>;
}

pub trait VerbAnchorReader {
    fn extract_verb_anchors(&self, files: &SpecFileSet) -> Result<Vec<VerbAnchor>, ReaderError>;
}

pub trait ConceptAnchorReader {
    fn extract_concept_anchors(
        &self,
        files: &SpecFileSet,
    ) -> Result<(Vec<ConceptAnchor>, Vec<Violation>), ReaderError>;
}

pub trait AnnotationReader {
    fn extract_annotations(
        &self,
        files: &SpecFileSet,
    ) -> Result<Vec<InvariantAnnotation>, ReaderError>;
}

pub trait SpecTreeReader {
    fn extract_spec_trees(&self, files: &SpecFileSet) -> Result<Vec<SpecTree>, ReaderError>;
}

pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
    fn relationships(&self, root: &Path) -> Result<Vec<Edge>, ReaderError>;
    fn answerable_relationships(&self, root: &Path) -> Result<Vec<EdgeKind>, ReaderError>;
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
