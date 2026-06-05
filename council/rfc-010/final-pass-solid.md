# SOLID Lens — Final Pass

**Date:** 2026-06-05  
**Input:** RFC-010 §12 (dry-run binding resolutions)  
**Verdict:** RATIFY

---

## §12-G — Slice-order hazard: merge R10-4 formatter arms into R10-3

**RATIFY the merge.**

The risk is confirmed by reading `application/src/text.rs:141` and `application/src/ndjson.rs`.
Both files have bare `_ =>` wildcards that fire on any `Violation` variant not yet matched
with an explicit arm. `Violation` is `#[non_exhaustive]` — the wildcard is the correct
Rust-safety measure for downstream crates, but inside `application/` (the defining-adjacent
crate), it means new `Violation::Cohesion(...)` emissions route to `"unknown violation"` on
text output and to a skeleton `{"schema_version":"3", "violation":"unknown"}` on NDJSON
output. Both are **silent useless gates**: the violation list is non-empty (exit 1), the CI
job fails, but the output tells the human nothing actionable. This is worse than a compile
error — it is a runtime-silent failure.

The merge is correct on ISP grounds as well: a `Cohesion` violation's text and NDJSON
representation belong to the same responsibility cluster as the cohesion check itself (R10-3).
Splitting formatter arms from the variant definition into a later slice (R10-4) creates a
temporal ISP violation — the interface (the `CohesionViolation` enum) exists, but the output
contract is incomplete until R10-4. That intermediate state is not a valid public interface.

**Constraint for R10-3 scope:** R10-3 must include:
1. The three `CohesionViolation` variants in `domain/src/context.rs`
2. `Violation::Cohesion` arm in `application/src/text.rs::format_violation` 
3. `Violation::Cohesion` arm in `application/src/ndjson.rs::violation_to_record`
4. A `cohesion_violation_to_record` helper (analogous to `context_violation_to_record`
   at `application/src/ndjson.rs:134`)
5. Dogfood test asserting that each cohesion variant round-trips through text and NDJSON
   before R10-3 is merged

R10-4 then handles only `SchemaVersion::V3`, the provenance fields on source objects, and the
15+ existing `"2"` → `"3"` assertion updates. R10-4 can ship without touching the cohesion
variant arms (they landed in R10-3). This is a clean slice boundary.

---

## §12-J — Cohesion variant set: 3 (dry-run) vs 4 (synthesis)

**RATIFY the dry-run's 3-variant set. Withdraw `ScatteredConcepts` and `SplitUnit` from
synthesis.**

**Mapping:**

| Dry-run (ships) | Synthesis (proposed) | Relation |
|---|---|---|
| `ContextWithoutCohesionUnit` | `UnitlessContext` | Semantically identical. Keep the dry-run name (verbose, but matches naming register of `ContextViolation::MembershipUnknown` — consistency wins). |
| `ConceptContextMismatch` | `MisfiledConcept` | Semantically identical. Keep `ConceptContextMismatch` — it is precise and self-documenting. |
| `SubConceptOrphan` | — (missing from synthesis) | New. Correct addition. |
| — (absent from dry-run) | `ScatteredConcepts` | Subsumed — see below. |
| — (absent from dry-run) | `SplitUnit` | Partially subsumed — see below. |

**Why `ScatteredConcepts` is correctly omitted:**

`ScatteredConcepts` fires when "H2 concepts under one H1 span more than one module." But
`ConceptContextMismatch` already fires on every individual concept whose
`ConceptNode.bounded_context ≠ H1 context`. When a context's concepts scatter across
modules, each scattered concept has a different `bounded_context` than its H1 — so
`ConceptContextMismatch` fires N times, once per offending concept. The human sees N
findings naming the offending concept and its actual context. `ScatteredConcepts` is an
aggregated summary view over the same data — useful UX, but not a distinct invariant
check. Introducing it as a separate variant would emit both the aggregate
(`ScatteredConcepts: context X spans modules A, B`) and the per-concept instances
(`ConceptContextMismatch: Foo in X is actually in A`) for the same violation — double-
reporting the same invariant breach. That is an ISP violation on the consumer side (they
must filter/deduplicate). Omit in v1; consider as a v4 reporting enhancement, not a new
cohesion invariant.

**Why `SplitUnit` is correctly omitted from v1:**

