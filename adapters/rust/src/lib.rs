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

mod edges;
mod normalize;

pub use normalize::normalize;

use domain::{ConceptNode, Edge, Graph, PubFnDecl, SignatureState, Source};
use ports::{Extraction, LanguageBackend, Reader, ReaderError, VerbReader};
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

/// Low-level Rust extractor.
///
/// Walks a source tree once and emits flat concepts + raw edges. Used by
/// [`RustReader`] to build a [`Graph`] and, in future, by cfdb's Rust
/// ingestor (RFC-005 / #83 reframe).
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
            .filter_entry(|e| !is_excluded_dir(e));

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

            let (parsed, path) = read_and_parse(entry.path().to_path_buf())?;
            extract_from_file(&parsed, &path, &mut concepts, &mut raw_edges);
        }

        Ok(Extraction {
            concepts,
            raw_edges,
        })
    }
}

/// High-level Rust reader.
///
/// Wraps [`RustBackend`] with language-neutral graph assembly: pulls
/// concepts + raw edges, filters edges against the discovered concept set,
/// and returns a [`Graph`] for the diff engine.
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

impl VerbReader for RustReader {
    fn extract_pub_fns(&self, root: &Path) -> Result<Vec<PubFnDecl>, ReaderError> {
        let mut pub_fns = Vec::new();

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

            let owned_unit = find_owned_unit(entry.path(), root);
            let (parsed, path) = read_and_parse(entry.path().to_path_buf())?;

            for item in &parsed.items {
                visit_top_level_fn(item, &path, owned_unit.as_deref(), &mut pub_fns);
            }
        }

        Ok(pub_fns)
    }
}

/// Separate parallel walk for `pub fn` items — per RFC-005 §3.2 dry-run
/// rust-systems-A finding: `visit_top_level_item` documents `Fn` as a
/// deliberately-excluded item and MUST NOT be extended. This sibling
/// exclusively handles `syn::Item::Fn`.
fn visit_top_level_fn(
    item: &syn::Item,
    path: &Path,
    owned_unit: Option<&str>,
    out: &mut Vec<PubFnDecl>,
) {
    if let syn::Item::Fn(f) = item {
        if !matches!(f.vis, Visibility::Public(_)) {
            return;
        }
        if is_test_gated(&f.attrs) {
            return;
        }
        let line = f.sig.ident.span().start().line;
        out.push(PubFnDecl {
            name: f.sig.ident.to_string(),
            source: Source::Code {
                path: path.to_path_buf(),
                line,
            },
            owned_unit: owned_unit.map(str::to_owned),
        });
    }
}

/// Find the owning crate for a given source file by walking up to the
/// nearest `Cargo.toml`, then computing the path relative to `root`.
/// Returns `None` if no `Cargo.toml` is found in the ancestor chain.
fn find_owned_unit(file_path: &Path, root: &Path) -> Option<String> {
    let mut dir = file_path.parent()?;
    loop {
        if dir.join("Cargo.toml").exists() {
            // Return workspace-relative path (e.g. "application",
            // "adapters/rust") when the Cargo.toml is under root;
            // fall back to the directory name when root == dir.
            if let Ok(rel) = dir.strip_prefix(root) {
                let s = rel.to_string_lossy();
                if !s.is_empty() {
                    return Some(s.replace('\\', "/"));
                }
            }
            return dir.file_name().and_then(|n| n.to_str()).map(str::to_owned);
        }
        let parent = dir.parent()?;
        if parent == dir {
            return None;
        }
        dir = parent;
    }
}

/// Read a Rust source file and parse it. Consumes `path` — on error the
/// path is moved into the resulting [`ReaderError`] variant; on success it
/// is handed back alongside the parsed file. This lets the caller avoid
/// cloning the path twice inside its walk loop (one clone per error
/// variant) and keeps the heavy-work of per-file I/O + parsing off the
/// hot path of the walker.
fn read_and_parse(path: std::path::PathBuf) -> Result<(File, std::path::PathBuf), ReaderError> {
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return Err(ReaderError::IoFailed {
                path,
                cause: e.to_string(),
            });
        }
    };
    match syn::parse_file(&source) {
        Ok(f) => Ok((f, path)),
        Err(e) => Err(ReaderError::ParseFailed {
            path,
            line: e.span().start().line,
            message: e.to_string(),
        }),
    }
}

fn is_excluded_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    EXCLUDED_DIRS.iter().any(|ex| name.as_ref() == *ex)
}

fn extract_from_file(
    file: &File,
    path: &Path,
    out: &mut Vec<ConceptNode>,
    edges_out: &mut Vec<Edge>,
) {
    for item in &file.items {
        visit_top_level_item(item, path, out);
        edges::emit_for_item(item, path, edges_out);
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

    #[test]
    fn rust_backend_detects_cargo_toml() {
        let d = TempDir::new().unwrap();
        assert!(!RustBackend.detect(d.path()), "no marker → false");
        write(d.path(), "Cargo.toml", "[package]\nname = \"x\"\n");
        assert!(RustBackend.detect(d.path()), "Cargo.toml present → true");
    }

    /// Self-dogfood: `extract_pub_fns` on this repo's `application/` crate
    /// yields a non-zero list that includes `run_check` per RFC-005 §7 Slice A.
    #[test]
    fn extract_pub_fns_self_dogfood_application_includes_run_check() {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // adapters/rust → workspace root is two levels up
        let workspace = manifest
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let app_dir = workspace.join("application");
        if !app_dir.exists() {
            // Tolerate running outside the real workspace (e.g. isolated tmpfs)
            return;
        }
        let fns = RustReader.extract_pub_fns(&app_dir).expect("dogfood");
        assert!(
            !fns.is_empty(),
            "application/ should yield at least one pub fn"
        );
        assert!(
            fns.iter().any(|f| f.name == "run_check"),
            "expected run_check in pub fn list; got: {:?}",
            fns.iter().map(|f| f.name.as_str()).collect::<Vec<_>>()
        );
    }

    // Self-dogfood: extract_pub_fns on the application crate yields run_check.
    #[test]
    fn extract_pub_fns_finds_pub_fns() {
        let d = TempDir::new().unwrap();
        write(d.path(), "Cargo.toml", "[package]\nname = \"testcrate\"\n");
        write(
            d.path(),
            "src/lib.rs",
            "pub fn alpha() {} pub fn beta() {} fn private() {}",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        let mut names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn extract_pub_fns_skips_test_gated() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "#[cfg(test)] pub fn skip_me() {} pub fn keep_me() {}",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "keep_me");
    }

    #[test]
    fn extract_pub_fns_excludes_target_dir() {
        let d = TempDir::new().unwrap();
        write(d.path(), "src/lib.rs", "pub fn real_fn() {}");
        write(d.path(), "target/gen.rs", "pub fn generated() {}");
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "real_fn");
    }

    #[test]
    fn rust_backend_extract_returns_concepts_and_edges() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "pub struct Foo { bar: Bar } pub struct Bar;",
        );
        let extraction = RustBackend.extract(d.path()).unwrap();
        let mut names: Vec<String> = extraction.concepts.iter().map(|n| n.name.clone()).collect();
        names.sort();
        assert_eq!(names, vec!["Bar", "Foo"]);
        // Raw edges include the Foo→Bar field dependency, unfiltered.
        assert!(
            extraction
                .raw_edges
                .iter()
                .any(|e| e.source_concept == "Foo" && e.target == "Bar"),
            "expected raw Foo→Bar dependency edge, got: {:?}",
            extraction.raw_edges
        );
    }
}
