# RFC-010 Round-2 Rust Systems — Feasibility Bounds for the Converged Model

**For:** ddd synthesis  
**Scope:** Three deliverables A / B / C (operator charge). Feasibility-anchored; no re-review.

---

## A. cfdb-query adapter feasibility

### What the dep chain actually is

cfdb workspace `Cargo.toml` root declares `publish = false`. This means no crates.io path
is available. The only valid dep types are:

1. **Path dep** — requires graph-specs and cfdb to share a file-system root (monorepo or
   sibling checkouts at a known relative path). This is already the case in this dev
   environment (`/var/mnt/workspaces/graph-specs-rust` and `/var/mnt/workspaces/cfdb`
   are siblings), and CI presumably mounts both. Path deps are the idiomatic choice here.

2. **Git dep** — valid but adds `Cargo.lock` churn and makes the rev pinning a manual
   obligation. Harder to keep in sync with the lockstep bump protocol (cfdb CLAUDE.md §3).

**Verdict: path dep is the correct dep type.** The repos are already siblings; cfdb is
already consumed as a sibling by graph-specs' CI cross-dogfood step.

### Exact crates that `adapters/cfdb-query/Cargo.toml` must link

```toml
[dependencies]
domain     = { path = "../../domain" }
ports      = { path = "../../ports" }
# cfdb side — path dep pointing at sibling checkout
cfdb-core     = { path = "../../../cfdb/crates/cfdb-core" }
cfdb-petgraph = { path = "../../../cfdb/crates/cfdb-petgraph" }
cfdb-query    = { path = "../../../cfdb/crates/cfdb-query" }
cfdb-concepts = { path = "../../../cfdb/crates/cfdb-concepts" }
```

`cfdb-concepts` is an unconditional dep of `cfdb-petgraph` (confirmed in its Cargo.toml).
It cannot be dropped. The minimum dep count is 4 cfdb crates.

Transitive additions to the graph-specs workspace dep graph:
- `petgraph` (new)
- `chumsky` (new — the Cypher parser in cfdb-query; macro-heavy, ~15-30s cold compile)
- `regex` (new — used by cfdb-petgraph evaluator)
- `indexmap` (new)

`serde`, `serde_json`, `thiserror`, `toml` are already in the graph-specs workspace.

### How the keyspace is loaded at check-time

The keyspace is a **JSON file on disk** at `<workspace>/.cfdb/db/<keyspace-name>.json`.
The adapter loads it via `cfdb_petgraph::persist::load(&mut store, &ks, &path)` which
deserializes into an in-memory `PetgraphStore` (a `petgraph::StableDiGraph` in RAM).
There is no server, no socket, no embedded database engine.

**Precondition:** the JSON file must exist before `graph-specs check` runs the cfdb-query
adapter. In CI this is already satisfied (the `cfdb-check` job runs `cfdb extract` first).
For local developer invocations the composition root must fall back to the source-walking
adapter when the file is absent — or document the precondition clearly in error output.

### Is the cfdb-query adapter MVP or follow-up?

**Follow-up slice (R10-6).** The path-dep arrangement is straightforward, but:

1. The 4-crate dep chain adds petgraph + chumsky to the graph-specs compile graph. This
   is not a blocker but it is real cold-compile cost the workspace doesn't pay today.
2. The adapter crate does not exist yet. It is a new crate requiring a new `Cargo.toml`,
   workspace `members` entry, and optional dep wiring in `application/`.
3. R10-3 (the source-walking adapter + cohesion rule) can ship and prove the rule is
   correct. R10-6 then adds the cfdb-query path as a second, interchangeable adapter.

This is consistent with what the RFC already says. The feasibility verdict is:
**cfdb-query adapter in R10-6 is feasible via path dep; it is correctly sequenced as a
post-R10-3 slice; it requires a new crate, not just new files.**