`SplitUnit` (one code module documented under multiple H1 contexts) IS a distinct invariant:
it cannot be reduced to a set of `ConceptContextMismatch` findings when `bounded_context` on
`:Item` is authoritative — if all concepts in module M have `bounded_context = "X"`, but
some are documented under H1 `"Y"`, those fire `ConceptContextMismatch`. The remaining
concepts (documented under H1 "X") do not fire. So `SplitUnit` IS separately detectable.
However: it is the rarest case in practice, requires a GROUP-BY-module pass over all
concepts (a different computational shape from the per-concept mismatch check), and §12-A
establishes that even the per-concept mismatch check requires the cfdb-query adapter for
repos without `specs/contexts/`. Introducing `SplitUnit` in v1 without a proven corpus
example adds complexity without validation. Defer to v2 (post-agentry dogfood proves
the need) or document as an explicitly-deferred variant in `CohesionViolation`'s doc
comment.

**Final ISP/CCP verdict on the 3-variant set:**

`ContextWithoutCohesionUnit`, `ConceptContextMismatch`, `SubConceptOrphan` form a cohesive
family. Their shared change reason is: "the spec-side heading tree violates the context ⊃
concept ⊃ sub-concept structural rule." All three are intra-spec or spec-vs-code cohesion
violations. They belong together in `CohesionViolation`. `SplitUnit` would belong there
too when it lands — no taxonomy churn required, just a new variant.

**`violation_key` rank assignment:** rank 12, `(cv.context_name(), 12)`. The three variants
share one `violation_key` arm via the `CohesionViolation::context_name()` method. This is
correct and matches the `ContextViolation` precedent at rank 8.

---

## §12-K — `SubConceptOrphan` detection in R10-2 (TreeAssembler), R10-3 consumes flag

**RATIFY the responsibility boundary.**

`SubConceptOrphan` requires the bit "was there an H2 between this H1 and this H3?" This is
parse-time state, not diff-time state. The `ContextTreeState` struct (prescribed in my
round-2 section A) holds exactly the information needed: it tracks H1 → H2 → H3 parent
links as it runs the separate parser pass. When it sees `Event::Start(Tag::Heading{H3})`
and `current_h2: None`, the orphan is detectable at that moment. The diff engine in R10-3
receives `Vec<ConceptNode>` — parse position is gone. Re-detecting the orphan in R10-3 would
require re-parsing or carrying a flag, both of which are worse than detecting it at parse time.

**The clean boundary:** R10-2's `extract_context_tree_from_source` (or the `ContextTreeState`)
detects orphan H3s and **emits them as `CohesionViolation::SubConceptOrphan` values directly
into a returned `Vec<CohesionViolation>`**, alongside the parent-link output. The R10-3
cohesion diff pass merges these pre-emitted violations into its output rather than re-
deriving them. This is the same pattern used for `ReaderError` — parse-time failures are
surfaced at the port boundary, not re-detected downstream.

**`SubConceptOrphan` fields:**
```rust
SubConceptOrphan {
    sub_concept: String,      // the H3 heading text
    context: String,          // the enclosing H1 context name
    spec_source: Source,      // location of the H3 heading
}
```

`context_name()` on `SubConceptOrphan` returns `context.as_str()` — consistent with the
other variants' accessor.

---

## Final status of SOLID concerns

| SOLID RC | Status |
|---|---|
| RC-1 SRP (TreeAssembler) | RESOLVED — §3.2 + §12-K + dry-run confirms separate pass <15 |
| RC-2 ISP/violation wrapping | RESOLVED — `Violation::Cohesion(CohesionViolation)`, rank 12 |
| RC-3 LSP/module granularity | RESOLVED — `module_qpath` from file-path derivation; cfdb `:Item` props verbatim; parity defined |
| RC-4 CRP | RESOLVED — OQ-5 answered by §12-E: `codefacts` Cargo feature in `ports`; cfdb-core only dep |
| §12-G (slice order) | RATIFIED — formatter arms fold into R10-3 |
| §12-J (variant set) | RATIFIED — 3-variant dry-run set; `SplitUnit` deferred with comment |
| §12-K (detection boundary) | RATIFIED — SubConceptOrphan emitted by R10-2 TreeAssembler |

**FINAL VERDICT: RATIFY**

No remaining blockers. The RFC as hardened by §12 (binding) satisfies all SOLID + Component
Principles constraints. The `CohesionViolation` family is ISP-clean, CCP-coherent, and
correctly scoped. The TreeAssembler separation enforces SRP within the markdown reader's
complexity budget. The `module_qpath` definition gives both adapters a concrete, verifiable
parity contract. The `codefacts` Cargo feature keeps the CRP ratio above the 25% threshold
for all existing adapters.
