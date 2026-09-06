mod bullets;
mod contexts;
mod front_matter;
#[allow(clippy::redundant_pub_crate)]
mod grounding;
mod invariants;
mod markdown_utils;
mod section;
mod tree;

use crate::front_matter::{has_behavioral_substance, is_behavioral_context};
use crate::invariants::extract_annotations_from_source;
use crate::markdown_utils::path_under_dir;
use crate::section::extract_from_source;
use domain::{
    ConceptAnchor, ConceptNode, ContextDecl, Edge, Graph, InvariantAnnotation, SpecTree,
    VerbAnchor, Violation,
};
use ports::{
    AnnotationReader, ConceptAnchorReader, ContextReader, LoadedFile, ReaderError,
    SignatureNormalizer, SpecFileSet, SpecLoader, SpecReader, SpecTreeReader, VerbAnchorReader,
};
use std::path::Path;
use walkdir::WalkDir;

pub struct MarkdownReader {
    normalizers: &'static [&'static dyn SignatureNormalizer],
}

impl MarkdownReader {
    #[must_use]
    pub const fn new(normalizers: &'static [&'static dyn SignatureNormalizer]) -> Self {
        Self { normalizers }
    }
}

impl SpecReader for MarkdownReader {
    fn extract(&self, files: &SpecFileSet) -> Result<Graph, ReaderError> {
        let normalizers = self.normalizers;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (path, source) in concept_files(files)
            .into_iter()
            .map(|f| (f.path.clone(), f.text.clone()))
        {
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
                normalizers,
            )?;
        }

        Ok(Graph::new(nodes, edges))
    }
}

impl SpecTreeReader for MarkdownReader {
    fn extract_spec_trees(&self, files: &SpecFileSet) -> Result<Vec<SpecTree>, ReaderError> {
        tree::assemble_spec_trees(files)
    }
}

impl ContextReader for MarkdownReader {
    fn extract_contexts(&self, files: &SpecFileSet) -> Result<Vec<ContextDecl>, ReaderError> {
        contexts::walk_contexts(files)
    }
}

struct BulletSink<'a> {
    nodes: &'a mut Vec<ConceptNode>,
    edges: &'a mut Vec<Edge>,
    verb_anchors: &'a mut Vec<VerbAnchor>,
    concept_anchors: &'a mut Vec<ConceptAnchor>,
    malformed: &'a mut Vec<Violation>,
}

impl MarkdownReader {}

impl VerbAnchorReader for MarkdownReader {
    fn extract_verb_anchors(&self, files: &SpecFileSet) -> Result<Vec<VerbAnchor>, ReaderError> {
        let mut verb_anchors: Vec<VerbAnchor> = Vec::new();

        for (path, source) in concept_files(files)
            .into_iter()
            .map(|f| (f.path.clone(), f.text.clone()))
        {
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
                &[],
            )?;
        }

        Ok(verb_anchors)
    }
}

impl ConceptAnchorReader for MarkdownReader {
    fn extract_concept_anchors(
        &self,
        files: &SpecFileSet,
    ) -> Result<(Vec<ConceptAnchor>, Vec<Violation>), ReaderError> {
        let mut concept_anchors: Vec<ConceptAnchor> = Vec::new();
        let mut malformed: Vec<Violation> = Vec::new();

        for (path, source) in concept_files(files)
            .into_iter()
            .map(|f| (f.path.clone(), f.text.clone()))
        {
            let mut nodes_scratch: Vec<ConceptNode> = Vec::new();
            let mut edges_scratch: Vec<Edge> = Vec::new();
            let mut verb_anchors_scratch: Vec<VerbAnchor> = Vec::new();
            extract_from_source(
                &source,
                &path,
                &mut BulletSink {
                    nodes: &mut nodes_scratch,
                    edges: &mut edges_scratch,
                    verb_anchors: &mut verb_anchors_scratch,
                    concept_anchors: &mut concept_anchors,
                    malformed: &mut malformed,
                },
                &[],
            )?;
        }

        Ok((concept_anchors, malformed))
    }
}

impl AnnotationReader for MarkdownReader {
    fn extract_annotations(
        &self,
        files: &SpecFileSet,
    ) -> Result<Vec<InvariantAnnotation>, ReaderError> {
        let mut result = Vec::new();

        for (path, source) in concept_files(files)
            .into_iter()
            .map(|f| (f.path.clone(), f.text.clone()))
        {
            let dialect = grounding::read(&path, &source)?;
            extract_annotations_from_source(&source, &path, &dialect, &mut result);
        }

        Ok(result)
    }
}

impl SpecLoader for MarkdownReader {
    fn load(&self, root: &Path) -> Result<SpecFileSet, ReaderError> {
        let mut files = Vec::new();
        for entry in WalkDir::new(root) {
            let entry = entry.map_err(|e| ReaderError::WalkFailed {
                root: root.to_path_buf(),
                cause: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().is_none_or(|ext| ext != "md") {
                continue;
            }
            let path = entry.path();
            let text = std::fs::read_to_string(path).map_err(|e| ReaderError::IoFailed {
                path: path.to_path_buf(),
                cause: e.to_string(),
            })?;
            files.push(LoadedFile {
                path: path.to_path_buf(),
                text,
            });
        }
        Ok(SpecFileSet::new(files))
    }
}

#[must_use]
pub fn concept_files(files: &SpecFileSet) -> Vec<&LoadedFile> {
    let scoped = files
        .files()
        .iter()
        .any(|f| path_under_dir(&f.path, "concepts"));
    files
        .files()
        .iter()
        .filter(|f| !path_under_dir(&f.path, "contexts"))
        .filter(|f| !scoped || path_under_dir(&f.path, "concepts"))
        .collect()
}

#[cfg(test)]
mod tests;
