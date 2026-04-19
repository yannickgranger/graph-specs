// arch-context-no-syn-in-domain.cypher
//
// Rule: no item in the `domain` crate may invoke a path starting with
// `syn::`.
//
// Rationale: `syn` is a Rust parser. Its only legitimate home is the
// `reading` context (RFC-001 §3.8) — concretely `adapter-rust`, where
// the Rust-source reader walks AST nodes. A `syn::` invocation inside
// `domain` would mean the pure diff engine is reaching into parsing
// infrastructure, violating port purity (ReaderError is the contract;
// syn types must not leak past `adapter-rust`). This rule enforces the
// boundary at the call-graph level rather than relying on review to
// catch an errant `use syn::...`.
//
// Expected: zero rows on a clean tree. Any row is a violation.

MATCH (caller:Item)-[:INVOKES_AT]->(cs:CallSite)
WHERE caller.crate =~ 'domain.*'
  AND cs.callee_path =~ '^syn::.*'
  AND caller.is_test = false
  AND cs.is_test = false
RETURN caller.qname, cs.callee_path, cs.file
