# Clean Architecture Lens — RFC-010 Round 2 (Converged Position)

**Lens:** clean-arch
**Round:** 2 — deliberation, converge with ddd synthesis
**Date:** 2026-06-05

---

## A. Layering of the ladder (where each new concept lives)

The Dependency Rule requires all arrows to point inward: `adapters/* → ports → domain`. The analysis below confirms every proposed type lands in the correct tier.

### `AbstractionLevel` — domain

CLEAN. `AbstractionLevel { Context, Concept, Member }` is a pure value enum with no infrastructure dependency. Its change-reason is the spec dialect model ("what depth does an abstraction sit at?"). Analogue: `EdgeKind` in `domain/src/lib.rs:149`, which classifies relationships. `AbstractionLevel` classifies depth. Both are domain concepts; neither touches I/O or build systems.

The RFC-004 `BuildSystemKind` rejection does NOT apply here. `BuildSystemKind` was rejected because `CargoCrate`/`ComposerPackage` names a build-system artifact — the type's variants were infrastructure identifiers. `AbstractionLevel::Context` / `AbstractionLevel::Concept` name abstraction hierarchy positions — they are pure domain vocabulary.

Round-1 RC-1 (my "rename module/crate field names") stands as an independent concern and is addressed under B below. It does not affect `AbstractionLevel` placement.

### Context resolver — `domain/src/context.rs` as a pure function

The context resolver (§3.4 option A: "infer owning context from module co-location") must live in `domain/src/context.rs`. The existing `context_for_concept` at line 238 already does a simpler form. The new resolver takes `(Vec<ConceptNode with provenance>, Vec<ContextDecl>)` and returns a context assignment — purely functional, no I/O.

The DDD lens (RC-1, RC-3) identified a self-dogfood collision: `specs/concepts/core.md` has H1 "Core concepts" (descriptive title), while the owning context is "equivalence" (from `specs/contexts/equivalence.md:1`). This is a concrete input to the resolver: it receives both an H1 inference ("Core concepts") and an explicit `specs/contexts/` declaration ("equivalence") for the same set of concepts. The resolver's rule — "`specs/contexts/` wins when present" — handles this correctly only if it matches by path/ownership, not by name equality. The resolver must not compare H1 text against context names to decide precedence; it must compare which concepts each declaration claims to own. This is implementable without any I/O (the `ContextDecl.owned_units` vector already carries the ownership mapping). The resolver stays in domain.

### `CodeFacts` port — `ports/src/lib.rs`

`CodeFacts` must live in `ports/src/lib.rs` alongside `Reader`, `VerbReader`, and `ContextReader`. This was RC-2 in round 1; confirmed here. The signature:

```rust
pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
}
```

uses only domain types (`ConceptNode`) and port-resident types (`ReaderError`, `Path`). No cfdb types in the signature. The diff engine in `domain/src/diff.rs` never calls this port — it receives already-materialized `Vec<ConceptNode>` (with provenance fields) passed in via `CheckInput`, exactly as it receives `Graph` today. `application/src/lib.rs::run_check` is the composition root that calls the selected adapter.

The SOLID lens (CRP-1) identified that `CfdbQueryAdapter` compiles against 5 port traits while using 1 (20%). The resolution is: `CodeFacts` stays in `ports/src/lib.rs` for now (the existing ports crate has no heavy infrastructure deps — walkdir/pulldown-cmark are in the adapters, not in ports itself). If future adapters need only `CodeFacts` and none of the other four traits, a `ports-codefacts` split is warranted then. At R10-6, the only new adapter is one cfdb-query crate that will grow to implement `CodeFacts` — the CRP threshold is not breached yet. Prescribe an issue to track the split if/when a second `CodeFacts`-only adapter arrives.

### cfdb-query adapter — new crate `adapters/cfdb-query`, links cfdb concretely

CLEAN at the adapter tier. The dependency arrow is `adapters/cfdb-query → ports → domain` — this is the correct direction. The adapter carries `cfdb-core`, `cfdb-query`, `cfdb-petgraph`, and `cfdb-concepts` as Cargo dependencies in its own `Cargo.toml`. None of these appear in `domain/Cargo.toml` or `ports/Cargo.toml`.

