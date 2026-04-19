// arch-context-no-cross-layer-unwrap.cypher
//
// Rule: no `.unwrap()` in non-test items inside `adapter-markdown` or
// `adapter-rust`.
//
// Rationale: the domain/ports rule (arch-ban-unwrap-domain-ports) already
// bans `.unwrap()` in the hexagonal core. This rule extends the same ban
// to the adapter crates — the `reading` bounded context per RFC-001.
// Adapters sit on the boundary between raw input (markdown text, syn
// AST) and the domain model; an unwrap there masks a reader-error path
// that should have surfaced through `ReaderError`. Test fixtures are
// exempt (as in the domain/ports rule): `cs.is_test = false` +
// `caller.is_test = false` restrict the match to production code.
//
// Expected: zero rows on a clean tree. Any row is a violation.

MATCH (caller:Item)-[:INVOKES_AT]->(cs:CallSite)
WHERE caller.crate =~ '(adapter-markdown|adapter-rust).*'
  AND cs.callee_path =~ '^unwrap$'
  AND caller.is_test = false
  AND cs.is_test = false
RETURN caller.qname, caller.crate, cs.file, cs.callee_path
