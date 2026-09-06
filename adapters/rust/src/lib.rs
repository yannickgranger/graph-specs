mod anchor_resolver;
mod cache;
mod cfg_gate;
pub use cache::{parse, ParseCache};
mod concepts;
mod edges;
mod provenance;
mod pub_fns;
mod walk;

#[cfg(test)]
mod tests;

pub use anchor_resolver::RustAnchorResolver;
pub use signature_norm::normalize;

use domain::{ConceptNode, Edge, EdgeKind, Graph, PubFnDecl};
use ports::{
    CodeFacts, CodeFileSet, CodeLoader, CodeReader, Extraction, LanguageBackend, LoadedFile,
    ReaderError, VerbReader,
};
use std::path::Path;
use walkdir::WalkDir;

pub struct RustBackend {
    cache: ParseCache,
}

impl RustBackend {
    #[must_use]
    pub const fn new(cache: ParseCache) -> Self {
        Self { cache }
    }
}

impl LanguageBackend for RustBackend {
    fn detect(&self, code_root: &Path) -> bool {
        code_root.join("Cargo.toml").exists()
    }

    fn extract(&self, _code_root: &Path) -> Result<Extraction, ReaderError> {
        let mut concepts = Vec::new();
        let mut raw_edges: Vec<Edge> = Vec::new();

        self.cache.for_each(|path, parsed, unit, module_path| {
            concepts::extract_from_entry(
                parsed,
                unit,
                module_path,
                path,
                &mut concepts,
                &mut raw_edges,
            );
        });

        Ok(Extraction {
            concepts,
            raw_edges,
        })
    }
}

#[derive(Debug, Default)]
pub struct RustLoader;

impl CodeLoader for RustLoader {
    fn load(&self, root: &Path) -> Result<CodeFileSet, ReaderError> {
        let mut files = Vec::new();
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
        Ok(CodeFileSet::new(files))
    }
}

pub struct RustReader {
    cache: ParseCache,
}

impl RustReader {
    #[must_use]
    pub const fn new(cache: ParseCache) -> Self {
        Self { cache }
    }
}

impl CodeReader for RustReader {
    fn extract(&self, _files: &CodeFileSet) -> Result<Graph, ReaderError> {
        let mut concepts = Vec::new();
        let mut raw_edges: Vec<Edge> = Vec::new();
        self.cache.for_each(|path, parsed, unit, module_path| {
            concepts::extract_from_entry(
                parsed,
                unit,
                module_path,
                path,
                &mut concepts,
                &mut raw_edges,
            );
        });
        let edges = edges::filter_by_known_concepts(raw_edges, &concepts);
        Ok(Graph::new(concepts, edges))
    }
}

impl CodeFacts for RustReader {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
        Ok(CodeReader::extract(self, &RustLoader.load(root)?)?.nodes)
    }

    fn relationships(&self, root: &Path) -> Result<Vec<Edge>, ReaderError> {
        Ok(CodeReader::extract(self, &RustLoader.load(root)?)?.edges)
    }

    fn answerable_relationships(&self, _root: &Path) -> Result<Vec<EdgeKind>, ReaderError> {
        Ok(vec![
            EdgeKind::Implements,
            EdgeKind::DependsOn,
            EdgeKind::Returns,
        ])
    }
}

impl VerbReader for RustReader {
    fn extract_pub_fns(&self, _root: &Path) -> Result<Vec<PubFnDecl>, ReaderError> {
        let mut pub_fns = Vec::new();
        self.cache.for_each(|path, parsed, unit, _module_path| {
            for item in &parsed.items {
                pub_fns::visit_top_level_fn(item, path, unit, &mut pub_fns);
                pub_fns::visit_impl_block(item, path, unit, &mut pub_fns);
            }
        });
        Ok(pub_fns)
    }
}
