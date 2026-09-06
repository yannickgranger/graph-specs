use super::*;
use domain::Source;
use std::io::Write;
use tempfile::TempDir;

fn write(dir: &Path, rel: &str, content: &str) {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("create parent dir for test fixture");
    }
    let mut f = std::fs::File::create(&full).expect("create test fixture file");
    f.write_all(content.as_bytes())
        .expect("write test fixture content");
}

fn extract(dir: &Path) -> Vec<String> {
    let g = RustReader
        .extract(dir)
        .expect("extract must succeed for test fixture");
    let mut names: Vec<String> = g.nodes.into_iter().map(|n| n.name).collect();
    names.sort();
    names
}

#[test]
fn captures_pub_struct_enum_trait_type() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "pub struct Foo; pub enum Bar { X } pub trait Baz {} pub type Qux = u32;",
    );
    assert_eq!(extract(d.path()), vec!["Bar", "Baz", "Foo", "Qux"]);
}

#[test]
fn ignores_private_items() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "struct Priv; pub struct Pub;");
    assert_eq!(extract(d.path()), vec!["Pub"]);
}

#[test]
fn ignores_cfg_test_items() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "pub struct Keep; #[cfg(test)] pub struct Skip;",
    );
    assert_eq!(extract(d.path()), vec!["Keep"]);
}

#[test]
fn ignores_items_inside_inline_mod() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "pub struct Top; pub mod inner { pub struct Inner; }",
    );
    assert_eq!(extract(d.path()), vec!["Top"]);
}

#[test]
fn ignores_tests_benches_examples_dirs() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub struct Real;");
    write(d.path(), "tests/it.rs", "pub struct TestFixture;");
    write(d.path(), "benches/b.rs", "pub struct Bench;");
    write(d.path(), "examples/e.rs", "pub struct Example;");
    assert_eq!(extract(d.path()), vec!["Real"]);
}

#[test]
fn ignores_target_and_claude_dirs() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub struct Real;");
    write(d.path(), "target/gen.rs", "pub struct Gen;");
    write(d.path(), ".claude/w.rs", "pub struct W;");
    assert_eq!(extract(d.path()), vec!["Real"]);
}

const CACHEDIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
                            # This file is a cache directory tag.\n";

#[test]
fn cache_tagged_build_dir_is_excluded_whatever_its_name() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub struct Real;");
    write(d.path(), "target-musl/CACHEDIR.TAG", CACHEDIR_TAG);
    write(
        d.path(),
        "target-musl/release/build/libsqlite3-sys-abc/out/bindgen.rs",
        "pub struct sqlite3_stmt; pub struct sqlite3_vfs;",
    );
    assert_eq!(extract(d.path()), vec!["Real"]);
}

#[test]
fn untagged_dir_sharing_an_excluded_prefix_is_still_walked() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub struct Real;");
    write(d.path(), "targets/registry.rs", "pub struct Registry;");
    assert_eq!(extract(d.path()), vec!["Real", "Registry"]);
}

#[test]
fn foreign_cachedir_signature_does_not_exclude() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub struct Real;");
    write(
        d.path(),
        "generated/CACHEDIR.TAG",
        "Signature: not-a-cache\n",
    );
    write(d.path(), "generated/g.rs", "pub struct Generated;");
    assert_eq!(extract(d.path()), vec!["Generated", "Real"]);
}

#[test]
fn source_file_named_like_an_excluded_dir_is_still_read() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/target.rs", "pub struct TargetRepo;");
    write(d.path(), "src/tests.rs", "pub struct TestHarness;");
    assert_eq!(extract(d.path()), vec!["TargetRepo", "TestHarness"]);
}

#[test]
fn line_numbers_are_recorded() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "\n\npub struct OnLine3;\n");
    let g = RustReader.extract(d.path()).expect("extract must succeed");
    match &g.nodes[0].source {
        Source::Code { line, .. } => assert_eq!(*line, 3),
        Source::Spec { .. } => panic!("expected Code source"),
    }
}

#[test]
fn rust_backend_detects_cargo_toml() {
    let d = TempDir::new().expect("create temp dir");
    assert!(!RustBackend.detect(d.path()), "no marker → false");
    write(d.path(), "Cargo.toml", "[package]\nname = \"x\"\n");
    assert!(RustBackend.detect(d.path()), "Cargo.toml present → true");
}

fn node_named<'a>(g: &'a Graph, name: &str) -> &'a ConceptNode {
    g.nodes
        .iter()
        .find(|n| n.name == name)
        .unwrap_or_else(|| panic!("missing concept {name}"))
}

#[test]
fn provenance_lib_rs_collapses_to_crate_root() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "mycrate/Cargo.toml",
        "[package]\nname=\"mycrate\"\n",
    );
    write(d.path(), "mycrate/src/lib.rs", "pub struct Foo;");
    let g = RustReader.extract(d.path()).expect("extract must succeed");
    let foo = node_named(&g, "Foo");
    assert_eq!(foo.unit(), Some("mycrate"));
    assert_eq!(foo.module_path(), Some("mycrate"));
    assert_eq!(foo.context(), None, "context resolved later, not by reader");
}

