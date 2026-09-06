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