The rust-systems lens (RC-3) correctly identified that this is a new workspace crate, not a few hundred lines, and that the compile cost of `petgraph` + `chumsky` is real. This is an engineering trade-off that OQ-4 must price explicitly — it does not violate the Dependency Rule.

**Correcting Invariant 3:** The RFC's current text "graph-specs links no cfdb crate directly" is too broad and misleading. The correct invariant is: "**`domain` and `ports` link no cfdb crate.** The cfdb-query adapter (`adapters/cfdb-query`) links cfdb-petgraph, cfdb-core, cfdb-query, and cfdb-concepts as Cargo dependencies — that is its job as an adapter. Infra concretion lives in the adapter tier; the inner rings stay cfdb-free." This restatement is accurate, not a relaxation of architectural discipline.

---

## B. Language-agnostic provenance naming (converged with ddd Q3)

Round-1 RC-1 proposed renaming `ConceptNode.crate` and `ConceptNode.module` to language-agnostic terms. The DDD lens (§3) found that "module" and "crate" ARE domain concepts in graph-specs' equivalence context (the domain IS about structural co-location) and the naming is a deliberate cfdb alignment decision. I contest the DDD lens's permissiveness here, but the resolution is a single converged vocabulary decision.

**The constraint from clean-arch:** A field named `crate` on a domain type is a Cargo-specific identifier. RFC-004 established the principle at `domain/src/context.rs:17`. The `OwnedUnit` type there is deliberately named to avoid "crate" or "package" — it means "the named ownership unit, whatever the language calls it." The same discipline must apply to `ConceptNode`.

**The constraint from ddd (cfdb alignment):** cfdb uses `Module` and `Crate` as language-agnostic labels in its schema (`labels.rs:18-19`). For cfdb, a PHP namespace maps to `Module`, an npm package maps to `Crate`. The labels are already abstracted. If graph-specs aligns to cfdb's vocabulary, it should align to cfdb's ABSTRACTED labels, not to the Rust-specific words those labels happen to resemble.

**Converged vocabulary (one decision):**

| Field intent | cfdb label it aligns to | Proposed field name on `ConceptNode` |
|---|---|---|
| The module/namespace scope containing this concept | `:Module` (cfdb-extractor: Rust mod path; cfdb-extractor-php: PHP namespace) | `container: Option<String>` |
| The crate/package/library unit containing this concept | `:Crate` (cfdb: cfdb uses "Crate" as its language-agnostic label) | `unit: Option<String>` |
| The resolved owning bounded context | `:Context` (cfdb label) | `context: Option<String>` |

Using `unit` matches `OwnedUnit` in `domain/src/context.rs:17` (established language-agnostic precedent). Using `container` avoids "module" (Rust) and "namespace" (PHP). Using `context` matches the `:Context` cfdb label and the `ContextDecl.name` field in the existing domain model.

The DDD lens and clean-arch lens converge on this vocabulary. The ddd round-1 conclusion ("the naming is a deliberate cfdb alignment decision, manageable") is correct IF the alignment is to cfdb's abstracted labels (Module, Crate, Context) rather than to Rust-concrete terms. With `container`/`unit`/`context` the naming IS consistent with cfdb's abstracted intent while remaining language-agnostic in graph-specs' domain.

The rust-systems lens (RC-2) prescribed `module: Option<String>` and `crate_name: Option<String>` as a compile-safe migration. The vocabulary change from (module, crate_name) to (container, unit) is mechanical — it does not change the migration path: all 12 construction sites set `container: None, unit: None, context: None` in R10-1.

---

## C. "Define-clean-then-conform" does not violate clean-arch

The council's revised framing is correct: prescribe the right architecture, then require conforming migrations. This is standard clean-arch practice. The concern would arise only if the prescribed architecture required inner layers to import outer layers to implement the migration — which is not the case here.

How the conform-migrations land without architectural risk:

