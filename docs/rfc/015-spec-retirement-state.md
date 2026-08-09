---
title: RFC-015 — spec retirement state: a second marker value, its two marker records, and the one edge exemption
status: Draft — awaiting §2.3 four-lens architect review
date: 2026-08-09
authors: agentry-captain-2026-08-09
companion: yg/agentry docs/rfc/RFC-spec-state-marker.md §11 (Amendment A2, council-produced, operator-ratified 2026-08-09)
prior-art: RFC-013 (spec state marker — this RFC amends its §3.1 "One legal value" clause AND its §3.4 skip rule, whose source-side scoping leaves the target side uncovered), RFC-014 (grounding polarity — a different axis, §6), RFC-012 §3.3 (anchor resolution as backing-item presence)
---

# RFC-015 — spec retirement state

## §1 — Problem

RFC-013 gave the marker **one** legal value, `draft`, meaning *declared
ahead of code*, and made ratification the deletion of the line. That
models a concept's birth. It does not model its death, and the omission
is not cosmetic.

**There is no legal intermediate commit for a retirement.** Removing a
concept means removing a heading and its backing item. Whoever moves
first is red: code-first is `missing_in_code`, spec-first is
`missing_in_specs`. The only green shape is a single atomic spec+code
edit. The upstream consumer cannot author that shape — its spec layer
and its code layer are written by different actors under different
permissions, and its coder is denied `specs/` outright — so the one
green path is unreachable there, and the paths that are reachable are
all red.

The concrete trigger. Upstream ran the same removal three times
(`brf_3065_s6_assertion_scope_removal`, v3–v5, 2026-08-07/08). Each
attempt died on this finding:

```
edge missing in code: Assertion --DEPENDS_ON--> AssertionScope (brief_contract.md:54)
```

The coder's diff was correct every time. The finding's only fix lay in
a directory that actor may not write, and no state existed that made
the half-done removal legal. Upstream's three-strike backstop tripped,
and the diagnosis was not "the checker is wrong" — the edge diff is
doing exactly its job — but "the state machine has no state for this".

**A second, independent motivation: RFC-013 §3.4 is incomplete, and this
RFC amends it.** The gap is in the *rule*, not in its implementation.

§3.4 scopes its skip to "all code-obligating checks **sourced at** that
heading", then examines the edge pass and concludes "the edge pass
already satisfies it by construction (its matched-concept filter is
built from code presence)". That conclusion is **true for the source
side and silent on the target side**, and the code is faithful to it:
`domain/src/diff/edge.rs` filters on `e.source_concept`, and `marked` is
read in `diff/concept.rs` and nowhere in the edge pass.

The consequence is that **a marked heading reds the tree the moment any
live concept declares `- depends on:` it** — which makes a pre-landed
marked heading unusable for precisely the concepts most worth
pre-landing. §3.4's own stated rationale reaches this case and its rule
does not: *"firing `EdgeMissingInCode` on a pending concept would make
row 3 unreachable in practice"* is exactly as true when the pending
concept is the edge's target.

Nothing has fired yet, and the reason is corpus accident rather than
design. This repo carries **no marked heading at all** (`0 pending`), so
the case is unreachable here today. Upstream carries 18 pending
concepts and 15 distinct edge targets whose intersection is **empty**
(executed 2026-08-09) — one `- depends on:` bullet naming a pending
concept would fire it. Slice B repairs the rule.

## §2 — Scope

**Ships.**

1. A second legal marker value, `retired`, under the existing
   `- status:` prefix and the existing front-matter trigger.
2. Two new marker records for its two matrix rows, both non-violation.
3. **One** suppression rule on `EdgeMissingInCode`, keyed on a
   conjunction (§3.4). This **amends RFC-013 §3.4**: that rule's
   "checks sourced at that heading" scoping, and its finding that the
   edge pass "already satisfies it by construction", hold source-side
   only.

**Does not ship.**

- Any change to `EdgeMissingInSpec`, under any marker, on either
  endpoint (§4 invariant 5).
- Any new violation variant, including an anti-resurrection one (§6).
- Any change to `polarity:` (RFC-014) — a different axis (§6).
- Any NDJSON breaking change: two added discriminator values are
  additive, so `schema_version` stays `"4"` per §Schema evolution.

