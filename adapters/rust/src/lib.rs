//! Rust code reader — stub implementation of [`ports::Reader`].
//!
//! Real parsing (tree-sitter-rust, optionally upgraded to `syn` for higher
//! fidelity) lands in follow-up issues.

use domain::Graph;
use ports::Reader;
use std::path::Path;

#[derive(Debug, Default)]
pub struct RustReader;

impl Reader for RustReader {
    fn extract(&self, _root: &Path, _globs: &[String]) -> Graph {
        Graph
    }
}
