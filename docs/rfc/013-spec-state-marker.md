# RFC-013 — spec state marker

**Status:** Ratified (4-lens unanimous, 2026-07-29)
**Date:** 2026-07-29
**Companion:** yg/agentry docs/rfc/RFC-spec-state-marker.md (council-ratified 2026-07-29)
**Prior art:** RFC-009 (ImplementsDraftConcept), PR #118 (status:draft suppression), RFC-010 §3.5 (cohesion), RFC-012 §3.3 (behavioral exemption)

## §1 — Problem

The upstream consumer (yg/agentry) ratified a design change to how
draft-vs-ratified spec state is represented: **ratification state is
data in the spec text, never file location**
(`RFC-spec-state-marker.md`, council-ratified 2026-07-29). The upstream
tree retires its `specs/drafts/` directory and its promotion machinery;
draft state moves into the enforced `specs/concepts/` surface as
in-file markers. The checker is the only reader of that state, and the
current checker cannot express the ratified contract. Four gaps:

1. **No per-heading marker.** `status: draft` exists only as file
   front-matter. One unrealized concept inside an existing ratified doc
   either demotes the whole doc or gets its own file — which is
   location-as-state renamed.
2. **Wrong polarity on draft-with-code.** RFC-009's
   `ImplementsDraftConcept` reports code backing a draft heading as a
   *violation*. Under the ratified contract that condition is the
   normal, expected mid-arc state — the signal that the heading is
   **realized and ready to ratify** (ratification = the marker's
   deletion, performed upstream by a human). A violation is the wrong
   category: it reds a gate on the happy path.
3. **Pending state is invisible.** Draft headings with no backing code
   are skipped silently. The upstream workflow needs them *enumerated*
   every run — the pending list is the transcription worklist; a state
   field with a producer and no reader rots (the upstream corpus
   accumulated 17 drained draft shells precisely because nothing read
   them).
4. **Draft docs are exempt from the cohesion check** (drafts are
   skipped before `assemble_spec_trees`). That exemption is an
   H1-only-prose evasion channel: a doc can enter the enforced surface
   carrying no cohesion unit at all by declaring itself draft.

## §2 — Scope

**Ships:**

1. Per-heading marker: `- status: draft` recognised as a reserved
   bullet prefix when it is the first non-blank content line below an
   H2/H3 concept heading. Marks exactly that heading.
2. File-level `status: draft` front-matter semantics change: the file
   is **parsed, not skipped**; every concept heading in it is marked.
3. The six-row enforcement matrix (§3.2), including: marked heading
   with no backing item → **`pending` report record**; marked heading
   with backing item → **full equivalence enforced + `realized` report
   record**; equivalence failure under a marker is an ordinary
   violation.
4. Cohesion tightening: draft files enter the spec-tree walk;
   `ContextWithoutCohesionUnit` applies to them; a marked heading
   counts as a cohesion unit.
5. `run_check` outcome widens from `Vec<Violation>` to an outcome type
   carrying violations + pending + realized. Text and NDJSON formatters
   render the two new record kinds; exit code is a function of
   violations only.
6. Retirements: `Violation::ImplementsDraftConcept`,
   `MarkdownReader::extract_draft_concepts`,
   `CheckInput::draft_concepts` + `with_draft_concepts`.
7. `specs/dialect.md`, `specs/ndjson-output.md`, `specs/concepts/*`
   updated in the same PRs as the code they describe (§3 dual-control
   rule).

**Does not ship (non-goals in §6):** the upstream citation fence, any
corpus migration, any `ratified` marker value, any automation that
writes or removes markers.

## §3 — Design

### §3.1 — Marker sources and grammar

Two marker sources, one meaning: *this heading's concept is declared
ahead of its code; ratification is pending.*

- **File scope** (existing trigger, new semantics): `status: draft` in
  leading front-matter marks **every** concept heading in the file.
  No per-heading override exists under a draft file — a per-heading
  bullet inside one is redundant, inert text.
- **Heading scope** (new): a bullet whose prefix is `- status: draft`
  (value match ASCII-case-insensitive, mirroring the front-matter
  test), appearing as the **first non-blank content line** after an H2
  or H3 concept heading. Anything after the value on the same line —
  e.g. the upstream authoring convention
  `- status: draft (per <RFC>.md §<clause>)` — is **tolerated and
  ignored**; the parenthetical is an upstream authoring requirement
  enforced by that tree's own fences, never gate-parsed.

Grammar properties, each load-bearing:

