---
title: RFC-009 — clear diagnostic when code implements a draft spec
status: Ratified
date: 2026-05-31
authors: agentry-captain-2026-05-31
companion: yg/agentry#1379 item 2
prior-art: PR #118 (status:draft suppression — draft headings are now retained as a no-obligation index); specs/dialect.md ## Draft specs
---

# RFC-009 — `ImplementsDraftConcept` diagnostic

## §1 — Problem

PR #118 (`fix/draft-spec-suppression`) added `status: draft` front-matter to suppress draft headings from the concept-equivalence check. The suppression is intentionally asymmetric: a `status: draft` heading **imposes no code-existence obligation** (code is not required to exist yet). However, if code implementing that draft-spec heading already exists, it surfaces as a *generic* `MissingInSpecs` orphan — indistinguishable from a forgotten spec.

This is the gap documented in yg/agentry#1379 item 2: the diagnostics for "code with no spec anywhere" and "code whose spec exists but is still draft" are conflated. A developer reading `MissingInSpecs: Widget` cannot tell whether to (a) write a spec for Widget or (b) promote the existing draft Widget spec.

## §2 — Scope

**Ships:**

1. New `Violation::ImplementsDraftConcept { name: String, draft_source: Source }` domain variant.
2. `CheckInput::draft_concepts: Vec<ConceptNode>` field + `with_draft_concepts` builder method.
3. Orphan pass in `domain::diff` branches on the draft-concept index: matching code orphans emit `ImplementsDraftConcept`; non-matching orphans continue to emit `MissingInSpecs`.
4. Text and NDJSON formatters render the new variant. `schema_version` stays `"2"` (additive change per `specs/ndjson-output.md` §Schema evolution).
5. RFC-009, `specs/concepts/core.md`, and `specs/ndjson-output.md` updated.
6. Unit tests in `domain/src/diff/tests.rs` exercised via `CheckInput::with_draft_concepts`.

**Does not ship (Slice B):**

- Markdown reader populating `draft_concepts` from `status: draft` files.
- Integration / dogfood proof that real `status: draft` specs produce `ImplementsDraftConcept` in CI.

Until Slice B, `run_check` passes an empty `draft_concepts` list, so real-run behavior is unchanged from today.

## §3 — Design

### §3.1 — Variant shape

```rust
/// A `pub` code item whose name matches a heading living in a
/// `status: draft` spec.
ImplementsDraftConcept { name: String, draft_source: Source }
```

Two load-bearing fields; no `Option`, no `bool`. `draft_source` points at the draft heading so the developer can navigate to it directly.

### §3.2 — `CheckInput` extension

```rust
pub struct CheckInput {
    pub graph: Graph,
    pub contexts: Vec<ContextDecl>,
    pub verb_ownership: VerbOwnership,
    pub draft_concepts: Vec<ConceptNode>,   // new; empty by default
}
```

Both existing `const fn` constructors (`new`, `with_graph_and_contexts`) default `draft_concepts` to `Vec::new()`, keeping all call sites unchanged. The builder `with_draft_concepts(self, Vec<ConceptNode>) -> Self` is the wiring point for Slice B and the unit test.

### §3.3 — Orphan pass

Before the `for (_, code_node) in code_by_name` loop, build a borrow map:

```rust
let draft_by_name: HashMap<&str, &Source> = draft_concepts
    .iter()
    .map(|n| (n.name.as_str(), &n.source))
    .collect();
```

Inside the loop, branch on the map:

- Match → push `Violation::ImplementsDraftConcept { name, draft_source: cloned_src }`.
- No match → push `Violation::MissingInSpecs` exactly as today.

Loop semantics are unchanged: continue-on-each-item, no short-circuit.

### §3.4 — Formatters

**Text:**
```
implements draft spec: Widget (specs/concepts/drafts.md:7) — promote the draft (flip status:, set code_landing_pr) or remove the code
```

