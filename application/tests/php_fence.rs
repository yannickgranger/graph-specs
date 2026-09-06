use std::path::Path;
use tempfile::TempDir;

fn write(root: &Path, name: &str, body: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn cargo_toml(root: &Path) {
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
}

fn tree(spec: &str, php_attribute: &str) -> (TempDir, TempDir) {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(specs.path(), "concepts/orders.md", spec);
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");
    write(
        code.path(),
        "src/Orders/OrderService.php",
        &format!("<?php\n\n{php_attribute}\nclass OrderService {{}}\n"),
    );
    (specs, code)
}

#[test]
fn a_php_fence_and_an_attribute_that_disagree_raise_the_within_side_record() {
    let (specs, code) = tree(
        "## OrderService\n\n```php\nclass OrderService { public function place(): void {} }\n```\n",
        "#[Spec(signature: \"class OrderService { public function place(Order $o): Receipt {} }\")]",
    );
    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let drift = outcome
        .violations
        .iter()
        .filter(|v| matches!(v, domain::Violation::SignatureDriftWithinSide { .. }))
        .count();
    assert_eq!(drift, 1, "{:?}", outcome.violations);
}

#[test]
fn a_php_fence_normalizes_to_the_declaration_without_its_body_or_comments() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/orders.md",
        "## OrderService\n\n```php\n// a comment the normalizer drops\n#[Attr]\nfinal class OrderService implements Enrolable { public function place(): void {} }\n```\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let drift = outcome
        .violations
        .iter()
        .find_map(|v| match v {
            domain::Violation::SignatureDrift { name, spec_sig, .. } if name == "OrderService" => {
                Some(spec_sig.clone())
            }
            _ => None,
        })
        .expect("the fence normalized and the code side disagreed");
    assert_eq!(drift, "final class OrderService implements Enrolable");
}

#[test]
fn a_php_fence_that_does_not_parse_is_unparseable_with_the_fence_tag_naming_the_language() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/orders.md",
        "## OrderService\n\n```php\nclass OrderService {\n```\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let raw = outcome
        .violations
        .iter()
        .find_map(|v| match v {
            domain::Violation::SignatureUnparseable { name, error, .. }
                if name == "OrderService" =>
            {
                Some(error.clone())
            }
            _ => None,
        })
        .expect("an unparseable php fence is a signature_unparseable finding");
    assert!(
        raw.starts_with("php: "),
        "the fence tag names the language: {raw}"
    );
}

#[test]
fn two_php_fences_in_one_section_are_unparseable() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();
    write(
        specs.path(),
        "concepts/orders.md",
        "## OrderService\n\n```php\nclass OrderService {}\n```\n\n```php\nclass OrderService {}\n```\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let error = outcome
        .violations
        .iter()
        .find_map(|v| match v {
            domain::Violation::SignatureUnparseable { name, error, .. }
                if name == "OrderService" =>
            {
                Some(error.clone())
            }
            _ => None,
        })
        .expect("more than one fence of a language is unparseable");
    assert!(error.contains("2 normalizable fenced blocks"), "{error}");
}
