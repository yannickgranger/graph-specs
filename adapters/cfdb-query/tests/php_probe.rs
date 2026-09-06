use std::path::Path;

use adapter_cfdb_query::CfdbQueryReader;
use ports::CodeFacts;

#[test]
fn a_keyspace_whose_items_carry_no_producer_mark_is_a_could_not_run() {
    let json = r#"{
        "schema_version": { "major": 0, "minor": 5, "patch": 0 },
        "nodes": [
            { "id": "item:App\\Service\\Foo", "label": "Item",
              "props": { "name": "Foo", "qname": "App\\Service\\Foo" } },
            { "id": "item:App\\Service\\Bar", "label": "Item",
              "props": { "name": "Bar", "qname": "App\\Service\\Bar" } }
        ],
        "edges": [
            { "src": "item:App\\Service\\Foo", "dst": "module:App\\Service",
              "label": "IN_MODULE" }
        ]
    }"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let keyspace = dir.path().join("php.json");
    std::fs::write(&keyspace, json).expect("write php keyspace");

    let err = CfdbQueryReader::new(&keyspace)
        .concepts(Path::new("/some/php/root"))
        .expect_err("an undiscriminated keyspace is refused, never read as the other producer")
        .to_string();

    assert!(err.contains("could not run the concept channel"), "{err}");
    assert!(err.contains("App\\Service\\Foo"), "{err}");
    assert!(err.contains("php_construct"), "{err}");
    assert!(err.contains("bounded_context"), "{err}");
}

#[test]
fn a_keyspace_carrying_two_producers_marks_refuses_naming_both() {
    let json = r#"{
        "schema_version": { "major": 0, "minor": 5, "patch": 0 },
        "nodes": [
            { "id": "item:App\\Service\\Foo", "label": "Item",
              "props": { "name": "Foo", "qname": "App\\Service\\Foo",
                         "php_construct": "class_declaration" } },
            { "id": "item:domain::Thing", "label": "Item",
              "props": { "name": "Thing", "qname": "domain::Thing",
                         "crate": "domain", "bounded_context": "equivalence" } }
        ],
        "edges": []
    }"#;
    let dir = tempfile::tempdir().expect("tempdir");
    let keyspace = dir.path().join("mixed.json");
    std::fs::write(&keyspace, json).expect("write mixed keyspace");

    let err = CfdbQueryReader::new(&keyspace)
        .concepts(Path::new("/ws"))
        .expect_err("one producer per keyspace: a disagreement refuses, never drops one side")
        .to_string();

    assert!(err.contains("two producers"), "{err}");
    assert!(err.contains("php"), "{err}");
    assert!(err.contains("rust"), "{err}");
}