**NDJSON:**
```json
{"schema_version":"2","violation":"implements_draft_concept","name":"Widget","draft_source":{"kind":"spec","path":"specs/concepts/drafts.md","line":7}}
```

### §3.5 — Sort slot

`violation_key` assigns slot `12` to `ImplementsDraftConcept`. Existing slots 0–11 are unchanged (no renumbering).

## §4 — Invariants

1. `schema_version` stays `"2"`. Adding a new variant is a compatible non-breaking change per `specs/ndjson-output.md` §Schema evolution.
2. When `draft_concepts` is empty (all existing callers), the orphan pass is identical to pre-RFC-009 behavior — only `MissingInSpecs` is emitted.
3. A code orphan whose name appears in `draft_concepts` emits **exactly one** `ImplementsDraftConcept` and **zero** `MissingInSpecs`.
4. `CheckInput::new` and `with_graph_and_contexts` remain `const fn`; existing call sites compile without modification.

## §5 — Architect lenses

### Clean architecture

The extension is confined to the domain crate (`Violation` enum, `CheckInput` struct, orphan pass in `diff.rs`) and the two formatter adapters. No port or adapter boundary is crossed by the new type — `draft_concepts: Vec<ConceptNode>` reuses the existing domain type. Port purity preserved. **RATIFY.**

### Domain-driven design

`ImplementsDraftConcept` is a distinct ubiquitous-language term: "the code exists; the spec heading exists but is not yet ratified." Naming it separately from `MissingInSpecs` prevents the homonym trap where one violation name means two different root causes. No new bounded context. **RATIFY.**

### SOLID + component principles

SRP: `CheckInput` gains one field; `diff` gains one map-construction statement and a branch. Both are narrow additions inside the concept-orphan responsibility. No existing responsibility widens. ISP: the `with_draft_concepts` builder is opt-in — callers that don't use it are unaffected. **RATIFY.**

### Rust systems

Two load-bearing fields with no `Option` or `bool` — correct. `HashMap<&str, &Source>` borrows from `draft_concepts` for the duration of the loop; no clone until a match is found (only `draft_source` is cloned). `Vec::new()` in `const fn` context is stable since Rust 1.64. No orphan-rule or feature-flag concern. **RATIFY.**

## §6 — Non-goals

- Slice B (markdown producer + integration proof). Filed as a companion issue.
- Per-concept opt-in for draft suppression. Draft suppression is file-level via `status: draft` front-matter.
- Any change to `MissingInCode` (spec-only orphan) — draft status is a code-side concern only.

## §7 — Issue decomposition

### Issue A — domain + formatters (this PR)

**Deliverables:** `Violation::ImplementsDraftConcept`, `CheckInput::draft_concepts` + `with_draft_concepts`, orphan-pass branch, text + NDJSON formatters, unit tests, RFC-009, spec updates.

**Tests:**
- Unit: `implements_draft_concept_when_code_orphan_matches_draft_heading` — construct `CheckInput` with empty spec graph and one `draft_concepts` entry named "Widget"; pass code graph with "Widget"; assert `ImplementsDraftConcept` fires, `MissingInSpecs` absent.
- Unit: `orphan_without_draft_match_is_missing_in_specs` — no draft concepts; code orphan still yields `MissingInSpecs`.
- Self dogfood (graph-specs on graph-specs): zero new violations (empty `draft_concepts` list in `run_check` means behavior unchanged).
- Cross dogfood: zero findings on cfdb at pinned SHA (no cfdb-side impact).

### Issue B — markdown reader + wiring (Slice B)

**Deliverables:** markdown reader retains draft headings into `draft_concepts`; `run_check` wires `with_draft_concepts`; integration fixture with a `status: draft` spec file and a matching code item asserts `ImplementsDraftConcept` in NDJSON output.

**Tests:**
- Integration fixture: synthetic `status: draft` spec file + matching code item → NDJSON contains `implements_draft_concept`.
- Self dogfood: confirm zero violations on graph-specs own tree (no draft headings currently).
- Cross dogfood: zero findings on cfdb at pinned SHA.
