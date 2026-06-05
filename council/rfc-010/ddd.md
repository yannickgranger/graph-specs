# DDD Lens Review — RFC-010 Abstraction-level equivalence

**Reviewer:** DDD specialist (council round 1)
**Date:** 2026-06-05
**Verdict:** REQUEST CHANGES

---

## Bounded context map

RFC-010 touches the `equivalence` bounded context (all new domain types belong there — `AbstractionLevel`, `CodeFacts` port, cohesion violations), with a secondary concern in `reading` (markdown reader H1 parsing changes).

---

## 1. Homonym report

### 1.1 "Context" — three definers, partially resolved, one gap remains

Three definers exist in this ecosystem:

- **(A) `specs/contexts/<name>.md` H1** — RFC-001's explicit context declaration. Parsed by `MarkdownReader::extract_contexts`. Source: `/var/mnt/workspaces/graph-specs-rust/specs/contexts/equivalence.md:1`.
- **(B) `specs/concepts/<file>.md` H1** — RFC-010's proposed inline context declaration. H1 is file-scoped; multiple files can each declare a distinct context.
- **(C) `.cfdb/concepts/<name>.toml`** — cfdb-concepts' crate→context resolver. Source: `/var/mnt/workspaces/cfdb/crates/cfdb-concepts/src/lib.rs:8–15`.

RFC-010 §4 Invariant 1 proposes: "if both (A) and (B) name the same context, (A) is canonical-upstream and (B) conforms." OQ-1 explicitly flags this as open and defers (C).

**Assessment: partially sound but with one concrete gap on graph-specs' own tree.** The `contexts/` vs `concepts/` disambiguation (A wins over B) is correct Evans practice — the explicit declaration should dominate the inline inference. However, the RFC has a self-dogfood collision it does not acknowledge: `specs/concepts/core.md:1` has H1 text `"Core concepts"` (a descriptive title, not a context name), while the owning context is `"equivalence"` declared in `specs/contexts/equivalence.md`. Under RFC-010's rule, the H1 text of `core.md` would be treated as a context declaration producing `"Core concepts"` as the inferred context — which does NOT match `"equivalence"`. The Invariant 1 "contexts/ wins" rule cannot save this because the names differ; neither conforms to the other. This is a concrete split-brain the RFC does not address.

(C) is correctly deferred, but the deferral must state what happens if a repo has `.cfdb/concepts/*.toml` but no `specs/contexts/`: does cfdb's resolver override the H1 inference? The RFC is silent and leaves a tripartite homonym partially open.

### 1.2 "Concept" — semantic divergence between graph-specs and cfdb

RFC-010 Invariant 2 claims vocabulary alignment with cfdb: "`Context`/`Module`/concept provenance names and containment shape match cfdb's `Context`/`Module`/`Item`."

This claim is **partially false by omission.** In cfdb's schema:
- `:Item` is what graph-specs calls a "concept" (a pub struct/enum/trait). Source: `/var/mnt/workspaces/cfdb/crates/cfdb-core/src/schema/labels.rs:21` (`pub const ITEM`).
- `:Concept` in cfdb is a *distinct, separate* node label — an **overlay** label assigned by `enrich_concepts`, denoting a named business concept mapped from TOML files, not the same as a code item. Source: `/var/mnt/workspaces/cfdb/crates/cfdb-core/src/schema/describe/nodes.rs:306–328`.

The RFC proposes emitting `ConceptNode` with cfdb-aligned vocabulary, but cfdb uses `Item` for what graph-specs calls "Concept." The `:Concept` node in cfdb refers to something at a higher abstraction level (the TOML-declared named concept, similar to what graph-specs calls a "context"). This is a genuine homonym: the word "Concept" means different things in the two systems. Invariant 2 as written is misleading — it should say "aligns with `:Item`/`:Module`/`:Crate`/`:Context`" not "with `:Concept`."

### 1.3 "Member" — premature vocabulary

RFC-010 introduces `AbstractionLevel::Member` for H3/H4. In agentry (the target dogfood corpus), H4 is used in two distinct ways:

- As **method/invariant prose** under H2 concepts (`#### Degrade-open contract` under `## Outcome` — `/var/mnt/workspaces/agentry/specs/concepts/ac_verifier.md:31`).
- As **pub types that are concepts** in files where H2 is absent: `secret_resolver.md` has `#### SecretResolver`, `#### OrgKey`, `#### ResolveCtx` at H4 with NO H2 at all (`/var/mnt/workspaces/agentry/specs/concepts/secret_resolver.md:104,146,211`). These are exactly the types that would be L2 Concepts if the file were structured differently.

Similarly, H3 in agentry (`captain_cli.md:123` `### UnsatisfiedRolePrecondition`, `:185` `### QueryResults`) denotes **nested pub types owned by an H2 concept**, not methods/fields. RFC-010 §3.1 collapses H3 and H4 into `Member`, but the real agentry corpus shows H3 is frequently used for sub-concepts, not members.

