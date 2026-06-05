# RFC-010 Rust Systems Review

**Lens:** rust-systems
**Round:** 1
**Verdict:** REQUEST CHANGES
**Blocking RCs:** 3  Advisory RCs: 4

---

## RC-1 [BLOCKING] — `const fn violation_key` cannot gain new arms without ceasing to be `const fn`

**Location:** `domain/src/diff.rs:120`

The function signature is:

```rust
const fn violation_key(v: &Violation) -> (&str, u8) {
```

`const fn` in stable Rust (current toolchain: see `rust-toolchain.toml`) cannot match on
`String` fields via `.as_str()` — the existing arms already call `.as_str()` on `String`
fields inside `name`, `concept`, `qname`. This works today because `const fn` can call
`str::as_str()` on a `&String` (it is a const-stable operation). Adding four new Violation
variants that carry `String` context names and module names does NOT break const-fn
compilation per se. HOWEVER:

`Violation` is `#[non_exhaustive]`. A `const fn` matching a `#[non_exhaustive]` enum from
the same crate is fine — you are the defining crate. But any downstream crate matching
`Violation` in a `const fn` gets a hard error because `#[non_exhaustive]` prohibits
exhaustive matching outside the defining crate. This is not new — it exists today — but it
compounds with the RFC's expansion.

The actual blocker: the four proposed variants (`ContextOwnsScatteredConcepts`,
`ConceptMisfiled`, `ContextWithoutCohesionUnit`, `ModuleSplitAcrossContexts`) carry
multi-field payloads. The RFC proposes wrapping them under `Violation::Context(...)` already
(§3.5 says "all wrapped to keep the existing Violation taxonomy stable"). If they are
wrapped as `Violation::CohesionViolation(CohesionViolation)` mirroring the existing
`Violation::Context(ContextViolation)` pattern, then `violation_key` needs exactly one new
arm:

```rust
Violation::CohesionViolation(cv) => (cv.concept(), 12),
```

That is fine for `const fn`. BUT: the RFC says "four new variants" and §7 R10-4 says
"`violation_key` exhaustive ranks" — if the implementation puts them as four flat
`Violation` enum arms, then `violation_key` must match all four, each calling `.as_str()`
on a `String` field, which IS const-stable. So there is no compile blocker IF the match
arms only call `.as_str()`. But the RFC is ambiguous about whether these are flat arms or a
wrapped inner enum. **Resolve explicitly in R10-1.**

Rank assignment: the RFC lists existing ranks 0–11 (from reading the actual match in
`diff.rs:120–134`). The four new variants need ranks 12–15 (or 9–12 if interleaved with
verb variants). The RFC does not assign ranks — R10-4 defers this. This deferral is a
compile-time gap: `violation_key` will not compile until all match arms are present and the
new variants are known.

**Resolution:** R10-1 must specify whether the four cohesion variants are flat `Violation`
arms or wrapped in a new `CohesionViolation` inner enum (mirroring `ContextViolation`),
AND must define the rank values for `violation_key`. R10-4 cannot defer this — the function
must be exhaustive before any code in R10-3 or R10-4 compiles.

---

## RC-2 [BLOCKING] — `ConceptNode` field-add triggers struct-literal exhaustiveness failures at 19+ sites

**Migration scope (exhaustive grep across non-worktree source):**

Construction sites (`ConceptNode { ... }` struct literals):
- `adapters/markdown/src/lib.rs:424` — `flush_pending`
- `adapters/rust/src/lib.rs:260` — `emit`
- `adapters/rust/src/edges.rs:282` — test helper `nodes()`
- `domain/src/diff/context_tests.rs:11` — `code_node` helper
- `domain/src/diff/context_tests.rs:92` — inline construction
- `domain/src/diff/tests.rs:26` — `spec()`
- `domain/src/diff/tests.rs:36` — `code()`
- `domain/src/diff/tests.rs:46` — `spec_with_sig()`
- `domain/src/diff/tests.rs:56` — `code_with_sig()`
- `domain/src/diff/tests.rs:66` — `spec_unparseable()`
- `domain/src/diff/verb.rs:186` — `make_code_node()`
- `domain/src/report.rs:358` — inline construction

That is **12 construction sites** in production + test code. All will fail to compile if
`module` and `crate_name` (or whatever the field names become) are added as non-optional
fields without a default. `ConceptNode` does NOT derive `Default`
(`domain/src/lib.rs:100–105`) and is not `#[non_exhaustive]`.