## §3 — Design

### §3.1 — Grammar: a second value, and why it is not a sequence

`- status: retired`, ASCII-case-insensitive on the value exactly as
`draft` is, in the same slot: the **first non-blank content line** after
an H2 or H3 concept heading. Trailing content on the line — the upstream
`(per <RFC>.md §<clause>)` convention — is tolerated and ignored on the
same terms as today. The front-matter trigger accepts the value at file
scope with the same whole-file semantics.

**This amends RFC-013 §3.1's "One legal value" clause.** The clause's
*rationale* survives intact and is worth restating, because the obvious
next proposal is the one this RFC rejects:

- `draft` declares **code owed to exist**. Ratification is deletion of
  the line.
- `retired` declares **code owed to be gone**. It is written **while
  the backing item is still present**, and it is **never deleted**. The
  heading survives carrying it, permanently.
- **Neither value transitions to the other, and no third value exists.**
  RFC-013's "presence flag, never a state machine" holds under two
  values as it did under one: there is still no transition to
  implement, because the progress axis is the **code**, not a second
  marker.

**Why not a `retiring` → `retired` pair.** It is the obvious design and
a future reader will propose it again. The upstream council argued it
through four rounds and ruled against it on a cascade ground rather than
a design preference: *minimise the amendment of ratified text.* One
value amends §3.1's letter; a pair overturns its text **and** manufactures
a state machine §3.1 denies. The design arguments are genuinely balanced,
and where that is so the smaller amendment wins.

The pair's strongest evidence, accepted and read the other way: the legal
intermediate and a *resurrection* are byte-identical (verified upstream by
tree hash). **Identical trees are identical states** — same marker, same
code, same owed work. Demanding different verdicts from them would demand
that the checker read history, which it cannot and must not. What one
value gives up is the *violation form* of an anti-resurrection tooth,
stated plainly in §6 rather than glossed.

**Mis-placement fails loud, unchanged.** A `- status: retired` bullet
that is not the first non-blank content line is inert; the heading reads
*unmarked*, and rows 1/5 fire as today.

### §3.2 — Enforcement matrix

RFC-013's six rows are unchanged. Rows 3 and 4 are hereby read as
`draft`-specific; two rows are added:

| # | Heading | Pub item | Result |
|---|---------|----------|--------|
| 1 | unmarked | absent | violation `missing_in_code` (unchanged) |
| 2 | unmarked | present | pass (unchanged) |
| 3 | `draft` | absent | skipped for equivalence; `pending` record, non-violation (unchanged) |
| 4 | `draft` | present | full equivalence enforced; `realized` record, non-violation (unchanged) |
| 5 | no heading | present | violation `missing_in_specs` (unchanged) |
| 6 | doc declares no concept heading | — | violation `context_without_cohesion_unit` (unchanged; a heading marked with **either** value counts as a cohesion unit) |
| **7** | **`retired`** | **present** | **full equivalence enforced; `retirement_debt` record, non-violation** |
| **8** | **`retired`** | **absent** | **skipped for equivalence; `retirement_census` record, non-violation** |

Row 7 is the mirror of row 4 and row 8 the mirror of row 3: in both
lifecycles the **code-present** cell carries the owed work and the
**code-absent** cell carries none.

**Escalation on contradiction only, unchanged and load-bearing.** Row 7
enforces equivalence in full: a retired heading whose backing item exists
and whose equivalence *fails* produces that ordinary violation. Marker/code
co-presence is **not** itself the contradiction — under this design it is
the window every correct retirement opens, by the design's own requirement
that the marker be written while the code is still there. Never escalation
by age, count, or branch.

### §3.3 — Reader

`ConceptNode`'s `marked: bool` (RFC-013 §3.3) widens to carry which value
was read. The field's doc comment transcribes §3.1's rationale and must be
reconciled rather than left: *a presence flag, never a state machine* stays
true under two values — neither transitions — but *"the only transition is
deletion of the marker"* is true of `draft` only, and false of `retired`,
which is never deleted. The reconciled comment states both.

Everything else in §3.3 is unchanged: the graph remains the single carrier
of marker state, and anchors/verbs/edges under a marked heading are
extracted as normal.

