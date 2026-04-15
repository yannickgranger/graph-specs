//! Application orchestration library.
//!
//! Thin glue between the readers (markdown + Rust), the pure diff engine
//! in [`domain`], and the CLI front-end in `main.rs`. Exposed as a library
//! so integration tests can drive the check end-to-end without going
//! through process boundaries when they choose to.

use adapter_markdown::MarkdownReader;
use adapter_rust::RustReader;
use domain::{diff, Violation};
use ports::{Reader, ReaderError};
use std::path::Path;

/// Run the full concept-level equivalence check.
///
/// Reads all `*.md` files under `specs_dir` and all `*.rs` files under
/// `code_dir` (honouring the adapter-specific filter rules), then emits
/// the set-difference as a list of [`Violation`]s.
///
/// # Errors
///
/// Propagates any [`ReaderError`] from the underlying markdown or Rust
/// reader — typically I/O, parse, or directory-walk failures.
pub fn run_check(specs_dir: &Path, code_dir: &Path) -> Result<Vec<Violation>, ReaderError> {
    let specs_graph = MarkdownReader.extract(specs_dir)?;
    let code_graph = RustReader.extract(code_dir)?;
    Ok(diff(specs_graph, code_graph))
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
}
