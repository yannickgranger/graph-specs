# SOLID + Component Principles Review — RFC-010

**Lens:** SOLID + Component Principles (SRP, ISP, OCP, CCP, CRP, SDP, SAP, ADP)  
**RFC:** docs/rfc/009-abstraction-level-equivalence.md  
**Round:** 1 — adversarial  
**Verdict:** REQUEST CHANGES

---

## 1. Stability Metrics Table (pre-RFC baseline)

Dependency graph from Cargo.toml files. Ca = afferent couplings (dependents), Ce = efferent couplings (dependencies), I = Ce/(Ca+Ce), A = pub_traits/total_pub_types, D = |A+I-1|.

| Crate | Ca | Ce | I | A | D | Zone |
|---|---|---|---|---|---|---|
| domain | 4 | 0 | 0.00 | 0.00 | 1.00 | Zone of Pain (by-design: value-type core) |
| ports | 3 | 1 | 0.25 | 0.67 | 0.08 | Main sequence (good) |
| adapter-rust | 2 | 2 | 0.50 | 0.00 | 0.50 | Zone of Pain |
| adapter-markdown | 1 | 3 | 0.75 | 0.00 | 0.25 | Acceptable |
| application | 0 | 4 | 1.00 | 0.20 | 0.20 | Main sequence (composition root) |

Evidence:
- `domain/Cargo.toml`: no `[dependencies]` block — Ce=0
- `ports/Cargo.toml:5`: `domain = { workspace = true }` — Ce=1
- `adapters/markdown/Cargo.toml:8-10`: depends on adapter-rust, domain, ports — Ce=3
- `adapters/rust/Cargo.toml:7-8`: depends on domain, ports — Ce=2
- `application/Cargo.toml:6-10`: depends on all four — Ce=4

RFC-010 impact (post-RFC):
- `AbstractionLevel` enum added to `domain` — Ce unchanged, Ca unchanged, I=0.00, D=1.00 (unchanged)
- `CodeFacts` trait added to `ports` — A rises from 4/6=0.67 to 5/7=0.71, D improves slightly to 0.04 (good)
- New `CfdbQueryAdapter` crate (R10-6) — Ce=1 (ports only, by design), Ca=0 initially, I=1.00
  — This is acceptable for a leaf adapter. ADP: no cycle introduced if CfdbQueryAdapter only depends on ports.

**SDP check:** All dependency arrows point from higher-I to lower-I (unstable → stable). Post-RFC arrows remain directionally sound. No SDP violation introduced.

**ADP check:** Proposed dependency graph is acyclic. CfdbQueryAdapter → ports → domain. No cycle.

---

## 2. CRP Violation Matrix

Ports traits after RFC-010: `Reader`, `VerbReader`, `ContextReader`, `LanguageBackend`, `CodeFacts` (5 total).

| Adapter | Reader | VerbReader | ContextReader | LanguageBackend | CodeFacts | Utilization |
|---|---|---|---|---|---|---|
| MarkdownReader | yes | no (direct pub fn, not trait) | yes | no | no | 2/5 = 40% |
| RustReader/RustBackend | yes | yes | no | yes | yes (proposed R10-3) | 4/5 = 80% |
| CfdbQueryAdapter (R10-6) | no | no | no | no | yes | 1/5 = 20% |

