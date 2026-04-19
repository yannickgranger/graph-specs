//! Application orchestration library.
//!
//! Thin glue between the readers (markdown + Rust), the pure diff engine
//! in [`domain`], and the CLI front-end in `main.rs`. Exposed as a library
//! so integration tests can drive the check end-to-end without going
//! through process boundaries when they choose to.

use adapter_markdown::MarkdownReader;
use adapter_rust::RustReader;
use domain::{diff, CheckInput, Violation};
use ports::{ContextReader, Reader, ReaderError};
use std::path::Path;

pub mod ndjson;
pub mod text;

/// Run the full equivalence check across all levels configured in the
/// current domain (concept, signature, edge, and — when context files
/// are present — bounded context).
///
/// `specs_dir` is walked by both the concept reader and the context
/// reader. Each reader skips the other's subtree (`concepts/` vs
/// `contexts/`), so pointing `--specs specs/` at a v0.4 tree picks up
/// both sides; a v0.3 tree with no `contexts/` subdir yields an empty
/// context list and the pass is a no-op.
///
/// # Errors
///
/// Propagates any [`ReaderError`] from the underlying markdown or Rust
/// reader — typically I/O, parse, or directory-walk failures. Cyclic
/// import declarations surface as `ReaderError::ParseFailed` from the
/// context reader.
pub fn run_check(specs_dir: &Path, code_dir: &Path) -> Result<Vec<Violation>, ReaderError> {
    let specs_graph = MarkdownReader.extract(specs_dir)?;
    let spec_contexts = MarkdownReader.extract_contexts(specs_dir)?;
    let code_graph = RustReader.extract(code_dir)?;
    Ok(diff(
        CheckInput::new(specs_graph, spec_contexts),
        code_graph,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, content: &str) {
        let full = dir.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&full).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn empty_trees_yield_no_violations() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        assert!(run_check(specs.path(), code.path()).unwrap().is_empty());
    }

    #[test]
    fn matching_tree_yields_no_violations() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(specs.path(), "a.md", "## Foo\n## Bar\n");
        write(
            code.path(),
            "src/lib.rs",
            "pub struct Foo; pub enum Bar { X }",
        );
        assert!(run_check(specs.path(), code.path()).unwrap().is_empty());
    }

    /// v0.4: `--specs specs/` with both `concepts/` and `contexts/`
    /// subdirs. The two readers scope themselves so neither trips on
    /// the other's dialect. This test asserts the wiring returns `Ok`
    /// — unit-matching semantics are covered in
    /// `domain::diff::context_tests`; dogfood against this repo's own
    /// specs is the end-to-end integration proof.
    #[test]
    fn v04_layout_does_not_collide_on_shared_specs_root() {
        let specs = TempDir::new().unwrap();
        let code = TempDir::new().unwrap();
        write(specs.path(), "concepts/core.md", "## Foo\n## Bar\n");
        write(
            specs.path(),
            "contexts/only.md",
            "# only\n\n## Owns\n\n- fixture\n",
        );
        write(
            code.path(),
            "fixture/src/lib.rs",
            "pub struct Foo; pub enum Bar { X }",
        );
        assert!(run_check(specs.path(), code.path()).is_ok());
    }
}
