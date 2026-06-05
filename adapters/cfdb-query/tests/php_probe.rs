//! RFC-010 §12-D / §11.5 PHP empty-provenance probe.
//!
//! A PHP `:Item` carries `name`/`qname` but no containment props — `file`,
//! `crate`, `module_qpath`, `bounded_context` live on edges, not props (PHP is
//! edge-only). The prop-read ACL therefore yields ZERO provenance-bearing
//! concepts for a PHP keyspace and, critically, does NOT crash or reject it.
//! This is the evidence that PHP is *not* "nearly free" via prop-reads and
//! needs an edge-traversal path (deferred to RFC-011).

use std::path::Path;

use adapter_cfdb_query::CfdbQueryReader;
use ports::CodeFacts;

#[test]
fn php_style_propless_keyspace_yields_no_concepts_without_crashing() {
    // PHP `:Item`s: a name, no `kind`/`file`/containment props. The
    // concept-population filter requires `kind` and `file`, so all are
    // silently dropped — empty result, no error.
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