If `AbstractionLevel::Member` is "emitted but not diffed" it is low-stakes for this RFC, but the enum definition encodes an incorrect ontology that future L3 RFCs will inherit.

---

## 2. Context relationship diagram

Under RFC-010's proposed changes:

- `equivalence` context owns `CodeFacts` port (new), `AbstractionLevel` enum (new), cohesion violations (new). These are Published Language additions.
- `reading` context conforms to `CodeFacts` (source-walking adapter).
- cfdb-query adapter relationship is Conformist to cfdb's fact schema (reads cfdb's keyspace without translation).

The `equivalence → cfdb` relationship is Named "vocabulary alignment" but is actually a **Published Language** consumption: graph-specs reads cfdb's `:IN_MODULE`/`:IN_CRATE`/`:Context` labels directly. That's fine — but the RFC should name the pattern explicitly so future RFCs know the relationship type. Currently unnamed.

---

## 3. Cross-context type analysis

`ConceptNode` already crosses the `equivalence`/`reading` boundary as Published Language (RFC-001 ratified this). RFC-010 proposes adding `module`, `crate`, and `context` provenance fields to `ConceptNode`. These fields name cfdb-layer concepts (`module`, `crate`) — which are infrastructure-level identifiers. The clean-arch lens (§5.1 question) asks whether this is a domain abstraction concern or a build-system leak; from a DDD perspective, `module` and `crate` ARE domain concepts in graph-specs' equivalence context (the domain IS about structural co-location). So the cross-context type pollution risk is manageable, but the naming overlap with cfdb's `Module`/`Crate` labels should be documented as a deliberate alignment decision.

---

## 4. Aggregate boundary analysis

RFC-010 proposes Context ⊃ Concept ⊃ Member as an aggregate, with "all H2s under one H1 co-locate in one module" as the aggregate-consistency invariant.

**The co-location claim in agentry holds for the majority but breaks in 6 of 50 files** (verified empirically): `boundary_signaling.md`, `fsm_merge_rail.md`, `git_operator.md`, `refusal.md`, `secret_resolver.md`, `secrets.md` have an H1 (context) but zero H2s. The RFC's aggregate invariant — "H2s under one H1 co-locate in one module" — silently vacuously succeeds for these files (no H2s = no scattered concepts to detect). That is not a violation, but it means the cohesion check cannot fire on files that use H4-as-concept, potentially masking real scatter.

For `captain_cli.md`, H3s (`### UnsatisfiedRolePrecondition`) are nested pub types under H2 concepts — these are L2 concepts at H3 depth, not L3 members. The aggregate model misclassifies them as Members, which means the cohesion check for them is silent (Members are emitted not diffed). This is a concrete soundness gap.

---

## 5. Language validation (ubiquitous language)

**"Context" as H1-per-file:** Evans defines a bounded context as a linguistic boundary, not a file boundary. In agentry, each of 38–50 files claims a distinct bounded context — that is a very large number. Some of these (`shared_kernel.md`, `boundary_signaling.md`) are plausibly bounded contexts; others look like modules within a context. The inline H1 form makes this ambiguity undetectable by the tool since it accepts any H1 as a context declaration. RFC-001's `specs/contexts/` form forced explicit Owns/Exports/Imports, making context boundaries machine-checkable; the inline form removes that pressure. From a language standpoint, allowing any H1 to BE a context weakens the DDD discipline RFC-001 introduced.

**"Member" vs "ConceptMember":** In Evans, a "Member" is not standard DDD vocabulary. The standard is `Entity`, `Value Object`, `Aggregate`, `Domain Service`, `Domain Event`. The RFC introduces `Member` as "method/field of a concept." This is acceptable shorthand for this tool's domain (not a DDD implementation guide), but should be documented clearly to avoid confusion with DDD entity membership.

---

## RC list

**RC-1 [BLOCKING] — Resolve the self-dogfood context-name collision in `specs/concepts/core.md`.**

`core.md:1` has H1 `"Core concepts"` (a title), not the context name `"equivalence"`. Under RFC-010's H1-as-context rule, this produces context `"Core concepts"` which conflicts with the existing `specs/contexts/equivalence.md` context `"equivalence"`. The names are different strings; Invariant 1's "contexts/ wins" rule requires matching names to apply. The RFC must either: (a) rename `core.md`'s H1 to `"equivalence"` to make it conform, (b) clarify the matching rule works on ownership derivation not name equality, or (c) admit that `core.md` requires the no-H1-context legacy path. This is a self-hosting failure — a tool that cannot dogfood its own pattern cannot claim the pattern is sound.