1. **graph-specs self-conform (same PR per repo discipline):** `specs/concepts/core.md` H1 is "Core concepts" but the owning context is "equivalence". The RFC must prescribe that self-hosting files conform in the same PR that introduces the H1-context check. The DDD lens (RC-1) identified this as a blocking self-dogfood collision. From a clean-arch standpoint this is a data migration (updating the spec file), not an architectural one. It belongs in R10-2 (markdown reader H1 handling) — the same PR that activates the H1-context check must also fix `core.md` so the tool dogfoods itself cleanly from commit 1 of that slice.

2. **agentry conform (target dogfood, non-gated):** RFC-010 §3.7 already specifies agentry target dogfood results are "reported not gated" on the RFC-010 PR. This is the correct approach — agentry's conform-migration is not a precondition for graph-specs' architecture.

3. **No inner-layer import from outer:** The context resolver runs over already-materialized data (no I/O); the `specs/contexts/` matcher in `domain/src/context.rs:238` already exists and uses only `Source`, `ContextDecl`, `OwnedUnit` — all domain types. Extending it to handle the H1-inference path adds no new import edge.

---

## Cross-lens convergence positions

### With SOLID (SRP-1, ISP-1, LSP-1)

- **SRP-1 (TreeAssembler pass):** AGREE. The H1-context state tracking must be in a separate state struct or a separate pass, not woven into the existing `handle_event` / `SectionState`. The RFC-005 precedent (separate `extract_annotations_from_source`) establishes the pattern. R10-2 prescription must specify this explicitly.

- **ISP-1 (wrapping strategy):** AGREE. The four cohesion variants should be wrapped as `Violation::Cohesion(CohesionViolation)` mirroring `Violation::Context(ContextViolation)`. This keeps `violation_key` at 13 arms (one new arm for `Cohesion`), avoids `ndjson.rs` line bloat, and is consistent with the existing taxonomy. The RFC must specify this in §3.5 and in R10-1 / R10-4 prescriptions.

- **LSP-1 (module granularity definition):** AGREE this is blocking. "Module" as a matching key must be precisely defined before R10-6's parity test is meaningful. The SOLID lens identifies the gap: source-walker emits file-path-derived module; cfdb emits fully-qualified Rust mod path. I favor option (a) from the SOLID lens: define the cohesion unit as **crate-granular** (not module-granular), matching `OwnedUnit` semantics from RFC-001. This avoids the file-vs-mod-path divergence entirely: two concepts co-locate if they share the same `unit` (crate/package), not the same sub-module. Sub-module granularity is the deeper integration that OQ-4 can address when cfdb-query is primary. This resolves LSP-1 without scope expansion.

### With DDD (RC-1, RC-2, RC-3, RC-4)

- **RC-1 (core.md self-dogfood collision):** ENDORSE as blocking. The context resolver must not silently produce "Core concepts" as a context name. R10-2 must fix `core.md` in the same PR.

- **RC-2 (Invariant 2 vocabulary correction):** ENDORSE. `ConceptNode` maps to cfdb's `:Item`, not `:Concept`. Correcting Invariant 2 removes a future split-brain in the cfdb-query adapter implementation.

- **RC-3 (tripartite homonym precedence):** ENDORSE as advisory (not blocking for this RFC since (C) is deferred). The precedence rule `specs/contexts/ > concepts/ H1 > cfdb concepts.toml` should be stated even though (C) is deferred — deferring the resolver does not defer the precedence declaration.

- **RC-4 (H3-as-sub-concept vs Member):** ENDORSE as advisory. `AbstractionLevel::Member` collapsing H3 and H4 is an incorrect ontology if agentry uses H3 for sub-types. Given Member is "emitted not diffed," the immediate impact is zero. The clean-arch implication: if L3 diffing lands and the enum is wrong, fixing it requires modifying `domain` — but `#[non_exhaustive]` on `AbstractionLevel` makes this a source-breaking change only outside the defining crate. Clean-arch cost is manageable.

### With rust-systems (RC-1, RC-2, RC-3)

