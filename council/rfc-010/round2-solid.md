# SOLID + Component Principles — Round 2 Deliberation

**Round:** 2 — convergence  
**Date:** 2026-06-05  
**Input:** clean-arch.md, ddd.md, rust-systems.md (round 1); operator corrections  

---

## Operator corrections accepted

1. agentry is not an oracle — the council prescribes the correct model; agentry migrates.
2. "cleaning boundaries is the council's job" — prescriptions below are normative, not descriptive.

---

## A. SRP separation prescription (RC-1 resolution)

### Problem statement

The markdown reader at 647 lines carries six distinct pass-types sharing one `SectionState`
struct and one `handle_event` dispatcher. Complexity ceiling evidence:

- `extract_annotations_from_source` (line 473) already scores 16 — above the `<15` budget
- `extract_verb_anchors` (line 112), `extract` (line 42), `extract_invariant_annotations` 
  (line 174) each score 11 — individually acceptable but structurally duplicated walk loops

RFC-010 adds H1-context tracking and H4-member attachment. If woven into `handle_event` and
`SectionState`, the existing 9-scored `handle_event` will breach 15. The `SectionState`
struct gains `current_h1_context: Option<String>` and `h4_member_pending: Option<(String, usize)>` 
fields — at which point the struct models three simultaneous state machines (concept, verb, 
context-tree) that happen to parse the same byte stream.

### Prescribed separation

**Two distinct parse passes per file, each with its own state type:**

**Pass 1 — existing `SectionState` + `handle_event` (unchanged scope):**  
Responsible for: H2/H3 → `ConceptNode`, bullet edges, verb anchors.  
Does NOT see H1 or H4. The `handle_event` match keeps its current arms. No new fields on
`SectionState`. Complexity stays ≤ 15.

**Pass 2 — new `ContextTreeState` + `extract_context_tree_from_source`:**  
Responsible for: H1 → context name + context metadata; H2/H3 attachment to preceding H1
(parent-link assignment); H4 → `Member` emission (optional, emitted-not-diffed per §2).  
This is a fresh `Parser::new(source).into_offset_iter()` per file, following the exact
precedent established at `adapters/markdown/src/lib.rs:474–475` for
`extract_annotations_from_source`.

```rust
struct ContextTreeState<'a> {
    path: &'a Path,
    /// Name of the most recent H1 seen; None before the first H1.
    current_context: Option<String>,
    /// Parent links accumulated for this file: concept_name → context_name.
    parent_links: Vec<(String, String)>,
    /// Members accumulated for this file (H4, emitted not diffed).
    members: Vec<MemberNode>,
    heading_buf: String,
    in_heading_at: Option<usize>,
    current_level: Option<HeadingLevel>,
}
```

`extract_context_tree_from_source` handles only `Event::Start(Tag::Heading{...})` and
`Event::End(TagEnd::Heading(...))` and `Event::Text`. It ignores bullets, code blocks, and
everything else — single-concern, trivially below complexity 15.

**Integration in `MarkdownReader::extract`:**  
Both passes run per file. Pass 1 produces `Vec<ConceptNode>` (unchanged). Pass 2 produces
`Vec<(concept_name, context_name)>` parent links. After both passes, the parent links are
applied as a post-processing step to attach context provenance to the already-built
`ConceptNode`s:

```rust
// Post-processing: attach context provenance from Pass 2 to ConceptNode
// provenance field (whatever clean-arch prescribes the field be called).
let concept_contexts: HashMap<String, String> = parent_links.into_iter().collect();
for node in &mut nodes {
    if let Some(ctx) = concept_contexts.get(&node.name) {
        node.context_unit = Some(ctx.clone());
    }
}
```

This keeps `handle_event` untouched and the context-tree concern entirely isolated in
`ContextTreeState`. The two parsers share no mutable state. Each pass is independently
testable. Complexity budget is preserved.

