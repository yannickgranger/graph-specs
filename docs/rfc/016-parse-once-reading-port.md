# RFC-016 — parse-once reading

**Status:** Revision 4.1 (2026-08-19) — registration-ready — pending operator merge
**Date:** 2026-08-19
**Prior art:** RFC-001 §3.6 (ContextReader — one port trait per opt-in read capability), RFC-004 (LanguageBackend / Extraction — single-walk backend contract), RFC-005 §3.2 (VerbReader — the same precedent applied to verbs), RFC-012 (AnchorResolver), RFC-010 R10-3 (spec-tree assembly feeding the cohesion pass)

## §1 Problem

A SOLID + tech-debt audit of `yg/graph-specs-rust` at `6fd46418` (2026-08-19, two-auditor pass, findings re-verified against head; file:line citations below are pinned at that SHA) found the reading pipeline is the single seam where the codebase's discipline has decayed. Three findings, one root cause:

1. **Port decay (DIP).** The reading pipeline drives only part of its extractions through `ports` traits (`Reader`, `VerbReader`, `ContextReader`, `CodeFacts`): `run_check` performs seven unconditional read operations plus a conditional anchor-resolver index, and `run_report` adds the annotation read. The capabilities added after RFC-005 bypass the port layer as concrete calls: `MarkdownReader.extract_verb_anchors` (`application/src/lib.rs:48`), `MarkdownReader.extract_concept_anchors` (`application/src/lib.rs:51`), `MarkdownReader.extract_invariant_annotations` in `run_report` (`application/src/report.rs:35`), and the free function `assemble_spec_trees` (`application/src/lib.rs:58`), whose return type `SpecTree` is an adapter-owned type imported by `application` (`application/src/lib.rs:8`) — an adapter type crossing the port boundary in the wrong direction. Every capability added after RFC-005 skipped the trait discipline RFC-001 §3.6 and RFC-005 §3.2 established.
2. **Leaf-adapter dependency edge.** `adapters/markdown/Cargo.toml:8` depends on `adapter-rust` to reuse the one function `adapter_rust::normalize`. The defect is a release-granule one (REP), not a raw stability-metric one: `adapter-markdown`'s build graph improperly carries `adapter-rust`'s entire transitive surface (walkdir pipeline, full `syn` item visiting, ~1,261 lines of unrelated extraction logic) for one nine-line normalizer, so any edit to that unrelated logic invalidates `adapter-markdown`'s incremental rebuild and everything downstream. It also breaks the backend symmetry RFC-004 defines and leaves the roadmapped third backend (PHP / TypeScript) no home for shared normalization except duplicating it or adding more adapter-to-adapter edges.
3. **Multi-pass I/O.** One `graph-specs check` walks and re-reads the specs tree five times (`extract`, `extract_contexts`, `extract_verb_anchors`, `extract_concept_anchors`, `assemble_spec_trees`) and the code tree three times (`RustReader::extract`, `extract_pub_fns`, `RustAnchorResolver::index`), with `syn::parse_file` re-parsing every `.rs` file on each code pass. The two spec-side walkers additionally encode the same `concepts/`-vs-`contexts/` partitioning rule in two places (`adapters/markdown/src/lib.rs:188-207`, `contexts.rs:289-321`). Negligible on this repo; linear waste on real targets (qbot-core), and the same failure mode already measured in the companion toolchain.

The root cause is shared: every read capability owns its own walk. The fix is one load, many extractions.

## §2 Scope

Ships:

- A loaded-file aggregate (`SpecFileSet` / `CodeFileSet`) and loader ports in `ports`.
- Every spec-side and code-side read capability re-anchored as a port trait over a loaded file set — except `CodeFacts`, whose contract already declares its root advisory and whose ACL implementor reads a fact store, not a tree (§3.3); the four currently-inherent capabilities gain traits; `SpecTree` and `HeadingNode` move to `domain` with their inherent methods.
- A crate-internal parse cache on the Rust adapter collapsing the three code-side `syn` passes to one.
- A shared signature-normalization workspace member; `adapter-markdown`'s dependency on `adapter-rust` is removed, decoupling its incremental rebuilds from code-side extraction changes.
- `application` wiring: one load per tree per run.
- Spec transcription for the new/changed port surface in `specs/concepts/`, plus the paired `Exports`/`Imports` entries in `specs/contexts/equivalence.md` and `specs/contexts/reading.md` for every type that crosses the context boundary (derived from this RFC after ratification, per methodology).