The composition root in `application/src/lib.rs` should gate it behind a Cargo feature
(e.g. `features = ["cfdb-query-adapter"]`) so the source-walking adapter remains the
default for repos without a keyspace.

---

## B. "module" granularity — converged derivation for both adapters

### What cfdb emits for module_qpath

`cfdb_core::qname::module_qpath(module_stack)` is `module_stack.join("::")`.

The stack is initialized with `vec![crate_name.replace('-', "_")]` at the crate root
(`file_walker.rs:52`). Nested `mod foo` blocks push/pop the stack.

Results:
- Item at crate root (`domain/src/lib.rs`): `module_qpath = "domain"` (just the crate name)
- Item in `domain/src/diff/context.rs` (reached via `mod diff` then `mod context`):
  `module_qpath = "domain::diff::context"`
- The `IN_MODULE` edge is NOT emitted for crate-root items (`file_walker.rs:118` guard:
  `module_stack.len() > 1`). Only `IN_CRATE` exists for those items.

### What the source-walking adapter must produce

For parity, the source-walking adapter in graph-specs must derive `module_qpath` by the
same rule: `file_path` relative to the crate's `src/` root, extension stripped,
path separators replaced with `::`. Edge cases:

| File | Expected module_qpath |
|---|---|
| `domain/src/lib.rs` | `"domain"` (crate root — no sub-module) |
| `domain/src/diff.rs` | `"domain::diff"` |
| `domain/src/diff/context.rs` | `"domain::diff::context"` |
| `adapters/rust/src/lib.rs` | `"adapter_rust"` (crate name, hyphen→underscore) |

**The existing `find_owned_unit` at `adapters/rust/src/lib.rs:162` gives the crate
directory only** (e.g. `"domain"` or `"adapters/rust"`). It does NOT produce the module
qpath for files below `src/`. A new helper is needed in the source-walking adapter:

```rust
fn module_qpath_from_file(file_path: &Path, crate_root: &Path, crate_name: &str) -> String {
    let src_root = crate_root.join("src");
    let rel = match file_path.strip_prefix(&src_root) {
        Ok(r) => r,
        Err(_) => return crate_name.replace('-', "_"),
    };
    // lib.rs or main.rs at crate root → just crate name
    let stem = rel.with_extension("");
    let stem_str = stem.to_string_lossy();
    if stem_str == "lib" || stem_str == "main" {
        return crate_name.replace('-', "_");
    }
    let segments = stem_str.replace(std::path::MAIN_SEPARATOR, "::");
    format!("{}::{}", crate_name.replace('-', "_"), segments)
}
```

This derivation is not in the codebase. It is ~10 lines. Not blocking — R10-3 implements
it when adding `module` provenance to the source-walking adapter.

### Cohesion unit granularity decision

The RFC proposes **module-granular cohesion** (option A: all H2 concepts under one H1
should co-locate in one module). This is directly compatible with both adapters:

- Source-walking: `module_qpath_from_file` above
- cfdb-query: read `n.props["module_qpath"]` from the `:Item` node

**The council should converge on full module_qpath (not file path, not crate dir) as the
cohesion unit.** Reasons:
1. cfdb already stores `module_qpath` on every `:Item` — it is a first-class fact.
2. File path and module qpath are isomorphic for normal `src/` layouts (no `#[path]`
   magic), but module qpath is the semantic concept.
3. The existing `owning_unit_str` in `domain/src/diff/context.rs:204–213` extracts the
   crate dir from the Code source path (`path.split_once("/src/").map(|(unit, _)| unit)`).
   For the cohesion rule, this must be extended or replaced with the full module qpath.
   `owning_unit_str` is an internal function — changing it is a domain-internal refactor.

---

## C. Migration cost (precise counts)

### C1. ConceptNode field-add — 12 sites, Option<String>, zero application/ churn

Fields to add: `module: Option<String>`, `crate_name: Option<String>`.