**R10-2 prescription update:** Issue R10-2 must specify: "Implement `ContextTreeState` and
`extract_context_tree_from_source` as a separate parser pass. Do not modify `SectionState`
or `handle_event`. Use a fresh `Parser::new(source)` per file per the RFC-005 §3.2
precedent. Verify with `ra-query` that both `handle_event` and
`extract_context_tree_from_source` score ≤ 14 post-implementation."

---

## B. Violation taxonomy (RC-2 resolution, converged with rust-systems)

### Convergence with rust-systems RC-1

rust-systems RC-1 correctly identifies that wrapping four new variants inside a
`Violation::CohesionViolation(CohesionViolation)` inner enum (mirroring the existing
`Violation::Context(ContextViolation)` at `domain/src/lib.rs:293`) yields exactly one new
arm in `violation_key`, preserving `const fn` compatibility and keeping the ndjson emitter's
match below `too_many_lines`.

### Convergence with ddd: the correct variant set

ddd's round-1 analysis (RC-4 advisory) reveals that the four RFC-proposed variants are based
on the agentry corpus as-found, not on the correct domain model. The operator correction
applies: agentry migrates, the model is prescriptive.

The correct cohesion violations, derived from the clean domain model:

1. **`ScatteredConcepts { context: String, modules: Vec<String>, ... }`** — H2 concepts
   under one H1 context span more than one code unit. (The headline check.)
2. **`MisfiledConcept { concept: String, declared_context: String, actual_unit: String, ... }`** — a
   concept's code unit differs from the unit its H1 context resolves to.
3. **`UnitlessContext { context: String, ... }`** — an H1 context whose concepts resolve to
   no code unit whatsoever. (A declared abstraction owning nothing.)
4. **`SplitUnit { unit: String, contexts: Vec<String>, ... }`** — one code unit's concepts
   are documented under multiple distinct H1 contexts. (The inverse of ScatteredConcepts.)

Note: the original RFC names (`ContextOwnsScatteredConcepts`, `ConceptMisfiled`,
`ContextWithoutCohesionUnit`, `ModuleSplitAcrossContexts`) encode the same four diagnostics —
they are not wrong in kind, only verbose. Rename to `ScatteredConcepts`, `MisfiledConcept`,
`UnitlessContext`, `SplitUnit` for consistency with the existing naming register
(`MissingInCode`, `MissingInSpecs`, `SignatureDrift` — no prefix, action-oriented).

### Prescribed structure

```rust
// domain/src/context.rs (alongside ContextViolation)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CohesionViolation {
    ScatteredConcepts {
        context: String,
        modules: Vec<String>,
        spec_source: Source,
    },
    MisfiledConcept {
        concept: String,
        declared_context: String,
        actual_unit: String,
        spec_source: Source,
        code_source: Source,
    },
    UnitlessContext {
        context: String,
        spec_source: Source,
    },
    SplitUnit {
        unit: String,
        contexts: Vec<String>,
        code_source: Source,
    },
}

impl CohesionViolation {
    pub const fn context_name(&self) -> &str {
        match self {
            Self::ScatteredConcepts { context, .. } => context.as_str(),
            Self::MisfiledConcept { declared_context, .. } => declared_context.as_str(),
            Self::UnitlessContext { context, .. } => context.as_str(),
            Self::SplitUnit { unit, .. } => unit.as_str(),
        }
    }
}
```

```rust
// Violation::Cohesion arm added to domain/src/lib.rs Violation enum
/// Intra-context cohesion violation (v0.5). Wraps CohesionViolation
/// variants so consumers that do not opt into cohesion checking match
/// one arm rather than four. Mirrors Violation::Context(ContextViolation).
Cohesion(CohesionViolation),
```

```rust
// violation_key gains exactly one new arm at rank 12:
Violation::Cohesion(cv) => (cv.context_name(), 12),
```