- **RC-1 (violation_key rank assignment):** ENDORSE. Cohesion variants wrapped as `Violation::Cohesion(CohesionViolation)` → `violation_key` gains one arm at rank 12. Rank assignment must be in R10-1, not deferred to R10-4.

- **RC-2 (Option fields, 12 construction sites):** ENDORSE. `container: Option<String>`, `unit: Option<String>`, `context: Option<String>` (using converged vocabulary from B above). All 12 sites set `None` in R10-1. R10-2/R10-3 fill the fields.

- **RC-3 (cfdb-query is a new crate with real deps):** ENDORSE the engineering reality. This does not change the architectural verdict (adapter-tier cfdb linkage is clean) but the RFC §3.3 and §3.8 must be updated to state the Cargo dep chain explicitly rather than describing it as a "few hundred lines."

---

## Summary of clean-arch RC resolutions for the synthesis

| Round-1 RC | Status | Converged resolution |
|---|---|---|
| RC-1 (field naming language-biased) | MODIFIED → converged vocab | `container`/`unit`/`context` as `Option<String>` fields |
| RC-2 (CodeFacts placement unspecified) | CONFIRMED BLOCKING | `ports/src/lib.rs`; diff engine never calls port |

New positions from round-2 cross-lens review:

| Finding | From lens | Clean-arch position |
|---|---|---|
| Invariant 3 restatement | rust-systems RC-3 | ENDORSE: inner rings (domain/ports) cfdb-free; adapter crate links cfdb concretely — that is its job |
| core.md H1 collision | ddd RC-1 | ENDORSE BLOCKING: R10-2 must fix `core.md` in same PR as H1-context check |
| Invariant 2 vocabulary (Concept vs Item) | ddd RC-2 | ENDORSE BLOCKING: correction required to prevent cfdb-query adapter split-brain |
| Cohesion unit = crate-granular (resolves LSP-1) | solid LSP-1 | ENDORSE: unit-granular cohesion avoids file-vs-mod-path divergence; sub-module granularity is OQ-4 |
| `Violation::Cohesion(CohesionViolation)` wrapping | solid ISP-1 / rust-systems RC-1 | ENDORSE: one new `violation_key` arm at rank 12 |
| TreeAssembler separate pass | solid SRP-1 | ENDORSE: separate state struct for H1 context tracking |

---

## Cross-project vocabulary reconciliation — AUGMENT

*Added after operator instruction: "propose ONE containment vocabulary shared by graph-specs AND cfdb."*

### What cfdb's schema actually says (verified, not assumed)

cfdb's `:Item` node (`cfdb-core/src/schema/describe/nodes.rs:97-119`) carries these three provenance properties today:

| cfdb `:Item` property | Type | Meaning |
|---|---|---|
| `crate` | string | Cargo package name (e.g. `cfdb_core`) |
| `module_qpath` | string | Fully-qualified `::` module path (e.g. `cfdb_core::schema::labels`) |
| `bounded_context` | string | Resolved bounded context name (e.g. `cfdb`) |

cfdb's structural containment: `(:Item) -[:IN_MODULE]-> (:Module {qpath: "..."}) -[:IN_CRATE]-> (:Crate {name: "..."})`. The `module_qpath` on `:Item` is a **denormalized copy** of the `IN_MODULE` target's `qpath` — both represent the same fact.

cfdb uses `"crate"` (the Rust word) as an `:Item` property name and as a `:Module` property name. The `:Crate` node label uses the Rust word. cfdb also uses `bounded_context` — not `context` — as the per-item property.

### The vocabulary gap

My round-2 proposal of `(container, unit, context)` was based on cfdb's *labels* (`:Module`, `:Crate`, `:Context`). But cfdb's actual *property names on `:Item`* are `(module_qpath, crate, bounded_context)` — different from the label names. This matters because the cfdb-query adapter must produce `ConceptNode` fields by reading cfdb's item properties, not by reading node labels. The field names on `ConceptNode` should align with the cfdb `:Item` property names the adapter reads — otherwise there is a translation layer that adds confusion without adding value.