Does not ship: any wire-format change, any new violation variant, any CLI surface change, any third language backend.

## §3 Design

### §3.1 File sets live in `ports`

```rust
pub struct LoadedFile {
    pub path: PathBuf,
    pub text: String,
}

pub struct SpecFileSet {
    files: Vec<LoadedFile>,
}

pub struct CodeFileSet {
    files: Vec<LoadedFile>,
}
```

Naming avoids the `Source` stem entirely: `domain::Source` is the equivalence context's Published Language for "where a concept was found" (side + path + line), and a same-context homonym on that stem is a language defect. A `LoadedFile` is a different pipeline stage — a loaded artifact, path + text, no line, no side discriminant of its own.

Both sets are aggregates, not bags: `files` is private; construction is `new(files: Vec<LoadedFile>) -> Self`, which sorts by path; reading is `files(&self) -> &[LoadedFile]`. Sorted-by-path iteration order is load-bearing for NDJSON byte-stability and is restated as a §4 invariant.

Placement follows the `ReaderError` rationale already recorded in `ports/src/lib.rs`: these describe reading operations, not domain concerns. `ports` keeps its current dependency set (`domain` + `thiserror`); `LoadedFile` uses stdlib types only. Text-level only — no `syn` types in `ports` (mirrors the `arch-context-no-syn-in-domain` ban's intent one layer up).

### §3.2 Loader ports

```rust
pub trait SpecLoader {
    fn load(&self, root: &Path) -> Result<SpecFileSet, ReaderError>;
}

pub trait CodeLoader {
    fn load(&self, root: &Path) -> Result<CodeFileSet, ReaderError>;
}
```

The loader owns the single walk: directory traversal, extension filter, `read_to_string`, then `SpecFileSet::new` / `CodeFileSet::new` for the deterministic sort. Subtree partitioning (`concepts/` vs `contexts/`) is no longer the walker's job — capability extractors filter by path prefix in memory, through one shared path-predicate helper in the markdown adapter so the partitioning rule has a single owner.

Two monomorphic traits, not one generic `Loader<T>`: each has exactly one implementor, no polymorphic call site exists, and the shape matches `Reader`/`ContextReader`/`VerbReader`.

`MarkdownReader` implements `SpecLoader`. A new unit struct `RustLoader` implements `CodeLoader` — code-side reading gains cache-holding reader construction in §3.4, so the loader must remain constructible before any parse state exists.

### §3.3 Capability traits take file sets

#### §3.3.1 — The split

The existing traits keep their one-capability-per-trait shape (RFC-001 §3.6 / RFC-005 §3.2 precedent — this RFC extends that discipline, it does not replace it), and their methods change input: `&SpecFileSet` / `&CodeFileSet` instead of `&Path`. One trait cannot keep its name: `Reader` is the only trait implemented on both sides (`MarkdownReader` and `RustReader`), and two nominal input types cannot share one non-generic method signature (E0053), so `Reader` splits into `SpecReader` and `CodeReader` and the shared name retires — the same monomorphic-traits ruling as §3.2, applied to the one place revision 2 missed it. The value of a trait with one production implementor here is testability plus machine-enforced policy (the port-bypass ban family in `.cfdb/queries/`), not hypothetical substitutability. The four unported capabilities gain sibling traits of the same shape:

#### §3.3.2 — The port table

| Capability today | Port after this RFC |
|---|---|
| `Reader::extract(&Path)` — one trait, implemented by both adapters | splits: `SpecReader::extract(&SpecFileSet)` on `MarkdownReader`; `CodeReader::extract(&CodeFileSet)` on `RustReader`; the shared `Reader` name retires |
| `ContextReader::extract_contexts(&Path)` | same trait, file-set input |
| `VerbReader::extract_pub_fns(&Path)` | same trait, file-set input |
| `MarkdownReader.extract_verb_anchors` (inherent) | `VerbAnchorReader::extract_verb_anchors(&SpecFileSet)` |
| `MarkdownReader.extract_concept_anchors` (inherent) | `ConceptAnchorReader::extract_concept_anchors(&SpecFileSet)` |
| `MarkdownReader.extract_invariant_annotations` (inherent) | `AnnotationReader::extract_annotations(&SpecFileSet)` |
| `assemble_spec_trees` (adapter free fn, adapter type out) | `SpecTreeReader::extract_spec_trees(&SpecFileSet) -> Vec<domain::SpecTree>` |
| `LanguageBackend::extract(&Path)` (RFC-004) | same trait, `&CodeFileSet` input — single implementor (`RustBackend`, cache-holding per §3.4); recorded here as an RFC-004 amendment |
| `CodeFacts::concepts(&Path)` | signature unchanged — its contract already declares `root` advisory (the cfdb-query ACL impl reads a fact store fixed at construction and must not be forced to accept a file set it cannot use); `RustReader`'s impl serves from its held `ParseCache` instead of walking, so the walk still collapses |
| `RustAnchorResolver::index(&Path)` (inherent constructor) | `RustAnchorResolver::index(&CodeFileSet, &ParseCache)` — the `AnchorResolver` port trait itself (`resolve(&self, qname)`) is unchanged; only the constructor re-anchors |

Error-contract honesty: after re-anchoring, capability trait methods can structurally return only `ReaderError::ParseFailed`; `IoFailed` and `WalkFailed` become the exclusive province of the loader traits in §3.2, and each capability trait's contract documentation says so.

**Amendment 2026-09-06.** The table above predates `SignatureNormalizer` and its implementers `RustSignatures` and `PhpSignatures` (graph-specs-004-multi-language-adapter-contract#3.6; landed by graph-specs #263, #265 and #267): a normalizer takes a fenced block of text handed to it and never a file set, so no row of the table applies to it and slice S1 of §7 leaves the three untouched.

#### §3.3.3 — `SpecTree` relocates to `domain`

`SpecTree` relocates to `domain` — the struct, its field type `HeadingNode` (`SpecTree.nodes: Vec<HeadingNode>` makes the pair inseparable), and its four inherent methods (`context_id`, `concept_declarations`, `cohesion_violations`, `has_cohesion_unit`). Rust forbids an inherent `impl` on a type defined outside the current crate (E0116), and the methods touch only domain types today (`AbstractionLevel`, `CohesionViolation`, `behavioral_exemption_applies`), so the move is both compilable and consistent with existing precedent. Only assembly — the pulldown-cmark event walk and `ReaderError` production — stays adapter-side behind `SpecTreeReader`.

After this table there is no path by which `application` names an adapter type other than the reader and loader structs it instantiates at the composition root and the `ParseCache` handle it constructs there and threads into code-side constructors.

### §3.4 Code-side parse cache

Ports carry text; `syn` stays adapter-internal. The Rust adapter gains a crate-internal `ParseCache` — a newtype over `Rc<RefCell<HashMap<PathBuf, Entry>>>`, never exposed through `ports` — built once per run by `adapter_rust::parse(root: &Path, code_set: &CodeFileSet) -> ParseCache`. `parse` takes `root` because containment provenance (`unit` / `module_path`, the RFC-010 R10-3 pair the cfdb-query parity invariant matches at 0-mismatch) is root-relative and the re-anchored trait signatures no longer carry a path: each cache `Entry` bundles the parsed `syn::File` with the file's pre-computed `unit` and `module_path`, so every consumer reads provenance out of the cache and never needs `root` after construction. `parse` is a free function, not a port method, so the second parameter breaks no table shape. `RustReader`, `RustBackend`, and `RustAnchorResolver::index` all take the handle at construction (mirroring `RustAnchorResolver`'s existing hold-pre-built-state shape): `RustReader` and `RustBackend` stop being unit structs and become cache-holding values constructed at the composition root via `new(cache: ParseCache)`; `ParseCache` is `Clone` (an owned handle to the same underlying `Rc`) and its inner field is private, so `syn::File` stays fully encapsulated. The cache is constructed fresh per run from the loaded set, held by value in `run_check` / `run_report`, and is never a process-global — repeated in-process runs (the integration tests drive `run_check` many times against distinct trees) each get their own cache.

`Rc`/`RefCell` is deliberate: the workspace is a single-threaded CLI with zero `Send`/`Sync` surface today. A future parallelized walk would replace the cell with a concurrent map; that is out of scope and recorded in §6.

Spec-side parse deduplication is explicitly not attempted: `pulldown_cmark::Parser` is a cheap streaming lexer over an in-memory string, and the measured cost on the spec side was the five redundant walk+read passes, which §3.2 eliminates. Only the code side carries an expensive parse (`syn::parse_file`) worth caching.

### §3.5 Shared normalization home

New workspace member at directory `adapters/signature`, crate name `signature-norm` — never bare `signature`, which would shadow the crates.io RustCrypto crate name in tooling output; the workspace publishes nothing (no `publish` fields, no registry step in CI), so the collision is cosmetic, and the hyphenated name forecloses it anyway. The crate contains `normalize` moved verbatim from `adapter-rust/src/normalize.rs` with its unit tests, and is intentionally `syn`-specific — it is the Rust signature normalizer, not a language-neutral home; per-language normalization for future backends arrives with the RFC-004 `<lang>-items` roadmap, not by genericizing this crate.

Both `adapter-rust` and `adapter-markdown` depend on it; `adapter-markdown`'s `adapter-rust` dependency is deleted. Dependency direction: adapters point at a shared leaf utility; no adapter points at an adapter. The payoff is incremental-rebuild decoupling: today any edit to code-side extraction invalidates `adapter-markdown`'s rlib and everything downstream; after this slice the coupling is gone.

### §3.6 Application wiring

`run_check` and `run_report` each perform exactly one load per tree — `MarkdownReader.load(specs_dir)?` and `RustLoader.load(code_dir)?` — then `adapter_rust::parse(code_dir, &code_set)` once (`code_dir` is already the enclosing function's parameter), then hand the sets (and, code-side, cache-holding readers constructed from the `ParseCache`) to capability traits. Spec-side wiring is unchanged in spirit from today. Code-side wiring changes shape deliberately: the bare-value `RustReader.extract(...)` call pattern is replaced by constructing one cache-holding reader per run and calling all capabilities on it — that churn is scoped in S3, not discovered mid-PR.

## §4 Invariants

- NDJSON schema v5 (`graph-specs-004-multi-language-adapter-contract#3.5` as amended) is byte-stable: `check` and `report --verb-coverage` NDJSON and text outputs on this repo's own tree are byte-identical before and after every slice, with the one exception this RFC's own order creates — a slice that relocates a file (§3.5 moves `normalize`) changes the `path` and `line` fields of the records the moved lines carry and nothing else, so for such a slice the invariant reads on the record set with `path` and `line` held out, and the baseline is re-recorded at the slice's merge (amendment of 2026-09-06, measured on graph-specs #264 by its blind seat). A baseline is comparable only under the command that recorded it, invoked from inside the tree — `graph-specs <verb> --specs specs/concepts --code .`, `--format ndjson` for the NDJSON values — because the outputs carry the paths they were given and the same binary on the same tree hashes differently from another working directory. Baseline at `24a2f91` (the tree of the §3.5 merge `03cd714`): check NDJSON `054106275d32108e…` and text `7a97df8972134e69…`, both unchanged from the base `a14c87e`; report NDJSON `2129630ba89f5a20…` and text `8d8b32002fa51dca…`, against `cd85796aef9bcc97…` and `1b2b9f104316b7d3…` at the base — 138 records on both sides, equal once `path` and `line` are held out: one path, `normalize` from `adapters/rust/src/normalize.rs` to `adapters/signature/src/lib.rs`, and seven line numbers in `adapters/rust/src/lib.rs` each shifted by one for the deleted `mod normalize;`. Earlier baseline at `6fd46418`, schema v4: report NDJSON `eb538512…`, report text `508e250b…`, check output empty at zero violations.
- `SpecFileSet` / `CodeFileSet` iteration order is sorted-by-path and load-bearing for that byte-stability; the aggregates enforce it at construction (§3.1) and S1's loader-determinism unit test is the regression fence.
- Whole-tree-in-memory is the accepted resource model: a file set holds every matching file's text for the duration of one run. Accepted bound — source trees at qbot-core scale are tens of megabytes; the previous model re-read the same bytes three to five times.
- Exit codes unchanged. Self-dogfood stays 0 violations. Cross-dogfood on cfdb at its pinned SHA stays exit 0.
- `cfdb` ban rules stay 0 across all six rules, including `arch-context-no-syn-in-domain` (unaffected: file sets are text-level) and `arch-ban-multiple-walk-pub-fns-callers`.
- Own-gate: the tree declares L3 in `keel.json` and the corpus-wide ungrounded count is zero and stays zero, every heading grounded at the pin (amendment 2026-09-06 — the L2 worklist of 57 headings at the 2026-08 baseline is drained, so the former "does not increase" bound is met vacuously and the invariant is the stricter one the gate already enforces); new port concepts land with spec headings transcribed from this RFC in the same PR, with paired `Exports`/`Imports` context entries.
- Existing test corpus passes; the emitter golden tripwire (landed 2026-08-19) stays green.

## §5 Architect lenses

### §5.1 Clean architecture

### §5.2 Domain-driven design

### §5.3 SOLID + component principles

### §5.4 Rust systems

### §5.5 Blind seats (registration PR, revision 2)

## §6 Non-goals

- No change to the `#[non_exhaustive]` policy on domain enums (RFC-001 RC-3/RC-4 stands; the emitter silent-degrade hole is closed by the golden tripwire test, not by an API break).
- No grounding work on `specs/concepts/` headings — the operator-convened grounding council (cascade plan frontier 4) owns that; this RFC's spec transcriptions are additions derived from a ratified RFC, not retrofits.
- No third language backend, no shared `<lang>-items` crates (RFC-004 roadmap unchanged — §3.5 makes room for it, nothing more).
- No parallel directory walk or concurrent parse: the `ParseCache` is `Rc`/`RefCell` by design for a single-threaded CLI; parallelism would replace the cell with a concurrent map under its own RFC.
- No wire, CLI, or exit-code change of any kind.

## §7 Issue decomposition

Vertical slices, one issue each. Every slice ends green on: self-dogfood 0 violations, byte-identity against the §4 baselines, cross-dogfood exit 0, cfdb rules 0, own-gate L2 holds, clippy `-D warnings`, fmt.

**S1 — loader ports + concept extraction over file sets.** `LoadedFile` / `SpecFileSet` / `CodeFileSet` / `SpecLoader` / `CodeLoader` in `ports` (private-field aggregates, sorting constructors); `MarkdownReader` implements `SpecLoader`; new `RustLoader` unit struct implements `CodeLoader`; the `Reader` trait splits into `SpecReader` / `CodeReader` over file-set input (§3.3); `application` wires the two loads. Spec headings for the new port types land in the same PR, transcribed from §3.1–§3.2, with paired `Exports`/`Imports` entries in `specs/contexts/equivalence.md` + `reading.md`.
```
Tests:
  - Unit: loader determinism (two loads of the same fixture tree yield identical ordered sets; an unsorted input Vec is sorted by the constructor); SpecReader::extract over an in-memory SpecFileSet fixture equals the pre-RFC output on the same tree; object-safety compile proof (Box<dyn SpecLoader>, Box<dyn CodeLoader>) mirroring ports/tests/context_reader.rs
  - Self dogfood (graph-specs on graph-specs): check exit 0, 0 violations, NDJSON byte-identical to baseline
  - Cross dogfood (graph-specs on cfdb at pinned SHA): exit 0
  - Target dogfood (on qbot-core at pinned SHA): none — rationale: behavior-preserving refactor; wire stability is proven by the self + cross rows
```

**S2 — remaining spec capabilities behind traits.** `ContextReader` re-anchored; `VerbAnchorReader`, `ConceptAnchorReader`, `AnnotationReader`, `SpecTreeReader` introduced; `SpecTree` + `HeadingNode` + the four inherent methods move to `domain`; the shared path-partitioning predicate lands in the markdown adapter; the inherent methods and the `application` → `adapter_markdown::SpecTree` import are deleted.
```
Tests:
  - Unit: each capability over in-memory SpecFileSet fixtures (capability paths perform no I/O by construction — input type carries the text); object-safety compile proofs for the four new traits
  - Self dogfood: check + report byte-identical to baseline; 0 violations
  - Cross dogfood: exit 0
  - Target dogfood: none — rationale: same as S1
```

**S3 — code side over file sets + parse cache.** `adapter_rust::parse(root: &Path, &CodeFileSet) -> ParseCache` lands, computing per-file provenance (`unit` / `module_path`) at construction; `RustReader` / `RustBackend` become cache-holding structs constructed from the handle; `RustAnchorResolver::index` re-anchors to `(&CodeFileSet, &ParseCache)`; `VerbReader` re-anchored; the three code walks collapse to one. Scoped churn: the bare-value call sites are rewritten — `application/src/lib.rs` (three production calls at the pin `6fd46418`: `extract` at :49, `extract_pub_fns` at :50, and `concepts` at :127 inside `application::code_facts()`), `application/src/report.rs` (one), `adapters/cfdb-query/tests/parity.rs` (one), and the ~20 bare-value sites in `adapters/rust/src/tests.rs`. `application::code_facts()` has no production caller in `main.rs` and does not share `run_check`/`run_report`'s one-load-per-run — its rewritten body performs its own local load, parse, construct sequence.
```
Tests:
  - Unit: parse-cache hit assertion — one parsed unit per file across sequential capability calls on a two-capability fixture run; extraction outputs equal pre-RFC outputs on the same fixture; provenance parity — `unit` / `module_path` read from the cache equal the pre-RFC walk-derived values on a multi-crate fixture whose root is not the process cwd
  - Self dogfood: byte-identical to baseline; 0 violations
  - Cross dogfood: exit 0
  - Target dogfood: report --verb-coverage record count on qbot-core at its pinned SHA unchanged pre/post slice, reported in the PR body
```

**S4 — signature-norm crate.** Directory `adapters/signature`, crate `signature-norm` created; `normalize` and its tests move verbatim; both adapters re-point; `adapter-markdown`'s `adapter-rust` dependency deleted; `specs/concepts/equivalence.md` and `reading.md` normalize clauses re-transcribed from §3.5.
```
Tests:
  - Unit: the moved normalize suite passes unchanged in the new crate
  - Self dogfood: byte-identical to baseline; 0 violations; a slice test asserts via cargo metadata that adapter-markdown has no adapter-rust dependency
  - Cross dogfood: exit 0
  - Target dogfood: none — rationale: pure relocation, proven by the unit row
```

**S5 — retirement + transcription closure.** Dead inherent surface removed; `specs/concepts/reading.md` reads as the §3.3 table and gains the load-vs-extract paragraph in its context prose; doc references updated; the opening prose of `specs/contexts/reading.md` is re-transcribed so the context map no longer describes the pre-RFC fused walk+parse+extract shape and no longer names the retired `Reader` trait (its line 8 Conformist clause moves to `SpecReader`/`CodeReader`); any vestigial re-exports dropped; a slice test greps that `application` names no `adapter_markdown` type outside reader-struct construction.
```
Tests:
  - Unit: none — rationale: deletions only; the compiler and the existing suite are the verification surface
  - Self dogfood: check 0 violations (the equivalence gate is the test that specs and surface now agree)
  - Cross dogfood: exit 0
  - Target dogfood: none — rationale: no behavior surface
```