Construction sites requiring `module: None, crate_name: None` fill-in (verified by grep,
no application/ sites):

| File | Function |
|---|---|
| `adapters/markdown/src/lib.rs:424` | `flush_pending` |
| `adapters/rust/src/lib.rs:260` | `emit` |
| `adapters/rust/src/edges.rs:282` | test helper `nodes()` |
| `domain/src/diff/context_tests.rs:11` | `code_node()` helper |
| `domain/src/diff/context_tests.rs:92` | inline |
| `domain/src/diff/tests.rs:26` | `spec()` |
| `domain/src/diff/tests.rs:36` | `code()` |
| `domain/src/diff/tests.rs:46` | `spec_with_sig()` |
| `domain/src/diff/tests.rs:56` | `code_with_sig()` |
| `domain/src/diff/tests.rs:66` | `spec_unparseable()` |
| `domain/src/diff/verb.rs:186` | `make_code_node()` |
| `domain/src/report.rs:358` | inline |

**application/src/ has zero ConceptNode construction sites.** Confirmed.

### C2. SchemaVersion V3 — mechanical, 15+ test assertion updates

Steps:
1. Add `V3` variant to `SchemaVersion` enum (`domain/src/lib.rs:41`)
2. Add `Self::V3 => "3"` arm to `const fn as_str`
3. Change `CURRENT` to `Self::V3`
4. Update every test asserting `r["schema_version"] == "2"` — there are 15+ in
   `application/src/ndjson.rs` alone. All become `"3"`.

The `const fn as_str` match is straightforward because `V3` is a unit variant — no
`String` fields, no const-stability concerns.

### C3. `violation_key` exhaustive arms — must ship in R10-1, not R10-4

The function at `domain/src/diff.rs:120` is `const fn`. It must be exhaustive or the
crate does not compile. The RFC's deferral of "exhaustive ranks" to R10-4 is a compile
blocker: as soon as R10-1 adds the new Violation variants to the enum, `violation_key`
fails to compile.

**Recommended shape for the four cohesion variants:** wrap them in a
`CohesionViolation(CohesionViolation)` inner enum, mirroring the existing
`Violation::Context(ContextViolation)` pattern. Then `violation_key` gains one arm:

```rust
Violation::CohesionViolation(cv) => (cv.concept(), 12),
```

`CohesionViolation` needs `fn concept(&self) -> &str` returning the H1 context name
(present on all four variants). This is the minimum change that keeps `violation_key`
under the `const fn` constraints and preserves the outer `Violation` enum's stability.

**Rank 12** for the cohesion wrapper is clean: it slots after the verb variants
(ranks 9–11) and leaves room for future additions.

If the synthesizer prefers four flat arms instead of a wrapper:

```rust
Violation::ContextOwnsScatteredConcepts { context, .. } => (context.as_str(), 12),
Violation::ConceptMisfiled { context, .. } => (context.as_str(), 13),
Violation::ContextWithoutCohesionUnit { context, .. } => (context.as_str(), 14),
Violation::ModuleSplitAcrossContexts { module, .. } => (module.as_str(), 15),
```

Both compile. The wrapped form is architecturally consistent with the existing pattern.
Council decides — but R10-1 must commit to one shape and define the ranks.

---

## Feasibility verdict for the converged model

The converged model is feasible if:

1. R10-1 picks the cohesion variant wrapping shape and defines rank values in the RFC
   before any implementation begins.
2. `ConceptNode` gains `module: Option<String>` and `crate_name: Option<String>` in
   R10-1, with all 12 construction sites updated.
3. The source-walking adapter in R10-3 implements `module_qpath_from_file` (the ~10-line
   derivation above) to produce module qpaths compatible with cfdb's convention.
4. R10-6 (cfdb-query adapter) ships as a new `adapters/cfdb-query` crate with path deps
   to the 4 cfdb crates, gated behind a Cargo feature in `application/`.