- **One legal value.** There is no `- status: ratified` and no second
  value. Ratification is **deletion of the line** — a presence flag,
  never a state machine. Any other `- status:` bullet remains an
  unrecognised prefix under the existing dialect rule (inert text).
- **No subtree inheritance.** A marker binds only to the heading whose
  block it opens; a marked H2 does not mark its H3s. The checker
  models H2 and H3 as flat peers; inheritance would invent a hierarchy
  the diff does not model and make ratification non-local.
- **Mis-placement fails loud, not silent.** A marker bullet that is not
  the first non-blank content line is inert; the heading reads
  *unmarked* and the anti-invention check (`MissingInCode`) fires if
  its code is absent. The failure mode of a malformed marker is a
  visible violation, never a silent suppression.
- A marker bullet under an H1, in a `specs/contexts/` file, or outside
  any concept block is inert (the contexts dialect is untouched).

### §3.2 — Enforcement matrix

| # | Heading | Pub item | Result |
|---|---------|----------|--------|
| 1 | unmarked | absent | violation `missing_in_code` (unchanged) |
| 2 | unmarked | present | pass (unchanged) |
| 3 | marked | absent | skipped for equivalence; **`pending` report record**, non-violation |
| 4 | marked | present | **full equivalence enforced**; **`realized` report record**, non-violation |
| 5 | no heading | present | violation `missing_in_specs` (unchanged; a marked heading SATISFIES its pub item, so heading + type co-land without the code author touching specs) |
| 6 | doc declares no concept heading | — | violation `context_without_cohesion_unit`, marked or not — marking never suppresses the doc-level check; a marked heading COUNTS as a cohesion unit |

Row 4 is the polarity flip of RFC-009: what `ImplementsDraftConcept`
reported as a violation becomes the `realized` record. **Escalation on
contradiction only:** a marked heading whose backing item exists and
whose equivalence *fails* (signature drift, edge mismatch, verb
mismatch, context mismatch) produces that ordinary violation — a
marker can never park a real divergence. Never escalation by age,
count, or branch.

Row 6 is a tightening: today draft docs never reach the cohesion pass;
after this RFC they do, and the RFC-012 `cohesion: behavioral`
exemption applies to them on the same terms as any other doc.

### §3.3 — Reader

- `Reader::extract`, `extract_verb_anchors`, `extract_concept_anchors`
  and `assemble_spec_trees` stop skipping `status: draft` files.
- `ConceptNode` gains a `marked: bool` field (default `false`), set by
  the front-matter trigger or the per-heading bullet. The graph is the
  single carrier of marker state — the RFC-009 side index
  (`extract_draft_concepts` / `CheckInput::draft_concepts`) retires.
- Verb anchors, `- impl:` anchors, and edge bullets under a marked
  heading are extracted as normal but impose **no code obligation while
  the concept is pending** (§3.4); once realized they are enforced in
  full.

### §3.4 — Diff

Two new **marker record** kinds, distinct from `Violation` (they are
not failures and never affect the exit code). "Marker record", never
"report record": `report` already names the RFC-005 verb-coverage
subcommand and its `ReportOutput` aggregate in this same bounded
context, and its NDJSON discriminator is `"record"` — a second meaning
for the word inside one context is a ubiquitous-language violation
(DDD lens, review round 1):

- `Pending { concept, spec_source }` — marked heading, no backing item.
  Emitted **instead of** `MissingInCode`. All code-obligating checks
  sourced at that heading (its edge bullets, verb anchors, impl
  anchors) are skipped — with no backing item there is nothing to
  compare, and firing `EdgeMissingInCode` on a pending concept would
  make row 3 unreachable in practice. The skip is one uniform rule
  across every diff sub-pass, not a per-pass improvisation; the edge
  pass already satisfies it by construction (its matched-concept
  filter is built from code presence), while the verb and anchor
  passes gain marker-awareness (SOLID lens, review round 1).
- **`- impl:`-anchored marked concepts:** for a concept whose backing
  is declared by an anchor, "backing item present" IS the anchor's
  resolution outcome — row 3 vs row 4 and the RFC-012 resolution are
  the same fact, not two. A marked heading whose anchor does not
  resolve is row 3: `pending`, and the `DanglingAnchor` violation is
  suppressed for it (the dangling target is precisely the
  declared-ahead-of-code state the marker announces). A marked heading
  whose anchor resolves is row 4: `realized`, with every anchor-borne
  check (abstraction level included) enforced in full — an anchor that
  resolves to a contradicting item is an ordinary violation, per the
  escalation-on-contradiction rule.
