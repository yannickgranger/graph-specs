// arch-ban-verb-in-bullet-prefixes.cypher
//
// Rule: EdgeKind must not contain a Verb variant; verb anchoring is a
// separate pass and must not be expressed as a relationship kind.
//
// Rationale: the v0.5 verb-anchoring pass (diff/verb.rs) is independent
// of the v0.3 edge pass (diff/edge.rs). Fusing them via a Verb EdgeKind
// would violate opt-in semantics: an edge-anchored concept would show up
// in the edge diff even when no verb bullet is declared. The boundary is
// enforced at two levels:
//   1. parse_bullet_edge / BULLET_PREFIXES never contains "verb:" — verb
//      bullets are dispatched exclusively through parse_verb_bullet.
//   2. EdgeKind has no Verb variant — verb anchors are typed as VerbAnchor,
//      not Edge.
//
// Text-grep fallback (CI):
//   grep -rn '"verb:"' adapters/markdown/src/lib.rs should NOT match
//   BULLET_PREFIXES.
//   grep -n 'Verb,' domain/src/lib.rs should NOT match inside EdgeKind.
//
// Expected: zero rows. Any row means EdgeKind has grown a Verb variant.

MATCH (item:Item)
WHERE item.kind = 'EnumVariant'
  AND item.parent_path =~ '^domain::EdgeKind$'
  AND item.name = 'Verb'
RETURN item.qname AS violation, item.file AS location
