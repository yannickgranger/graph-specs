//! Markdown spec reader — concept-level.
//!
//! Walks a directory tree, parses each `*.md` file with `pulldown-cmark`,
//! and emits a [`ConceptNode`] for every `##` or `###` heading. Per the
//! dialect spec (`specs/dialect.md`), prose, tables, images, and links
//! are ignored — only `h2`/`h3` heading text, fenced `rust` blocks
//! (v0.2), and recognised bullet prefixes (v0.3) participate.
//!
//! Headings containing generic parameters are normalised: `## Graph<T>`
//! records the concept as `Graph`.
//!
//! ## v0.3 bullet edges
//!
//! Inside a concept section, bullet lines beginning with one of the
//! recognised relationship prefixes are collected as declared edges:
//!
//! - `- implements: <Target>` → [`EdgeKind::Implements`]
//! - `- depends on: <Target>` → [`EdgeKind::DependsOn`]
//! - `- returns: <Target>` → [`EdgeKind::Returns`]
//!
//! Prefix matching is case-sensitive. Bullets that do not match any
//! prefix are prose and are ignored. Empty targets (`- implements:`)
//! are also ignored.

mod bullets;
mod contexts;
mod front_matter;
mod grounding;
mod invariants;
mod markdown_utils;
mod section;
mod tree;

pub use bullets::{parse_impl_bullet, parse_verb_bullet};
pub use tree::{assemble_spec_trees, assemble_tree, HeadingNode, SpecTree};

use crate::front_matter::{
    blank_front_matter, has_behavioral_substance, is_behavioral_context, is_draft,
};
use crate::invariants::extract_annotations_from_source;
use crate::markdown_utils::path_under_dir;
use crate::section::extract_from_source;
use domain::{
    ConceptAnchor, ConceptNode, ContextDecl, Edge, Graph, InvariantAnnotation, VerbAnchor,
};
use ports::{ContextReader, Reader, ReaderError};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct MarkdownReader;

impl Reader for MarkdownReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            // Draft files are **parsed, not skipped**. Every
            // concept heading in one is marked, which
            // relaxes its code-existence obligation without hiding it — the
            // marker surfaces as a pending/realized record instead.
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            let mut concept_anchors_scratch: Vec<ConceptAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut nodes,
                &mut edges,
                &mut verb_anchors_scratch,
                &mut concept_anchors_scratch,
            );
        }

        Ok(Graph::new(nodes, edges))
    }
}

impl ContextReader for MarkdownReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        contexts::walk_contexts(root)
    }
}