The RFC's implied approach (add fields) is structurally sound — it is a field-add, not a
struct churn (no existing fields removed or renamed). But "field-add-only" still requires
updating every struct literal. 12 sites is mechanical and bounded, but it must happen in
R10-1 atomically or the entire tree fails to compile during the slice.

The NDJSON emitter (`application/src/ndjson.rs`) and the text formatter
(`application/src/text.rs`) do NOT construct `ConceptNode` — they only read it via
`Violation` fields. They will not break at struct-literal sites. However, the NDJSON
emitter does pattern-match `Violation` variants (`ndjson.rs:43`) and will need a new arm
for the four cohesion variants, landing in R10-4.

**Resolution:** R10-1 should add `module: Option<String>` and `crate_name: Option<String>`
(Option so legacy construction sites can pass `None` and the flat-check path remains
unbroken per Invariant 4/5). The 12 construction sites then set `module: None, crate_name:
None` as a compile-time-safe migration. R10-2 and R10-3 fill the fields.

---

## RC-3 [BLOCKING] — The `CodeFacts` trait as written is NOT object-safe and the cfdb-query adapter requires linking 3 crates graph-specs currently has no dependency on

**Part A — object safety**

The proposed trait:
```rust
pub trait CodeFacts {
    fn concepts(&self, root: &Path) -> Result<Vec<ConceptNode>, ReaderError>;
}
```

This signature is object-safe: no generic methods, no `Sized` bound, no associated types.
However, the RFC never says whether the composition root will store `Box<dyn CodeFacts>` or
`impl CodeFacts`. If the composition root in `application/src/lib.rs` uses dynamic
dispatch (needed to swap adapters at runtime without feature flags), the trait must remain
object-safe. The current proposed signature passes this check.

The risk is that implementors will want to add `where Self: Sized` methods or generic
parameters as the trait evolves. Recommend adding an explicit `#[cfg_attr(test,
mockall::automock)]`-compatible note or a doc comment asserting object-safe is a design
invariant so future methods don't break it. Advisory, not blocking, UNLESS the composition
root is designed to use `Box<dyn CodeFacts>`.

**Part B — cfdb-query adapter dependency chain [BLOCKING]**

This is the critical finding. The RFC claims the cfdb-query adapter "reads containment
straight from a cfdb keyspace." Here is what that actually requires after reading
`/var/mnt/workspaces/cfdb/`:

1. **cfdb's store is NOT a server or embedded database.** It is a **JSON file on disk**
   (`<workspace>/.cfdb/db/<keyspace>.json`) plus an in-memory petgraph (`PetgraphStore`).
   There is no socket, no HTTP API, no embedded RocksDB or SQLite. The "keyspace" is a
   JSON file loaded into a `petgraph::StableDiGraph` in RAM.

2. **To query a cfdb keyspace, graph-specs must link `cfdb-petgraph`.** The query execution
   path is: `cfdb_query::QueryBuilder` → `Query` AST → `cfdb_petgraph::PetgraphStore::execute()`.
   `cfdb-petgraph` depends on: `cfdb-core`, `cfdb-concepts`, `petgraph`, `serde`,
   `serde_json`, `thiserror`, `regex`, `indexmap`, `toml`. It optionally pulls `git2`
   (feature `git-enrich`) and `syn`+`sha2` (feature `quality-metrics`).

3. **Minimum dependency chain for the cfdb-query adapter:**
   - `cfdb-core` — the node/edge/query/store types (zero external deps)
   - `cfdb-query` — the `QueryBuilder` and `parse()` (depends on: `cfdb-core`, `serde`,
     `serde_json`, `thiserror`, `chumsky`)
   - `cfdb-petgraph` — the `StoreBackend` implementation (depends on: `cfdb-core`,
     `cfdb-concepts`, `petgraph`, `serde`, `serde_json`, `thiserror`, `regex`, `indexmap`,
     `toml`)
   - `cfdb-concepts` — the `.cfdb/concepts/*.toml` crate→context map resolver (the
     `enrich_bounded_context` path in `cfdb-petgraph` calls this; Cargo.toml confirms it
     as an unconditional dep)

4. **This is NOT a few-hundred-line adapter.** It is a **crate dependency addition** of
   four cfdb crates (cfdb-core, cfdb-query, cfdb-petgraph, cfdb-concepts) into
   `graph-specs`' adapter crate for cfdb. `petgraph` alone adds significant compile cost.
   `chumsky` (the Cypher parser in cfdb-query) is a substantial macro-heavy parser
   combinator library that adds 15-30s cold compile time.

