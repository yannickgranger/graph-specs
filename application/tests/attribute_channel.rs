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

#[test]
fn the_attribute_channel_and_the_markdown_spec_disagreeing_raises_the_within_side_record() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/orders.md",
        "# orders\n\n## OrderService\n\n```rust\nfn place(&self) {}\n```\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");
    write(
        code.path(),
        "src/Orders/OrderService.php",
        "<?php\n\n#[Spec(signature: \"fn place(&self, o: Order)\")]\nclass OrderService {}\n",
    );

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    let drift: Vec<_> = outcome
        .violations
        .iter()
        .filter(|v| matches!(v, domain::Violation::SignatureDriftWithinSide { .. }))
        .collect();
    assert_eq!(drift.len(), 1, "{:?}", outcome.violations);

    let domain::Violation::SignatureDriftWithinSide {
        name,
        side,
        sources,
    } = drift[0]
    else {
        unreachable!()
    };
    assert_eq!(name, "OrderService");
    assert_eq!(*side, domain::DiffSide::Spec);
    assert_eq!(sources.len(), 2);
    assert!(
        matches!(
            sources[0].source,
            domain::Source::Spec {
                format: domain::SpecFormat::Markdown,
                ..
            }
        ),
        "markdown is the canonical upstream and is reported first: {:?}",
        sources[0].source
    );
    assert!(matches!(
        sources[1].source,
        domain::Source::Spec {
            format: domain::SpecFormat::InlineAttribute,
            ..
        }
    ));
}

#[test]
fn an_attribute_agreeing_with_the_markdown_spec_raises_nothing() {
    let specs = TempDir::new().unwrap();
    let code = TempDir::new().unwrap();

    write(
        specs.path(),
        "concepts/orders.md",
        "# orders\n\n## OrderService\n\n```rust\nfn place(&self) {}\n```\n",
    );
    cargo_toml(code.path());
    write(code.path(), "src/lib.rs", "pub struct OrderService;\n");
    write(
        code.path(),
        "src/Orders/OrderService.php",
        "<?php\n\n#[Spec(signature: \"fn place (& self) { }\")]\nclass OrderService {}\n",
    );

    let outcome = application::run_check(specs.path(), code.path(), None).unwrap();
    assert!(
        !outcome
            .violations
            .iter()
            .any(|v| matches!(v, domain::Violation::SignatureDriftWithinSide { .. })),
        "the two agree: {:?}",
        outcome.violations
    );
}