**R10-1 prescription update:** Must specify `CohesionViolation` as a new inner enum in
`domain/src/context.rs` alongside `ContextViolation`, with the four variants above and the
`context_name()` method. R10-4 adds the ndjson/text delegate (analogous to
`context_violation_to_record` at `application/src/ndjson.rs:134`).

---

## C. LSP of the two CodeFacts adapters (RC-3 resolution)

### Canonical "unit" granularity decision

The module-granularity mismatch I identified in round 1 (source-walk=file, cfdb=Rust-module-
path) is confirmed by rust-systems RC-6: "`find_owned_unit` returns the workspace-relative
crate directory" and "module path requires `file_path.strip_prefix(src_root)`."

The clean-arch lens RC-1 prescription converges: the field names `module` and `crate` on
`ConceptNode` should be language-agnostic. Adopting `container` and `unit` (per clean-arch
resolution option 1):

- `unit: Option<String>` — the crate/package/library granularity (what `find_owned_unit`
  already produces for Rust: `"adapters/rust"`, `"domain"`)
- `container: Option<String>` — the module/namespace/file granularity within the unit

**Prescribed canonical granularity for `container`:** **file-relative-to-src-root**, NOT
Rust-module-path.

Rationale: the cohesion check's invariant is "concepts under one H1 co-locate in one code
unit." Co-location in practice means the same file or the same small directory. Using the
full Rust module qpath (`domain::diff::context`) as the container adds discriminators that
make the check OVER-precise: two types in `domain/src/diff/context.rs` and
`domain/src/diff/context_tests.rs` belong to the same module `domain::diff::context` but
different files. The file-relative form (`diff/context`) is sufficient for co-location
detection and is derivable by both adapters without tracking inline `mod` stacks.

**Source-walking adapter derivation:**
```
container = file_path
    .strip_prefix(src_root)        // src_root = <crate>/src/
    .strip_suffix(".rs")
    .replace('/', "::")            // diff/context -> diff::context
```
This does NOT require tracking inline `mod` nesting. It derives the module path from the
file path — the conventional one-file-per-module rule (which graph-specs' RustReader
already assumes: "Inline mod contents are intentionally not recursed").

**cfdb-query adapter normalization:** cfdb stores the full module qpath including inline
mods. When reading from cfdb, the adapter must TRUNCATE the module qpath to file-granularity:
strip any inline-mod segments that are not file boundaries. Since cfdb's extractor at
`item_visitor/visits.rs:366` pushes `mod_name` for inline mods, the file-path form is
recoverable from the stored `IN_MODULE` edge by stripping the last segment(s) corresponding
to inline mods. Alternatively, cfdb also emits `IN_MODULE` from `File` nodes — the cfdb-
query adapter should traverse `(:Item)-[:IN_MODULE]->(:File)-[:IN_CRATE]->(:Crate)` rather
than `(:Item)-[:IN_MODULE]->(:Module)` to get file-granular containment directly. This
requires verifying cfdb's File node emission — check `/var/mnt/workspaces/cfdb/crates/cfdb-extractor/src/file_walker.rs:105-124` confirms `:File -[IN_MODULE]-> :Module` emission; whether `(:Item)-[:IN_FILE]` exists is for rust-systems to verify.

**LSP parity guarantee:** Both adapters must emit `container` using the file-relative
module-path convention. The parity test in R10-6 asserts that for every `ConceptNode` in
graph-specs' own tree, the source-walking adapter and the cfdb-query adapter produce
identical `unit` and `container` values.

**R10-3 prescription update:** Specify that `container` is derived from file path relative
to the nearest `src/` directory, NOT from the Rust inline-mod stack.  
**R10-6 prescription update:** Specify that the cfdb-query adapter traverses
`(:Item)-[:IN_MODULE]->(:File)` (file-granular) not `(:Item)-[:IN_MODULE]->(:Module)`
(deepest-mod-granular) to produce compatible `container` values.

---

## D. CRP resolution (RC-4)

