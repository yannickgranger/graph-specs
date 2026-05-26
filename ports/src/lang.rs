//! Language-backend port: pluggable code-side extractors.
//!
//! Each `impl LanguageBackend for FooBackend` implements concept and edge
//! extraction for one source language (Rust, PHP, TypeScript). The trait is
//! the seam that lets graph-specs add new languages without touching the
//! diff engine in [`domain`] or the [`crate::Reader`] adapter that wraps a
//! backend into the language-neutral [`domain::Graph`] shape consumed by
//! `application`.
//!
//! ## Dispatch
//!
//! [`LanguageBackend::detect`] returns true when the backend is the right
//! choice for the given `code_root` — typically by inspecting marker files
//! (`Cargo.toml` for Rust, `composer.json` for PHP, `package.json` /
//! `tsconfig.json` for TypeScript). The CLI's `check` command iterates
//! registered backends and uses the first whose `detect` returns true.
//!
//! ## Layering
//!
//! `LanguageBackend` is intentionally low-level: it returns a flat
//! [`Extraction`] of concepts and raw edges, NOT a [`domain::Graph`]. Graph
//! assembly (filtering raw edges against the known-concept set, etc.) is
//! performed by the calling [`crate::Reader`] adapter, in language-neutral
//! code. This keeps each backend small, single-walk, and testable in
//! isolation.
//!
//! ## Future shared crates
//!
//! The roadmap (graph-specs-rust #83 reframe, 2026-04-27) is for each
//! `impl LanguageBackend for FooBackend` to be a thin wrapper over a
//! language-specific extractor crate (`<lang>-items`) shared with cfdb.
//! Today this trait is satisfied by in-tree adapters; the wrapper shape
//! is preserved so the eventual move to shared crates is mechanical.

use crate::ReaderError;
use domain::{ConceptNode, Edge};
use std::path::Path;

/// Output of one backend extraction pass over a source root.
///
/// `concepts` carries the language's pub-equivalent items as
/// [`ConceptNode`] values (already enriched with [`domain::SignatureState`]
/// where the backend supports signature equivalence). `raw_edges` carries
/// every edge the backend declared, BEFORE filtering against the
/// known-concept set — that filter is the calling adapter's job, in
/// language-neutral code.
#[derive(Debug, Default)]
pub struct Extraction {
    pub concepts: Vec<ConceptNode>,
    pub raw_edges: Vec<Edge>,
}

/// Code-side extractor for one source language.
///
/// One backend implementation per language; each walks `code_root` exactly
/// once per [`extract`](Self::extract) call and emits the union of concepts
/// and declared edges. The trait itself stays free of [`domain::Graph`] so
/// backends can be tested without depending on diff semantics.
///
/// # Example
///
/// ```
/// use ports::{Extraction, LanguageBackend, ReaderError};
/// use std::path::Path;
///
/// struct StubBackend;
///
/// impl LanguageBackend for StubBackend {
///     fn detect(&self, code_root: &Path) -> bool {
///         code_root.join("stub.marker").exists()
///     }
///     fn extract(&self, _code_root: &Path) -> Result<Extraction, ReaderError> {
///         Ok(Extraction::default())
///     }
/// }
///
/// let backend = StubBackend;
/// assert!(!backend.detect(Path::new("/tmp")));
/// let extraction = backend.extract(Path::new("/tmp")).unwrap();
/// assert!(extraction.concepts.is_empty());
/// assert!(extraction.raw_edges.is_empty());
/// ```
pub trait LanguageBackend {
    /// Returns true when this backend should be used for `code_root`.
    ///
    /// Detection is typically by marker file (`Cargo.toml` for Rust). An
    /// instance argument (`&self`) is taken even when the answer is
    /// stateless so future configurable backends (PHP namespace-prefix,
    /// TS `tsconfig.json` resolution rules) can carry config without
    /// breaking the trait shape.
    #[must_use]
    fn detect(&self, code_root: &Path) -> bool;

    /// Walk `code_root` once and emit concepts + raw edges.
    ///
    /// Concepts are the language's pub-equivalent items: `pub` types in
    /// Rust; `class` / `interface` / `trait` in PHP; `interface` /
    /// `type` / exported `class` in TypeScript. Concept name normalization
    /// is the backend's responsibility (e.g. PHP strips a configured
    /// namespace prefix).
    ///
    /// Raw edges are emitted unfiltered. Backends without edge support
    /// emit an empty `raw_edges` vector — see [`Extraction::default`].
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] if a source file cannot be read,
    /// [`ReaderError::ParseFailed`] if a file fails to parse, or
    /// [`ReaderError::WalkFailed`] if the directory traversal fails.
    fn extract(&self, code_root: &Path) -> Result<Extraction, ReaderError>;
}
