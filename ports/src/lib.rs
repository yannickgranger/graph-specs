//! Language-neutral port traits.
//!
//! Concrete readers (markdown specs, Rust code, later PHP / TypeScript)
//! implement [`Reader`] and produce graphs of identical shape. The diff
//! engine in [`domain`] operates on graphs, not on source languages.
//!
//! Code-side adapters additionally implement [`LanguageBackend`] (see
//! [`lang`]), which is the lower-level, language-specific extractor that
//! emits flat [`ConceptNode`] / [`Edge`] vectors. The [`Reader`] adapter
//! wraps a [`LanguageBackend`] with language-neutral graph assembly.

mod lang;

pub use lang::{Extraction, LanguageBackend};

use domain::{ConceptNode, ContextDecl, Graph, PubFnDecl};
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

/// Verb-extraction port — per RFC-005 §3.2, sibling to [`ContextReader`].
///
/// Not every adapter extracts pub-fn declarations (markdown has no code
/// items to walk); returning an empty `Vec` is the correct implementation
/// for adapters that do not extract verbs. Independent of [`Reader::extract`]
/// — never invoked by `check`, only by the `report` subcommand (Slice B).
///
/// Mirrors RFC-001 §3.6's introduction of [`ContextReader`] as a separate
/// port trait for an opt-in read capability. Per RFC-005 §3.2.
pub trait VerbReader {
    /// Collect every top-level `pub fn` (name + source + owning crate)
    /// under `root`. Returns an empty `Vec` on adapters that do not extract
    /// verbs — no panic, no error.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source file cannot be read,
    /// [`ReaderError::ParseFailed`] if the reader's parser rejects a file,
    /// or [`ReaderError::WalkFailed`] if the directory traversal fails.
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError>;
}

/// Separate from [`Reader`] per RFC-001 clean-arch lens — not every
/// adapter parses context files. Markdown implements both; rust
/// implements only [`Reader`].
pub trait ContextReader {
    /// An empty `Vec` is the valid v0.3 result (no `specs/contexts/`).
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source file cannot be read,
    /// [`ReaderError::ParseFailed`] if a context file is malformed
    /// (unknown pattern, missing required section, duplicate owner), or
    /// [`ReaderError::WalkFailed`] if the directory traversal fails.
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError>;
}

/// Code-side containment port (RFC-010 §3.3).
///
/// Where [`Reader::extract`] produces a full type-equivalence [`Graph`]
/// (concepts + edges + signatures), `CodeFacts` answers a narrower
/// question: *what concepts does the code contain, and what is each one's
/// language-agnostic containment provenance* — the `module_path` / `unit` /
/// `context` triple on [`ConceptNode`]. The abstraction-ladder cohesion
/// pass (R10-3) needs that triple to resolve each concept's code-side
/// bounded context.
///
/// Two adapters implement it under the §3.3 routing rule: the source-walking
/// `RustReader` (multi-crate repos — the parity reference) and the
/// feature-gated cfdb-query ACL (per-crate repos). Both emit the **agnostic**
/// triple, never cfdb's Rust-specific prop names, so the diff engine stays
/// language-neutral.
pub trait CodeFacts {
    /// Return every code-side concept under `root`, each carrying its
    /// containment provenance.
    ///
    /// Adapters that read a pre-extracted fact store (the cfdb-query ACL)
    /// may treat `root` as advisory and read from a source fixed at
    /// construction; source-walking adapters walk `root` directly.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source or fact file cannot be
    /// read, [`ReaderError::ParseFailed`] if it cannot be parsed, or
    /// [`ReaderError::WalkFailed`] if directory traversal fails.
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
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
