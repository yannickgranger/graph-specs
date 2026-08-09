---
title: RFC-015 — spec retirement state: a second marker value, its two marker records, and the obligation rule the edge pass was never given
status: Draft (revision 3) — four rounds of lens findings folded; awaiting §2.3 four-lens review of this text
date: 2026-08-09
authors: agentry-captain-2026-08-09
companion: yg/agentry docs/rfc/RFC-spec-state-marker.md §11 (Amendment A2, council-produced, operator-ratified 2026-08-09)
prior-art: RFC-013 (spec state marker — this RFC amends its §3.1 "One legal value" clause, its §3.3 marker carrier, and its §3.4 obligation-skip rule), RFC-014 §3.3 (which adopted RFC-013 §3.4's rule verbatim and inherited its scoping — amended here too), RFC-012 §3.3 (anchor resolution as backing-item presence)
---

# RFC-015 — spec retirement state

**Revision 3.** Revision 1 was reviewed by three lenses and returned
REQUEST CHANGES from each (`clean-arch` C0–C3, `ddd` D1–D8, `solid`
F1–F12). This revision folds every blocking condition. §5 records what
the review found, including the three findings that were withdrawn by
their own authors after re-execution — that record is load-bearing and
is not decoration.

## §1 — Problem

RFC-013 gave the marker **one** legal value, `draft`, and made
ratification the deletion of the line. That models a concept's birth.
It does not model its death.

**There is no legal intermediate commit for a removal.** Whoever moves
first is red: code-first is `missing_in_code`, spec-first is
`missing_in_specs`. The only green shape is a single atomic spec+code
edit, which the upstream consumer structurally cannot author — its spec
and code layers are written by different actors, and its coder is denied
`specs/` outright.

Upstream ran the same removal three times
(`brf_3065_s6_assertion_scope_removal` v3–v5) and each died on:

```
edge missing in code: Assertion --DEPENDS_ON--> AssertionScope (brief_contract.md:54)
```

The coder's diff was correct every time. The edge diff was doing its
job. The state machine has no state for this.

### §1.1 — The second problem, which is the larger one

The edge pass consults an edge's **source** concept and never its
**target**. So a marked heading reds the tree the moment any live
concept declares `- depends on:` it — which makes a pre-landed marked
heading unusable for precisely the concepts most worth pre-landing.

**This is a defect in the RULE, not in the code.** RFC-013 §3.4 scopes
its skip to *"all code-obligating checks **sourced at** that heading"*,
then examines the edge pass and concludes it *"already satisfies it by
construction"*. That is true source-side and silent target-side, and
`domain/src/diff/edge.rs:35` is faithful to it — it filters on
`e.source_concept`, and `marked` is read in `diff/concept.rs` and at
`diff.rs:166`, never in the edge pass.

**The defective sentence has five carriers**, found by grepping its
shared words:

| # | Carrier | Layer |
|---|---|---|
| 1 | `docs/rfc/013-spec-state-marker.md:163` | origin |
| 2 | `docs/rfc/014-grounding-polarity.md:217` | adopted **verbatim** (`:219`) |
| 3 | `specs/concepts/equivalence.md:393-394` | enforced |
| 4 | `specs/dialect.md:283-285` | enforced |
| 5 | `specs/dialect.md:359-362` | enforced |

**Why it propagated, and the reason is grammatical rather than
careless.** All five carriers are **one-place predicates governing a
two-place check.** Each phrases the rule as a property of *a heading*.
But `EdgeMissingInCode` ranges over an ordered **pair** — a source
heading's bullet and a target heading — and the finding is keyed to one
endpoint: `edge.rs:76` constructs it with `concept: spec_edge.source_concept`
and `diff.rs:322` sorts it the same way. **A one-place sentence can only
reach the endpoint the finding is keyed to, and that endpoint is the
source.**

So *"sourced at"* was never sloppy vocabulary. It is the only scoping a
one-place phrasing can express, and the omission was forced by the
sentence's arity. That explains what carelessness cannot: why RFC-014
§3.3 adopted the rule verbatim while looking directly at the ungated
pass list and still drew only the source-side conclusion, and why no
amount of careful re-reading surfaces it. **You cannot re-read a
one-place sentence into a two-place gap.**

It also disposes of the option this RFC's own first revision reached
for. **There is no correct statement of this rule anywhere in the tree**,
and none of the five carriers is promotable. `specs/dialect.md:361-362`
— *"A heading that compels nothing cannot be missing anything"* — is
closer only in *vocabulary*, obligation rather than mechanism; it is
still wrong in arity, because on `## S - depends on: Ghost` the finding
fires against **S**, and S compels plenty. **The canonical statement
must be authored, not promoted.** §3.4 authors it.

**Grounding, stated as measured rather than as a bare number.** Upstream
carries **19 marked headings** = 18 `pending` (marked, item absent) + 1
`realized` (marked, item present), against 15 distinct edge targets,
intersection empty. The load-bearing figure is the **18**: the defect
fires only on marked-and-absent targets. This repo carries **no marked
heading at all** and 34 distinct edge targets, so the case is
unreachable here today and self-dogfood cannot exercise it (§7).

## §2 — Scope

**Ships.**

1. A second legal marker value, `retired`, under the existing
   `- status:` prefix and the existing front-matter trigger.
2. Two new marker records for its two matrix rows, both non-violation.
3. **The obligation rule stated once and directionally complete**
   (§3.4), with the edge pass and the verb pass both consuming it.

**Amendment scope.** This RFC amends **RFC-013 §3.1** (one legal value),
**§3.3** (the marker carrier), **§3.4** (the obligation-skip rule) and
**RFC-014 §3.3** (which adopted §3.4's rule verbatim). Amending only
RFC-013 would leave the identical defective sentence ratified in
RFC-014, and the next author reading it — as the adopting author already
did once — would get the un-amended rule.

**The dated RFC files are NOT edited.** RFC-015 declares the amendment;
`013-spec-state-marker.md` and `014-grounding-polarity.md` stand
untouched. Silently correcting a ratified RFC would erase the trail that
made this class visible — it was found by grepping the shared words. A
design record that gets corrected in place stops being a record.

**Does not ship.**

- Any change to `EdgeMissingInSpec`, under any marker, on either
  endpoint (§4 invariant 5).
- Any new violation variant, including an anti-resurrection one (§6).
- Any change to **`polarity:` values or semantics**. The obligation set
  the edge pass consumes already carries non-`Declared` names
  (`diff.rs:113-117`), so polarity targets are covered as a consequence;
  that is a consistency repair, not a polarity change. §6 names the
  resulting silence as a deliberate non-goal.
- **`unbound`'s under-enforcement** (§3.4) — pre-existing, RFC-014's
  implementation, filed as #187. RFC-015 is an **obligation-axis** RFC;
  that is a **binding-axis** defect. A category boundary, not a scope
  judgement — which is why no amount of goodwill lets this RFC absorb
  it (§6).
- Any NDJSON breaking change. `schema_version` stays `"4"`; §3.5 carries
  the precedent and the one thing the precedent does not cover.

## §3 — Design

### §3.1 — Grammar: a second value, and why it is not a sequence

`- status: retired`, ASCII-case-insensitive on the value exactly as
`draft` is, in the same slot: the **first non-blank content line** after
an H2 or H3 concept heading. Trailing content is tolerated and ignored
on the same terms. The front-matter trigger accepts the value at file
scope with the same whole-file semantics.

**This amends RFC-013 §3.1's "One legal value" clause.** The clause's
*rationale* survives intact:

- `draft` declares **code owed to exist**. Ratification is deletion of
  the line.
- `retired` declares **code owed to be gone**. Written **while the
  backing item is still present**, and **never deleted**.
- **Neither value transitions to the other, and no third value exists.**
  "A presence flag, never a state machine" holds under two values as it
  did under one — there is still no transition to implement, because the
  progress axis is the **code**.

**Why not a `retiring` → `retired` pair.** The upstream council argued
it through four rounds and ruled on a cascade ground: *minimise the
amendment of ratified text.* One value amends §3.1's letter; a pair
overturns its text **and** manufactures the state machine §3.1 denies.

The pair's strongest evidence, accepted and read the other way: the
legal intermediate and a resurrection are byte-identical (verified by
tree hash). **Identical trees are identical states.** Demanding
different verdicts from them demands that the checker read history,
which it cannot. What one value gives up is the *violation form* of an
anti-resurrection tooth, stated plainly in §6.

**Mis-placement fails loud, unchanged.** A `- status: retired` bullet
that is not the first non-blank content line is inert; the heading reads
*unmarked*, and rows 1/5 fire as today.

### §3.2 — Enforcement matrix

RFC-013's six rows are unchanged. Rows 3 and 4 are `draft`-specific; two
rows are added:

| # | Heading | Pub item | Result |
|---|---------|----------|--------|
| 1 | unmarked | absent | violation `missing_in_code` (unchanged) |
| 2 | unmarked | present | pass (unchanged) |
| 3 | `draft` | absent | skipped for equivalence; `pending` record (unchanged) |
| 4 | `draft` | present | full equivalence enforced; `realized` record (unchanged) |
| 5 | no heading | present | violation `missing_in_specs` (unchanged) |
| 6 | doc declares no concept heading | — | `context_without_cohesion_unit`; a heading marked with **either** value counts as a cohesion unit |
| **7** | **`retired`** | **present** | **full equivalence enforced; `retirement_incomplete` record, non-violation** |
| **8** | **`retired`** | **absent** | **skipped for equivalence; `retirement_complete` record, non-violation** |

**Row 8 carries row 3's obligation skip in full, and this must be stated
rather than inferred.** A row-8 concept imposes no obligation through
its `- verb:` bullets, its `- impl:` anchors, or its edge bullets —
identically to row 3. §3.4 names the mechanism; the point here is that
"the mirror of row 3" is not a substitute for saying so, because
**silence resolves to armed**: the verb pass's skip is a vector built
from the `pending` list (`diff.rs:200`), a row-8 concept is not in it,
and `verb.rs:307-318` carries a control asserting the violation *does*
fire on an empty unobliged set.

**Escalation on contradiction only, unchanged.** Row 7 enforces
equivalence in full: a retired heading whose backing item exists and
whose equivalence *fails* produces that ordinary violation. Marker/code
co-presence is not itself the contradiction — under this design it is
the window every correct retirement opens.

### §3.3 — Reader, and which sites read what

`ConceptNode`'s `marked: bool` widens to carry which value was read.
Its doc comment transcribes §3.1's rationale and must be reconciled
rather than left: *"a presence flag, never a state machine"* stays true
under two values, since neither transitions; *"the only transition is
deletion of the marker"* is true of `draft` only and false of `retired`.

**The widening is load-bearing across every site that reads marker
state, and each site reads a different question.** The realizing slice
pins which sites need the *value* and which need only *markedness*, so
a later reader does not have to re-derive it: the concept pass
dispatches on the value (rows 3/4 vs 7/8); the anchor-suppression set at
`diff.rs:164-168` needs only "is marked"; the obligation verdict (§3.4)
needs only "compels no code item".

### §3.4 — The obligation rule, stated once

**The rule, non-directional and mechanism-free.** It is stated here, in
two legs, and every other carrier cites it rather than restating it.

> **`unobliged`** — this heading compels no code item to exist.
> Members: a heading marked with either value whose item is absent,
> `forbidden`, and `illustrative`. It governs existence-shaped findings:
> an `unobliged` heading neither imposes a code-existence demand through
> its own declarations, **nor bears one demanded of it by another
> heading's declarations.** *This is the whole of what RFC-015 fixes.*
>
> **`unbound`** — this heading describes no code item. Member:
> `illustrative`, alone. It governs every check presupposing that the
> heading describes that item. **Known under-enforced — see §6 and
> issue #187.**

**Two named predicates, not one rule with a caveat, and the difference
is not stylistic.** Three separate wordings of the subordinated form
were drafted during review and every one of them banned
`ForbiddenConceptReintroduced` — the finding RFC-014 exists to produce.
`forbidden` is the witness: it is **`unobliged` and bound**. The guard
consumes the code node precisely because *"the heading documents it, as
banned"* (`concept.rs:111-113`), and the violation carries `code_source`
from the matched node, so it is definitionally a check over a bound pair.

The trap is that the member **sets** nest — `unbound` ⊂ `unobliged` —
while the **predicates** do not, and `forbidden` is the row that proves
it. **Set inclusion does not license clause subordination:** a
subordinate clause quantifies over its main clause's subject, so hanging
the binding predicate off "compels no code item" asserts it of the whole
`unobliged` extension rather than of the subset. The containment makes
the premise true and the conclusion false, which is why three careful
authors made the same move. **The subordinate form is forbidden here on
grammatical grounds, with `forbidden` named as the witness.**

Defining `unbound` by **membership** rather than by a polarity-class
quantifier is what makes `ForbiddenConceptReintroduced` safe by
construction rather than by a carve-out someone has to reason about.

Required properties differ by predicate, because their arities differ.
`unobliged` must be **two-place** — `EdgeMissingInCode` ranges over two
headings, so reaching the target endpoint requires it — and stated in
**obligation vocabulary**, never mechanism, so it cannot go stale when
the pass structure changes. `unbound` is **one-place**:
`ConceptContextMismatch` ranges over one heading and its code item, and
misfires not because a second endpoint went unconsulted but because it
presupposes a binding the heading refuses. Stating it two-place would
assert a symmetry that does not exist. **Both must carry explicit
boundedness** — complete, or visibly incomplete with a reference. That
is the property whose absence produced this entire class.

**What the rule is keyed on: `unobliged`, never the literal value.**
The suppression applies when the target **compels no code item** —
covering `draft` targets (§1.1's defect), `retired` targets, and
non-`Declared` polarity alike. A key on the `retired` value would fix
retirement and leave the creation defect open.

**What answers "compels no code item": the concept pass's own row
verdicts**, never a re-derived name-match set. RFC-013 §3.4 ruled that
"backing item present" is one fact with **two** spellings — name match
or resolved `- impl:` anchor — and the candidate sets in the tree
(`matched_concepts` at `diff.rs:128-132`, the anchor snapshot at
`:164-168`) encode only the first. The row verdicts come out of the
dispatch that already routes both spellings, so consuming them inherits
the ruled predicate by construction rather than re-implementing it.
`diff.rs:200-201` is the precedent: the verb pass is already told, from
the concept pass's own output, under a name that names the
**consequence** and never a source.

**Both passes are told; neither infers.** The edge pass stops inheriting
the rule "by construction" and consumes the obligation verdict, as the
verb pass already does. One derivation, two consumers — this removes a
carrier of "does this heading compel a code item?" rather than adding a
fifth.

**Ordering and precedence, two mechanical consequences.** The obligation
verdict is built after the concept pass, so the edge pass consumes it
from there; the realizing slice confirms the reorder is observationally
inert against the declared order-independence. And `edge.rs:67` tests
`EdgeTargetUnknown` **before** the missing-in-code arm: the suppression
is a guard inside the `else if !matched` arm only, and
`known_concepts` is **never** filtered — filtering it would silently
convert one finding into another rather than suppress.

**`EdgeMissingInSpec` is NEVER suppressed** — under either value, on
either endpoint. It is the only checker-side detector of a
`- depends on:` bullet deleted while its code edge still lives.

### §3.5 — Records, rendering, and cleanliness

**Record names.** `retirement_incomplete` (row 7) and
`retirement_complete` (row 8). Both are observational — they predicate
the artifact, not an actor's obligation — and symmetric in form. The
upstream council's `debt`/`census` are not used: `debt` names an actor's
obligation that §6 puts out of scope, is a history word where the
checker sees only a tree, and already names the companion tool's product
in this repo's own positioning (`README.md:90-96`); `census` is a
collective noun predicated of single rows, never totalled, and carries
none of row 8's terminality. These become Published Language the moment
Slice A merges, and `specs/ndjson-output.md` classifies no `marker`-value
rename — there is no clean retraction path, so the cheapest moment is
now.

**Rendering and cleanliness are two rules, and only one of them is
about zero.** `application/src/text.rs:47-52` states the rendering rule:
every count is always represented, even at zero, because *"an absent
segment would be indistinguishable from a formatter that forgot to
render it."* Cleanliness is a different question: which counts must be
zero.

- **Rendering** — all lists always represented: `pending`, `realized`,
  and both retirement records. As drafted in revision 1, row 8 was in
  neither rule, so a tree with 0 and a tree with 50 printed an identical
  summary.
- **Cleanliness** — `0 violations, 0 realized-unratified, 0
  retirement-incomplete`. `pending` is rendered and is not a cleanliness
  term; it is the transcription worklist. `retirement_complete` is
  rendered and is not a cleanliness term; it never drains, and a
  never-draining term inside the clean state makes the clean state
  unreachable.

```
retirement incomplete: AssertionScope (specs/concepts/brief_contract.md:56)
retirement complete: PrePushRebaseDecision (specs/concepts/agent_contract.md:665)
0 violations, 2 pending, 0 realized-unratified, 1 retirement-incomplete, 1 retirement-complete
```

**NDJSON.** Two new values under the existing `marker` discriminator.
`schema_version` stays `"4"`, on the precedent at
`specs/ndjson-output.md:344` — RFC-013's marker suppression and
RFC-014's two polarity narrowings *"change which headings qualify, not
what the discriminator means"*, all ruled Additive without a bump. **The
one thing that precedent does not cover** is the rung: the edge
exemption changes which **edges** qualify, not which headings. It is the
same class one rung down, and this RFC rules it Additive on that basis
rather than by silent extension. `specs/ndjson-output.md:433`
(§Determinism) is extended for the two new lists.

### §3.6 — CLI

No new subcommand, no new flags.

## §4 — Invariants

1. **A marker never parks a spec↔code divergence.** Rows 4 and 7 both
   enforce equivalence in full. Precisified from revision 1: no
   spec↔code divergence is parked — the suppressed edge cannot exist in
   code, because the target does not — but the design does create
   assertions true by suppression rather than by verification, and §6
   names that class rather than leaving it inside this invariant.
2. **The unmarked tree is untouched.** Rows 1, 2, 5 byte-for-byte;
   identical violations, exit code, text (modulo the new summary
   segments) and NDJSON.
3. **Exit code is a function of violations only.**
4. **The suppression is keyed on obligation, one-directional.**
5. **`EdgeMissingInSpec` fires under every marker value and every
   polarity, on both endpoints.**
6. **Self and cross dogfood stay at zero findings** — and prove only
   invariant 2, not the feature (§7).
7. **No transition exists to implement.** Neither value advances.

## §5 — Architect lenses

**Revision 1 review: three verdicts, all REQUEST CHANGES**
(`clean-arch` C0–C3, `ddd` D1–D8, `solid` F1–F12). `rust-sys` did not
return a seat; its findings reached the record through the other three
and are attributed where used. **This revision is not ratified** —
§2.3 requires four verdicts on this text.

**Three findings were withdrawn by their own authors after
re-execution**, and the record keeps them because it is evidence about
how much the surviving findings are worth:

- `clean-arch` filed, then withdrew, a claim that a name-match key would
  park a real divergence — refuted by its own fixture showing the case
  fires today with no marker present and is structurally un-actionable.
  It asked the two lenses that had adopted it to withdraw too.
- `solid` cleared the cohesion pass on the grounds that
  `ConceptContextMismatch` is code-fact-gated, then corrected itself:
  true on the marker axis, false on the polarity axis. *"I tested the
  code-absent cell and generalised from it."*
- `clean-arch` proposed the canonical rule, then corrected it as
  incomplete in exactly the way it diagnoses — one leg reading as though
  it covered the field.
- `clean-arch` and `solid` then drafted a binding leg each, and **both
  were false** — each banned `ForbiddenConceptReintroduced`, by
  different routes. `ddd` refuted both with the same witness and ruled
  two named predicates instead. Three wordings of one error, from three
  authors, is why §3.4 forbids the subordinate form on grammatical
  grounds rather than warning against it.

Two corrections were the author's. Revision 1 offered
`specs/dialect.md:361-362` as the corrected rule already in the tree;
`ddd` refuted it by execution — that sentence attributes to the wrong
heading, since the finding fires against the *source*, which does compel
a code item. And revision 2 scribed the two-leg form after the lenses
had already begun refuting it; this revision replaces it.

**The reusable finding, one level above this RFC.** A one-place sentence
cannot express a two-place rule, and a subordinate clause cannot express
an independent predicate. Both are cases of **a sentence's form silently
bounding what it can assert, with an author who is being faithful** —
which is why "sourced at" survived three RFCs and eight lens verdicts
without anyone being careless.

**Open for this review:** the canonical statement's placement (§3.4
states it; `specs/dialect.md` is ruled its home, in a section of its
own under neither axis); the `retirement_incomplete` /
`retirement_complete` names; and whether leg 2's boundary is drawn in
the right place.

## §6 — Non-goals, and the residual class

**No anti-resurrection tooth in violation form.** A resurrected concept
and an incomplete retirement are the same state and produce the same
cell, so a resurrection is **reported and merges with exit 0**.
Recovering one would require a violation class this RFC does not create.

**Assertions true by suppression rather than by verification** — the
residual class, named once with its instances enumerated rather than
accumulated as separate bullets:

1. A `- depends on:` into a target that compels no code item. Transient
   under `draft`; **permanent** under `retired`.
2. A `- depends on:` aimed at a `forbidden` name. Silent under this
   RFC. The correct detector is a *positive* finding — the bullet aims
   at an expelled name — which is a new violation class §2 does not
   ship. Named here as a deliberate non-goal with a successor, not
   discovered later as an accidental silence. The trade is still
   favourable: today that cell emits a finding whose remedy is
   self-defeating, and silence is worse than a right finding and better
   than a harmful one.

**What this RFC changes about the two predicates, which refutes the
collapse a reader will reach for.** `retired`+absent joins `unobliged`
— it compels no code item, which is exactly the argument for row 8's
verb-pass skip. It does **not** join `unbound`: with no code item there
is nothing for a binding-presupposing check to fire on, and row 7
describes its item in full.

| | `unobliged` | `unbound` |
|---|---|---|
| today | `draft`+absent, `forbidden`, `illustrative` | `illustrative` |
| after RFC-015 | `draft`+absent, **`retired`+absent**, `forbidden`, `illustrative` | `illustrative` |

**One predicate grows in this RFC and the other does not.** A reader
tempted to collapse them as *"`unbound` is just a special case of
`unobliged`"* is refuted by the document they are reading.

**Naming, as measured.** `unobliged` has **nine call sites** across
`domain/src/diff.rs` and `domain/src/diff/verb.rs`, and **zero** in
`specs/` or `docs/rfc/`. `unbound` has **zero hits anywhere**. The
concept every lens converged on had a name in the code and none in the
intent layer — the most economical account of how a defective sentence
survived three RFCs and eight lens verdicts: the right word already
existed, in the one place no RFC author reads.

**`unbound` is under-enforced, and the adjacent instance is two cells
with two different mechanisms.** `ConceptContextMismatch` fires on a
non-`Declared` heading — asserting that the heading's declared context
disagrees with the code item's, on a premise the polarity falsifies:

- **`illustrative`** — the code node is deliberately retained
  (`concept.rs:123`, *"No `remove` — that is the whole point"*), so the
  cohesion pass sees it.
- **`forbidden`** — the guard *does* remove the node
  (`concept.rs:113`), but the removal never reaches the cohesion pass,
  which reads `code_for_context`, a separate clone taken at
  `diff.rs:75-79`. This is the worse cell: the tree says both *"this
  name is expelled, remove the item"* and *"this heading's declared
  context disagrees with where the item lives"* — guidance about where
  to put an item the other finding says must not exist.

**The marker axis is clean on the binding axis** — rows 3 and 8 have no
code node, and row 7 firing a mismatch is correct, since row 7 enforces
equivalence in full. This is a **polarity-axis defect only**,
pre-existing, RFC-014's implementation, not created by this RFC and not
closed by the obligation verdict (the cohesion pass consumes no
obligation verdict, and `declared_contexts` is snapshotted at
`diff.rs:86` before the concept pass runs). **Filed as #187; not a
deliverable of these slices.**

**Not the polarity axis.** `polarity:forbidden` means *this name was
never legitimate*; retirement means *this concept was legitimate and was
removed by decision X*. They differ in re-entry rules — and that
difference is **upstream-governed and not on the wire**, since the
checker sees a tree and never a sequence. They are distinct in meaning,
in re-entry, and in **every code-present cell** — `draft`→`realized`,
`retired`→row 7, `forbidden`→`ForbiddenConceptReintroduced`,
`illustrative`→`MissingInSpecs` — and they **converge in every
code-absent cell**, where none compels a code item and one obligation
set treats them alike. They differ there in *reporting*, not in
obligation. That is a well-formed design, and the axes stay
distinguishable exactly where a distinction has consequences.

**No authoring rules.** Whether a heading may be deleted, whether a
removed `- depends on:` must be recorded, and who may write a marker are
upstream's fences.

**No history reading.** The checker sees a tree, never a sequence.

## §7 — Issue decomposition

Both slices carry a **spec co-land**, at RFC-013 §7's granularity. This
repo's dual control makes it a gate: *"adding a new concept / trait /
output variant is specs-gated."* Revision 1 omitted it entirely and
prescribed a self-dogfood inertness that its own deliverables falsify —
an implementer delivering against that prescription would have had to
break something to pass it.

**Corpus reality, quantified.** This repo carries **0 marked headings**
against **34** distinct edge targets. Self- and cross-dogfood are
structurally incapable of exercising either slice and prove only
invariant 2. Real coverage is Unit plus **Integration fixture** — and
that row is deliberately *not* called "target dogfood", which
`docs/cross-fixture-bump.md:62` reserves for qbot-core at a pinned SHA
and `:64` attaches a decision procedure to under which a failure is
*"new signal on the rescue target — that is the payload, not a bug"*.
That is false of an author-written fixture, and §2.4 copies these blocks
into the issue body verbatim.

### Slice A — the value parses and both records emit end-to-end

Grammar (§3.1), rows 7–8 (§3.2), the reader widening and site table
(§3.3), the two record kinds, both formatters (§3.5).

**Spec co-land, same PR:** `specs/concepts/equivalence.md` — two new
`##` headings plus the `- depends on:` bullets on `## CheckOutcome`
(`:355-385`); `specs/dialect.md:238-241` (the "One legal value" clause
§3.1 falsifies), `:262-267` (the marker-effect table gains rows 7–8) and
`:269-270` (the exit-code note); `specs/ndjson-output.md:40`, `:433`
(§Determinism) and `:437`, plus §Marker records.

**Arity.** `concept_pass` is at 6 parameters and goes to 8 with two more
record sinks, past clippy's default of 7 under `-D warnings` with
pedantic and nursery. The slice acknowledges the decision without
prescribing a shape — and note that any parameter object absorbing the
sinks is a `domain` pub type, so it is specs-gated too: **the arity fix
and the spec co-land are one blocker, not two.**

```
Tests:
  - Unit: value parse (both values, case-insensitivity, trailing-parenthetical
    tolerance, mis-placement inert → missing_in_code fires); row 7 vs row 8
    selection driven by the SAME "backing item present" fact as rows 3/4 —
    exercised in BOTH spellings, name match and resolved `- impl:` anchor;
    row 8's obligation skip over `- verb:` bullets and `- impl:` anchors.
  - Self dogfood: 0 findings — proves invariant 2 only, NOT the feature;
    the slice is inert here only after the spec co-land lands, and the
    slice's own `graph-specs check --specs specs/ --code .` must return 0.
  - Cross dogfood (cfdb at pinned SHA): 0 findings, same reason.
  - Integration fixture: one retired+present and one retired+absent concept
    emit exactly one retirement_incomplete and one retirement_complete,
    exit 0, schema_version "4"; the summary renders every list at zero.
  - Target dogfood: none — rationale: no live tree carries the value.
```

### Slice B — the obligation rule, both consumers

The rule of §3.4 stated once and consumed by the edge pass and the verb
pass, replacing the source-side inheritance.

**Spec co-land, same PR — the three prose carriers**, which correct here
rather than in Slice A because the two-directional rule lands here:
`specs/concepts/equivalence.md:393-394` (becomes a **citation**, never a
statement — a universal rule stated under `## PendingRecord` is scoped
to one record kind and is false there after this RFC);
`specs/dialect.md:283-285` and `:359-362` (both cite the canonical
statement). The canonical statement itself lands in `specs/dialect.md`
**in a section of its own**, under neither the markers section nor the
polarity section — placed under either, one meaning drawing on two
sources reads as that axis's rule which the other happens to obey.

```
Tests:
  - Unit: the six-cell matrix {unmarked, draft, retired} × {target item
    present, absent}, asserting suppression in exactly the two
    compels-nothing-and-absent cells; plus the anchored-presence cell (a
    marked target backed by a resolved `- impl:` is PRESENT, not absent).
  - Unit: EdgeMissingInSpec fires in all six cells — invariant 5, and the
    one-directionality of the exemption.
  - Unit: EdgeTargetUnknown precedence — a suppressed target yields no
    EdgeTargetUnknown, a genuinely unknown target still does, and
    known_concepts is never filtered (pinned today at diff/tests.rs:341).
  - Unit: the polarity cross-check — an edge into a non-`Declared` target
    is suppressed by the same rule, and its reverse tooth still fires.
  - Unit: the missing mirror of diff/tests.rs:671, which pins the source
    side while its comment claims the edge pass "satisfies it by
    construction" — more than it proves. The target-side assertion for a
    draft-marked absent target is §1.1's defect and is red today.
  - Self dogfood: 0 findings — invariant 2 only.
  - Cross dogfood (cfdb at pinned SHA): 0 findings.
  - Integration fixture: the §1 shape reaches 0 violations; the same
    fixture with the target's item still PRESENT reaches 1, proving the
    key is obligation and not the marker alone.
  - Target dogfood: none — rationale: no live tree carries the value.
```

**Slice ordering is forced.** B's rule keys on a state only A can parse,
and §1.1's defect is closed by B, not A.
