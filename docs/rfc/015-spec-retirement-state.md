---
title: RFC-015 — spec retirement state: a second marker value, its two marker records, and the obligation rule the edge pass was never given
status: Ratified (revision 14, 2026-08-09) — three seats RATIFY on this text (#189, `7fff334`); the fourth seat was absent and is carried by a single author-documented override per CLAUDE.md §2.3, recorded in §5.1
date: 2026-08-09
authors: agentry-captain-2026-08-09
companion: yg/agentry docs/rfc/RFC-spec-state-marker.md §11 (Amendment A2, council-produced, operator-ratified 2026-08-09)
prior-art: RFC-013 (spec state marker — this RFC amends its §3.1 "One legal value" clause, its §3.3 marker carrier, and its §3.4 obligation-skip rule), RFC-014 §3.3 (which adopted RFC-013 §3.4's rule verbatim and inherited its scoping — amended here too), RFC-012 §3.3 (anchor resolution as backing-item presence)
---

# RFC-015 — spec retirement state

**Revision 14.** Revision 1 was reviewed by three lenses and returned
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
`domain/src/diff/edge.rs:36` is faithful to it — it filters on
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

**The pattern appears three times, not once.** Two comments in the tree
assert an inheritance that does not exist, each source-side only:
`diff.rs:193-199` — *"the edge pass inherits the same rule by
construction, via the `matched_concepts` filter above"* (RFC-013's) —
and `diff.rs:122-127` — *"which is how the edge pass inherits RFC-014
§3.3's uniform obligation skip"* (RFC-014's). This RFC's own first
drafts made it a third time, keying the target side on a source-side
predicate. Three independent authors, one shape, and the third instance
occurred inside the fix for the first two. That is the strongest
evidence available that the defect is structural rather than an
oversight.

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
- Any change to **`polarity:` values or semantics** — and the
  disclaimer is exact rather than approximate. `diff.rs:122-127` already
  states that *"a non-`declared` concept is excluded from
  `matched_concepts`, which is how the edge pass inherits RFC-014 §3.3's
  uniform obligation skip"*. That claim is **source-side only**, and the
  target side never inherited it. So this RFC does not change behaviour
  RFC-014 governs; it **completes RFC-014's own stated rule at the one
  seam that missed it**, exactly as it completes RFC-013's. The axis is
  untouched. §6 names the resulting silence as a deliberate non-goal.
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
  **Superseded in part (operator ruling 2026-08-20, recorded as
  `agentry-spec-state-marker` §12):** "never deleted" is scoped to a clean
  cascade. Until the operator calls stability, a record whose backing item
  is absent is removed. The marker's carrier, its two records and the
  obligation rule are untouched.
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

**The product with polarity, stated rather than left inferable.**
RFC-014 §3.3's precedence is **terminal and evaluated first** — the code
says so at `concept.rs:41-45`: *"evaluated first, and terminal. A
non-`declared` heading never reaches the marked dispatch below."* This
RFC extends that precedence to the new value unchanged: **a heading
carrying `- status: retired` AND a non-`Declared` polarity emits no
marker record at all**, exactly as a `draft` one does not.

It is stated because it is not inferable. RFC-014's inertness rationale
was *"there is nothing for `marked` to relax"* — true of `draft`, and it
does **not** transfer unexamined, because `retired` does not only relax:
row 7 *adds* an emission alongside full enforcement. Row 8 read alone
says a record is emitted; §3.3's precedence says none is. Today's code
answers correctly, but that is the code answering a question no RFC
asked — which is precisely why RFC-014 published its 2×3 product rather
than leaving it derivable. The rows above plus this paragraph are the
3×3.

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

### §3.4 — What a heading obliges, and what it describes

**The rule, stated as named predicates.** Each is stated once here and
cited, never restated, by every carrier.

> **`unobliged`** — this heading compels no code item to exist.
> Members: a heading marked with either value whose item is absent,
> `forbidden`, and `illustrative`. It governs the **source side**: an
> `unobliged` heading imposes no code-existence demand through its own
> declarations. This is what the verb pass and the anchor pass already
> consume.
>
> **`unpointable`** — **the members are normative and are stated
> first**: marked-with-either-value + absent; `illustrative` + absent;
> `forbidden` + absent; `forbidden` + present.
>
> **Read as a rule, derived from that list and never stated beside it**:
> this heading offers no legitimate code item to point at, and its own
> declared state accounts for that, on exactly three grounds —
> **marked**, so an item is owed to exist or owed to be gone;
> **`illustrative`**, so the heading compels nothing and an absent item
> is legitimate; **`forbidden`**, so the name is expelled and no item of
> it is legitimate, present or absent.
>
> It governs the **target side**: no heading bears a code-existence
> demand made of it by another heading's declarations. *This is what
> RFC-015 adds.*
>
> **`unbound`** — this heading describes no code item. Member:
> `illustrative`, alone. It governs every check presupposing that the
> heading describes that item. **Known under-enforced — see §6 and
> issue #187.**

**The derivation is checkable cell by cell, and it is stated that way
because spot-checking the motivating cell is how both previous versions
passed review.** Every heading state × item presence, with the ground:

| heading state | item | member? | ground |
|---|---|---|---|
| unmarked `declared` | absent | **no** | nothing accounts for the absence — that absence *is* the finding (row 1) |
| unmarked `declared` | present | **no** | there is a legitimate item to point at |
| marked (`draft`/`retired`) | absent | **yes** | the marker accounts for it: owed to exist, or owed to be gone |
| marked | present | **no** | there is a legitimate item to point at |
| `illustrative` | absent | **yes** | the heading compels nothing, so its absence is legitimate |
| `illustrative` | **present** | **no** | **the item IS a legitimate target** — see below |
| `forbidden` | absent | **yes** | the name is expelled |
| `forbidden` | present | **yes** | the name is expelled; the item's existence is itself the violation |

**The `illustrative` + present row is falsified by the checker, not just
by this list.** With the code edge actually present, executed:

```
## S - depends on: T   |   ## T <!-- polarity:illustrative -->
code: pub struct T;  pub struct S { pub f: T }

→ missing in specs: T      1 violation
  (no `edge missing in code` — the edge MATCHED)
```

The checker **accepts** an item under an `illustrative` heading as a
legitimate edge target. `polarity.rs:40-43` states why: such an item
*"falls through to the orphan sweep as `MissingInSpecs` — the marker
cannot launder unspecced public surface past the gate."* **Unspecced,
not illegitimate.** Contrast `forbidden`, where
`ForbiddenConceptReintroduced` means *remove the item*: there the name
genuinely is illegitimate, and that is the only class the expulsion
ground covers.

**The accounting clause is load-bearing and is not a restatement of the
member list.** Without it the definition reads on **item absence
alone** — and an *unmarked, `declared`* heading whose item is absent
satisfies that, while being **matrix row 1**, which invariant 2 requires
byte-for-byte:

```
## S - depends on: Ghost   |   ## Ghost   (unmarked, declared, no item)
→ missing in code: Ghost + edge missing in code: S --DEPENDS_ON--> Ghost   2 violations
```

An implementer keying on the definition rather than the enumeration
suppresses that edge and silently breaks invariant 2. **Revision 1
carried the sentence that named this exact failure** — *"item-absence
alone moves an existing matrix row: target unmarked, item absent, bullet
present is today's 2-violation cell; suppressing on absence alone
silently changes it"* — and the rewrite that introduced an
absence-keyed definition deleted it. It is restored here, because it is
the only place the document says **why** absence alone is the wrong key.
Nothing accounts for row 1's absence; that absence *is* the finding.

**The predicates are per-HEADING; the key is per-NAME. That conversion
must be stated, because the tree performs it permissively today.**
Two headings may share a name across files — `marker.rs`'s own
`record_key` contemplates it — and the edge pass keys on an edge's
target, which is a name, not a heading. `diff.rs:113-117` resolves the
collision in the permissive direction: **any** heading with that name
carrying a non-`Declared` polarity puts the name into the set.

**A name is `unpointable` only if EVERY heading carrying it is.** The
conservative direction, and it is not a preference — the permissive one
parks a real divergence, executed:

```
alpha.md  ## S  - depends on: T   |  ## T  <!-- polarity:illustrative -->
beta.md   ## T                    (declared, and it owns the code item)
code      pub struct S;  pub struct T;   (no code edge)

→ edge missing in code: S --DEPENDS_ON--> T      1 violation
```

`missing in specs: T` does **not** co-fire — the declared heading in
`beta` consumed the code node, so the orphan sweep never sees it. The
edge finding is the only violation, and a permissive key suppresses it
to `0 violations, exit 0` with a satisfiable divergence behind it. Note
this is the *canonical* use of `illustrative` — a heading in one context
illustrating a type really declared in another — not an exotic shape.

**Why the target side needs its own predicate, and why that is not
obvious.** The source-side question is *what does this heading oblige*;
the target-side question is *can this edge exist*. Those come apart in
exactly one cell: an `illustrative` heading **whose item is present**.
It compels nothing — so it is `unobliged` — but an edge into it is
perfectly satisfiable, and the remedy is clean and actionable:

```
illustrative + item present + no code edge  → edge missing in code + missing in specs   2
illustrative + item present + code edge     → missing in specs                          1
```

Adding the field clears the edge finding and introduces nothing. Keying
the target side on `unobliged` would suppress that — **parking a real
spec↔code divergence**, which RFC-013 §3.2 forbids in the words the
whole marker design rests on.

`unobliged` is a correct predicate in the wrong job, and obligation
genuinely is independent of whether code happens to exist. Two questions
answered by one name is the "sourced at" defect one turn later: a name
that fits the source side, extended to the target side without re-asking
whether the question is the same. It was caught because a lens ran the
cell rather than read the definition.

**Named predicates rather than one rule with caveats — and the names and
the guard below rest on different grounds, which matters because one of
those grounds was withdrawn.** The **names** rest on untransmissibility
(a caveat clause cannot be faithfully copied, which is this document's
own central finding one level down) and on distinct change axes,
demonstrated inside this amendment: `unobliged` and `unpointable` gain
`retired`+absent, `unbound` gains nothing.

**The guard below is separate and PROSPECTIVE.** It binds the author
Slice B creates — a fresh transcriber landing the canonical statement in
`specs/dialect.md`, handling predicates whose member sets nest, with no
access to this review. Its evidence is that three separate wordings of
the subordinated form were drafted here and every one of them banned
`ForbiddenConceptReintroduced` — the finding RFC-014 exists to produce.
That evidence is unaffected by the later ruling that the subordinated
form was never present in *this* text: the guard is not scored against
this document, and retiring it on that basis would remove it in the
revision immediately before the one where it does its only work.
`forbidden` is the witness: it is **`unobliged` and bound**. The guard
consumes the code node precisely because *"the heading documents it, as
banned"* (`concept.rs:111-113`), and the violation carries `code_source`
from the matched node, so it is definitionally a check over a bound pair.

The trap is that the member **sets** nest while the **predicates** do
not. **Two containments, each with its own witness, and the second one
has already shipped a defect:**

- **`unbound` ⊂ `unobliged`**, witness **`forbidden`** — unobliged, and
  bound. Hanging the binding predicate off "compels no code item" bans
  `ForbiddenConceptReintroduced`. Caught in drafting, three times.
- **`unpointable` ⊂ `unobliged`**, witness **`illustrative` + present**
  — unobliged, and pointable. Treating the target-side predicate as
  covered by `unobliged` on the strength of the nesting is **exactly
  what produced this document's own worst defect**, and it was caught by
  executing the cell rather than by reading the definition.

The second is the load-bearing one, on two counts. It has a **shipped**
instance rather than only drafted ones — and it is **extensionally
correct today**: *"`unpointable` is just `unobliged` minus
illustrative-with-an-item"* describes the current lattice exactly. That
makes it more dangerous, not less. A subordinate form that is wrong gets
caught; one that is right today gets adopted, and then (i) it couples
the target-side predicate to a source-side one whose membership **this
very RFC changes**, since `retired` + absent joins `unobliged`, and
(ii) it imports obligation vocabulary into a code-existence question,
which is precisely what caused G1. **Extensional correctness does not
license the subordinate form.**

**Set inclusion does not license clause subordination:** a subordinate
clause quantifies over its main clause's subject, so hanging either
predicate off "compels no code item" asserts it of the whole
`unobliged` extension rather than of the subset. The containment makes
the premise true and the conclusion false, which is why three careful
authors made the same move. **The subordinate form is forbidden here on
grammatical grounds, with `forbidden` named as the witness.**

Defining `unbound` by **membership** rather than by a polarity-class
quantifier is what makes `ForbiddenConceptReintroduced` safe by
construction rather than by a carve-out someone has to reason about.

**All three predicates are one-place, and that is a consequence of the
split rather than a coincidence.** `EdgeMissingInCode` ranges over two
headings, and a single one-place sentence cannot reach both endpoints —
which is §1.1's whole diagnosis. The resolution is not a two-place
predicate but **two one-place predicates, one per endpoint**:
`unobliged` on the source, `unpointable` on the target. The two-place
requirement stated in earlier revisions was right for a one-predicate
world and dissolved when the sentence was split. All three are stated in
**domain vocabulary**, never mechanism, so none goes stale when the pass
structure changes. `unbound` is likewise **one-place**:
`ConceptContextMismatch` ranges over one heading and its code item, and
misfires not because a second endpoint went unconsulted but because it
presupposes a binding the heading refuses. Stating it two-place would
assert a symmetry that does not exist **on the current check
inventory** — every binding-presupposing check today
(`ConceptContextMismatch`, `compare_signatures`,
`ForbiddenConceptReintroduced`, `DanglingAnchor`) ranges over one
heading and its item. That bound is stated rather than assumed: a
future two-place binding check would be unreachable from a one-place
`unbound`, and by this document's own thesis unnoticeable, so the
arity is recorded as bounded by inventory and not as a property of the
concept.

**And the durable question is not the arity at all.** *"Is this
predicate one- or two-place?"* depends on the check inventory, which is
exactly the mechanism-dependence this section forbids. The
inventory-independent question is: **does the rule carry a predicate for
every endpoint the governed checks range over?** That is what D9 did —
it did not make a predicate two-place, it added a second one-place
predicate at the endpoint that had none. Stated this way, `unbound`'s
gap becomes exactly expressible without appeal to any inventory: it
covers one endpoint, and if a two-place binding check ever appears the
repair is **a second binding predicate for the other endpoint**, in the
shape this document has already demonstrated. **All three must carry explicit boundedness** — complete, or
visibly incomplete with a reference, in enforcement **and** in arity.
That is the property whose absence produced this entire class.

**What the suppression is keyed on: `unpointable`, never the literal
value.** It applies when the target offers no legitimate item to point
at — covering `draft` targets (§1.1's defect), `retired` targets, and
the non-`Declared` cells that qualify. A key on the `retired` value would fix
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

**`unpointable`'s "absent" leg takes TWO carriers, one per member
class — a single carrier cannot serve it.** This is not a restatement — it is the
leg the earlier revision left unanswered, and the omission is
load-bearing. A **row-7 target that is anchored and resolved**
(`retired`, item present via `- impl:`, equivalence enforced in full)
reads as *absent* under a name-match set, so `unpointable` would swallow
it and suppress its edges for the whole retirement window — parking a
divergence inside the one cell row 7 exists to keep enforced. Executed:

```
## Bar  - depends on: Foo   |   ## Foo  - impl: some_fn
code    pub struct Bar;  pub fn some_fn() {}     (anchor resolves)

→ edge missing in code: Bar --DEPENDS_ON--> Foo    1 violation
```

`Foo` is backed and resolved, and absent from `matched_concepts`.

**But row verdicts cannot serve every member, and the member they fail
is the one D9 exists for.** `concept.rs:41-45` states it: *"RFC-014 §3.3
— evaluated first, and terminal. A non-`declared` heading never reaches
the marked dispatch below"*, and `continue`s. So an `illustrative`
heading produces **no row verdict at all** — while being the one member
whose presence must be answered, since `illustrative` + absent is a
member and `illustrative` + present is not. A single carrier stated for
the marked class and read as covering the field is the document's own
defect, one more turn:

| member class | needs a presence answer? | carrier |
|---|---|---|
| marked + absent | yes | the concept pass's **row verdicts**, both spellings |
| `forbidden` | **no** — both its cells are members | none needed |
| `illustrative` | yes | **code-side name presence** |

`illustrative`'s carrier is name presence and nothing more: RFC-014
OQ-4 already rules that anchors under a non-`Declared` heading fire
nothing, so the anchor spelling is vacuous there.

**Why row verdicts fail there is stronger than "the dispatch is never
reached" — the two cells are output-identical.** Executed:

```
illustrative + item PRESENT   → missing in specs: T   1 violations, 0 pending, 0 realized
illustrative + item ABSENT    →                       0 violations, 0 pending, 0 realized
```

Neither emits a marker record of any kind. So an implementer keying on
row verdicts cannot separate the member (`illustrative` + absent) from
the non-member (`illustrative` + present) **even in principle** — and
that is exactly the cell this predicate was created to discriminate.

**Name presence is right on a second, independent ground**, which
matters for a carrier rule that has already failed once: an
anchor-backed concept can never be a code-edge target at all
(`edges.rs:37-43` retains only edges whose target is a discovered code
concept; `concept.rs:47` tries the name match first, so an anchored
concept never name-matches). So for an anchored `illustrative` target
the edge is unsatisfiable regardless, and name presence gives the
correct outcome **by construction** rather than only because the anchor
question is moot there.

**The enumeration is normative and the rule is DERIVED from it — which
is a structural change, not a statement of precedence.** This membership
was previously stated three times, independently: a definition, an
enumeration, and a carrier table, each individually plausible. **No two
agreed, in two consecutive revisions, in opposite directions** — the
definition first read wider than the list (an unmarked `declared`
heading with an absent item satisfied it), and the repair then read
wider again in a new direction, admitting `illustrative` + present by
declaring no non-`Declared` item legitimate. That second reading is D9
verbatim, re-created inside the fix for D9.

The word that over-reached was one: *legitimate* is true of `forbidden`,
whose expelled name genuinely has no legitimate item, and **false of
`illustrative`**, whose item is not illegitimate but merely
*undescribed by that heading* — which is the `unbound` axis, not this
one. `illustrative` + absent is a member via **absence**, on the marked
cells' ground.

Naming a governing statement was not enough, because three independent
statements of one membership is a structure that **manufactures** this
defect rather than merely permitting it, and it did so twice running.
The rule is now a reading of the list; there is nothing left for the two
to disagree about.

**This is not a local repair — it is §3.4's own opening rule, extended
to membership.** That rule reads *"each is stated once here and cited,
never restated, by every carrier."* It governed the **predicates** and
was silent on their **membership**, and membership is what broke, twice.
Stated once and derived from is the same discipline one level down; a
future revision that restates membership anywhere new is the fourth
instance, not a new problem.

**And the shape is one this review has already diagnosed in code.** F5
found *four independent carriers of one predicate* in `diff.rs`; this is
*three independent statements of one membership* in prose. Identical
failure mode: each statement is established against the cells its author
had in mind and then asserted of the class, with nothing marking where
the narrowing stopped. Stating which carrier
answers which class is mandatory — an unanswerable leg gets answered by
whatever set is nearest to hand, which is the name-match set this
section already warns about.

**Every pass is told; none infers.** The edge pass stops inheriting its
rule "by construction" and is handed `unpointable`; the verb and anchor
passes keep consuming the obligation verdict, as they already do. **Two
derivations, not one** — the split added a predicate, and the earlier
revision's claim that the fix "removes a carrier rather than adding a
fifth" was true only while one predicate served both endpoints. What is
removed is not a carrier but an **inference**: no pass now derives its
own answer to a question the concept pass already decided.

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
   spec↔code divergence is parked, on **two** grounds and not one: in
   the absent cells the suppressed edge cannot exist, because the target
   does not; in `forbidden` + present it can exist, and is suppressed
   because its remedy is self-defeating — creating the edge requires the
   expelled item to persist, which `ForbiddenConceptReintroduced`
   forbids. The one cell answering to neither ground —
   `illustrative` + present — is **not** suppressed. Revision 3's
   single-ground justification is recorded as withdrawn rather than
   quietly replaced: it produced two false findings from two lenses in
   one round, in opposite directions, which is a better argument for
   stating both grounds than any reasoning about them. But the design does
   create
   assertions true by suppression rather than by verification, and §6
   names that class rather than leaving it inside this invariant.
2. **The unmarked tree is untouched**, and this is an independent ground
   for the `illustrative` + present exclusion rather than a restatement
   of invariant 1. That tree carries **no marker of either value** —
   `illustrative` is a polarity — so a rule that changed its violation
   count would alter a marker-free tree in a cell this RFC has no
   motivation to reach: neither the retirement arc nor §1.1's defect
   touches it. Recorded because the harm there is bounded (`polarity.rs`
   makes `MissingInSpecs` structurally certain in that cell, so it cannot
   ship green *there*) while the defect is not — §3.4's two-heading
   configuration does ship green. Bounded impact is not no defect.
   Rows 1, 2, 5 byte-for-byte;
   identical violations, exit code, text (modulo the new summary
   segments) and NDJSON.
3. **Exit code is a function of violations only.**
4. **The suppression is keyed on `unpointable`, one-directional.**
   Not on obligation: that key is what G1 refuted, and an implementer
   reading §4 for the contract must not find the superseded rule here.
5. **`EdgeMissingInSpec` fires under every marker value and every
   polarity, on both endpoints.**
6. **Self and cross dogfood stay at zero findings** — and prove only
   invariant 2, not the feature (§7).
7. **No transition exists to implement.** Neither value advances.

## §5 — Architect lenses

**Revision 1 review: four lenses, all REQUEST CHANGES**
(`clean-arch` C0–C3, `ddd` D1–D8, `solid` F1–F12, `rust-sys`). Those
verdicts were rendered against revision 1 at `951b3b9` and speak to no
later text. The round that speaks to **this** text, and the override it
rests on, are recorded in §5.1.

`rust-sys`'s seat reached the record through the other three lenses
rather than directly, and the attribution is corrected here: **D6 — the
`target dogfood` collision, its `cross-fixture-bump.md:62`/`:64`
evidence and the "payload, not a bug" argument — is entirely
`rust-sys`'s finding**, relayed under `ddd`'s number at its request. It
also filed the verb-pass half of F3 independently, supplied the general
cause for F4 (an anchor-backed concept can never be a code-edge target,
`edges.rs:37-43`), and settled the amendment-ledger fix.

**One rule about reviewing that this document earned, and it is not
about the design.** A verdict rendered on text shaped by the reviewer's
own condition must check what the fix **admits**, not only what it
**excludes** — those are different questions, and a ratification here
was reached by asking only the second. The definition written to satisfy
B1 was cleared against the cell B1 complained about, while its repair
silently admitted the cell the whole predicate exists to exclude.

**Findings withdrawn by their own authors after re-execution.** Each is
kept because a live clause elsewhere rests on it, named inline:

- The `(a)` polarity ruling was **right about the question put to it and
  wrong about a question nobody put to it** — correct for the source
  side, over-reaching on the target side, which D9 corrects. Recorded as
  a **scope** error rather than a detection error, because its author's
  own first account was harsher than the facts and a later lens checked
  whether any condition actually leaned on the harsher version. None
  did.
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
  were false as they wrote them** — each banned
  `ForbiddenConceptReintroduced`, by different routes, because each
  subordinated the binding predicate to the obligation one. `ddd`
  refuted both with the same witness and ruled two named predicates.
  **`ddd` subsequently withdrew that refutation as against this
  document**, on audit: the two predicates here carry independent
  subjects and so never inherit each other's extension. The named form
  is kept because a named predicate cannot be silently dropped in
  transcription the way a caveat can. **That rationale is withdrawn**: it
  describes a PARTIAL-transcription failure, and RFC-014 §3.3
  transcribed fully and faithfully — which is §1.1's whole force. Shipped
  as written it would have invited a future reader to check the reason,
  find it aimed at a non-occurrence, and retire the names on this
  document's own logic. The names are kept on three grounds that
  survive: they close the **layer gap** §1.1 diagnoses and §6 measures —
  a concept load-bearing in the inner layer with no name in the layer
  that governs it; **restatement is the propagation mechanism** this RFC
  documents five times, and a named predicate is cited rather than
  restated; and **#187 needs a subject**, since an unnamed deferred rule
  is precisely how a fourth carrier gets authored. Three wordings of one error from three
  authors is why §3.4 forbids the subordinate form on grammatical
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

**Put to this round, carried as ratified-as-written (§5.1):** the
canonical statement's placement (§3.4 states it; `specs/dialect.md` is
ruled its home, in a section of its own under neither axis); the
`retirement_incomplete` / `retirement_complete` names; and whether
`unbound`'s boundary is drawn in the right place.

### §5.1 — Ratification, and the one override it rests on

**Ratified at revision 14 on 2026-08-09.** Three seats returned RATIFY
against this exact text. The round is recorded in the tree rather than
in a session log: `#189` / `7fff334`, whose subject carries the tally,
and the text ratified there is byte-identical to what this file holds
apart from this section and the front matter.

**The override, stated at its true width.** CLAUDE.md §2.3 admits *"a
single author-documented override"* where the four verdicts are not all
RATIFY. RFC-005 §5.5 spent its override on one contested placement, with
three seats voting. This one is spent differently, and the difference is
the point: it carries an **absent seat**, not a dissent. `rust-sys`
rendered no verdict on this text. Nothing rejected it; the fourth chair
was empty.

What that leaves unadjudicated is nameable rather than vague. It is the
three questions the paragraph above put to this round — cleared by the
three voting lenses, and chartered to the seat that did not vote:

1. the canonical statement's placement;
2. the `retirement_incomplete` / `retirement_complete` names;
3. whether `unbound`'s boundary is drawn in the right place — the
   question #187 owns and this RFC declines (§6).

All three ship as written. `rust-sys`'s lens — crate granularity,
placement, trait-object safety — is the one chartered to press item 1
hardest, and item 1 is a placement ruling. **A reader who later finds
that placement wrong should read this section, not a lens verdict, as
its warrant.**

**§2.3's second condition is met on the face of the document:** both
slices in §7 carry a named `Tests:` block, and §2.4 copies them into the
issue bodies verbatim.

**Directed by the operator, 2026-08-09**, declining a second round on
text three seats had already cleared. Recorded inline because §2.3
requires it inline — and because an override whose warrant lives outside
the document is not documented.

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

**What this RFC changes about the three predicates, which refutes the
collapse a reader will reach for.** `retired`+absent joins `unobliged`
— it compels no code item, which is exactly the argument for row 8's
verb-pass skip. It does **not** join `unbound`: with no code item there
is nothing for a binding-presupposing check to fire on, and row 7
describes its item in full.

| | `unobliged` | `unpointable` | `unbound` |
|---|---|---|---|
| today | `draft`+absent, `forbidden`, `illustrative` | `draft`+absent, `forbidden`, `illustrative`+absent | `illustrative` |
| after RFC-015 | + **`retired`+absent** | + **`retired`+absent** | unchanged |

**Two predicates grow in this RFC and the third does not** — and
`unobliged` and `unpointable` grow together while differing in exactly
one cell, which is the coupling the guard forbids relying on. A reader
tempted to collapse them as *"`unbound` is just a special case of
`unobliged`"* is refuted by the document they are reading.

**Naming, as measured — scoped, because this document falsifies the
unscoped claim.** In the tree **before this RFC**, `unobliged` appeared
at nine sites across `domain/src/diff.rs` and `domain/src/diff/verb.rs`
— a local, a parameter, a comment and uses, none of them call sites —
and **zero** times in `specs/` or `docs/rfc/`; `unbound` appeared
nowhere. `specs/` remains at zero for all three names and is the
load-bearing half. This RFC itself now carries them, which is the point
rather than a counterexample. The
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

**The two carriers answer one structural fact two ways, and that is
bounded rather than an oversight.** A row-7 target that is anchored and
resolved reads *present*, so it is not suppressed and
`EdgeMissingInCode` fires un-actionably (F4a). An `illustrative` target
that is anchored and resolved reads *absent* under name presence, so it
is suppressed and does not fire. **Both edges are equally unmatchable**
— an anchor-backed concept can never be a code-edge target, since the
code edge carries the anchored item's name and never the concept's. The
divergence is deliberate: row 7 must stay enforced in full, and
`illustrative` need not. It is bounded entirely by F4a being a
pre-existing, out-of-scope false positive, and **if F4a is ever fixed
the two answers converge and this paragraph dissolves.** Recorded so the
next reader does not read it as an inconsistency nobody noticed.

**Not the polarity axis.** `polarity:forbidden` means *this name was
never legitimate*; retirement means *this concept was legitimate and was
removed by decision X*. They differ in re-entry rules — and that
difference is **upstream-governed and not on the wire**, since the
checker sees a tree and never a sequence. They are distinct in meaning,
in re-entry, and in **every code-present cell**. Those cells are a
product, not a list of four alternatives: marker value and polarity are
**independent attributes of one heading**, so a `declared` heading's
marker decides between `realized` and row 7, while a non-`Declared`
polarity is terminal and decides the cell on its own
(`ForbiddenConceptReintroduced`, `MissingInSpecs`) with the marker never
read. They **converge in every code-absent cell**, where none compels a
code item and one obligation set treats them alike. They differ there in *reporting*, not in
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
  - Unit: the six-cell marker matrix {unmarked, draft, retired} × {target
    item present, absent}, asserting suppression in exactly the two
    marked-and-absent cells; plus the anchored-presence cell (a marked
    target backed by a resolved `- impl:` is PRESENT, not absent).
  - Unit: the polarity matrix WITH the presence axis —
    {forbidden, illustrative} × {item present, absent} — asserting
    suppression in three of the four and `EdgeMissingInCode` FIRING in
    illustrative+present, plus a fixture pinning that adding the field
    clears it (2 violations → 1). Without the presence axis the one cell
    where the rule was wrong could not fail, which is how it reached
    revision 3.
  - Unit: EdgeMissingInSpec fires in all six cells — invariant 5, and the
    one-directionality of the exemption.
  - Unit: EdgeTargetUnknown precedence — a suppressed target yields no
    EdgeTargetUnknown, a genuinely unknown target still does, and
    known_concepts is never filtered (pinned today at diff/tests.rs:341).
  - Unit: the reverse tooth — `EdgeMissingInSpec` fires on every cell of
    both matrices, under either marker value and either polarity.
  - Unit: the missing mirror of diff/tests.rs:671, which pins the source
    side while its comment claims the edge pass "satisfies it by
    construction" — more than it proves. The target-side assertion for a
    draft-marked absent target is §1.1's defect and is red today.
  - Self dogfood: 0 findings — invariant 2 only.
  - Cross dogfood (cfdb at pinned SHA): 0 findings.
  - Integration fixture: the §1 shape reaches 0 violations; the same
    fixture with the target's item still PRESENT reaches 1, proving the
    key is `unpointable` and not the marker alone.
  - Integration fixture (D12, the per-name resolution): two headings
    sharing one name, one `illustrative` and one declared owning the code
    item, with a live concept depending on that name. The edge finding
    MUST fire, and the tree must NOT reach 0 violations / exit 0 — which
    is what the permissive resolution produces. This is the cell where no
    other violation co-fires, so the suppression alone decides the gate
    colour.
  - Integration fixture (D12, the MIRROR — the suppress direction): the
    same shape with BOTH headings non-`Declared`, so the name IS
    `unpointable`. The edge MUST be suppressed. **This is the only test
    in the slice that can distinguish the collision rule from a no-op.**
    The unit matrices are all single-heading, so they never reach the
    per-name conjunction at all; the fire-direction fixture reaches it
    once, in the one direction where a correct implementation and a
    no-op AGREE — its correct answer is also "not `unpointable`". So
    without the mirror, an implementation correct on single-heading names
    that always answers "not `unpointable`" on a collision passes the
    entire slice. A plausible slip rather than a contrived one: "if any
    heading disagrees, don't suppress" over-simplifies to "if there's
    more than one heading, don't suppress" in a single edit.
  - Target dogfood: none — rationale: no live tree carries the value.
```

**Slice ordering is forced.** B's rule keys on a state only A can parse,
and §1.1's defect is closed by B, not A.