- `Realized { concept, spec_source }` — marked heading with a backing
  item (by name match or `- impl:` anchor resolution, exactly as an
  unmarked heading binds). Emitted **in addition to** the normal, fully
  enforced equivalence checks for that pair. The record is the
  ratification signal the upstream workflow keys on.

`Violation::ImplementsDraftConcept` retires with its producing branch
(its sort slot — 13 in `violation_key`, with `Cohesion` at 12 — is
retired, not reused; existing slots are not renumbered). The orphan
pass emits `missing_in_specs` exactly as today — a marked heading
satisfies its pub item, so no orphan fires for realized pairs.

**Both marker-record types live in `domain`, as siblings of
`Violation`, and the diff itself produces them** (clean-arch lens,
review round 1): the pending-vs-realized decision is the same
concept/code matching the diff already performs for rows 1–2; placing
the records in `application` would force a second derivation of that
match outside the domain — a split-brain on the identical decision.
`domain::diff`'s result widens accordingly; the application layer only
renders.

### §3.5 — Outcome type and formatters

#### §3.5.1 — CheckOutcome

`run_check` returns a `CheckOutcome { violations: Vec<Violation>,
pending: Vec<PendingRecord>, realized: Vec<RealizedRecord> }` (final
type names are the implementer's, reconciled at review; the record
types and the widened diff result are domain-owned per §3.4 — the
application layer wraps and renders, never derives). Exit code is
computed from `violations` alone — a tree whose only findings are
pending/realized exits 0. The new types live in a new domain module of
their own, not in the existing verb-coverage report module — that
module serves the Report subcommand's change-reason exclusively, and
folding Check-outcome types into it would couple two subcommands'
independent evolution (SOLID lens, review round 1).

#### §3.5.2 — Text

**Text** — records enumerated one per line, never a bare count, both
lists always represented in the summary even at zero:

```
pending: Digest (specs/concepts/execution.md:41)
realized — ratify: InboundAcl (specs/concepts/fleet_supervision.md:120)
3 violations, 1 pending, 1 realized-unratified
```

The clean state reads `0 violations, 0 realized-unratified` with
pending printed as the remaining transcription worklist.

#### §3.5.3 — NDJSON

**NDJSON** — two new record kinds under a `marker` discriminator key,
deliberately distinct from `violation` so the existing
violation-filtered stream is unchanged and the `marker`-filtered stream
is the upstream ratification worklist. (`marker`, not `report`: the
`report` subcommand's emitter already owns a `"record"` discriminator
for `verb_coverage`/`tier_histogram`/`homonym`; this schema is a
Published Language this repo names — the upstream RFC's illustrative
key is a content contract, not a literal one:)

```json
{"schema_version":"4","marker":"pending","concept":"Digest","source":{"kind":"spec","path":"specs/concepts/execution.md","line":41}}
{"schema_version":"4","marker":"realized","concept":"InboundAcl","source":{"kind":"spec","path":"specs/concepts/fleet_supervision.md","line":120}}
```

#### §3.5.4 — schema_version

`schema_version` bumps to `"4"` (rust-systems lens, review round 1,
ruled here rather than deferred): the `marker` key itself is cleanly
additive, but the retirement of the `implements_draft_concept`
violation kind means an entire `violation` discriminator value stops
being emitted — a case `specs/ndjson-output.md` §Schema evolution's
taxonomy does not currently cover on either side. This RFC rules it
BREAKING (a discriminator that silently stops appearing is a worse
failure mode for a pattern-matching consumer than a hard version bump)
and adds "removing a `violation` discriminator value entirely" to the
breaking-change list in §Schema evolution — closing a pre-existing gap
in that taxonomy, in the same PR as the code that first exercises it.

### §3.6 — CLI

No new subcommand, no new flags. `check` gains the records in both
existing formats.

## §4 — Invariants

1. **A marker never parks a divergence.** Marked + backing item +
   failing equivalence ⇒ the ordinary violation fires. Enforced by a
   dedicated unit test.
2. **No-marker trees are semantically identical.** A tree containing
   no `status: draft` front-matter and no marker bullets produces the
   same violations, the same exit code, text output byte-identical
   modulo the new summary segment, and ndjson byte-identical modulo
   the `schema_version` value (`"3"` → `"4"`, §3.5).
3. **Exit code is a function of violations only.** Pending/realized
   records never move it.
4. **Anti-invention teeth unchanged.** Rows 1, 2, 5 of the matrix are
   byte-for-byte today's behavior.