#[test]
fn provenance_main_rs_collapses_to_crate_root() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "app/Cargo.toml", "[package]\nname=\"app\"\n");
    write(d.path(), "app/src/main.rs", "pub struct Cli;");
    let g = RustReader.extract(d.path()).expect("extract must succeed");
    assert_eq!(node_named(&g, "Cli").module_path(), Some("app"));
}

#[test]
fn provenance_submodule_file_and_mod_rs() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "c/Cargo.toml", "[package]\nname=\"c\"\n");
    write(d.path(), "c/src/diff.rs", "pub struct A;");
    write(d.path(), "c/src/edge/mod.rs", "pub struct B;");
    let g = RustReader.extract(d.path()).expect("extract must succeed");
    assert_eq!(node_named(&g, "A").module_path(), Some("c::diff"));
    assert_eq!(node_named(&g, "B").module_path(), Some("c::edge"));
    assert_eq!(node_named(&g, "B").unit(), Some("c"));
}

#[test]
fn provenance_unit_is_relative_to_code_root_not_walked_path() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "adapters/markdown/Cargo.toml",
        "[package]\nname=\"adapter-markdown\"\n",
    );
    write(d.path(), "adapters/markdown/src/lib.rs", "pub struct R;");
    let g = RustReader.extract(d.path()).expect("extract must succeed");
    assert_eq!(node_named(&g, "R").unit(), Some("adapters/markdown"));
}

#[test]
fn extract_pub_fns_self_dogfood_application_includes_run_check() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    let app_dir = workspace.join("application");
    if !app_dir.exists() {
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

#[test]
fn extract_pub_fns_finds_pub_fns() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "Cargo.toml", "[package]\nname = \"testcrate\"\n");
    write(
        d.path(),
        "src/lib.rs",
        "pub fn alpha() {} pub fn beta() {} fn private() {}",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    let mut names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["alpha", "beta"]);
}

#[test]
fn extract_pub_fns_skips_test_gated() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "#[cfg(test)] pub fn skip_me() {} pub fn keep_me() {}",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "keep_me");
}

#[test]
fn extract_pub_fns_excludes_target_dir() {
    let d = TempDir::new().expect("create temp dir");
    write(d.path(), "src/lib.rs", "pub fn real_fn() {}");
    write(d.path(), "target/gen.rs", "pub fn generated() {}");
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "real_fn");
}

#[test]
fn rust_backend_extract_returns_concepts_and_edges() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "pub struct Foo { bar: Bar } pub struct Bar;",
    );
    let extraction = RustBackend.extract(d.path()).expect("extract must succeed");
    let mut names: Vec<String> = extraction.concepts.iter().map(|n| n.name.clone()).collect();
    names.sort();
    assert_eq!(names, vec!["Bar", "Foo"]);
    assert!(
        extraction
            .raw_edges
            .iter()
            .any(|e| e.source_concept == "Foo" && e.target == "Bar"),
        "expected raw Foo→Bar dependency edge, got: {:?}",
        extraction.raw_edges
    );
}

#[test]
fn impl_inherent_pub_method_extracted_as_type_method_qname() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "struct Foo; impl Foo { pub fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "Foo::bar");
}

#[test]
fn impl_inherent_private_method_skipped() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "struct Foo; impl Foo { fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert!(fns.is_empty(), "private method must not be extracted");
}

#[test]
fn impl_trait_method_extracted_without_pub() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "trait Trait { fn bar(); } struct Foo; impl Trait for Foo { fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert_eq!(
        fns.len(),
        1,
        "trait-impl method must be extracted even without pub"
    );
    assert_eq!(fns[0].name, "Foo::bar");
}

#[test]
fn impl_generic_type_stripped() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "struct Foo<T>(T); impl<T> Foo<T> { pub fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert_eq!(fns.len(), 1);
    assert_eq!(
        fns[0].name, "Foo::bar",
        "generic param must be stripped from type name"
    );
}

#[test]
fn impl_cfg_test_gated_skipped() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "#[cfg(test)] impl Foo { pub fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert!(fns.is_empty(), "cfg(test)-gated impl block must be skipped");
}

#[test]
fn impl_qualified_self_skipped() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "trait Other { type Item; } \
         trait Trait { fn bar(); } \
         impl Trait for <i32 as Other>::Item { fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert!(
        fns.is_empty(),
        "qualified-self impl must produce no decl; got: {fns:?}"
    );
}

#[test]
fn impl_non_path_self_skipped() {
    let d = TempDir::new().expect("create temp dir");
    write(
        d.path(),
        "src/lib.rs",
        "trait Trait { fn bar(); } impl Trait for [u8] { fn bar() {} }",
    );
    let fns = RustReader
        .extract_pub_fns(d.path())
        .expect("extract_pub_fns must succeed");
    assert!(fns.is_empty(), "non-Path self type must produce no decl");
}
