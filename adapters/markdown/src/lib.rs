//! Markdown spec reader — stub implementation of [`ports::Reader`].
//!
//! Real parsing (pulldown-cmark / custom spec dialect) lands in follow-up issues.

use domain::Graph;
use ports::Reader;
use std::path::Path;

#[derive(Debug, Default)]
pub struct MarkdownReader;

impl Reader for MarkdownReader {
    fn extract(&self, _root: &Path, _globs: &[String]) -> Graph {
        Graph
    }
}
