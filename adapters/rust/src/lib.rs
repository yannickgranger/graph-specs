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
            extract_from_file(&parsed, &path, code_root, &mut concepts, &mut raw_edges);
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
                visit_impl_block(item, &path, owned_unit.as_deref(), &mut pub_fns);
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

/// Parallel walk for impl-block pub methods (v0.6 impl-method anchoring).
///
/// Handles both inherent impls (`impl Foo { pub fn bar }`) and trait impls
/// (`impl Trait for Foo { fn bar }`). For trait impls, explicit `pub` is not
/// required because trait-impl methods are public by contract.
///
/// Does NOT modify `visit_top_level_fn` or `visit_top_level_item`.
fn visit_impl_block(
    item: &syn::Item,
    path: &Path,
    owned_unit: Option<&str>,
    out: &mut Vec<PubFnDecl>,
) {
    let syn::Item::Impl(item_impl) = item else {
        return;
    };
    if is_test_gated(&item_impl.attrs) {
        return;
    }
    let Some(type_root) = root_ident_of_self_ty(&item_impl.self_ty) else {
        return;
    };
    let is_trait_impl = item_impl.trait_.is_some();
    for inner in &item_impl.items {
        let syn::ImplItem::Fn(method) = inner else {
            continue;
        };
        if is_test_gated(&method.attrs) {
            continue;
        }
        let is_public = matches!(method.vis, Visibility::Public(_)) || is_trait_impl;
        if !is_public {
            continue;
        }
        let method_ident = &method.sig.ident;
        let line = method_ident.span().start().line;
        out.push(PubFnDecl {
            name: format!("{type_root}::{method_ident}"),
            source: Source::Code {
                path: path.to_path_buf(),
                line,
            },
            owned_unit: owned_unit.map(str::to_owned),
        });
    }
}

/// Extract the leading type-name identifier from an impl self-type.
///
/// Returns `None` for qualified-path self types (`<Foo as Trait>::Item`) —
/// their outer path's first segment is the associated-type name, not the
/// implementing type, so the qname would be wrong. Returns `None` for
/// non-`Path` types such as slices (`[T]`) or tuples.
fn root_ident_of_self_ty(ty: &syn::Type) -> Option<&syn::Ident> {
    let syn::Type::Path(tp) = ty else {
        return None;
    };
    // Skip qualified-path Self types like <Foo as Trait>::Item.
    if tp.qself.is_some() {
        return None;
    }
    tp.path.segments.first().map(|s| &s.ident)
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
    root: &Path,
    out: &mut Vec<ConceptNode>,
    edges_out: &mut Vec<Edge>,
) {
    // Containment provenance (RFC-010 §3.3) is per-file, so derive it once
    // and share across the file's top-level items: `unit` is the owning
    // crate relative to the code root (§12-I — NOT the raw walked path);
    // `module_path` is the crate-root-collapsed module path (§12-H).
    let unit = find_owned_unit(path, root);
    let module_path = module_path_of(path, root, unit.as_deref());
    for item in &file.items {
        visit_top_level_item(item, path, module_path.as_deref(), unit.as_deref(), out);
        edges::emit_for_item(item, path, edges_out);
    }
}

