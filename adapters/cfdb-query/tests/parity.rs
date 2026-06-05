//! RFC-010 R10-6 parity test.
//!
//! The cfdb-query ACL must derive the SAME agnostic `module_path` / `unit` as
//! the source-walk `CodeFacts` adapter (`RustReader`) on a REAL keyspace
//! extracted from graph-specs' own tree — 0-mismatch (RFC-010 §7).
//!
//! Both sides are the *real* adapters reached through the `CodeFacts` port —
//! the source-walk side is NOT reproduced inline. Env-guarded so the test
//! no-ops without a keyspace:
//!   `RFC010_KEYSPACE` = path to a `cfdb extract … .json`
//!   `RFC010_ROOT`     = the absolute code root the keyspace was extracted from
//!
//! `context` is deliberately excluded from the parity key — it is the
//! multi-crate-divergent field (RFC-010 §13-A): source-walk leaves it `None`
//! pending `specs/contexts/` resolution, cfdb sets a per-crate value.

use std::collections::BTreeSet;
use std::path::Path;

use adapter_cfdb_query::CfdbQueryReader;
use adapter_rust::RustReader;
use domain::ConceptNode;
use ports::CodeFacts;

type ParityKey = (String, Option<String>, Option<String>);

fn parity_key(c: &ConceptNode) -> ParityKey {
    (c.name.clone(), c.module_path.clone(), c.unit.clone())
}

#[test]
fn cfdb_query_matches_source_walk_on_module_path_and_unit() {
    let (Ok(keyspace), Ok(root)) = (
        std::env::var("RFC010_KEYSPACE"),
        std::env::var("RFC010_ROOT"),
    ) else {
        eprintln!("RFC010_KEYSPACE / RFC010_ROOT unset — skipping parity test");
        return;
    };
    let root = Path::new(&root);

    let source_walk = RustReader.concepts(root).expect("source-walk concepts");
    let acl = CfdbQueryReader::new(&keyspace)
        .concepts(root)
        .expect("cfdb-query concepts");

    assert!(!source_walk.is_empty(), "source-walk found no concepts");
    assert!(!acl.is_empty(), "cfdb-query found no concepts");

    let sw_set: BTreeSet<ParityKey> = source_walk.iter().map(parity_key).collect();
    let acl_set: BTreeSet<ParityKey> = acl.iter().map(parity_key).collect();

    let only_sw: Vec<&ParityKey> = sw_set.difference(&acl_set).collect();
    let only_acl: Vec<&ParityKey> = acl_set.difference(&sw_set).collect();

    eprintln!(
        "PARITY: source-walk {} | cfdb-query {} | only-source-walk {} | only-cfdb-query {}",
        sw_set.len(),
        acl_set.len(),
        only_sw.len(),
        only_acl.len()
    );
    for k in only_sw.iter().take(25) {
        eprintln!("  only source-walk: {k:?}");
    }
    for k in only_acl.iter().take(25) {
        eprintln!("  only cfdb-query:  {k:?}");
    }

    assert!(
        only_sw.is_empty() && only_acl.is_empty(),
        "module_path/unit parity mismatch: {} only-source-walk, {} only-cfdb-query",
        only_sw.len(),
        only_acl.len()
    );
}