Nothing in the converged model is infeasible. No blocking orphan-rule violations, no
object-safety traps in the proposed `CodeFacts` trait, no const-fn landmines that can't
be resolved by the wrapping shape decision.

The dry-run will be surprised by exactly one thing if the synthesis does not address it:
the R10-3 source-walking adapter produces module qpaths from file paths, but the existing
`owning_unit_str` function (used by the v0.4 context pass) only extracts crate dirs.
The cohesion rule needs the full module qpath, not the crate dir. If R10-3 reuses
`owning_unit_str` unchanged, the cohesion check will be crate-granular rather than
module-granular — semantically weaker than the RFC intends and inconsistent with cfdb.


---

## D. Cross-fertilization — cfdb-side consequences

These are changes cfdb must make, not just changes graph-specs makes consuming cfdb.
Each is a **paired cfdb RFC or issue** implied by the converged model.

---

### D1. cfdb needs a `ContextSource::SpecHeading` variant [paired cfdb RFC]

**Current state:** `cfdb_core::ContextSource` has exactly two variants: `Declared`
(`.cfdb/concepts/*.toml`) and `Heuristic` (auto-derived by prefix stripping).
`context_source.rs:27` — both are about code-side context resolution.

**What RFC-010 implies:** graph-specs will now be a *third* authoritative source for
context declarations — the H1 heading in `specs/concepts/*.md` declares a bounded context
by name. When the cfdb-query adapter or any downstream tooling asks "where does this
context come from?", the honest answer for a context that was discovered via a graph-specs
spec heading is neither `declared` (a TOML file) nor `heuristic` (prefix stripping) — it
is something like `spec_heading` or `spec_declared`.

**Why this matters:** cfdb's `:Context` nodes carry a `source` prop whose wire value is
`ContextSource::as_wire_str()`. The `#[non_exhaustive]` on `ContextSource` means adding a
variant is a cfdb-side RFC. If graph-specs starts creating `:Context` nodes via a future
deeper integration (or if a shared toolchain like a `spec-to-cfdb` bridge is ever built),
those nodes will either be mis-classified as `heuristic` (wrong) or require a new variant.

**Flag for synthesis:** this is not a blocker for R10-6 (the cfdb-query adapter only
READS; it does not WRITE to the keyspace). But it is a forward-planning flag — if the
integration ever deepens to graph-specs annotating a keyspace with spec-heading-derived
context facts, cfdb needs this variant. File a cfdb issue now, before the integration
ships, so the schema is ready.

---

### D2. cfdb `BELONGS_TO` is crate-granular; RFC-010 cohesion is module-granular

**Current state:** cfdb's `BELONGS_TO` edge runs from `:Crate` to `:Context`
(descriptor at `cfdb-core/src/schema/describe/edges.rs:95–103`). Context resolution is
**crate-granular** in cfdb — one crate belongs to one context. The `bounded_context` prop
on `:Item` is derived from its owning crate's TOML entry.

**What RFC-010 implies:** the cohesion rule checks that all H2 concepts under one H1
co-locate in one **module** (`module_qpath`). A module is finer-grained than a crate. The
`:Item.module_qpath` prop is already emitted by cfdb-extractor, so querying for module
cohesion is possible today. But cfdb has no `(:Module)-[:BELONGS_TO]->(:Context)` edge —
the `BELONGS_TO` edge descriptor only allows `:Crate` as the source.

**The gap:** if the cfdb-query adapter wants to assert "does this module map to this
context?", it must infer module-to-context by resolving item-to-context (via
`:Item.bounded_context`) and grouping by `module_qpath`. That works, but it is an indirect
derivation. A direct `(:Module)-[:BELONGS_TO]->(:Context)` edge does not exist.

**For R10-6:** no blocker. The adapter can derive module cohesion from `:Item.module_qpath`
+ `:Item.bounded_context` without needing a new edge. The query is:

```cypher
MATCH (i:Item)-[:IN_MODULE]->(m:Module)
WHERE i.visibility = 'pub'
RETURN m.qpath, i.bounded_context, collect(i.name)
```

**Paired cfdb RFC (forward-planning):** if module-to-context becomes a first-class fact
in cfdb (e.g. to support cfdb-native cohesion violation detection as a `.cypher` ban
rule), cfdb would need to add `(:Module)-[:BELONGS_TO]->(:Context)` to the schema and
emit it from the extractor. This is a cfdb schema extension requiring a `SchemaVersion`
patch bump and lockstep graph-specs cross-fixture PR.

---

### D3. cfdb `:Item.visibility` filter — graph-specs needs `pub`-only items, cfdb emits all visibilities

**Current state:** cfdb-extractor emits `:Item` nodes for ALL Rust items encountered
regardless of visibility, tagging each with `visibility = "pub" | "pub(crate)" | ...`
(`emit/mod.rs:276–278`). There is no cfdb-side pre-filter to pub-only.

**What RFC-010 implies:** the cfdb-query adapter must filter `WHERE i.visibility = 'pub'`
in every query that replicates what the source-walking adapter does (which only calls
`emit()` on `Visibility::Public(_)` items at `adapters/rust/src/lib.rs:253`).

**This is not a cfdb-side change** — the filter goes in the adapter's query, not in cfdb.
But it must be explicit in R10-6's implementation spec. If the cfdb-query adapter forgets
the visibility filter, the cohesion check will fire on non-pub types that graph-specs'
source-walking adapter ignores, producing false positives.

**Also note:** cfdb emits `"type_alias"` (not `"type"`) for `pub type Foo = Bar`
(`visits.rs:304`). The source-walking adapter in graph-specs emits these as
`Item::Type` (`adapters/rust/src/lib.rs:237`). The cfdb-query adapter must include
`kind IN ['struct', 'enum', 'trait', 'type_alias']` in its WHERE clause to match
graph-specs' concept scope. Omitting `type_alias` would silently miss `pub type`
declarations. This is a spec gap between graph-specs' `Item::Type` arm and cfdb's
`"type_alias"` string — not a cfdb-side change, but a vocabulary alignment note the
R10-6 issue must carry explicitly.

---

### D4. cfdb `ContextSource` has no `SpecFile` variant — implications for context identity deduplication

**Current state:** cfdb resolves context identity via `.cfdb/concepts/*.toml` overrides
(crate-granular, `Declared`) or prefix heuristic (`Heuristic`). graph-specs resolves
context identity via H1 heading text in `specs/concepts/*.md` (file-granular, no TOML
needed). These are two independent resolution paths that may produce the same context
name string but carry different provenance.

**RFC-010 Invariant 1** says "one owning context per concept; if both `specs/contexts/`
and `specs/concepts/` H1 name it, they denote the same context." But cfdb's keyspace may
have a `:Context` node named `"reading"` from TOML, while graph-specs has a context named
`"reading"` from the H1 of `specs/concepts/reading.md`. Today they are two independent
facts with no machine-checkable link.

**The deduplication gap:** if graph-specs' cfdb-query adapter asks "does this `:Context`
node in the keyspace correspond to the H1 context I'm checking?", it must match by name
string alone. This works as long as names are canonical, but it means the two tools can
drift (cfdb has `"Reading"` because of prefix heuristic capitalisation; graph-specs has
`"reading"` from the H1 heading) and the parity test would silently pass with empty
cohesion results rather than failing loudly.

**Paired cfdb RFC (forward-planning):** adding `ContextSource::SpecFile` and a
`:Context.spec_path` prop so a cfdb keyspace can record "this context was validated
against graph-specs spec file X" would make the deduplication machine-checkable. Not
needed for R10-6 (which reads, not writes), but worth flagging as the natural next
integration step after R10-6 proves the query path.

