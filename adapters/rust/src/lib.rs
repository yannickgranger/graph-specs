//! Rust code reader — concept-level.
//!
//! Walks a directory tree, parses each `*.rs` file with `syn`, and emits a
//! [`ConceptNode`] for every top-level `pub struct`, `pub enum`, `pub trait`,
//! `pub type`. Honours the filter rules documented in `specs/dialect.md`:
//! non-public items, `#[cfg(test)]`-gated items, and files under
//! `target/` / `.git/` / `.claude/` / `.proofs/` / per-crate `tests/`,
//! `benches/`, `examples/` are skipped, as is any directory carrying a
//! `CACHEDIR.TAG` marker — a build tree keeps its generated `pub` items off
//! the surface however `--target-dir` happened to name it.
//!
//! Scope: only top-level items in each file are visited. Concepts nested
//! inside `pub mod foo { ... }` are not extracted at this level.

mod anchor_resolver;
mod cfg_gate;
mod concepts;
mod edges;
mod normalize;
mod provenance;
mod pub_fns;
mod walk;

#[cfg(test)]
mod tests;

pub use anchor_resolver::RustAnchorResolver;
pub use normalize::normalize;

use domain::{ConceptNode, Edge, Graph, PubFnDecl};
use ports::{CodeFacts, Extraction, LanguageBackend, Reader, ReaderError, VerbReader};
use std::path::Path;
use walkdir::WalkDir;

/// Low-level Rust extractor.
///
/// Walks a source tree once and emits flat concepts + raw edges. Used by
/// [`RustReader`] to build a [`Graph`] and, in future, by cfdb's Rust
/// ingestor (RFC-005 / #83 reframe).
#[derive(Debug, Default)]
pub struct RustBackend;

impl LanguageBackend for RustBackend {
    fn detect(&self, code_root: &Path) -> bool {
        code_root.join("Cargo.toml").exists()
    }

    fn extract(&self, code_root: &Path) -> Result<Extraction, ReaderError> {
        let mut concepts = Vec::new();
        let mut raw_edges: Vec<Edge> = Vec::new();

        let walker = WalkDir::new(code_root)
            .into_iter()
            .filter_entry(|e| !walk::is_excluded_dir(e));

        for entry in walker {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: code_root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let (parsed, path) = walk::read_and_parse(entry.path().to_path_buf())?;
            concepts::extract_from_file(&parsed, &path, code_root, &mut concepts, &mut raw_edges);
        }

        Ok(Extraction {
            concepts,
            raw_edges,
        })
    }
}

/// High-level Rust reader.
///
/// Wraps [`RustBackend`] with language-neutral graph assembly: pulls
/// concepts + raw edges, filters edges against the discovered concept set,
/// and returns a [`Graph`] for the diff engine.
#[derive(Debug, Default)]
pub struct RustReader;

impl Reader for RustReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let Extraction {
            concepts,
            raw_edges,
        } = RustBackend.extract(root)?;
        let edges = edges::filter_by_known_concepts(raw_edges, &concepts);
        Ok(Graph::new(concepts, edges))
    }
}

impl CodeFacts for RustReader {
    /// Source-walk `CodeFacts` (RFC-010 §3.3): the concept set the [`Reader`]
    /// graph already carries, each node bearing the per-file containment
    /// provenance attached by `extract_from_file` (`module_path` collapsed to
    /// crate root, `unit` relative to the code root). This is the parity
    /// reference the cfdb-query ACL must match (0-mismatch on
    /// `module_path` / `unit`).
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
        Ok(Reader::extract(self, root)?.nodes)
    }
}

impl VerbReader for RustReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError> {
        let mut pub_fns = Vec::new();

        let walker = WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !walk::is_excluded_dir(e));

        for entry in walker {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let owned_unit = provenance::find_owned_unit(entry.path(), root);
            let (parsed, path) = walk::read_and_parse(entry.path().to_path_buf())?;

            for item in &parsed.items {
                pub_fns::visit_top_level_fn(item, &path, owned_unit.as_deref(), &mut pub_fns);
                pub_fns::visit_impl_block(item, &path, owned_unit.as_deref(), &mut pub_fns);
            }
        }

        Ok(pub_fns)
    }
}
