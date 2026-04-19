//! Language-neutral port traits.
//!
//! Concrete readers (markdown specs, Rust code, later PHP / TypeScript)
//! implement [`Reader`] and produce graphs of identical shape. The diff
//! engine in [`domain`] operates on graphs, not on source languages.

use domain::{ContextDecl, Graph};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Reader contract: extract a graph from a source root.
///
/// Adapters map their language-specific failures onto [`ReaderError`] at
/// the port boundary. No infrastructure types leak into this signature.
pub trait Reader {
    /// Walk `root` and produce a [`Graph`] of the concepts found.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source file cannot be read,
    /// [`ReaderError::ParseFailed`] if the reader's parser rejects a file,
    /// or [`ReaderError::WalkFailed`] if the directory traversal fails.
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError>;
}

/// v0.4 bounded-context reader contract: extract context declarations
/// from `specs/contexts/<name>.md`.
///
/// Separate port from [`Reader`] per RFC-001 round-1 clean-arch lens —
/// not every adapter parses context files. The markdown adapter will
/// implement both; the rust adapter implements only [`Reader`].
pub trait ContextReader {
    /// Walk `root` for context-declaration files and return the parsed
    /// [`ContextDecl`] list. An empty list is a valid result — v0.3 spec
    /// trees have no `specs/contexts/` directory.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source file cannot be read,
    /// [`ReaderError::ParseFailed`] if a context file is malformed
    /// (unknown pattern, missing required section, duplicate owner), or
    /// [`ReaderError::WalkFailed`] if the directory traversal fails.
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError>;
}

/// Failure modes of a [`Reader`] implementation.
///
/// Variants describe *reading operations*, not domain concerns — which is
/// why this type lives in the port layer rather than in [`domain`].
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