### Proposed shared vocabulary (one decision for both tools)

| What it represents | cfdb `:Item` property | Proposed `ConceptNode` field | What each tool does |
|---|---|---|---|
| Module scope (fully-qualified path) | `module_qpath` | `module_qpath: Option<String>` | graph-specs emits the Rust `::` path; cfdb already has it. Same string, same field name. |
| Containing package | `crate` | `crate_name: Option<String>` | Both use the Cargo package name. cfdb calls it `crate`; graph-specs uses `crate_name` to avoid conflict with the Rust keyword. PHP/TS: cfdb-extractor-php maps namespace root → `:Module.crate` already — same value, same field. |
| Owning bounded context | `bounded_context` | `bounded_context: Option<String>` | cfdb computes this via `cfdb-concepts::compute_bounded_context`; graph-specs computes it via the context resolver. Same semantic, same string. |

**This is the cross-fertilization decision:** graph-specs adopts cfdb's existing `:Item` property names verbatim. cfdb changes nothing. The two tools converge on `module_qpath` / `crate` (as `crate_name` in Rust) / `bounded_context` as the shared containment vocabulary.

**Why this is better than my round-2 `(container, unit, context)` proposal:**

1. `module_qpath` is a precise name: it specifies the format (`::` qualified path), not just "some scope thing." `container` was vague.
2. `crate_name` matches what cfdb stores as the `crate` property, with only a Rust-keyword-avoidance suffix. `unit` was unfamiliar vocabulary that cfdb would need to learn.
3. `bounded_context` matches cfdb's exact property name for the same concept — an adapter reading cfdb produces `bounded_context` strings and puts them into `ConceptNode.bounded_context` with zero translation. `context` was close but not exact.

**The one adjustment cfdb should make:** cfdb `:Item.bounded_context` is currently a `string` property computed at extraction time from `.cfdb/concepts/*.toml` or the heuristic. The cfdb-query adapter in graph-specs will read this property and populate `ConceptNode.bounded_context`. For this to work on repos that have not run cfdb's enrichment, the cfdb-query adapter needs a fallback: if `bounded_context` is empty or absent on a keyspace item, it falls back to querying the `:Context` node via the `BELONGS_TO` edge chain. This is an adapter-side concern, not a cfdb schema change. No cfdb modification required.

**Where cfdb might adapt (a prescription, not a demand):** cfdb's `:Module` node descriptor (`describe/nodes.rs:24-51`) currently says "A Rust module — either a `mod` block or a file-level module." If cfdb formally declares that `:Module` covers PHP namespaces, TS namespaces, and Go packages (as the extractors already implement), the description should say so. This makes the language-agnosticism explicit in cfdb's own Published Language. Worth filing as a cfdb documentation issue, not a schema change.

### Invariant 2 correction (shared with ddd RC-2)

The RFC's Invariant 2 now reads correctly as: "Graph-specs' `ConceptNode` provenance fields (`module_qpath`, `crate_name`, `bounded_context`) align with cfdb's `:Item` properties of the same semantic. `ConceptNode` maps to cfdb's `:Item`, NOT cfdb's `:Concept` (which is an enrichment-layer overlay)." This collapses ddd's RC-2 correction into the vocabulary alignment.

### LSP-1 resolution (updated)

With `module_qpath` as the field name, the SOLID LSP-1 gap ("module granularity: file-path vs Rust mod path") has a precise answer: `module_qpath` is the `::` module path, not the file path. The source-walking adapter must produce the same format as cfdb — which means the Rust reader needs to derive `module_qpath` from the file path by stripping the `src/` prefix, removing `.rs`, and replacing `/` with `::`. This is simple derivation (`domain/src/diff.rs` → `domain::diff`). The parity test in R10-6 is now precisely defined: both adapters must emit the same `module_qpath` string for the same item on graph-specs' own tree. File-path-granular cohesion is abandoned; `::` module path granularity is the shared definition. This resolves SOLID LSP-1 without the "crate-granular only" shortcut I proposed earlier (which would have lost information cfdb already has).

