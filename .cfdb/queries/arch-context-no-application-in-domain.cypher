// arch-context-no-application-in-domain.cypher
//
// Rule: no item in the `domain` crate may invoke a path starting with
// `application::`.
//
// Rationale: `domain` is a supplier context (RFC-001 §3.8 — `equivalence`).
// The `orchestration` context owns `application` and consumes the
// domain's Published Language. A call from domain into application
// inverts the dependency direction required by clean-architecture
// layering. Today Cargo deps make this impossible (domain does not list
// application as a dependency), but a future refactor might wire
// something up incorrectly — this rule is belt-and-suspenders at the
// syn level.
//
// Expected: zero rows on a clean tree. Any row is a violation.

MATCH (caller:Item)-[:INVOKES_AT]->(cs:CallSite)
WHERE caller.crate =~ 'domain.*'
  AND cs.callee_path =~ '^application::.*'
  AND caller.is_test = false
  AND cs.is_test = false
RETURN caller.qname, cs.callee_path, cs.file
