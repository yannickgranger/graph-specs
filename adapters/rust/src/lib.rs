//! Rust code reader — concept-level.
//!
//! Walks a directory tree, parses each `*.rs` file with `syn`, and emits a
//! [`ConceptNode`] for every top-level `pub struct`, `pub enum`, `pub trait`,
//! `pub type`. Honours the filter rules documented in `specs/dialect.md`:
//! non-public items, `#[cfg(test)]`-gated items, and files under
//! `target/` / `.git/` / `.claude/` / `.proofs/` / per-crate `tests/`,
//! `benches/`, `examples/` are skipped.
//!
//! Scope: only top-level items in each file are visited. Concepts nested
//! inside `pub mod foo { ... }` are not extracted at this level.

mod normalize;

pub use normalize::normalize;

use domain::{ConceptNode, Graph, SignatureState, Source};
use ports::{Reader, ReaderError};
use std::path::Path;
use syn::{Attribute, File, Visibility};
use walkdir::{DirEntry, WalkDir};

const EXCLUDED_DIRS: &[&str] = &[
    "target",
    ".git",
    ".claude",
    ".proofs",
    "tests",
    "benches",
    "examples",
    "node_modules",
];

#[derive(Debug, Default)]
pub struct RustReader;

impl Reader for RustReader {
    fn extract(&self, root: &Path) -> Result<Graph, ReaderError> {
        let mut nodes = Vec::new();

        let walker = WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !is_excluded_dir(e));

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

            let path = entry.path().to_path_buf();
            let source = std::fs::read_to_string(&path).map_err(|e| ReaderError::IoFailed {
                path: path.clone(),
                cause: e.to_string(),
            })?;
            let parsed = syn::parse_file(&source).map_err(|e| ReaderError::ParseFailed {
                path: path.clone(),
                line: e.span().start().line,
                message: e.to_string(),
            })?;

            extract_from_file(&parsed, &path, &mut nodes);
        }

        Ok(Graph { nodes })
    }
}

fn is_excluded_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    EXCLUDED_DIRS.iter().any(|ex| name.as_ref() == *ex)
}

fn extract_from_file(file: &File, path: &Path, out: &mut Vec<ConceptNode>) {
    for item in &file.items {
        visit_top_level_item(item, path, out);
    }
}

fn visit_top_level_item(item: &syn::Item, path: &Path, out: &mut Vec<ConceptNode>) {
    use syn::Item;
    match item {
        Item::Struct(s) => emit(&s.vis, &s.ident, &s.attrs, item, path, out),
        Item::Enum(e) => emit(&e.vis, &e.ident, &e.attrs, item, path, out),
        Item::Trait(t) => emit(&t.vis, &t.ident, &t.attrs, item, path, out),
        Item::Type(t) => emit(&t.vis, &t.ident, &t.attrs, item, path, out),
        // All other items (Mod, Fn, Impl, Const, Static, Use, Macro, etc.) are
        // not top-level concepts. Inline `mod` contents are intentionally not
        // recursed — per-file top-level only.
        _ => {}
    }
}

fn emit(
    vis: &Visibility,
    ident: &syn::Ident,
    attrs: &[Attribute],
    item: &syn::Item,
    path: &Path,
    out: &mut Vec<ConceptNode>,
) {
    if !matches!(vis, Visibility::Public(_)) {
        return;
    }
    if is_test_gated(attrs) {
        return;
    }
    let line = ident.span().start().line;
    out.push(ConceptNode {
        name: ident.to_string(),
        source: Source::Code {
            path: path.to_path_buf(),
            line,
        },
        signature: SignatureState::Normalized(normalize(item)),
    });
}

fn is_test_gated(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut gated = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                gated = true;
            }
            if meta.path.is_ident("feature") {
                if let Ok(value) = meta.value() {
                    if let Ok(lit) = value.parse::<syn::LitStr>() {
                        if lit.value().contains("test") {
                            gated = true;
                        }
                    }
                }
            }
            Ok(())
        });
        gated
    })
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

    fn extract(dir: &Path) -> Vec<String> {
        let g = RustReader.extract(dir).unwrap();
        let mut names: Vec<String> = g.nodes.into_iter().map(|n| n.name).collect();
        names.sort();
        names
    }

    #[test]
    fn captures_pub_struct_enum_trait_type() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "pub struct Foo; pub enum Bar { X } pub trait Baz {} pub type Qux = u32;",
        );
        assert_eq!(extract(d.path()), vec!["Bar", "Baz", "Foo", "Qux"]);
    }

    #[test]
    fn ignores_private_items() {
        let d = TempDir::new().unwrap();
        write(d.path(), "src/lib.rs", "struct Priv; pub struct Pub;");
        assert_eq!(extract(d.path()), vec!["Pub"]);
    }

    #[test]
    fn ignores_cfg_test_items() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "pub struct Keep; #[cfg(test)] pub struct Skip;",
        );
        assert_eq!(extract(d.path()), vec!["Keep"]);
    }

    #[test]
    fn ignores_items_inside_inline_mod() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "pub struct Top; pub mod inner { pub struct Inner; }",
        );
        // Inner is not top-level, so not extracted.
        assert_eq!(extract(d.path()), vec!["Top"]);
    }

    #[test]
    fn ignores_tests_benches_examples_dirs() {
        let d = TempDir::new().unwrap();
        write(d.path(), "src/lib.rs", "pub struct Real;");
        write(d.path(), "tests/it.rs", "pub struct TestFixture;");
        write(d.path(), "benches/b.rs", "pub struct Bench;");
        write(d.path(), "examples/e.rs", "pub struct Example;");
        assert_eq!(extract(d.path()), vec!["Real"]);
    }

    #[test]
    fn ignores_target_and_claude_dirs() {
        let d = TempDir::new().unwrap();
        write(d.path(), "src/lib.rs", "pub struct Real;");
        write(d.path(), "target/gen.rs", "pub struct Gen;");
        write(d.path(), ".claude/w.rs", "pub struct W;");
        assert_eq!(extract(d.path()), vec!["Real"]);
    }

    #[test]
    fn line_numbers_are_recorded() {
        let d = TempDir::new().unwrap();
        write(d.path(), "src/lib.rs", "\n\npub struct OnLine3;\n");
        let g = RustReader.extract(d.path()).unwrap();
        match &g.nodes[0].source {
            Source::Code { line, .. } => assert_eq!(*line, 3),
            Source::Spec { .. } => panic!("expected Code source"),
        }
    }
}