5. **The keyspace must exist at check-time.** The cfdb-query adapter cannot run against a
   repo that has not already run `cfdb extract --workspace . --db .cfdb/db --keyspace
   <name>`. The JSON file must exist. The RFC notes "dual-control CI builds a keyspace
   anyway, so the fact is already paid for" — but this is only true in CI, not in local
   `graph-specs check` invocations. A developer running `graph-specs check --specs specs/
   --code .` locally without a prior `cfdb extract` will get a runtime error unless the
   composition root falls back to source-walking.

6. **No cfdb Rust library API is published.** `cfdb-petgraph` has `publish.workspace = true`
   in its `Cargo.toml`, but the workspace `publish` key in `/var/mnt/workspaces/cfdb/Cargo.toml`
   needs to be checked — if it is `false`, graph-specs cannot take a crates.io dep and must
   use a path dep or git dep. This creates a repo coupling that Invariant 3 says must not
   exist ("graph-specs depends on `CodeFacts`, never on cfdb concretely").

**The architectural contradiction:** Invariant 3 states "graph-specs links no cfdb crate
directly." But the cfdb-query adapter, if it lives in graph-specs' codebase, DOES link cfdb
crates directly — it just does so inside an adapter, not in `domain`. The RFC's framing
that "the port keeps it clean" is accurate for the dependency graph between `domain` and
cfdb, but the adapter crate WILL carry the transitive cfdb-petgraph dep into the adapter's
Cargo.toml. Whether this violates the spirit of Invariant 3 is a judgment call — it does
NOT violate it literally (domain never deps on cfdb; the adapter does, which is the
adapter's job) — but the RFC text is misleading.

**Resolution:** The RFC must be updated in §3.3 and §3.8 to state concretely:
(a) the cfdb-query adapter requires linking `cfdb-petgraph` + `cfdb-core` + `cfdb-query` +
`cfdb-concepts` as Cargo dependencies of a new `adapters/cfdb-query` crate;
(b) the keyspace JSON file must exist at check-time (not just at CI time);
(c) the dep chain cold compile cost (petgraph + chumsky) is the trade-off OQ-4 must price.
R10-6 should be scoped as a **new crate** (`adapters/cfdb-query`) with explicit Cargo.toml
deps listed, not as "a few hundred lines."

---

## RC-4 [advisory] — `SchemaVersion` enum needs a V3 variant; `const fn as_str` must be extended

**Location:** `domain/src/lib.rs:41–58`

Current `SchemaVersion` has V1 and V2 only. The RFC proposes `schema_version: "3"` for the
NDJSON output. This requires:
- A new `SchemaVersion::V3` variant
- `const fn as_str` gains a new arm (`Self::V3 => "3"`)
- `SchemaVersion::CURRENT` changes to `Self::V3`
- All existing tests asserting `r["schema_version"] == "2"` break and must be updated (15+
  test assertions in `application/src/ndjson.rs` alone)

The `as_str` match is `const fn` — same analysis as RC-1, but simpler. Since V3 carries no
`String` field (it's just an enum variant), `const fn` is straightforward here.

**Advisory** because this is mechanical and bounded — but the 15+ test assertion updates
in `ndjson.rs` will need to happen in R10-4 and the author should budget for them.

---

## RC-5 [advisory] — `handle_event` single-pass guarantee under heading-tree assembly

**Location:** `adapters/markdown/src/lib.rs:275–323`

The current `handle_event` matches `HeadingLevel::H2 | HeadingLevel::H3` at the
`Event::Start` arm (line 285–287) and dispatches on both in one match. The RFC proposes
adding H1 as a `Context` node and H4 as `Member`. The issue is:

`extract_annotations_from_source` (line 473) already runs a **separate fresh parser** for
H4 handling ("Per RFC-005 §3.2: fresh parser per file, NOT shared with the concept walk").
This pattern was deliberately chosen to keep `handle_event` under the complexity budget.
If H4 `Member` emission is added to `handle_event`, the complexity score will exceed 15:

Current complexity of `handle_event`: not directly measured, but `extract_annotations_from_source`
(which is the H4 path) already scores 16 (ra-query result above). Adding H4 to
`handle_event` alongside H1 context tracking will push `handle_event` above 15.

The RFC's §2 says "H4 → Member (emitted not diffed)." If H4 is added to `handle_event`
(sharing state with the H1/H2/H3 logic), `SectionState` gains additional fields
(`current_h1_context`, `pending_member_of`, etc.) and the complexity budget is breached.

**Resolution per the existing RFC-005 precedent:** add a separate `extract_tree_from_source`
function and a separate parser pass for H1 context tracking (same pattern as
`extract_annotations_from_source`). This keeps `handle_event` below complexity 15 by
keeping H4/H1 logic in separate passes. Verify with ra-query after implementation.

---

## RC-6 [advisory] — `find_owned_unit` infers `module` as the crate dir, not the Rust module path

**Location:** `adapters/rust/src/lib.rs:162–183`

`find_owned_unit` walks up to the nearest `Cargo.toml` and returns the workspace-relative
crate directory (e.g. `"adapters/rust"`). The RFC wants `module` to mean the Rust module
path (e.g. `"diff::context"` for `domain/src/diff/context.rs`). These are different
concepts:

- `find_owned_unit` → gives `crate` (workspace-relative dir to Cargo.toml)
- Module path → requires knowing the file's path relative to `src/`, stripping `.rs`, and
  translating to `::` separators

The RFC uses both "module" and "crate" as separate fields on `ConceptNode`. The
source-walking adapter needs to produce BOTH:
- `crate`: available from `find_owned_unit` (already implemented)
- `module`: requires `file_path.strip_prefix(src_root).to_string().replace('/', "::")`

This derivation is simple but is not currently in the codebase. The RFC does not note this
as a gap. For the cfdb-query adapter, `IN_CRATE` gives the crate and `IN_MODULE` gives the
module qpath (the cfdb extractor stores `current_module_qpath` on each `IN_MODULE` edge) —
so the two adapters will naturally produce different module representations (cfdb uses
`domain::diff::context`; the source-walker needs to derive the same from the file path).
The parity test in R10-6 ("two adapters must agree on graph-specs' own tree") will surface
any mismatch, but it should be anticipated now.

---

## RC-7 [advisory] — Workspace `Cargo.toml` migration scope

**Files that need `[dependencies]` changes when R10-6 ships:**

Current workspace crates: `domain`, `ports`, `adapters/markdown`, `adapters/rust`,
`application`. The cfdb-query adapter requires a new crate `adapters/cfdb-query` (or
equivalent). Minimum Cargo.toml changes:

1. `Cargo.toml` (workspace root) — add `adapters/cfdb-query` to `members`
2. `adapters/cfdb-query/Cargo.toml` (NEW) — deps: `cfdb-core`, `cfdb-petgraph`,
   `cfdb-query`, `cfdb-concepts`, `domain`, `ports`
3. `application/Cargo.toml` — add `adapters/cfdb-query` as an optional dep, gated behind a
   feature flag (e.g. `cfdb-query-adapter`)
4. `.gitea/workflows/ci.yml` — the `cfdb-check` job must pass the keyspace path; the
   cfdb-query adapter needs the keyspace to exist before graph-specs runs

That is 4 files, but the cfdb path dep vs git dep vs crates.io dep question must be
resolved before R10-6 can be scoped. If cfdb is a path dep (monorepo), that is
straightforward. If it is a separate repo and `publish = false`, a git dep is required,
which adds `Cargo.lock` churn and reproducibility concerns.

---

## Summary table

| RC | Severity | Location | Blocker reason |
|----|----------|----------|----------------|
| RC-1 | BLOCKING | `domain/src/diff.rs:120` | `violation_key` match must be exhaustive; rank assignment not specified in RFC; interaction with `#[non_exhaustive]` on Violation |
| RC-2 | BLOCKING | `domain/src/lib.rs:101–105` + 12 construction sites | All `ConceptNode { ... }` literals fail to compile on field-add without `Option` or `Default` |
| RC-3 | BLOCKING | RFC §3.3, §3.8, R10-6 | cfdb-query adapter requires linking cfdb-petgraph+cfdb-core+cfdb-query+cfdb-concepts; these are NOT available without a Cargo dep; keyspace must exist at check-time; this is a new crate, not "a few hundred lines" |
| RC-4 | advisory | `domain/src/lib.rs:41` | `SchemaVersion::V3` + `as_str` arm + 15+ test assertion updates in ndjson.rs |
| RC-5 | advisory | `adapters/markdown/src/lib.rs:275` | H1+H4 addition will push `handle_event` above complexity 15 unless separate pass (RFC-005 precedent) |
| RC-6 | advisory | `adapters/rust/src/lib.rs:162` | `module` field requires file-path-to-module-path derivation not currently in codebase; parity test with cfdb-query adapter depends on matching this correctly |
| RC-7 | advisory | `Cargo.toml` + CI | 4-file Cargo.toml migration for R10-6; cfdb dep type (path/git/crates.io) unresolved |