### Decision: Cargo feature flag in `ports`, not a new crate

**Rationale for feature flag over new crate:**

A separate `ports-codefacts` crate creates ADP risk (new node in the DAG) and Cargo overhead
(new workspace member, new `[workspace.dependencies]` entry, version coupling). The feature
flag approach is simpler and achieves identical CRP isolation.

The clean-arch RC-2 establishes that `CodeFacts` lives in `ports/` — agreed. The CRP fix
is:

```toml
# ports/Cargo.toml
[features]
codefacts = []  # enables CodeFacts trait compilation

[dependencies]
# no new deps — CodeFacts uses only domain + std types already imported
```

```rust
// ports/src/lib.rs
#[cfg(feature = "codefacts")]
mod codefacts;
#[cfg(feature = "codefacts")]
pub use codefacts::CodeFacts;
```

Adapters that do NOT implement `CodeFacts` (MarkdownReader, RustReader in its non-provenance
role) do not enable this feature and do not compile against it.

The cfdb-query adapter (`adapters/cfdb-query/Cargo.toml`) declares:
```toml
ports = { workspace = true, features = ["codefacts"] }
```

`application/Cargo.toml` enables it only when wiring the cfdb-query adapter:
```toml
ports = { workspace = true, features = ["codefacts"] }  # when cfdb-query is enabled
```

**Additional CRP finding: cfdb-query adapter dependency chain is lighter than rust-systems estimated.**

rust-systems RC-3 identifies `cfdb-petgraph` + `cfdb-query` (with `chumsky`) as required.
However, the cfdb keyspace JSON format (`fact.rs:131-138`: `Node { id, label, props }` with
`Props = BTreeMap<String, PropValue>`) is directly `serde_json`-deserializable. The cfdb-
query adapter can read `IN_MODULE` and `IN_CRATE` edges by deserializing the keyspace JSON
directly with `serde_json` — without linking `petgraph` or `chumsky` at all.

Minimum viable dependency chain for `adapters/cfdb-query`:
- `cfdb-core` (node/edge/label types — `publish = false` at workspace level, so git dep)
- `domain` + `ports` (internal)
- `serde_json` (already in workspace)

This eliminates `petgraph` (~15s cold compile), `chumsky` (~20s cold compile), and
`cfdb-petgraph`/`cfdb-query` as hard deps. The adapter reads the JSON file directly, filters
for `IN_MODULE` and `IN_CRATE` labeled edges, and materializes `ConceptNode` provenance —
no Cypher query needed.

**Constraint on cfdb-core dep:** `cfdb/Cargo.toml:25` has `publish = false`. This means
`adapters/cfdb-query` must declare cfdb-core as a git dependency or a path dependency, not a
crates.io dep. The RFC (§3.8 / Invariant 3) states "links no cfdb crate directly" in the
context of `domain` and `ports` — but `adapters/cfdb-query` by definition links cfdb-core.
This is architecturally clean (adapter tier may link external libraries) but the RFC
invariant text needs correction to say "domain and ports link no cfdb crate; the
cfdb-query adapter in adapters/ is the single point of cfdb-core linkage."

**CRP result post-fix:**
- `MarkdownReader` links `ports` without `codefacts` feature: 2/4 = 50%
- `RustReader`/`RustBackend` links `ports` without `codefacts`: 3/4 = 75%
- `CfdbQueryAdapter` links `ports` with `codefacts` feature: 1/1 = 100% of the feature surface

CRP violation eliminated.

---

## Cross-council convergence positions

### Ratify from clean-arch

- RC-1 (field renaming `module`→`container`, `crate`→`unit`): RATIFY. Aligns with my LSP
  prescription above. `unit` matches `OwnedUnit` precedent at `domain/src/context.rs:17`.
- RC-2 (`CodeFacts` in `ports/`): RATIFY. My CRP prescription assumes this placement.
- A-1 (context resolver in `domain/src/context.rs`): RATIFY. The two-pass prescription
  above produces parent links that are resolved by a pure domain function, not by the adapter.

