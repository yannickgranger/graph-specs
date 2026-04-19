// arch-ban-unwrap-domain-ports.cypher
//
// Rule: no `.unwrap()` in non-test items inside the `domain` or `ports` crates.
//
// Rationale: `.unwrap()` panics on error. Inside the hexagonal core
// (domain + ports), every error path must be represented in the type
// system — that is why ReaderError, ViolationKind etc. exist. An unwrap
// there is an undeclared panic that bypasses the port's error contract.
//
// Test code is exempt: test fixtures legitimately unwrap known-good
// construction steps. `cs.is_test = false` restricts to prod code.
//
// The extractor (cfdb-extractor v0.1) emits one CallSite per call expr;
// `callee_path` is the textual path the author wrote. `.unwrap()` is a
// method call, so `callee_path = 'unwrap'` exactly — the `^unwrap$`
// anchor avoids matching `unwrap_or_default`, `unwrap_or_else`, etc.
//
// Expected: zero rows on a clean tree. Any row is a violation.

MATCH (caller:Item)-[:INVOKES_AT]->(cs:CallSite)
WHERE caller.crate =~ '(domain|ports).*'
  AND cs.callee_path =~ '^unwrap$'
  AND caller.is_test = false
  AND cs.is_test = false
RETURN caller.qname, caller.crate, cs.file, cs.callee_path