Evidence: `/var/mnt/workspaces/graph-specs-rust/specs/concepts/core.md:1` (`# Core concepts`) vs `/var/mnt/workspaces/graph-specs-rust/specs/contexts/equivalence.md:1` (`# equivalence`).

**RC-2 [BLOCKING] — Fix Invariant 2's vocabulary alignment claim.**

Invariant 2 states "Ladder levels reuse cfdb's vocabulary — `Context`/`Module`/concept provenance names… match cfdb's `Context`/`Module`/`Item`." The claim mixes the correct alignment (`:Context`, `:Module`, `:Item`) with an incorrect one (`:Concept`). In cfdb, `:Concept` is an enrichment-layer overlay node, NOT the code-item analog of graph-specs' `ConceptNode`. The two are semantically distinct. Invariant 2 must be corrected to say "`ConceptNode` maps to cfdb's `:Item` (not `:Concept`)" to prevent future implementers from conflating them in the cfdb-query adapter.

Evidence: `/var/mnt/workspaces/cfdb/crates/cfdb-core/src/schema/describe/nodes.rs:306–328` (`:Concept` is an overlay label), `/var/mnt/workspaces/cfdb/crates/cfdb-core/src/schema/labels.rs:21` (`:Item` is the structural code-item label).

**RC-3 [BLOCKING] — Address the tripartite "context" homonym for repos with `.cfdb/concepts/*.toml` but no `specs/contexts/`.**

OQ-1 defers (C) (cfdb's `.cfdb/concepts/*.toml` as resolver) but does not specify the precedence rule when a repo has `.cfdb/concepts/*.toml` in use but no `specs/contexts/`. If agentry later adopts cfdb and produces `.cfdb/concepts/*.toml` declaring contexts with names that differ from its H1s, the three-way conflict becomes real and silent. RFC-010 must state the precedence: `specs/contexts/` (A) > `concepts/*.md` H1 (B) > `.cfdb/concepts/*.toml` (C), with the A>B application requiring name equality, not file-scope matching. Without this, Invariant 1 is underdetermined.

**RC-4 [advisory] — Correct the `AbstractionLevel::Member` ontology for H3-as-sub-concept.**

RFC-010 §3.1 collapses H3 and H4 into `Member ("method/field of a concept")`. In agentry, H3 is used for **sub-concepts** (nested pub types), not methods: `captain_cli.md:123` (`### UnsatisfiedRolePrecondition` is a `pub enum`, a value object under `## DispatchPreflight`). If H3 becomes `Member` (emitted-not-diffed), these sub-types are invisible to the cohesion check. The RFC should either: (a) introduce `AbstractionLevel::SubConcept` for H3 (a nested pub type, diffed at L2) and reserve H4 for `Member`, or (b) document the rationale for collapsing H3 into Member and acknowledge the agentry sub-concept H3 pattern is out of scope for this RFC. Advisory because `Member` is emitted-not-diffed in this RFC, so no immediate check breakage occurs, but the ontology is wrong and will compound in the L3 RFC.

Evidence: `/var/mnt/workspaces/agentry/specs/concepts/captain_cli.md:123` (H3 is `UnsatisfiedRolePrecondition`, a pub enum sub-type), `/var/mnt/workspaces/agentry/specs/concepts/secret_resolver.md:104` (H4 used as L2-equivalent concept type).

**RC-5 [advisory] — Name the cfdb relationship pattern explicitly.**

RFC-010 §3.3 says graph-specs "may query a cfdb keyspace" but does not name the context-mapping pattern. The cfdb-query adapter reads cfdb's Published Language (`:IN_MODULE`, `:IN_CRATE` edge labels) and emits provenance into graph-specs' `ConceptNode`. This is a **Conformist** relationship: graph-specs adopts cfdb's keyspace vocabulary without a translation layer. Naming it Conformist in §3.3 and in Invariant 3 would close the context map (required by Evans and by this repo's own RFC-001 §3.1 discipline) and make it clear that if cfdb's edge labels change, graph-specs' cfdb-query adapter changes with them.

**Cross-lens flags for other council members:**

- Clean-arch lens: RC-1 (self-dogfood H1 collision) has clean-arch implications — the context resolver runs inside the diff engine and would produce `"Core concepts"` as a context name for graph-specs' own domain types. Ask: where does the resolver live if it must handle the A=B name-mismatch case?
- SOLID lens: The `AbstractionLevel` enum collapsing H3 and H4 into one variant (RC-4) is an OCP concern — adding H3-as-sub-concept later will require touching the diff engine in ways that `#[non_exhaustive]` does not protect against, since Member semantics (emitted-not-diffed) differ from Concept semantics (diffed).
- Rust systems lens: The Invariant 2 vocabulary misalignment (RC-2) has a concrete serialisation impact — if a cfdb-query adapter emits cfdb `:Concept` node provenance (which is a TOML-named business concept, not a code item), the `ConceptNode` constructor will receive wrong data.
