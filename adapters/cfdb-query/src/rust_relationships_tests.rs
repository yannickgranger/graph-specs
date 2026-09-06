use crate::CfdbQueryReader;
use domain::EdgeKind;
use ports::CodeFacts;
use std::path::Path;

fn rust_item(id: &str, name: &str, kind: &str, extra: &str) -> String {
    format!(
        r#"{{"id":"item:{id}","label":"Item","props":{{"kind":"{kind}","name":"{name}",
        "qname":"{id}","visibility":"pub","is_test":false,"line":1,
        "file":"/ws/domain/src/lib.rs","module_qpath":"domain","crate":"domain",
        "bounded_context":"equivalence"{extra}}}}}"#
    )
}

fn read(nodes: &[String], edges: &str) -> Vec<domain::Edge> {
    let json = format!(
        r#"{{"schema_version":{{"major":0,"minor":5,"patch":0}},"nodes":[{}],"edges":[{edges}]}}"#,
        nodes.join(",")
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rust.json");
    std::fs::write(&path, json).expect("write keyspace");
    CfdbQueryReader::new(&path)
        .relationships(Path::new("/ws"))
        .expect("relationships")
}

fn pairs(edges: &[domain::Edge]) -> Vec<(String, &'static str, String)> {
    edges
        .iter()
        .map(|e| {
            (
                e.source_concept.name.clone(),
                e.kind.as_label(),
                e.target.name.clone(),
            )
        })
        .collect()
}

#[test]
fn the_impl_block_join_yields_a_type_to_trait_edge() {
    let nodes = vec![
        rust_item("domain::Walker", "Walker", "struct", ""),
        rust_item("domain::Reader", "Reader", "trait", ""),
        rust_item("domain::Walker::impl_Reader", "impl", "impl_block", ""),
    ];
    let edges = r#"{"src":"item:domain::Walker::impl_Reader","dst":"item:domain::Walker","label":"IMPLEMENTS_FOR"},
        {"src":"item:domain::Walker::impl_Reader","dst":"item:domain::Reader","label":"IMPLEMENTS"}"#;
    assert_eq!(
        pairs(&read(&nodes, edges)),
        vec![("Walker".to_owned(), "IMPLEMENTS", "Reader".to_owned())],
        "cfdb routes both ends through the impl block; the ACL joins them into the walk's shape"
    );
}

#[test]
fn an_implements_edge_without_its_implements_for_partner_yields_nothing() {
    let nodes = vec![
        rust_item("domain::Reader", "Reader", "trait", ""),
        rust_item("domain::Walker::impl_Reader", "impl", "impl_block", ""),
    ];
    let edges = r#"{"src":"item:domain::Walker::impl_Reader","dst":"item:domain::Reader","label":"IMPLEMENTS"}"#;
    assert!(
        read(&nodes, edges).is_empty(),
        "the join needs both halves; half an impl block names no type"
    );
}

#[test]
fn a_field_type_the_producer_records_yields_a_depends_on_edge() {
    let nodes = vec![
        rust_item("domain::Holder", "Holder", "struct", ""),
        rust_item("domain::Held", "Held", "struct", ""),
        r#"{"id":"field:domain::Holder.item","label":"Field","props":{"index":0,"name":"item",
        "parent_qname":"domain::Holder","type_path":"Held","type_normalized":"Held"}}"#
            .to_owned(),
    ];
    assert_eq!(
        pairs(&read(&nodes, "")),
        vec![("Holder".to_owned(), "DEPENDS_ON", "Held".to_owned())]
    );
}

#[test]
fn a_generic_argument_is_carried_when_the_producer_records_the_whole_type() {
    let nodes = vec![
        rust_item("domain::Holder", "Holder", "struct", ""),
        rust_item("domain::Held", "Held", "struct", ""),
        r#"{"id":"field:domain::Holder.items","label":"Field","props":{"index":0,"name":"items",
        "parent_qname":"domain::Holder","type_path":"Vec<Held>","type_normalized":"Vec<Held>"}}"#
            .to_owned(),
    ];
    assert_eq!(
        pairs(&read(&nodes, "")),
        vec![("Holder".to_owned(), "DEPENDS_ON", "Held".to_owned())],
        "the head rule reads every head in the type expression, as the source walk does"
    );
}

#[test]
fn a_qualified_path_names_its_last_segment_not_its_crate() {
    let nodes = vec![
        rust_item("domain::Holder", "Holder", "struct", ""),
        rust_item("domain::Held", "Held", "struct", ""),
        r#"{"id":"field:domain::Holder.item","label":"Field","props":{"index":0,"name":"item",
        "parent_qname":"domain::Holder","type_path":"other::Held","type_normalized":"other::Held"}}"#
            .to_owned(),
    ];
    assert_eq!(
        pairs(&read(&nodes, "")),
        vec![("Holder".to_owned(), "DEPENDS_ON", "Held".to_owned())]
    );
}

#[test]
fn a_field_type_naming_nothing_in_the_workspace_yields_no_edge() {
    let nodes = vec![
        rust_item("domain::Holder", "Holder", "struct", ""),
        r#"{"id":"field:domain::Holder.item","label":"Field","props":{"index":0,"name":"item",
        "parent_qname":"domain::Holder","type_path":"HashMap","type_normalized":"HashMap"}}"#
            .to_owned(),
    ];
    assert!(
        read(&nodes, "").is_empty(),
        "the walk drops an edge whose target is no concept, and so does the translation"
    );
}

#[test]
fn a_signature_return_yields_a_returns_edge_from_its_owner() {
    let nodes = vec![
        rust_item("domain::Walker", "Walker", "struct", ""),
        rust_item("domain::Graph", "Graph", "struct", ""),
        rust_item(
            "domain::Walker::walk",
            "walk",
            "method",
            r#","signature":"fn(&self) -> Graph""#,
        ),
    ];
    assert_eq!(
        pairs(&read(&nodes, "")),
        vec![("Walker".to_owned(), "RETURNS", "Graph".to_owned())],
        "cfdb attributes the return to the function; the walk attributes it to the owning concept"
    );
}

#[test]
fn the_translated_edge_kinds_are_what_the_reader_says_it_answers() {
    let nodes = [rust_item("domain::Walker", "Walker", "struct", "")];
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("rust.json");
    let json = format!(
        r#"{{"schema_version":{{"major":0,"minor":5,"patch":0}},"nodes":[{}],"edges":[]}}"#,
        nodes.join(",")
    );
    std::fs::write(&path, json).expect("write keyspace");
    let answered = CfdbQueryReader::new(&path)
        .answerable_relationships(Path::new("/ws"))
        .expect("answerable");
    assert_eq!(
        answered,
        vec![EdgeKind::Implements, EdgeKind::DependsOn, EdgeKind::Returns],
        "a Rust keyspace now answers all three kinds, so no bullet is excused as unanswerable"
    );
}
