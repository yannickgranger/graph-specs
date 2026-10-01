use super::*;
use std::fs;
use tempfile::TempDir;

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir -p");
    fs::write(path, body).expect("write");
}

fn site(manifest: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "composer.json", manifest);
    dir
}

fn installed(dir: &TempDir, tree: &str, package: &str, declares_a_context: bool) {
    let root = format!("{tree}/{package}");
    write(dir.path(), &format!("{root}/composer.json"), "{}");
    write(
        dir.path(),
        &format!("{root}/src/Thing.php"),
        "<?php\n\nclass Thing\n{\n}\n",
    );
    if declares_a_context {
        write(
            dir.path(),
            &format!("{root}/specs/contexts/thing.md"),
            "# thing\n\n## Owns\n\n- Acme\\Thing\n",
        );
    }
}

#[test]
fn a_package_declaring_a_context_is_found_and_one_declaring_none_is_not() {
    let dir = site(r#"{"name":"acme/site"}"#);
    installed(&dir, "vendor", "acme/contracts", true);
    installed(&dir, "vendor", "acme/plain", false);

    let found = declaring_a_context(dir.path()).expect("walk the install tree");

    assert_eq!(
        found,
        vec![dir.path().join("vendor/acme/contracts/specs")],
        "a package qualifies on composer.json plus at least one specs/contexts/*.md, which is the same predicate cfdb indexes it by; one shipping no context file is a dependency read through its names alone"
    );
}

#[test]
fn the_install_tree_is_the_one_the_manifest_configures() {
    let dir = site(r#"{"name":"acme/site","config":{"vendor-dir":"./lib/"}}"#);
    installed(&dir, "lib", "acme/contracts", true);
    installed(&dir, "vendor", "acme/elsewhere", true);

    let found = declaring_a_context(dir.path()).expect("walk the install tree");

    assert_eq!(
        found,
        vec![dir.path().join("lib/acme/contracts/specs")],
        "composer installs where config.vendor-dir says, and a package under the default vendor/ is then not installed at all"
    );
}

#[test]
fn a_workspace_with_no_install_tree_yields_nothing() {
    let dir = site(r#"{"name":"acme/site"}"#);
    assert!(
        declaring_a_context(dir.path())
            .expect("walk a workspace with no vendor tree")
            .is_empty(),
        "a workspace that has installed nothing declares no foreign context, and the absence is not a refusal"
    );
}

#[test]
fn a_tree_with_no_manifest_reads_the_default_install_tree() {
    let dir = TempDir::new().expect("tempdir");
    installed(&dir, "vendor", "acme/contracts", true);

    let found = declaring_a_context(dir.path()).expect("walk a tree carrying no composer.json");

    assert_eq!(
        found,
        vec![dir.path().join("vendor/acme/contracts/specs")],
        "a --code root that is not a Composer workspace still has an install tree to read, and vendor/ is the default"
    );
}

#[test]
fn an_unreadable_manifest_refuses_rather_than_reading_the_default() {
    let dir = site("{ this is not json");
    let err = declaring_a_context(dir.path())
        .expect_err("a manifest that cannot be parsed must refuse, never fall back in silence");
    assert!(
        matches!(err, ReaderError::ParseFailed { .. }),
        "where the workspace installs its packages has no answer, which is a could-not-run and not an empty set: {err:?}"
    );
}

#[test]
fn the_packages_come_back_in_sorted_order() {
    let dir = site(r#"{"name":"acme/site"}"#);
    installed(&dir, "vendor", "zz/last", true);
    installed(&dir, "vendor", "acme/first", true);
    installed(&dir, "vendor", "acme/second", true);

    let found = declaring_a_context(dir.path()).expect("walk the install tree");

    assert_eq!(
        found,
        vec![
            dir.path().join("vendor/acme/first/specs"),
            dir.path().join("vendor/acme/second/specs"),
            dir.path().join("vendor/zz/last/specs")
        ],
        "two runs over one unchanged tree read the same order, which is what makes a verdict reproducible"
    );
}