impl MarkdownReader {
    /// Walk `root` and collect every `- verb: <ident>` anchor from concept
    /// spec files (`concepts/` subdir if present, else root). Skips
    /// `contexts/` files (different dialect).
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] or [`ReaderError::WalkFailed`] on
    /// I/O failures.
    pub fn extract_verb_anchors(&self, root: &Path) -> Result<Vec<VerbAnchor>, ReaderError> {
        let mut verb_anchors: Vec<VerbAnchor> = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            // RFC-013 §3.3: draft files are parsed, not skipped. A `- verb:`
            // anchor under a marked heading is extracted as normal; it simply
            // imposes no obligation while the concept is pending (the diff's
            // uniform obligation skip, RFC-013 §3.4).
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut concept_anchors_scratch: Vec<ConceptAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut nodes_scratch,
                &mut edges_scratch,
                &mut verb_anchors,
                &mut concept_anchors_scratch,
            );
        }

        Ok(verb_anchors)
    }

    /// Walk `root` and collect every `- impl: <qname>` concept anchor
    /// (RFC-012 §3.2) from concept spec files (`concepts/` subdir if
    /// present, else root). Mirrors [`MarkdownReader::extract_verb_anchors`]:
    /// skips `contexts/` (different dialect) and `status: draft` files.
    /// Detection only — resolving the qname against code is the
    /// `AnchorResolver` port's concern (R12-3).
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] or [`ReaderError::WalkFailed`] on
    /// I/O failures.
    pub fn extract_concept_anchors(&self, root: &Path) -> Result<Vec<ConceptAnchor>, ReaderError> {
        let mut concept_anchors: Vec<ConceptAnchor> = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut nodes_scratch,
                &mut edges_scratch,
                &mut verb_anchors_scratch,
                &mut concept_anchors,
            );
        }

        Ok(concept_anchors)
    }

    /// Extract all `[enforced-by:]` / `[prose-only:]` bracketed annotations
    /// from `#### Operational invariants` sections in spec files under `root`.
    ///
    /// Per RFC-005 §3.2: uses a **fresh** `Parser::new(source).into_offset_iter()`
    /// per file — NOT shared with the concept walk. The existing `handle_event`
    /// (H2/H3-only) is NOT extended; this method's parser loop introduces its
    /// own `HeadingLevel::H4` arm.
    ///
    /// **Failure mode (Invariant 7):** a bullet that looks like an annotation
    /// (contains `[enforced-by:` or `[prose-only:`) but fails the bracket
    /// grammar emits `tracing::warn!` and is dropped from the returned `Vec`.
    /// `Err` is reserved for I/O / fundamental parse failures only.
    ///
    /// # Errors
    ///
    /// Returns [`ReaderError::IoFailed`] or [`ReaderError::WalkFailed`] on
    /// I/O failures. Grammar errors are tolerated per Invariant 7.
    pub fn extract_invariant_annotations(
        &self,
        root: &Path,
    ) -> Result<Vec<InvariantAnnotation>, ReaderError> {
        let mut result = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            // Draft specs are skipped wholesale — see [`is_draft`] and the
            // matching guard in `extract`.
            if is_draft(&source) {
                continue;
            }

            extract_annotations_from_source(&source, &path, &mut result);
        }

        Ok(result)
    }
}

/// Walk `root` for concept-spec markdown files and read each into memory.
///
/// v0.4 layout: when `root` contains a `concepts/` subdir, only that subdir
/// is walked — this scopes the concept reader away from `contexts/*.md`
/// (different dialect) and prose sidecars like `specs/dialect.md` or
/// `specs/ndjson-output.md`. Absence of a `concepts/` subdir preserves v0.3
/// behaviour — walk the root directly. Even under the v0.3 fallback, a
/// nested `contexts/` subtree is excluded (defence in depth — it is owned
/// by the [`ContextReader`] impl).
///
/// Draft files are returned like any other (RFC-013 §3.3) — their marker
/// state is carried per-heading on [`ConceptNode::marked`], not by omitting
/// the file from the walk.
///
/// # Errors
///
/// Returns [`ReaderError::WalkFailed`] or [`ReaderError::IoFailed`] on I/O
/// failures.
fn walk_concept_sources(root: &Path) -> Result<Vec<(PathBuf, String)>, ReaderError> {
    let walk_root = concept_walk_root(root);
    let mut out = Vec::new();
    for entry in WalkDir::new(walk_root) {
        let entry = entry.map_err(|e| ReaderError::WalkFailed {
            root: root.to_path_buf(),
            cause: e.to_string(),
        })?;
        if let Some(pair) = read_concept_entry(&entry)? {
            out.push(pair);
        }
    }
    Ok(out)
}

/// `concepts/` subdir when present, else `root` itself.
fn concept_walk_root(root: &Path) -> PathBuf {
    let concepts_subdir = root.join("concepts");
    if concepts_subdir.is_dir() {
        concepts_subdir
    } else {
        root.to_path_buf()
    }
}

/// Read one walked entry into a `(path, source)` pair, or `None` when the
/// entry is skipped: not a file, not `.md`, or under a nested `contexts/`
/// subtree (defence in depth — owned by the [`ContextReader`] impl).
fn read_concept_entry(entry: &walkdir::DirEntry) -> Result<Option<(PathBuf, String)>, ReaderError> {
    if !entry.file_type().is_file() {
        return Ok(None);
    }
    if entry.path().extension().is_none_or(|ext| ext != "md") {
        return Ok(None);
    }
    if path_under_dir(entry.path(), "contexts") {
        return Ok(None);
    }
    let path = entry.path();
    let source = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
        path: path.to_path_buf(),
        cause: e.to_string(),
    })?;
    Ok(Some((path.to_path_buf(), source)))
}

#[cfg(test)]
mod tests;
