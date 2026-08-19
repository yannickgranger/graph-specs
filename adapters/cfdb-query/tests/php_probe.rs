use std::path::Path;

use adapter_cfdb_query::CfdbQueryReader;
use ports::CodeFacts;

#[test]
fn php_style_propless_keyspace_yields_no_concepts_without_crashing() {
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

    let facts = CfdbQueryReader::new(&keyspace)
        .concepts(Path::new("/some/php/root"))
        .expect("PHP keyspace loads without error");

    assert!(
        facts.is_empty(),
        "PHP edge-only items must yield no provenance-bearing concepts (RFC-010 §11.5), got {}",
        facts.len()
    );
}
