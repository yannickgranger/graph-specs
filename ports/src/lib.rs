//! Language-neutral port traits.
//!
//! Concrete readers (markdown specs, Rust code, later PHP / TypeScript)
//! implement these traits and produce graphs of identical shape. The diff
//! engine operates on graphs, not on source languages.

use domain::Graph;
use std::path::Path;

/// Reader contract: extract a graph from a source root.
///
/// Real error types and glob abstractions land in follow-up issues.
pub trait Reader {
    fn extract(&self, root: &Path, globs: &[String]) -> Graph;
}