fn visit_top_level_item(
    item: &syn::Item,
    path: &Path,
    module_path: Option<&str>,
    unit: Option<&str>,
    out: &mut Vec<ConceptNode>,
) {
    use syn::Item;
    match item {
        Item::Struct(s) => emit(
            &s.vis,
            &s.ident,
            &s.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Enum(e) => emit(
            &e.vis,
            &e.ident,
            &e.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Trait(t) => emit(
            &t.vis,
            &t.ident,
            &t.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        Item::Type(t) => emit(
            &t.vis,
            &t.ident,
            &t.attrs,
            item,
            path,
            module_path,
            unit,
            out,
        ),
        // All other items (Mod, Fn, Impl, Const, Static, Use, Macro, etc.) are
        // not top-level concepts. Inline `mod` contents are intentionally not
        // recursed — per-file top-level only.
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn emit(
    vis: &Visibility,
    ident: &syn::Ident,
    attrs: &[Attribute],
    item: &syn::Item,
    path: &Path,
    module_path: Option<&str>,
    unit: Option<&str>,
    out: &mut Vec<ConceptNode>,
) {
    if !matches!(vis, Visibility::Public(_)) {
        return;
    }
    if is_test_gated(attrs) {
        return;
    }
    let line = ident.span().start().line;
    // `context` stays `None` on the reader side — it needs `specs/contexts/`
    // Owns, resolved by the cohesion pass (RFC-010 §3.4). The cfdb-query ACL
    // (R10-6) populates `context` directly.
    out.push(
        ConceptNode::new(
            ident.to_string(),
            Source::Code {
                path: path.to_path_buf(),
                line,
            },
            SignatureState::Normalized(normalize(item)),
        )
        .with_provenance(
            module_path.map(str::to_owned),
            unit.map(str::to_owned),
            None,
        ),
    );
}

/// Derive a concept's crate-root-collapsed module path (RFC-010 §3.3/§12-H).
///
/// `unit` is the owning crate relative to the code root (from
/// [`find_owned_unit`]). The module segments are the path components between
/// `<unit>/src/` and the file, with a trailing `lib` / `mod` / `main`
/// collapsed to the crate root — so `domain/src/lib.rs` → `domain`,
/// `domain/src/diff.rs` → `domain::diff`, `domain/src/diff/mod.rs` →
/// `domain::diff`. Returns `None` when `unit` is unknown.
fn module_path_of(file_path: &Path, root: &Path, unit: Option<&str>) -> Option<String> {
    let unit = unit?;
    let rel = file_path
        .strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let after_unit = rel
        .strip_prefix(unit)
        .unwrap_or(&rel)
        .trim_start_matches('/');
    let after_src = after_unit.strip_prefix("src/").unwrap_or(after_unit);
    let stem = after_src.strip_suffix(".rs").unwrap_or(after_src);
    let mut segments: Vec<&str> = stem.split('/').filter(|s| !s.is_empty()).collect();
    if matches!(segments.last().copied(), Some("lib" | "mod" | "main")) {
        segments.pop();
    }
    if segments.is_empty() {
        Some(unit.to_owned())
    } else {
        Some(format!("{unit}::{}", segments.join("::")))
    }
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

    // --- RFC-010 §3.3 / R10-3 source-walk provenance ---

    fn node_named<'a>(g: &'a Graph, name: &str) -> &'a ConceptNode {
        g.nodes
            .iter()
            .find(|n| n.name == name)
            .unwrap_or_else(|| panic!("missing concept {name}"))
    }

    #[test]
    fn provenance_lib_rs_collapses_to_crate_root() {
        // A top-level type in `<crate>/src/lib.rs`: module_path == unit ==
        // the crate path relative to the code root (§12-H crate-root edge).
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "mycrate/Cargo.toml",
            "[package]\nname=\"mycrate\"\n",
        );
        write(d.path(), "mycrate/src/lib.rs", "pub struct Foo;");
        let g = RustReader.extract(d.path()).unwrap();
        let foo = node_named(&g, "Foo");
        assert_eq!(foo.unit.as_deref(), Some("mycrate"));
        assert_eq!(foo.module_path.as_deref(), Some("mycrate"));
        assert_eq!(foo.context, None, "context resolved later, not by reader");
    }

    #[test]
    fn provenance_main_rs_collapses_to_crate_root() {
        let d = TempDir::new().unwrap();
        write(d.path(), "app/Cargo.toml", "[package]\nname=\"app\"\n");
        write(d.path(), "app/src/main.rs", "pub struct Cli;");
        let g = RustReader.extract(d.path()).unwrap();
        assert_eq!(node_named(&g, "Cli").module_path.as_deref(), Some("app"));
    }

    #[test]
    fn provenance_submodule_file_and_mod_rs() {
        let d = TempDir::new().unwrap();
        write(d.path(), "c/Cargo.toml", "[package]\nname=\"c\"\n");
        write(d.path(), "c/src/diff.rs", "pub struct A;");
        write(d.path(), "c/src/edge/mod.rs", "pub struct B;");
        let g = RustReader.extract(d.path()).unwrap();
        // `diff.rs` → module segment; `edge/mod.rs` collapses the `mod`.
        assert_eq!(node_named(&g, "A").module_path.as_deref(), Some("c::diff"));
        assert_eq!(node_named(&g, "B").module_path.as_deref(), Some("c::edge"));
        assert_eq!(node_named(&g, "B").unit.as_deref(), Some("c"));
    }

    #[test]
    fn provenance_unit_is_relative_to_code_root_not_walked_path() {
        // §12-I: a nested crate's `unit` is the crate path relative to the
        // code root, never the absolute walked path.
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "adapters/markdown/Cargo.toml",
            "[package]\nname=\"adapter-markdown\"\n",
        );
        write(d.path(), "adapters/markdown/src/lib.rs", "pub struct R;");
        let g = RustReader.extract(d.path()).unwrap();
        assert_eq!(
            node_named(&g, "R").unit.as_deref(),
            Some("adapters/markdown")
        );
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

    // --- v0.6 impl-method anchoring tests ---

    #[test]
    fn impl_inherent_pub_method_extracted_as_type_method_qname() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "struct Foo; impl Foo { pub fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(fns[0].name, "Foo::bar");
    }

    #[test]
    fn impl_inherent_private_method_skipped() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "struct Foo; impl Foo { fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert!(fns.is_empty(), "private method must not be extracted");
    }

    #[test]
    fn impl_trait_method_extracted_without_pub() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "trait Trait { fn bar(); } struct Foo; impl Trait for Foo { fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert_eq!(
            fns.len(),
            1,
            "trait-impl method must be extracted even without pub"
        );
        assert_eq!(fns[0].name, "Foo::bar");
    }

    #[test]
    fn impl_generic_type_stripped() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "struct Foo<T>(T); impl<T> Foo<T> { pub fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert_eq!(fns.len(), 1);
        assert_eq!(
            fns[0].name, "Foo::bar",
            "generic param must be stripped from type name"
        );
    }

    #[test]
    fn impl_cfg_test_gated_skipped() {
        let d = TempDir::new().unwrap();
        write(
            d.path(),
            "src/lib.rs",
            "#[cfg(test)] impl Foo { pub fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert!(fns.is_empty(), "cfg(test)-gated impl block must be skipped");
    }

    #[test]
    fn impl_qualified_self_skipped() {
        let d = TempDir::new().unwrap();
        // <Foo as Other>::Item as self type — qself guard must fire, no decl.
        write(
            d.path(),
            "src/lib.rs",
            "trait Other { type Item; } \
             trait Trait { fn bar(); } \
             impl Trait for <i32 as Other>::Item { fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert!(
            fns.is_empty(),
            "qualified-self impl must produce no decl; got: {fns:?}"
        );
    }

    #[test]
    fn impl_non_path_self_skipped() {
        let d = TempDir::new().unwrap();
        // [T] as self type — non-Path type guard must fire.
        write(
            d.path(),
            "src/lib.rs",
            "trait Trait { fn bar(); } impl Trait for [u8] { fn bar() {} }",
        );
        let fns = RustReader.extract_pub_fns(d.path()).unwrap();
        assert!(fns.is_empty(), "non-Path self type must produce no decl");
    }
}