### Ratify from rust-systems

- RC-1 (wrap cohesion violations in `CohesionViolation` inner enum): RATIFY — prescribed above.
- RC-2 (`ConceptNode` field-add as `Option<String>` for backward compat): RATIFY. Both
  `container: Option<String>` and `unit: Option<String>` should be `Option` so the 12
  construction sites (enumerated by rust-systems) can set `None` as compile-safe migration.
- RC-5 (separate parser pass for H1/H4): RATIFY — prescribed in detail in section A above.
- RC-6 (module derivation gap): RATIFY — addressed in section C. The derivation is
  `file_path.strip_prefix(src_root).strip_suffix(".rs").replace('/', "::")`.
- RC-3 (cfdb dep chain): PARTIALLY CONTEST — the chumsky + petgraph dep chain is avoidable.
  The cfdb JSON keyspace can be read directly with `serde_json` + `cfdb-core` types only.
  The Cypher query layer (cfdb-query + cfdb-petgraph + chumsky) is NOT required for the
  simple `IN_MODULE`/`IN_CRATE` containment read. The keyspace JSON is
  `serde_json`-deserializable without petgraph or chumsky. This significantly reduces the
  dependency cost of R10-6.

### Positions on ddd findings

- ddd RC-1 (core.md H1 collision): RATIFY. The tool must dogfood its own pattern. The
  correct fix is option (a): rename `specs/concepts/core.md`'s H1 to `"equivalence"` so it
  conforms to `specs/contexts/equivalence.md`. This migration is the council's prescription,
  not agentry-deference.
- ddd RC-2 (Invariant 2 vocabulary misalignment — `:Concept` vs `:Item`): RATIFY. Add a
  note to the RFC: `ConceptNode` maps to cfdb's `:Item`, not `:Concept`. The cfdb-query
  adapter must query `(:Item {kind: "struct" | "enum" | "trait" | "type"})` nodes, not
  `(:Concept)` nodes.
- ddd RC-4 (H3 ontology): CONTEST the advisory status — this has SOLID implications. If H3
  is used for sub-concepts in agentry (and the operator confirms agentry migrates), then
  `AbstractionLevel::Member` for H3 is prescriptively wrong even if it is emitted-not-
  diffed. I recommend the RFC introduce `AbstractionLevel::SubConcept` for H3 and reserve
  `AbstractionLevel::Member` for H4. OCP: adding `SubConcept` later after shipping `Member`
  for H3 will require changing every H3 arm — that is not extension, it is modification.
  The `#[non_exhaustive]` tag protects downstream but not the diff engine itself.

---

## Summary of SOLID round-2 constraints for synthesis

| Item | Decision | Constraint for RFC |
|---|---|---|
| SRP / markdown reader | Two separate parser passes | `ContextTreeState` + `extract_context_tree_from_source` as fresh parser per file; `SectionState`/`handle_event` unchanged; ra-query verifies ≤14 post-impl |
| Violation taxonomy | `Violation::Cohesion(CohesionViolation)` | 4 variants: `ScatteredConcepts`, `MisfiledConcept`, `UnitlessContext`, `SplitUnit`; ranks 12 for `violation_key` |
| LSP parity | File-relative module path as canonical `container` | Both adapters derive from file path, not inline-mod stack; cfdb-query adapter traverses `(:Item)-[:IN_MODULE]->(:File)` |
| CRP | `ports` feature flag `"codefacts"` | `CodeFacts` gated; cfdb-query adapter links `cfdb-core` + `serde_json` only (no petgraph/chumsky) |
| H3 ontology | Prescribe `SubConcept` for H3 | Do not collapse H3 into `Member`; agentry migrates |
| core.md dogfood | Rename H1 to `"equivalence"` | Self-hosting must pass before RFC ratification |