5. **Self dogfood and cross dogfood stay at zero findings.** Neither
   graph-specs' own `specs/` nor the pinned cfdb companion carries
   draft front-matter or marker bullets (verified 2026-07-29), so both
   gates see invariant 2.
6. **The `violation`-keyed NDJSON stream is unchanged in shape** for
   all surviving variants (field sets and discriminator values;
   `schema_version` moves to `"4"` per §3.5).
7. **Perf: bounded constant-factor only.** Draft files move from ~1
   parse (the retiring draft-index walk) to one parse per surviving
   walk pass; every pass is a single streaming scan per file. No
   complexity-class change (rust-systems lens, review round 1).

## §5 — Architect lenses

### Clean architecture

### Domain-driven design

### SOLID + component principles

### Rust systems

## §6 — Non-goals

- **The citation parenthetical.** `(per <RFC>.md §<clause>)` after the
  marker value is an upstream authoring convention enforced by the
  upstream tree's own arch-check fences; the gate never parses it.
- **Corpus migration.** Moving/deleting the upstream `specs/drafts/`
  corpus is that tree's own arc (its RFC §4–§5); this checker change
  is its prerequisite, not its executor.
- **A `ratified` value, marker history, or any second state.**
  Deletion is the only transition, performed by a human upstream.
- **Automation that writes or removes markers.** The checker reads
  state; it never mutates spec text.
- **`specs/contexts/` dialect changes.** Markers bind to concept
  headings only.

## §7 — Issue decomposition

### Slice A — marker parse → pending/realized records end-to-end

**Deliverables:** per-heading bullet parse + file-front-matter
semantics change (§3.1, §3.3), `ConceptNode::marked`, diff branches for
rows 3–4 incl. pending-side obligation skip (§3.4), `CheckOutcome`,
text + NDJSON rendering + summary line (§3.5), retirement of
`ImplementsDraftConcept` / `extract_draft_concepts` /
`draft_concepts` / `with_draft_concepts`, `specs/dialect.md` +
`specs/ndjson-output.md` + `specs/concepts/*` updates for every touched
pub surface. The existing draft-diagnostic integration test asserts on
the retired variant and is **rewritten to assert `realized` records**
— replaced, not merely deleted (SOLID lens, review round 1). The
domain diff unit tests exercising the retired variant and the retiring
builder get the same disposition: **rewritten against
`Pending`/`Realized`, not deleted** (rust-systems lens, review
round 1). One structural note for the implementer: the anchor pass
runs before the concept loop, so honoring §3.4's anchored-marked rule
means the marked-name set is snapshotted before the passes that
consume it — the same pre-snapshot pattern the diff already uses for
its matched-concept sets.

**Tests:**
- Unit: marked-heading-absent-code → exactly one `pending`, zero
  `missing_in_code`; marked-heading-present-code → exactly one
  `realized`, equivalence still enforced (a deliberately drifted
  signature under a marker yields the drift violation); unmarked
  behavior byte-identical (invariant 2); mis-placed marker bullet is
  inert → `missing_in_code` fires (§3.1 fail-loud); pending concept's
  edge/verb bullets impose no obligation.
- Self dogfood (graph-specs on graph-specs): `0 violations, 0 pending,
  0 realized-unratified`; text output byte-identical modulo the new
  summary segment.
- Cross dogfood (cfdb at pinned SHA): zero findings, exit 0.
- Integration fixture: synthetic `specs/` with (a) a draft-front-matter
  file, (b) a ratified file containing one per-heading-marked concept
  with no code and one with code — NDJSON contains one
  `"marker":"pending"` and one `"marker":"realized"`, exit 0; adding a
  real divergence under the marked-with-code heading flips exit to
  nonzero.

### Slice B — cohesion tightening (matrix row 6)

**Deliverables:** draft files enter `assemble_spec_trees`; marked
headings count as cohesion units; `ContextWithoutCohesionUnit` fires on
draft docs (behavioral exemption unchanged); spec updates for touched
surfaces.

**Tests:**
- Unit: draft-front-matter doc with H1-only prose →
  `context_without_cohesion_unit`; same doc with one marked H2 → no
  cohesion violation (marked heading counts as unit).
- Integration fixture: H1-only draft file in a synthetic tree reds the
  check; adding a marked heading greens it back to pending-only.
- Self dogfood: unchanged zero findings.
- Cross dogfood: zero findings, exit 0.

Slice A precedes Slice B (B's "marked heading counts as a unit" needs
A's marker parse). Each slice is a dual-control PR per §3 of
`CLAUDE.md`; specs co-land with code.
