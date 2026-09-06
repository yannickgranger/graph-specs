mod anchor_resolver;
mod cfg_gate;
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
use ports::{CodeFacts, Extraction, LanguageBackend, Reader, ReaderError, VerbReader};
use std::path::Path;
use walkdir::WalkDir;

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
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError> {
        Ok(Reader::extract(self, root)?.nodes)
    }

    fn relationships(&self, root: &Path) -> Result<Vec<Edge>, ReaderError> {
        Ok(Reader::extract(self, root)?.edges)
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
