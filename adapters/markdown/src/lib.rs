mod bullets;
mod contexts;
mod front_matter;
#[allow(clippy::redundant_pub_crate)]
mod grounding;
mod invariants;
mod markdown_utils;
mod section;
mod tree;

pub use bullets::{parse_impl_bullet, parse_verb_bullet};
pub use tree::{assemble_spec_trees, assemble_tree, HeadingNode, SpecTree};

use crate::front_matter::{has_behavioral_substance, is_behavioral_context};
use crate::invariants::extract_annotations_from_source;
use crate::markdown_utils::path_under_dir;
use crate::section::extract_from_source;
use domain::{
    ConceptAnchor, ConceptNode, ContextDecl, Edge, Graph, InvariantAnnotation, VerbAnchor,
    Violation,
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
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            let mut malformed_scratch: Vec<Violation> = Vec::new();
            let mut concept_anchors_scratch: Vec<ConceptAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut BulletSink {
                    nodes: &mut nodes,
                    edges: &mut edges,
                    verb_anchors: &mut verb_anchors_scratch,
                    concept_anchors: &mut concept_anchors_scratch,
                    malformed: &mut malformed_scratch,
                },
            )?;
        }

        Ok(Graph::new(nodes, edges))
    }
}

impl ContextReader for MarkdownReader {
    fn extract_contexts(&self, root: &Path) -> Result<Vec<ContextDecl>, ReaderError> {
        contexts::walk_contexts(root)
    }
}

struct BulletSink<'a> {
    nodes: &'a mut Vec<ConceptNode>,
    edges: &'a mut Vec<Edge>,
    verb_anchors: &'a mut Vec<VerbAnchor>,
    concept_anchors: &'a mut Vec<ConceptAnchor>,
    malformed: &'a mut Vec<Violation>,
}

impl MarkdownReader {
    pub fn extract_malformed_anchors(&self, root: &Path) -> Result<Vec<Violation>, ReaderError> {
        let mut malformed: Vec<Violation> = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            let mut concept_anchors_scratch: Vec<ConceptAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut BulletSink {
                    nodes: &mut nodes_scratch,
                    edges: &mut edges_scratch,
                    verb_anchors: &mut verb_anchors_scratch,
                    concept_anchors: &mut concept_anchors_scratch,
                    malformed: &mut malformed,
                },
            )?;
        }

        Ok(malformed)
    }

    pub fn extract_verb_anchors(&self, root: &Path) -> Result<Vec<VerbAnchor>, ReaderError> {
        let mut verb_anchors: Vec<VerbAnchor> = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut concept_anchors_scratch: Vec<ConceptAnchor> = Vec::new();
            let mut malformed_scratch: Vec<Violation> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut BulletSink {
                    nodes: &mut nodes_scratch,
                    edges: &mut edges_scratch,
                    verb_anchors: &mut verb_anchors,
                    concept_anchors: &mut concept_anchors_scratch,
                    malformed: &mut malformed_scratch,
                },
            )?;
        }

        Ok(verb_anchors)
    }

    pub fn extract_concept_anchors(&self, root: &Path) -> Result<Vec<ConceptAnchor>, ReaderError> {
        let mut concept_anchors: Vec<ConceptAnchor> = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            let mut malformed_scratch: Vec<Violation> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut BulletSink {
                    nodes: &mut nodes_scratch,
                    edges: &mut edges_scratch,
                    verb_anchors: &mut verb_anchors_scratch,
                    concept_anchors: &mut concept_anchors,
                    malformed: &mut malformed_scratch,
                },
            )?;
        }

        Ok(concept_anchors)
    }

    pub fn extract_invariant_annotations(
        &self,
        root: &Path,
    ) -> Result<Vec<InvariantAnnotation>, ReaderError> {
        let mut result = Vec::new();

        for (path, source) in walk_concept_sources(root)? {
            let dialect = grounding::read(&path, &source)?;
            extract_annotations_from_source(&source, &path, &dialect, &mut result);
        }

        Ok(result)
    }
}

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

fn concept_walk_root(root: &Path) -> PathBuf {
    let concepts_subdir = root.join("concepts");
    if concepts_subdir.is_dir() {
        concepts_subdir
    } else {
        root.to_path_buf()
    }
}

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
