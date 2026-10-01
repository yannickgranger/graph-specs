use adapter_php::PhpAttributeReader;
use domain::{EdgeKind, SignatureState, SpecFormat};
use ports::{SpecLoader, SpecReader};

fn graph_at(dir: &std::path::Path) -> Result<domain::Graph, ports::ReaderError> {
    PhpAttributeReader.extract(&PhpAttributeReader.load(dir)?)
}

fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }
    dir
}

#[test]
fn an_attribute_yields_a_spec_fact_carrying_the_inline_attribute_format() {
    let dir = tree(&[(
        "Course.php",
        "<?php\nnamespace App\\Catalogue;\n\n#[Spec(implements: \"Enrolable\", signature: \"public function place(Order $o): Receipt\")]\nfinal class Course implements Enrolable {}\n",
    )]);
    let graph = graph_at(dir.path()).unwrap();

    assert_eq!(graph.nodes.len(), 1, "{:?}", graph.nodes);
    let node = &graph.nodes[0];
    assert_eq!(node.name, "Course");
    assert!(
        matches!(
            node.source,
            domain::Source::Spec {
                format: SpecFormat::InlineAttribute,
                ..
            }
        ),
        "the attribute channel is a spec source in the inline-attribute format: {:?}",
        node.source
    );
    assert_eq!(
        node.signature,
        SignatureState::Normalized("public function place(Order $o): Receipt".to_string())
    );

    assert_eq!(graph.edges.len(), 1, "{:?}", graph.edges);
    assert_eq!(graph.edges[0].kind, EdgeKind::Implements);
    assert_eq!(graph.edges[0].raw_target, "Enrolable");
}

#[test]
fn an_extends_key_is_read_without_a_finding_and_yields_no_edge() {
    let dir = tree(&[(
        "Course.php",
        "<?php\n\n#[Spec(extends: \"Base\", implements: \"Enrolable\")]\nclass Course extends Base implements Enrolable {}\n",
    )]);
    let graph = graph_at(dir.path()).unwrap();
    let findings = PhpAttributeReader.extract_findings(dir.path()).unwrap();

    assert!(
        findings.is_empty(),
        "extends is an accepted key: {findings:?}"
    );
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(
        graph.edges.len(),
        1,
        "only implements yields an edge; extends has no label in the ecosystem (cfdb-045 §3.3): {:?}",
        graph.edges
    );
    assert_eq!(graph.edges[0].raw_target, "Enrolable");
}

#[test]
fn an_unknown_key_is_a_finding_naming_the_key() {
    let dir = tree(&[(
        "Course.php",
        "<?php\n\n#[Spec(implements: \"Enrolable\", inherits: \"Base\")]\nclass Course {}\n",
    )]);
    let findings = PhpAttributeReader.extract_findings(dir.path()).unwrap();
    assert_eq!(findings.len(), 1, "{findings:?}");
    match &findings[0] {
        domain::Violation::UnknownAttributeKey { concept, key, .. } => {
            assert_eq!(concept, "Course");
            assert_eq!(key, "inherits");
        }
        other => panic!("expected an unknown-key finding, got {other:?}"),
    }
}

#[test]
fn a_php_file_without_the_attribute_yields_nothing() {
    let dir = tree(&[("Plain.php", "<?php\n\nclass Plain {}\n")]);
    let graph = graph_at(dir.path()).unwrap();
    assert!(
        graph.nodes.is_empty() && graph.edges.is_empty(),
        "{graph:?}"
    );
}

const ATTRIBUTED: &str = "<?php\nnamespace App\\Catalogue;\n\n#[Spec(implements: \"Enrolable\")]\nfinal class Course implements Enrolable {}\n";

#[test]
fn a_generated_tree_under_var_is_not_read_as_source() {
    let dir = tree(&[
        ("src/Course.php", ATTRIBUTED),
        ("var/phpstan/Cached.php", ATTRIBUTED),
        ("var/cache/dev/ContainerXyz/Dumped.php", ATTRIBUTED),
    ]);
    let graph = graph_at(dir.path()).unwrap();
    assert_eq!(
        graph.nodes.len(),
        1,
        "a built project writes `.php` under var/ — a phpstan cache, a dumped container — and it is machine-written, not the source a spec is compared against: {:?}",
        graph.nodes
    );
}

#[test]
fn a_file_under_var_that_is_not_utf_8_no_longer_stops_the_walk() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("var/phpstan")).unwrap();
    std::fs::write(dir.path().join("src/Course.php"), ATTRIBUTED).unwrap();
    std::fs::write(
        dir.path().join("var/phpstan/stale.php"),
        b"<?php\n\xff\xfe\n".as_slice(),
    )
    .unwrap();

    let graph = graph_at(dir.path()).expect(
        "a stale cache under var/ is never opened, so its bytes cannot decide whether the check runs",
    );
    assert_eq!(graph.nodes.len(), 1, "{:?}", graph.nodes);
}

#[test]
fn a_file_that_is_not_utf_8_inside_a_walked_tree_is_still_a_could_not_run_naming_it() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/Course.php"), ATTRIBUTED).unwrap();
    std::fs::write(
        dir.path().join("src/Broken.php"),
        b"<?php\n\xff\xfe\n".as_slice(),
    )
    .unwrap();

    let err = graph_at(dir.path())
        .expect_err("source the project owns that cannot be decoded is a could-not-run");
    let rendered = format!("{err}");
    assert!(
        rendered.contains("Broken.php") && rendered.contains("valid UTF-8"),
        "the refusal names the file that could not be decoded, so the reader has something to chase: {rendered}"
    );
}
