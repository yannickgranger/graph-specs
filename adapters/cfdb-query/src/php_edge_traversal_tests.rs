use super::*;
use crate::CfdbQueryReader;
use domain::{ContextDecl, OwnedUnit};
use ports::CodeFacts;
use std::path::Path;

const KEYSPACE: &str = r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
{"id":"crate:php-workspace","label":"Crate","props":{"name":"php-workspace"}},
{"id":"module:App\\Catalogue\\Domain","label":"Module","props":{"name":"App\\Catalogue\\Domain"}},
{"id":"module:App\\Marketing","label":"Module","props":{"name":"App\\Marketing"}},
{"id":"item:App\\Catalogue\\Domain\\Course","label":"Item","props":{"kind":"trait","line":3,
 "name":"Course","php_construct":"class_declaration","qname":"App\\Catalogue\\Domain\\Course"}},
{"id":"item:App\\Catalogue\\Domain\\CourseRepository","label":"Item","props":{"kind":"trait","line":3,
 "name":"CourseRepository","php_construct":"interface_declaration","qname":"App\\Catalogue\\Domain\\CourseRepository"}},
{"id":"item:App\\Catalogue\\Domain\\Course::rename","label":"Item","props":{"kind":"fn","line":5,
 "name":"rename","php_construct":"method_declaration","qname":"App\\Catalogue\\Domain\\Course::rename"}},
{"id":"item:App\\Marketing\\Flyer","label":"Item","props":{"kind":"trait","line":3,
 "name":"Flyer","php_construct":"class_declaration","qname":"App\\Marketing\\Flyer"}}
],"edges":[
{"src":"item:App\\Catalogue\\Domain\\Course","dst":"module:App\\Catalogue\\Domain","label":"IN_MODULE"},
{"src":"item:App\\Catalogue\\Domain\\Course","dst":"crate:php-workspace","label":"IN_CRATE"},
{"src":"item:App\\Catalogue\\Domain\\CourseRepository","dst":"module:App\\Catalogue\\Domain","label":"IN_MODULE"},
{"src":"item:App\\Catalogue\\Domain\\Course::rename","dst":"module:App\\Catalogue\\Domain","label":"IN_MODULE"},
{"src":"item:App\\Marketing\\Flyer","dst":"module:App\\Marketing","label":"IN_MODULE"}
]}"#;

fn surface(units: &[&str]) -> DeclaredSurface {
    DeclaredSurface::from_contexts(&[ContextDecl::new(
        "catalogue".to_string(),
        units.iter().map(|u| OwnedUnit((*u).to_string())).collect(),
        Vec::new(),
        Vec::new(),
        Source::Spec {
            path: PathBuf::from("specs/contexts/catalogue.md"),
            line: 1,
        },
    )])
}

fn read(units: &[&str]) -> Vec<ConceptNode> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("keyspace.json");
    std::fs::write(&path, KEYSPACE).expect("write keyspace");
    CfdbQueryReader::new(&path)
        .with_surface(surface(units))
        .concepts(Path::new("/ws"))
        .expect("read keyspace")
}

fn read_err(keyspace: &str) -> ports::ReaderError {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("keyspace.json");
    std::fs::write(&path, keyspace).expect("write keyspace");
    CfdbQueryReader::new(&path)
        .with_surface(surface(&["App\\A"]))
        .concepts(Path::new("/ws"))
        .expect_err("refused")
}

fn names(nodes: &[ConceptNode]) -> Vec<&str> {
    let mut out: Vec<&str> = nodes.iter().map(|n| n.name.as_str()).collect();
    out.sort_unstable();
    out
}

#[test]
fn a_class_and_an_interface_under_a_declared_prefix_are_both_concept_rung() {
    let nodes = read(&["App\\Catalogue"]);
    assert_eq!(names(&nodes), vec!["Course", "CourseRepository"]);
}

#[test]
fn a_method_under_a_declared_class_binds_nothing() {
    let nodes = read(&["App\\Catalogue"]);
    assert!(!names(&nodes).contains(&"rename"));
}