### §3.4 — Diff: the two records, and the one exemption

**Two new marker record kinds**, siblings of `Pending`/`Realized` in
`domain`, produced by the diff, never affecting the exit code — the
placement argument of RFC-013 §3.4 applies unchanged:

- `RetirementDebt { concept, spec_source }` — row 7. Emitted **in
  addition to** the fully enforced equivalence checks for that pair. It
  is the worklist entry: *this code is owed to be gone.*
- `RetirementCensus { concept, spec_source }` — row 8. Emitted instead
  of `MissingInCode`. The terminal record; it never drains.

**The exemption, and it must be a conjunction.**

> `EdgeMissingInCode` is suppressed when the edge's **target** concept
> carries the `retired` marker **AND** its backing item is **absent**.
> One direction only.

Both single-condition keys were executed upstream against real trees and
both are refuted:

- **Marker alone parks a real divergence.** Target marked, target item
  still *present* (the window row 7 describes), source field dropped for
  unrelated reasons: the genuine `edge missing in code` would be
  suppressed for the whole window. RFC-013 §3.2 forbids this in the words
  the whole marker design rests on — *a marker can never park a real
  divergence.*
- **Item-absence alone moves an existing matrix row.** Target *unmarked*,
  item absent, bullet present is today's 2-violation cell; suppressing on
  absence alone silently changes it.

Only the conjunction preserves both: the marked-with-item-present case
still fires, the unmarked row is untouched, and the retirement's second
commit goes green.

The cause of the defect is structural: the edge pass filters on
`e.source_concept` with **no target-side consultation**. Adding that
consultation is what closes both this RFC's case and the latent creation
defect of §1 — the same one-line-of-reasoning gap, reached from two
directions.

**`EdgeMissingInSpec` is NEVER suppressed** — under either value, on
either endpoint. It is the only checker-side detector of a
`- depends on:` bullet deleted while its code edge still lives. Upstream
governs that act with its own diff fence; this checker must not blind
itself to it under cover of the retirement mechanism.

### §3.5 — Outcome type and formatters