**CRP violation: CfdbQueryAdapter (R10-6) at 20%.** The `CodeFacts` trait is placed in the same `ports` crate as `Reader`, `VerbReader`, `ContextReader`, and `LanguageBackend`. The cfdb-query adapter will compile against all five — four of which it will never implement. This forces consumers of the cfdb-query adapter to depend on `pulldown-cmark`, `syn`, and `walkdir` (via the ports crate's transitive compile surface) for a feature they do not use.

The resolution is either: (a) extract `CodeFacts` into a standalone `ports-codefacts` crate, or (b) gate it behind a Cargo feature flag in `ports`. The RFC §3.3 does not address this.

**ISP pre-RFC baseline finding:** `VerbReader` is a separate trait (correct ISP application from RFC-005), but `extract_verb_anchors` on `MarkdownReader` at `adapters/markdown/src/lib.rs:112` is a `pub fn` on the struct, not an `impl VerbReader for MarkdownReader`. This asymmetry means the application layer calls the trait method on RustReader but the struct method on MarkdownReader — the port seam is leaky on the markdown side. RFC-010 does not fix this, but neither does it worsen it.

---

## 3. SRP Analysis — Markdown Reader Responsibility Load

Current `MarkdownReader` responsibilities (as measured by the module, at 647 lines):

1. Walk directory tree and filter files — `extract()` at line 42
2. Assemble heading-tree into flat concept bag (H2/H3 → ConceptNode) — `extract_from_source()` at line 258
3. Parse bullet edges (implements/depends-on/returns) — `finish_bullet()` at line 335
4. Parse verb anchors (`- verb:` bullets) — `parse_verb_bullet()` at line 381
5. Parse H4 operational-invariant annotations — `extract_annotations_from_source()` at line 473
6. Extract context declarations (delegates to `contexts` submod) — `extract_contexts()` at line 98

RFC-010 adds:
7. Track H1 context declarations and attach H2/H3 concepts to the preceding H1 (parent links)
8. Attach H4 members to their H2 parent

Complexity budget: `extract_annotations_from_source` already scores 16 (above the `<15` ceiling enforced by `ra-query`). Adding H1-tree assembly into `handle_event` — which already routes H2/H3, code blocks, bullets, and text — will push `handle_event` (currently 9) and `extract_from_source` (currently 11) above the ceiling.

**The RFC §5.3 question is confirmed: this is multiple responsibilities.** Reasons to change the markdown reader today: add a new heading-level rule (structural concerns), add a new bullet prefix (relationship concerns), add a new annotation grammar (invariant concerns), change tree structure. Each is orthogonal. The heading-tree assembly is driven by the *spec* domain model; the bullet-edge extraction is driven by the *graph* domain model; the annotation extraction is driven by the *report* domain model.

**SRP-1 [BLOCKING]** — The RFC must specify that H1-tree assembly (responsibility 7/8) lives in a separate `TreeAssembler` pass/type rather than being woven into the existing `handle_event` event handler. The current `handle_event` (line 275) already routes six distinct concerns; adding H1-context state tracking and child-node attachment puts it over budget.

---

## 4. OCP Analysis — AbstractionLevel as Extension Point

The RFC proposes `AbstractionLevel` as `#[non_exhaustive]`. This is the correct OCP signal. However:

**Where does the diff engine consume `AbstractionLevel`?** The current `diff()` function (`domain/src/diff.rs:26`) receives a `CheckInput` with a flat `Graph` (no levels). It passes through to five sub-passes (concept, signature, edge, verb, context). None of the sub-passes dispatch on heading level today.

RFC-010 adds a sixth pass (cohesion check). The cohesion check operates on `(concept, module, context)` tuples — it does not dispatch on `AbstractionLevel` values. Adding a future L3-diffing pass (deferred, OQ-2) *would* need to dispatch on `AbstractionLevel::Member`.

**The OCP gap:** When L3 diffing lands, `diff()` will need a new branch for `Member`. The `#[non_exhaustive]` tag prevents downstream exhaustive-match breakage, but the diff engine itself will require modification. This is expected for a new capability; the question is whether `AbstractionLevel` is the right extension point or whether the `CheckInput` + `diff()` boundary should carry the level policy.

**OCP-1 [advisory]** — The RFC should document the planned modification point explicitly: which file and function gains the L3 arm when OQ-2 resolves. Identifying this now prevents scope confusion when that RFC arrives. The current draft implies but does not state that `domain/src/diff/` gains a new submodule (analogous to `verb.rs`, `context.rs`). State it.

---

## 5. ISP/CCP Analysis — Four Variants vs. One Parameterized

RFC-010 proposes four new `Violation` variants:
- `ContextOwnsScatteredConcepts` — one context H1, concepts resolve to N>1 modules
- `ConceptMisfiled` — concept's code module differs from context's resolved module  
- `ContextWithoutCohesionUnit` — H1 context, concepts resolve to zero modules
- `ModuleSplitAcrossContexts` — one code module, documented under M>1 H1 files

**CCP verdict: CORRECT.** These four are not four expressions of one idea; they are four distinct diagnostic signals that require different remediation actions. A user receiving `ContextOwnsScatteredConcepts` must consolidate modules; a user receiving `ConceptMisfiled` must move a single type; `ContextWithoutCohesionUnit` means the abstraction is purely virtual; `ModuleSplitAcrossContexts` means the module boundary contradicts the spec boundary. Collapsing them into one parameterized variant would force all consumers to inspect the parameter to determine which remediation applies — that is an ISP violation on the consumer side.

**violation_key cost:** `const fn violation_key` at `domain/src/diff.rs:120` currently has 12 arms. Four new variants add 4 arms, reaching 16. This is mechanical and compile-enforced by `#[non_exhaustive]` — the compiler will reject a missing arm. The `u8` discriminator reaches 15 at most; no overflow. **No issue here.**

**The wrapping question:** The RFC §3.5 says these are "all wrapped to keep the existing Violation taxonomy stable." It does not say *how* they are wrapped. The existing `Violation::Context(ContextViolation)` pattern (introduced for RFC-001) is the precedent. The RFC should state explicitly whether these four variants are wrapped in a new `CohesionViolation` enum (analogous to `ContextViolation`) or added as top-level `Violation` variants.

**ISP-1 [BLOCKING]** — The RFC must choose one of:
  (a) `Violation::Cohesion(CohesionViolation)` where `CohesionViolation` holds the four variants, mirroring the `Violation::Context(ContextViolation)` pattern at `domain/src/lib.rs:293`. This keeps `violation_key` at 13 arms (not 16) and lets `ndjson.rs` and `text.rs` delegate cohesion rendering to a helper in the same way `context_violation_to_record` is called at `application/src/ndjson.rs:134`.
  (b) Four direct top-level `Violation` variants. This is simpler but note that `application/src/ndjson.rs` already has `#[allow(clippy::too_many_lines)]` at line 41 — adding four match arms will worsen this.

The RFC leaves this open ("all wrapped" is vague). The ndjson emitter has 36 `Violation::` pattern sites already; the text emitter has 26. Both files must gain matching arms for each new variant. Prescribe the wrapping strategy now so implementers do not diverge.

---

## 6. LSP Parity Claim — R10-6 Invariant Analysis

RFC-010 §7 R10-6 asserts: "the two adapters must agree on graph-specs' own tree (parity test)." This is an LSP substitutability claim. The review found a structural gap that makes this claim non-trivially true:

**Module granularity mismatch:**

- `RustReader` (source-walk) currently processes only top-level items per file. `adapters/rust/src/lib.rs:11`: "Scope: only top-level items in each file are visited. Concepts nested inside `pub mod foo { ... }` are not extracted at this level." The module provenance for a top-level item is its *file path*, not its *Rust module path*.

- cfdb's Rust extractor tracks the full inline module stack (`cfdb-extractor/src/item_visitor/visits.rs:366`: `self.module_stack.push(mod_name.clone())`). An item inside `pub mod inner { pub struct Foo; }` gets `IN_MODULE` pointing to `module:parent::inner`, not to the file.

- cfdb's PHP extractor (`cfdb-extractor-php/src/lib.rs:19`) uses PHP `namespace` → `:Module`. PHP has no files-as-modules concept; the namespace IS the module.

**LSP consequence:** For a codebase that uses inline `pub mod` blocks, the source-walking adapter emits `module = "file-path"` while the cfdb-query adapter emits `module = "fully-qualified-mod-path"`. These disagree. Cohesion violations based on module colocation will fire differently depending on which adapter is active. This violates LSP: a caller relying on `CodeFacts::concepts()` cannot substitute adapters freely.

**LSP-1 [BLOCKING]** — RFC-010 must specify the exact definition of "module" as the unit of comparison. Two candidates:
  (a) File-path granularity: cfdb-query adapter normalizes by dropping the inline-mod segment, reporting the file path. This loses information cfdb already has.
  (b) Rust-module-path granularity: source-walking adapter gains inline-mod tracking (dropping the "top-level only" restriction). This is scope expansion.

Without this definition, R10-6's parity test is undefined — two adapters that emit *different* module paths for the same item would each pass their own unit tests while violating the parity invariant on any real codebase with inline mods.

---

## 7. CCP Grouping Validation

**Do the proposed types share the same domain dependency signature?**

`AbstractionLevel` (domain): depends on nothing — pure enum. Change reason: new heading-level rung in the spec dialect.  
`ConceptNode` field additions (`module`, `crate`): depends on `std::path::PathBuf` (already present in `Source`). Change reason: containment provenance.  
`CodeFacts` trait (ports): depends on `domain::ConceptNode`. Change reason: new source adapter seam.  
Four new `Violation` variants: depend on `domain::Source`. Change reason: new cohesion rules.  

`AbstractionLevel` and the four `Violation` variants both belong in `domain` and change when the spec dialect model changes. `CodeFacts` belongs in `ports` and changes when a new adapter source type is needed. **CCP grouping is correct** — the RFC places types in the right crates. No finding here.

---

## 8. Pre-Existing Structural Finding: adapter-markdown depends on adapter-rust

Not introduced by RFC-010 but relevant to the SRP analysis:

`adapters/markdown/Cargo.toml:8`: `adapter-rust = { workspace = true }` — the only use is `adapter_rust::normalize(&item)` at `adapters/markdown/src/lib.rs:453`. The markdown reader calls the Rust normalizer to parse inline fenced `rust` blocks in spec files. This creates a same-layer coupling between two adapters that are peers, both depending on `ports`.

RFC-010 will add more code to the markdown reader. The deeper this file grows, the more costly it becomes to untangle this coupling later. The fix is extracting `normalize` into a function in `domain` or a shared utility — but this is pre-existing debt.

**[advisory, pre-existing]** — File a separate issue to move `normalize` out of `adapter-rust` into `domain` or a shared `normalize` module, breaking the peer-adapter coupling.

---

## Summary of Required Changes

| # | Finding | Severity | Resolution |
|---|---|---|---|
| SRP-1 | `handle_event` cognitive budget exceeded by H1-tree assembly; markdown reader accumulates 8 distinct responsibilities | BLOCKING | RFC must prescribe a `TreeAssembler` pass type (or equivalent separation) that holds H1-context tracking state separately from the existing `SectionState`; `handle_event` must not gain H1 + H4 state in the same struct |
| ISP-1 | Four new violation variants: wrapping strategy not specified; ndjson.rs already at `too_many_lines`; 36 existing match sites must grow | BLOCKING | RFC must specify `Violation::Cohesion(CohesionViolation)` vs four direct variants and update issue R10-4 accordingly |
| LSP-1 | "Module" definition diverges between source-walk (file-granular) and cfdb-query (Rust-module-path-granular); parity test R10-6 is undefined without this | BLOCKING | RFC must define the exact granularity of `module` in `ConceptNode`, and specify which adapter aligns to the other |
| CRP-1 | CfdbQueryAdapter (R10-6) compiles against 5 port traits while using 1 (20% < 25% threshold) | BLOCKING | `CodeFacts` trait must be in a separate `ports-codefacts` crate or behind a Cargo feature in `ports` |
| OCP-1 | L3 modification point in diff engine not named | advisory | Add a sentence naming the future modification site (e.g. "a new `domain/src/diff/member.rs` sub-pass") |
| pre-existing | `adapter-markdown` depends on `adapter-rust` for `normalize` — peer adapter coupling | advisory | File issue; not RFC-010 scope |

---

## Cross-Lens Convergences

- **Clean-arch lens** should examine whether `(module, crate)` on `ConceptNode` is an infrastructure leak (domain types carrying build-system vocabulary). The SRP finding above is independent: even if clean-arch ratifies the field names, the assembly logic still needs separation.
- **DDD lens** should examine the "module" granularity question (finding LSP-1) from the homonym angle: is "module" in graph-specs the same concept as "Module" in cfdb? If the two tools use the same word for different granularities, that is a homonym — the Published Language invariant (RFC-002) requires them to agree.
- **Rust-systems lens** should assess whether the `SectionState` struct at `adapters/markdown/src/lib.rs:219` has room to absorb H1-context tracking without exceeding the complexity budget, or whether a second state struct is needed.