#[test]
fn a_class_outside_every_declared_prefix_binds_nothing() {
    let nodes = read(&["App\\Catalogue"]);
    assert!(!names(&nodes).contains(&"Flyer"));
}

#[test]
fn a_second_declared_prefix_admits_its_own_class() {
    let nodes = read(&["App\\Catalogue", "App\\Marketing"]);
    assert_eq!(names(&nodes), vec!["Course", "CourseRepository", "Flyer"]);
}

#[test]
fn containment_is_read_by_edge_traversal_not_by_a_prop() {
    let nodes = read(&["App\\Catalogue"]);
    let course = nodes
        .iter()
        .find(|n| n.name == "Course")
        .expect("Course emitted");
    assert_eq!(
        course.module_path.as_deref(),
        Some("App\\Catalogue\\Domain")
    );
    assert_eq!(course.unit.as_deref(), Some("App\\Catalogue"));
}

#[test]
fn an_empty_surface_admits_nothing_and_the_keyspace_is_not_at_fault() {
    let nodes = read(&[]);
    assert!(nodes.is_empty());
}

#[test]
fn a_rust_keyspace_does_not_take_the_php_path() {
    let rust = r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
      {"id":"item:domain::Thing","label":"Item","props":{"kind":"struct","name":"Thing",
       "visibility":"pub","is_test":false,"file":"/ws/domain/src/lib.rs","module_qpath":"domain"}}
    ],"edges":[]}"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rust.json");
    std::fs::write(&path, rust).expect("write keyspace");
    let nodes = CfdbQueryReader::new(&path)
        .with_surface(surface(&["App\\Catalogue"]))
        .concepts(Path::new("/ws"))
        .expect("read keyspace");
    assert_eq!(names(&nodes), vec!["Thing"]);
}

#[test]
fn a_php_construct_value_the_reader_does_not_know_is_refused_not_dropped() {
    let json = r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
      {"id":"item:App\\A\\Told","label":"Item","props":{"kind":"trait","name":"Told",
       "php_construct":"class_declaration","qname":"App\\A\\Told"}},
      {"id":"item:App\\A\\Level","label":"Item","props":{"kind":"trait","name":"Level",
       "php_construct":"enum_declaration","qname":"App\\A\\Level"}}],"edges":[]}"#;
    let err = read_err(json).to_string();
    assert!(err.contains("App\\A\\Level"), "{err}");
    assert!(err.contains("enum_declaration"), "{err}");
}

#[test]
fn a_php_item_with_no_php_construct_is_refused_naming_the_item() {
    let json = r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
      {"id":"item:App\\A\\Told","label":"Item","props":{"kind":"trait","name":"Told",
       "php_construct":"class_declaration","qname":"App\\A\\Told"}},
      {"id":"item:App\\A\\Untold","label":"Item","props":{"kind":"trait","name":"Untold",
       "qname":"App\\A\\Untold"}}],"edges":[]}"#;
    let err = read_err(json).to_string();
    assert!(err.contains("App\\A\\Untold"), "{err}");
    assert!(err.contains("no `php_construct`"), "{err}");
}

#[test]
fn a_construct_ratified_as_below_the_rung_is_not_refused() {
    let json = r#"{"schema_version":{"major":0,"minor":8,"patch":0},"nodes":[
      {"id":"item:App\\A\\Told","label":"Item","props":{"kind":"trait","name":"Told",
       "php_construct":"class_declaration","qname":"App\\A\\Told"}},
      {"id":"item:App\\A\\Mix","label":"Item","props":{"kind":"trait","name":"Mix",
       "php_construct":"trait_declaration","qname":"App\\A\\Mix"}},
      {"id":"item:App\\slugify","label":"Item","props":{"kind":"fn","name":"slugify",
       "php_construct":"function_definition","qname":"App\\slugify"}}],"edges":[]}"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("keyspace.json");
    std::fs::write(&path, json).expect("write keyspace");
    let nodes = CfdbQueryReader::new(&path)
        .with_surface(surface(&["App\\A"]))
        .concepts(Path::new("/ws"))
        .expect("read keyspace");
    assert_eq!(names(&nodes), vec!["Told"]);
}