`CheckOutcome` widens with the two record lists (final names the
implementer's, reconciled at review). Exit code stays a function of
`violations` alone.

**Text** — enumerated one per line, both lists always represented even at
zero, exactly as RFC-013 §3.5:

```
retirement debt: AssertionScope (specs/concepts/brief_contract.md:56)
retirement census: PrePushRebaseDecision (specs/concepts/agent_contract.md:665)
0 violations, 2 pending, 0 realized-unratified, 1 retirement-debt
```

The **clean state** reads `0 violations, 0 realized-unratified, 0
retirement-debt`. `pending` and the census are printed beside it and are
not terms in it — the two cells that hold no owed work.

**NDJSON** — two new values under the existing `marker` discriminator.
Additive per §Schema evolution, so **`schema_version` stays `"4"`**; no
existing discriminator value stops being emitted.

```json
{"schema_version":"4","marker":"retirement_debt","concept":"AssertionScope","source":{"kind":"spec","path":"specs/concepts/brief_contract.md","line":56}}
{"schema_version":"4","marker":"retirement_census","concept":"PrePushRebaseDecision","source":{"kind":"spec","path":"specs/concepts/agent_contract.md","line":665}}
```

### §3.6 — CLI

No new subcommand, no new flags.

## §4 — Invariants

1. **A marker never parks a divergence.** Rows 4 and 7 both enforce
   equivalence in full. The exemption's conjunction exists solely to keep
   this true; a marker-only key would break it.
2. **The unmarked tree is untouched.** Rows 1, 2, 5 are byte-for-byte
   today's behavior, and a tree carrying no marker of either value
   produces identical violations, exit code, text (modulo the new summary
   segment) and NDJSON (`schema_version` unchanged at `"4"`).
3. **Exit code is a function of violations only.**
4. **The exemption is one-directional and conjunction-keyed.** Neither
   condition alone suppresses anything.
5. **`EdgeMissingInSpec` fires under every marker value, on both
   endpoints.**
6. **Self dogfood and cross dogfood stay at zero findings.** Neither this
   repo's `specs/` nor the pinned cfdb companion carries a marker of
   either value.
7. **No transition exists to implement.** Neither value advances; there
   is no code path that rewrites one marker into another.

## §5 — Architect lenses

*Awaiting §2.3 review. Four verdicts required (clean-arch, DDD,
SOLID + components, rust-systems), each RATIFY, or a single
author-documented override recorded inline. Not ratified until then.*

Questions this draft deliberately leaves to the lenses rather than
pre-deciding:

- **DDD:** are `retirement_debt` / `retirement_census` the right names
  in this bounded context's ubiquitous language, given RFC-013 §3.4
  already ruled `marker` over `report` to avoid a second meaning for
  `record`? "Debt" and "census" are the upstream council's words, not
  necessarily this context's.
- **Clean-arch / SOLID:** the exemption requires the edge pass to consult
  target-side marker state it does not read today. Where does that
  consultation live so it does not duplicate the concept/code match the
  diff already performs?
- **Rust-systems:** the `marked: bool` widening (§3.3) — the shape is the
  implementer's, but the field is load-bearing across every pass that
  reads marker state.

## §6 — Non-goals

- **No anti-resurrection tooth in violation form.** A resurrected concept
  and a not-yet-completed retirement are the same state and produce the
  same cell, so a resurrection surfaces as a nonzero `retirement_debt`
  term — regenerated every run, named at `file:line` — and **exits 0**.
  Stated in the form the upstream council settled on, because softer
  wordings let a reader picture something in CI noticing, and nothing
  does: *the design has no anti-resurrection tooth; a resurrected concept
  is reported and merges with exit 0.* Recovering one would require a
  violation class this RFC does not create.
- **No `retiring` intermediate value** (§3.1).
- **Not the polarity axis.** `polarity:forbidden` (RFC-014) means *this
  name was never legitimate*; retirement means *this concept was
  legitimate and was removed by decision X*. They differ in re-entry
  rules — a retired concept may legitimately be re-created under a new
  ratified decision, a banned name may not — and a non-`Declared`
  polarity substitutes the equivalence finding out entirely, which §3.2
  forbids.
- **No authoring rules.** Whether a heading may be deleted, whether a
  removed `- depends on:` must be recorded, and who may write a marker
  are upstream's fences, enforced in that tree. This RFC governs only
  what the checker reads and reports.
- **No history reading.** The checker sees a tree, never a sequence.

## §7 — Issue decomposition

### Slice A — the value parses and both records emit end-to-end

Grammar (§3.1), matrix rows 7–8 (§3.2), the reader widening (§3.3), the
two record kinds (§3.4), and both formatters (§3.5). Vertical: a marker
in a spec file reaches NDJSON and text output in one slice.

```
Tests:
  - Unit: value parse (both values, case-insensitivity, trailing-parenthetical
    tolerance, mis-placement inert); row 7 vs row 8 selection from
    (value, backing-item presence).
  - Self dogfood (graph-specs on graph-specs): unchanged at 0 findings —
    this repo carries no marker of either value (invariant 6), so the slice
    must be observationally inert here.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): unchanged at 0
    findings, same reason.
  - Target dogfood (on a synthetic fixture, since no live tree carries the
    value yet): a fixture with one retired+present and one retired+absent
    concept emits exactly one retirement_debt and one retirement_census,
    exit 0, and schema_version stays "4".
```

### Slice B — the edge exemption

Target-side marker consultation in the edge pass, keyed on the
conjunction (§3.4).

```
Tests:
  - Unit: the conjunction's truth table, all four cells — marked+present,
    marked+absent, unmarked+present, unmarked+absent — asserting suppression
    in exactly one, and asserting EdgeMissingInSpec fires in all four.
  - Self dogfood (graph-specs on graph-specs): unchanged at 0 findings.
  - Cross dogfood (graph-specs on cfdb at pinned SHA): unchanged at 0
    findings.
  - Target dogfood (synthetic fixture): the §1 shape — a live concept
    declaring `- depends on:` a retired concept whose item is absent —
    reaches 0 violations; the same fixture with the item still present
    reaches 1, proving the conjunction rather than the marker alone.
```

**Slice ordering is not free.** Slice B's suppression keys on a value only
Slice A can parse. A also repairs the latent creation defect only once B
lands, since B is where target-side consultation is added — so §1's second
motivation is closed by B, not A.
