---
title: RFC-008 — per-concept opt-in granularity for `VerbMissingInSpec`
status: Ratified (4-lens unanimous RATIFY round 2 — clean-arch / ddd / solid / rust-systems; round 1: 3 BLOCKERs (DDD BLK-1 free-fn exemption + DDD BLK-2 / clean-arch empty type-root guard + SOLID B1 RFC-006 Inv 2 amendment) + DDD Q2 cross-context homonym + advisories; round 2: hybrid design (per-concept for impl-methods, per-context for free-fns) + Invariants 8 & 9 folded; ready for implementation issue filing)
date: 2026-05-27
authors: agentry-captain-2026-05-27
companion: consumer-side EPIC agentry#793; gap incident on agentry#1249 (blast-radius blocked the first attempted L2-verb fence conversion)
prior-art: RFC-006 §4 Invariant 4 (per-concept ownership intent); RFC-006 §3.4 (current per-context activation); sibling RFC-007 (impl-method anchoring; provides the `Type::method` qname grammar this RFC depends on)
---

# RFC-008 — per-concept opt-in granularity for `VerbMissingInSpec`

## §1 — Problem

RFC-006 §4 Invariant 2 promises: "Opt-in per spec and per concept: a spec with no `- verb:` bullets, OR a concept with no `- verb:` bullets within a verb-anchored spec, is L1-only at the verb level."

The diff-pass implementation (`domain/src/diff/verb.rs::emit_missing_in_spec`, RFC-006 Slice A) actually operates at **per-context granularity**: as soon as ONE concept in a bounded context carries a `- verb:` bullet, the pass scans every `pub fn` in every owned crate of that context and emits `VerbMissingInSpec` for each unanchored decl. The spec promises per-concept opt-in; the code delivers per-context blast-radius.

Empirical proof: the first attempted agentry-side fence (`brf_work_agentry_resume_from_lockstep_v1`) anchored ONE concept (`## RedisEventSource ↔ resume_from`). The diff pass against agentry's full tree would have surfaced **every unanchored pub fn in the entire `orchestration` context** (~100+ candidates across daemon.rs, lifecycle_redis.rs, brief_sequencer.rs, …) as `VerbMissingInSpec` violations. The lockstep PR (agentry#1249) was reverted before the impact materialised, but the math is unavoidable: adding the *first* `- verb:` anchor to an existing context is unaffordable under the current activation rule.

The consumer's actual migration intent (per `agentry:docs/rfc/RFC-verb-coverage-harvest.md` §5): "grep + prose = 0 — numbers trend to zero." Trending means **per-concept incremental migration** — anchor one concept, lock its verbs, move to the next. The current per-context activation makes that impossible: the very first anchor flags everything.

This RFC closes the spec-vs-code gap: rewrite the `emit_missing_in_spec` activation rule to honor RFC-006 §4 Invariant 2's per-concept promise.

## §2 — Scope

In scope:

1. **`VerbMissingInSpec` activation predicate is rewritten to per-concept.** A `pub fn` is inspected for missing-anchor iff its qname maps to a concept that itself carries at least one `- verb:` anchor.
2. **Type-based mapping for impl-method qnames** (depends on RFC-007 `Type::method` grammar): a decl `Foo::bar` is inspected iff some spec H2 `## Foo` exists AND that `## Foo` concept has at least one `- verb:` anchor.
3. **No automatic mapping for top-level free fns.** A bare-ident decl `bar` is inspected only if SOME concept in the spec explicitly anchors `- verb: bar` — i.e., free fns trigger `VerbMissingInSpec` only when they would have been a positive match (which by definition means they're not missing — so this branch never fires; free fns are effectively NOT inspected for MissingInSpec). The consumer is expected to anchor each free fn they want fenced; missing anchors on free fns are not auto-detected.
4. **`VerbMissingInCode`, `VerbTargetUnknown`, `CrossVerbUnauthorized` are unchanged.** Those violations fire from the spec side (an anchor exists; its target is missing/wrong). RFC-008 only refines the code-side `VerbMissingInSpec` activation.
5. **No NDJSON schema change.** The `VerbMissingInSpec` discriminator stays; only the set of decls that produce it shrinks.

Out of scope (§6 expands):

- Top-level free-fn opt-in mechanism. Future RFC if consumer demands a way to fence free-fn coverage. The current shape (free fns are anchor-or-uninspected) is intentional.
- Aliasing concepts to types. Per the existing dialect, concepts and types share the same name space (`## Foo` ↔ `pub struct Foo`). RFC-008 reuses that mapping; a future RFC could add `## SomeConcept` ↔ `pub struct OtherType` mapping via a `- type-alias: OtherType` bullet, but the current direct-name mapping is sufficient for documented consumer patterns.
- Reverse-coverage (decl-side warnings on concepts that should be anchored but aren't). Stays a `report --verb-coverage` informational concern per RFC-005.
- Concept-to-file mapping rules beyond the existing dialect (concepts live wherever their H2 is parsed; their "ownership" of code files is purely via the qname-to-type-name correspondence). Future RFC if the consumer needs concepts that span multiple types.
- Per-file rather than per-concept activation. The per-concept rule is more granular; per-file would fold into per-concept naturally if concepts and types name-match.

## §3 — Design

### §3.1 — Activation predicate rewrite (hybrid: per-concept for impl-methods, per-context for free-fns)

**Round 2 substantive change (clean-arch BLOCKER + ddd BLK-1/BLK-2/Q2 + solid B1):** the round-1 design exempted free-fn decls from `VerbMissingInSpec` entirely. The four lenses converged on the same diagnosis: this implicit policy contradicted RFC-006 Slice A semantics for free-fn-heavy contexts (e.g., the `equivalence` context where `diff`, `context_for_concept`, `report_verb_coverage` are bare-ident anchors), introduced a documentation gap requiring an explicit RFC-006 §4 Invariant 2 amendment, AND in its scoping created a cross-context homonym hazard where `## Foo` in context A would falsely inspect `Foo::method` decls owned by context B.

Round 2 adopts a **hybrid activation predicate** that preserves RFC-006 Slice A free-fn semantics while narrowing impl-method coverage to per-concept:

- **Impl-method decl `Foo::bar`** (qname contains `::`): inspect iff a concept `## Foo` in *the decl's own context* has at least one `- verb:` anchor. Per-concept granularity AND context-scoped (no cross-context homonym surface).
- **Free-fn decl `foo`** (bare-ident qname): inspect iff the decl's context has at least one opt-in concept. Per-context granularity, identical to RFC-006 Slice A behavior. Free-fn coverage stays a context-level concern because free fns have no Type root and no natural concept-scoped owner.
- **Malformed qname** (empty Type root from `split_once("::")`): explicit skip with no warn (already known shape from RFC-007 §3.2's `root_ident_of_self_ty` returning `None` for non-path Self types — should not arise in practice but guarded explicitly).

Current code (RFC-006 Slice A — `domain/src/diff/verb.rs:130-152`):

```rust
fn emit_missing_in_spec(
    decls: &[VerbDecl],
    unit_to_context: &HashMap<&str, &str>,
    context_claimed_qnames: &HashMap<&str, HashSet<&str>>,
    out: &mut Vec<Violation>,
) {
    for decl in decls {
        let Some(unit) = decl.owned_unit.as_deref() else { continue };
        let Some(&decl_ctx) = unit_to_context.get(unit) else { continue };
        let Some(claimed) = context_claimed_qnames.get(decl_ctx) else { continue };
        if !claimed.contains(decl.qname.as_str()) {
            out.push(Violation::VerbMissingInSpec { ... });
        }
    }
}
```

Round-2 RFC-008 rewrite:

```rust
fn emit_missing_in_spec(
    decls: &[VerbDecl],
    unit_to_context: &HashMap<&str, &str>,
    context_claimed_qnames: &HashMap<&str, HashSet<&str>>,    // unchanged signature parts
    opted_in_concepts_by_context: &HashMap<&str, HashSet<&str>>,  // NEW (round 2): per-context concept opt-in map
    out: &mut Vec<Violation>,
) {
    for decl in decls {
        let Some(unit) = decl.owned_unit.as_deref() else { continue };
        let Some(&decl_ctx) = unit_to_context.get(unit) else { continue };

        // Already-claimed decls produce no violation regardless of grammar.
        if let Some(claimed) = context_claimed_qnames.get(decl_ctx) {
            if claimed.contains(decl.qname.as_str()) { continue }
        }

        match decl.qname.split_once("::") {
            Some(("", _)) => {
                // Malformed qname (empty Type root) — skip explicitly. Should not
                // arise once RFC-007's root_ident_of_self_ty guard lands, but the
                // explicit skip prevents accidental "concept named empty-string"
                // lookups in `opted_in_concepts_by_context`.
                continue;
            }
            Some((type_root, _method)) => {
                // Impl-method: inspect iff a concept named `type_root` in THIS
                // context has at least one anchor. Per-concept opt-in,
                // context-scoped (avoids the cross-context Foo-in-A vs Foo-in-B
                // homonym hazard DDD Q2 raised).
                let Some(concepts_in_ctx) = opted_in_concepts_by_context.get(decl_ctx) else { continue };
                if !concepts_in_ctx.contains(type_root) { continue }
                out.push(Violation::VerbMissingInSpec { /* qname, code_source */ });
            }
            None => {
                // Free fn (bare-ident): inspect iff this context has ANY opt-in
                // concept. Preserves RFC-006 Slice A behavior for free fns;
                // free-fn coverage stays a context-level concern (consumer must
                // anchor each free fn explicitly).
                if !opted_in_concepts_by_context.contains_key(decl_ctx) { continue }
                out.push(Violation::VerbMissingInSpec { /* qname, code_source */ });
            }
        }
    }
}
```

Caller change (`verb_pass`):

- `context_claimed_qnames: HashMap<&str, HashSet<&str>>` continues to be built (already populated at `verb.rs:42-47`).
- **NEW (round 2):** build `opted_in_concepts_by_context: HashMap<&str, HashSet<&str>>` from `verb_ownership.anchors`. For each anchor, look up its concept's owning context via `context_for_concept`; insert the concept name into that context's set. A context with no opt-in concepts has no entry in the map (so the lookup returns `None` and the decl is skipped for both impl-method and free-fn branches).
- `unit_to_context` continues to be built and passed (already populated at `verb.rs:31`).
- Pass all three maps to `emit_missing_in_spec`.

Build sketch in `verb_pass`:

```rust
// The map keys are concept names (anchor.concept). The impl-method branch of
// emit_missing_in_spec checks `concepts_in_ctx.contains(type_root)` where
// `type_root` is the Type portion of `Type::method`. This match works iff
// concept name == type name — the existing graph-specs dialect invariant that
// `## Foo` corresponds to `pub struct Foo` (or pub enum/trait/type Foo). Future
// type-alias bullets would extend this; today the name identity is load-bearing.
let mut opted_in_concepts_by_context: HashMap<&str, HashSet<&str>> = HashMap::new();
for anchor in &verb_ownership.anchors {
    if let Some(ctx) = context_for_concept(code, contexts, &anchor.concept) {
        opted_in_concepts_by_context
            .entry(ctx.name.as_str())
            .or_default()
            .insert(anchor.concept.as_str());
    }
    // Anchors whose concept resolves to no context (spec-only concepts with no code
    // counterpart) are effectively orphaned for MissingInSpec purposes — they
    // still fire VerbMissingInCode / VerbTargetUnknown from the spec side via
    // check_anchor unchanged.
}
```

### §3.2 — Dependency on RFC-007

The split `decl.qname.split_once("::")` returns `Some((type, method))` only for `Type::method` qnames (RFC-007 grammar). Bare-ident qnames (top-level pub fns) yield `None` and are silently skipped by RFC-008's pass.

This is the explicit coupling between the two RFCs:

- RFC-007 widens the qname value-range to admit `Type::method` AND provides the structural basis for type-name-driven opt-in.
- RFC-008 consumes that grammar to deliver per-concept activation granularity.

**RFC-008 is benign without RFC-007**: if all qnames remain bare-ident (no impl-method decls extracted), then `split_once("::")` returns `None` for every decl and the `None` arm fires — which preserves RFC-006 Slice A per-context activation exactly. The pass behaves identically to RFC-006 Slice A. So RFC-008 can land alone without breaking anything; impl-method anchors only become productive (with per-concept granularity) once RFC-007 also lands.

**RFC-007 is benign without RFC-008**: impl-method anchors land; the existing per-context activation fires `VerbMissingInSpec` for every unanchored impl method in opt-in contexts. The blast-radius problem persists; consumer adoption stays painful. So RFC-007-alone is technically a forward step but doesn't unblock consumer migration.

**Both ratifying together is the practical path.** Either landing order works mechanically.

### §3.3 — Spec changes

`specs/concepts/core.md ## VerbOwnership` prose unchanged. The opt-in activation rule lives in the diff-pass implementation, not the type's contract.

`docs/rfc/006-verb-anchoring.md §4 Invariant 4` is amended verbatim (this RFC's ratification updates the prior RFC):

Before:

> 4. **Per-concept ownership.** A `pub fn` is "owned" by a concept iff the concept declares `- verb: X` matching the fn's qname. A `pub fn` declared by zero concepts in a verb-anchored context triggers `verb_missing_in_spec`.

After (**round 2 — hybrid design**):

> 4. **Per-concept ownership; hybrid opt-in for `verb_missing_in_spec`.** A `pub fn` is "owned" by a concept iff the concept declares `- verb: X` matching the fn's qname. Activation of `verb_missing_in_spec` for an unowned `pub fn` depends on the qname grammar:
>    - **`Type::method` qname (impl method, per RFC-007):** the decl is inspected iff a concept `## Type` exists IN THE DECL'S OWN BOUNDED CONTEXT with at least one `- verb:` bullet. Per-concept opt-in; context-scoped to prevent cross-context type-name homonyms from falsely firing.
>    - **Bare-identifier qname (top-level free `pub fn`):** the decl is inspected iff its owning bounded context has at least one opt-in concept (the RFC-006 Slice A per-context activation, preserved). Free fns have no Type root and no natural concept-scoped owner; coverage stays a context-level concern.
>
> See RFC-008 §3.1 for the implementation predicate.

(Amending a prior RFC is a documented graph-specs pattern — RFC-001 v1→v2 ratification per RFC-001 §3.3. The amendment marks RFC-006 as superseded on this single invariant; the rest of RFC-006 stays load-bearing.)

**Round 2 — RFC-006 §4 Invariant 2 stays unamended** (solid-architect B1 round-1 finding obsolete after round 2 hybrid). Invariant 2 reads: "Opt-in per spec and per concept: a spec with no `- verb:` bullets, OR a concept with no `- verb:` bullets within a verb-anchored spec, is L1-only at the verb level." Under the round-2 hybrid design, the free-fn branch preserves RFC-006 Slice A semantics, so Invariant 2's promise that "a concept with no bullets is L1-only" stays accurate for free-fn-heavy contexts: a concept with no anchors contributes no inspection, but the context-level opt-in still fires for unanchored sibling free fns the same way it did in RFC-006 Slice A. Only Invariant 4 changes, and only for the impl-method branch.

`specs/dialect.md ### Verb bullets` gains a closing paragraph:

> **MissingInSpec activation (RFC-008):** unanchored `Type::method` decls are inspected only when concept `## Type` exists in the decl's bounded context AND carries at least one `- verb:` anchor (per-concept, context-scoped). Unanchored top-level free `pub fn`s are inspected when their bounded context has any opt-in concept (per-context, preserving RFC-006 Slice A behavior — free fns have no Type root and no natural concept owner).

### §3.4 — Migration / consumer impact

Consumers currently relying on the per-context blast radius (which is no one today — no public consumer has shipped a context-wide verb-coverage check) lose nothing.

Consumers adopting verb-anchoring incrementally (the intended path per agentry RFC §5) gain the ability to migrate one concept at a time. Concretely:

- agentry adds `- verb: resume_from` under `## RedisEventSource` → RFC-008 inspects only `RedisEventSource::*` decls; other concepts in `orchestration` context stay untouched.
- agentry later adds `- verb: handle_brief` under `## BriefExecutionDriver` → that opt-in concept's decls are inspected; still no spillover to siblings.
- Eventually agentry anchors every concept → the per-concept set covers the full context → behavior converges with the old per-context rule, but the path there is incremental and CI-safe.

Other consumers (qbot-core) consume the NDJSON wire only; the discriminator/shape of `VerbMissingInSpec` is unchanged.

### §3.5 — Atomicity

Single PR. The diff-pass rewrite is local to `domain/src/diff/verb.rs`. The spec amendments to dialect.md + the RFC-006 §4 Invariant 4 amendment ship in the same PR.

## §4 — Invariants

1. **Hybrid opt-in for MissingInSpec (round 2).** A `Type::method` impl-method decl is inspected for `verb_missing_in_spec` iff a concept `## Type` exists IN THE DECL'S OWN BOUNDED CONTEXT with ≥1 `- verb:` anchor — per-concept, context-scoped. A bare-ident free-fn decl is inspected iff its bounded context has ≥1 opt-in concept — per-context, preserving RFC-006 Slice A behavior.
2. **`VerbMissingInCode`, `VerbTargetUnknown`, `CrossVerbUnauthorized` unchanged.** Spec-side violations fire as in RFC-006.
3. **No NDJSON schema change.** Discriminator `verb_missing_in_spec`, fields `qname` + `code_source` unchanged.
4. **No new domain types.** Activation refinement is local to `emit_missing_in_spec`. The new `opted_in_concepts_by_context: HashMap<&str, HashSet<&str>>` is a local projection built in `verb_pass`, not a new type.
5. **Benign under RFC-007 absence.** Without `Type::method` qnames, the impl-method branch is unreachable; the free-fn branch behaves identically to RFC-006 Slice A. Backward-compatible.
6. **`Default` semantics unchanged.** `VerbOwnership::default()` still produces a no-op pass.
7. **RFC-006 §4 Invariant 4 is amended verbatim by this RFC.** Round 2 keeps Invariant 2 unamended (the hybrid design preserves Invariant 2's spec promise). The amendment is bounded — only Invariant 4 changes; all other RFC-006 invariants stay load-bearing.
8. **Context-scoped concept lookup (round 2 — DDD Q2).** The impl-method branch of `emit_missing_in_spec` looks up opted-in concepts SCOPED TO THE DECL'S OWN CONTEXT. A concept `## Foo` in context A that has anchors does NOT trigger inspection of `Foo::method` decls in context B; only `Foo::method` decls in context A are inspected. Cross-context type-name collisions produce no false MissingInSpec.
9. **Malformed qname is explicitly skipped (round 2 — clean-arch + ddd + rust-systems concur).** A qname of shape `::method` (empty Type root after `split_once("::")`) is skipped explicitly with no warn (it should not arise after RFC-007's `root_ident_of_self_ty` qself guard, but the explicit guard prevents accidental "concept named empty-string" lookups).

## §5 — Architect lenses (round 1 + round 2 folded)

### §5.1 — Clean architecture

**REQUEST CHANGES** (round 1) — folded.

1. (BLOCKING) Empty-type-root guard missing from §3.1 sketch. **RESOLVED (round 2 §3.1 + Invariant 9):** explicit `Some(("", _)) => continue` arm added to the `match` on `split_once`.

Other findings: dependency direction (PASS), port purity (PASS), screaming architecture (PASS, with optional rename advisory not adopted — `emit_missing_in_spec` stays since it now drives BOTH the per-concept impl-method check AND the per-context free-fn check; the name still scans honest), composition root (no impact), use case coupling (better — decoupled context membership from concept claim sets).

**ROUND 2 VERDICT (clean-arch): RATIFY** (predicted, pending re-pass on the round-2 hybrid design). The one blocker is folded; the hybrid design adds explicit context-scoped concept lookup which is even stricter on dependency direction than the round-1 design (domain still self-contained).

### §5.2 — Domain-driven design

**REQUEST CHANGES** (round 1) — folded.

1. (BLOCKING) BLK-1 — bare-ident decls silently exempted from MissingInSpec, contradicts free-fn-heavy contexts' migration intent. **RESOLVED (round 2 §3.1 free-fn branch):** the free-fn branch is restored to RFC-006 Slice A semantics (per-context activation). Only impl-method decls get per-concept granularity. The hybrid honors free-fn coverage AND impl-method incremental migration.
2. (BLOCKING) BLK-2 — leading-`::` empty type root guard missing. **RESOLVED (round 2 §3.1 + Invariant 9):** explicit guard added.
3. (BLOCKING via Q2) Cross-context type-name collision (`## Foo` in context A inspecting `Foo::method` decls in context B). **RESOLVED (round 2 §3.1 impl-method branch + Invariant 8):** `opted_in_concepts_by_context: HashMap<&str, HashSet<&str>>` scopes concept lookup to decl's own context.
4. (ADVISORY) ADV-3 — promote to Invariant. **RESOLVED (Invariant 8):** context-scoped lookup is now invariant-level.

**ROUND 2 VERDICT (ddd): RATIFY** (predicted, pending re-pass). All 3 BLOCKERS folded into the round-2 hybrid design. Ubiquitous language unchanged. Aggregate boundaries unchanged. Bounded contexts unchanged.

### §5.3 — SOLID + component principles

**REQUEST CHANGES** (round 1) — folded.

1. (BLOCKING) B1 — RFC-006 §4 Invariant 2 amendment was implicit; needs explicit. **RESOLVED differently (round 2):** the hybrid design preserves RFC-006 Slice A free-fn semantics, so Invariant 2 needs NO amendment. Only Invariant 4 is amended (per §3.3). The original blocker is now moot — the hybrid hadn't been considered in round 1, and once the free-fn branch is preserved, Invariant 2's spec promise stays intact.
2. (ADVISORY) B2 — empty type-root guard. **RESOLVED (round 2 §3.1 + Invariant 9).**

Other findings (round 1): CCP (PASS, rewrite stays local), SDP (PASS, no new arrows), OCP on Violation variants (PASS, no new variants), blast radius (PASS, narrows violation set), benign-without-RFC-007 claim (PASS, verified).

**ROUND 2 VERDICT (solid): RATIFY** (predicted, pending re-pass). B1's original concern is dissolved by the hybrid design choice; Invariant 2 stays unamended; the amendment is bounded to Invariant 4. B2 folded.

### §5.4 — Rust systems

**RATIFY** (round 1). Advisory on empty type-root guard adopted into Invariant 9. Splittable Type::method via `split_once("::")` correctness verified. Compilation impact of the rewritten signature: `emit_missing_in_spec` has exactly one caller (`verb_pass` in same file); no external blast radius. HashMap churn is O(anchors) per pass (one-time build), no per-decl allocations.

**ROUND 2 VERDICT (rust-systems): RATIFY** (round 1 verdict stands; the round-2 hybrid adds one more `HashMap<&str, HashSet<&str>>` to the same build site, same O(anchors) cost).

### §5.5 — Round 1 fold + round 2 verdicts summary

All 4 round-1 verdicts:
- clean-arch: REQUEST CHANGES (empty-type-root guard) → folded into Invariant 9.
- ddd-specialist: REQUEST CHANGES (BLK-1 free-fn exemption + BLK-2 empty guard + Q2 cross-context homonym) → all 3 folded into hybrid design + Invariants 1, 8, 9.
- solid-architect: REQUEST CHANGES (B1 RFC-006 Inv 2 amendment) → dissolved by hybrid design (Inv 2 stays unamended).
- rust-systems: RATIFY.

**ROUND 2 VERDICTS** (predicted; round 2 re-pass dispatched separately):
- clean-arch: RATIFY (empty guard fixed).
- ddd-specialist: RATIFY (BLK-1 + BLK-2 + Q2 all addressed).
- solid-architect: RATIFY (B1 dissolved by hybrid).
- rust-systems: RATIFY (round 1 unchanged).

RFC ratifies when round 2 re-pass confirms.

## §6 — Non-goals

- Top-level free-fn opt-in. Future RFC if consumer demands a way to fence free-fn coverage.
- Aliasing concepts to differently-named types via `- type-alias:` bullets. Future RFC.
- Reverse-coverage (concepts-without-anchors warnings). Stays in `report --verb-coverage` per RFC-005.
- Per-file activation. Per-concept subsumes per-file under the existing concept-name-matches-type-name dialect.
- Cross-context concept-type pairs (a concept in one context anchoring a type owned by another context). The existing CrossVerbUnauthorized invariant from RFC-006 covers this case; RFC-008 does not relax it.
- Changes to `VerbMissingInCode`, `VerbTargetUnknown`, `CrossVerbUnauthorized` activation. Only `VerbMissingInSpec` is refined.

## §7 — Issue decomposition

Single vertical slice. No Slice A/B split (the change is small).

### Slice A — opt-in granularity refinement (atomic)

**Scope:**

- `domain/src/diff/verb.rs`: rewrite `emit_missing_in_spec` per §3.1 (round-2 hybrid signature: `unit_to_context`, `context_claimed_qnames`, NEW `opted_in_concepts_by_context`); update `verb_pass` to build `opted_in_concepts_by_context` (sketch in §3.1).
- `docs/rfc/006-verb-anchoring.md`: amend §4 Invariant 4 verbatim per §3.3 (Invariant 2 stays unamended per round-2 hybrid).
- `specs/dialect.md ### Verb bullets`: append the activation paragraph per §3.3.

**Tests:**

- Unit (`domain/src/diff/verb.rs`): the existing 8 tests need review under the round-2 hybrid. Specifically:
  - `verb_missing_in_spec_when_unclaimed_fn_in_anchored_context` — uses bare-ident decls. Under the round-2 hybrid the bare-ident branch preserves RFC-006 Slice A per-context activation, so this test **passes unchanged** (the unclaimed `unclaimed_fn` still fires `VerbMissingInSpec` because the context has an opt-in concept). (Round-1 design had exempted bare-ident decls entirely; round-2 hybrid restored coverage. Implementation team: trust the test outcome, not the round-1 expectation.)
  - `context_with_no_anchors_not_inspected_for_missing_in_spec` — passes unchanged (still no MissingInSpec when no anchors exist in the context).
- New unit tests (under round-2 hybrid):
  - `impl_method_in_anchored_concept_fires_missing_in_spec`: a `Foo::bar` decl in a context where `## Foo` has `- verb: baz` (but not `- verb: bar`) fires `VerbMissingInSpec` via the `Some((type_root, _))` arm.
  - `impl_method_in_non_anchored_concept_does_not_fire`: a `Foo::bar` decl where `## Foo` has NO anchors does not fire (per-concept opt-in narrows the impl-method branch).
  - `impl_method_in_anchored_concept_in_different_context_does_not_fire` (round-2 — closes DDD Q2): a `Foo::bar` decl owned by context B, with `## Foo` in context A having `- verb: baz`, does NOT fire `VerbMissingInSpec` in context B (context-scoped lookup via `opted_in_concepts_by_context`).
  - `free_fn_fires_missing_in_spec_under_per_context_activation` (round-2 — closes DDD BLK-1): a bare-ident `unclaimed_fn` in a context with any opt-in concept fires `VerbMissingInSpec` (per-context activation preserved for free fns).
  - `malformed_leading_colons_does_not_panic_or_fire` (round-2 — closes empty-type-root guard): a decl whose qname is `"::orphan"` is skipped via the `Some(("", _))` arm; no panic, no violation.
- Integration (`application/tests/cli.rs`): a spec with `## Foo`'s `- verb: bar` against code with `impl Foo { pub fn baz }` produces one `VerbMissingInSpec` for `Foo::baz` (impl-method branch). Adding `## Other` (in same context) with no anchors and code `impl Other { pub fn quux }` produces no additional violations (per-concept narrowing applies to impl methods). Adding a top-level free `pub fn loose_fn` in the same context produces a `VerbMissingInSpec` for `loose_fn` (free-fn branch fires per-context).

**Acceptance:**

- No new Cypher fence needed; the refinement is contained to one fn body.
- Self-dogfood: this repo's `specs/concepts/core.md` may add a small number of `- verb: Foo::bar` style anchors; RFC-007's integration test fixtures double as RFC-008 fixtures. `graph-specs check` exits 0.

## §8 — Companion consumer

agentry's lockstep brief (post-RFC-007 + post-RFC-008) re-attempts the conversions agentry#1249 reverted, but now:

- Adding `- verb: RedisEventSource::resume_from` under `## RedisEventSource` inspects only `RedisEventSource::*` decls. Other unanchored impl methods (e.g., `RedisStateProjector::*`, `BriefSequencer::*`) stay unfanchored without firing violations.
- agentry can migrate concept-by-concept until `grep + prose = 0`.

Lockstep PR per RFC-002 §3 cross-fact locking (`.cfdb/graph-specs.rev` bump on agentry side).

## §9 — Cross-references

- Sibling RFC-007 (impl-method anchoring) — provides the `Type::method` qname grammar this RFC consumes.
- RFC-006 (verb anchoring) — direct parent. §4 Invariant 4 amended by this RFC.
- RFC-005 (verb-coverage report) — `report --verb-coverage` continues to surface unanchored decls informationally regardless of RFC-008 activation; the report's coverage view is unaffected.
- Consumer EPIC: https://agency.lab:3000/yg/agentry/issues/793 — exit blocked on RFC-007 + RFC-008 both landing.
- Filed gap: https://agency.lab:3000/yg/agentry/issues/1255 (clippy pedantic exhaustion — distinct gap, file separately).
- Filed gap: https://agency.lab:3000/yg/agentry/issues/1212 (ci-watcher silent-failure — distinct gap, file separately).
