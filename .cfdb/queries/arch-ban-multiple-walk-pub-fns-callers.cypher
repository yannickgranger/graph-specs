// arch-ban-multiple-walk-pub-fns-callers.cypher
//
// Rule: only extract_pub_fns (the VerbReader impl on RustReader) may
// invoke walk_pub_fns or visit_top_level_fn. Any other caller is a
// scope leak that bypasses the VerbReader port boundary.
//
// Rationale: walk_pub_fns / visit_top_level_fn are internal helpers that
// materialise every top-level pub fn in the code tree. Per RFC-005 §3.2
// (rust-systems-A), this traversal is encapsulated behind the VerbReader
// port and must not be reachable from any other entry point. If a second
// caller exists, the verb-extraction boundary has been violated and the
// "single walk per check run" performance invariant breaks.
//
// Text-grep fallback (CI):
//   grep -rn 'walk_pub_fns\|visit_top_level_fn' adapters/rust/src/
//   should list ONLY lines inside the extract_pub_fns function body.
//
// Expected: zero rows. Any row is an unauthorised caller.

MATCH (caller:Item)-[:INVOKES_AT]->(cs:CallSite)
WHERE cs.callee_path =~ '.*(walk_pub_fns|visit_top_level_fn)$'
  AND NOT caller.name = 'extract_pub_fns'
  AND cs.is_test = false
RETURN caller.qname AS caller_fn, caller.crate AS crate, cs.file AS file, cs.callee_path AS callee
