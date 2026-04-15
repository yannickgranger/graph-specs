//! Language-neutral port traits.
//!
//! Concrete readers (markdown specs, Rust code, later PHP / TypeScript)
//! implement [`Reader`] and produce graphs of identical shape. The diff
//! engine in [`domain`] operates on graphs, not on source languages.

use domain::Graph;
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
