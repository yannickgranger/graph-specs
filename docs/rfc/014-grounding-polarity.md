# RFC-014 — grounding polarity

**Status:** Ratified — 2026-07-30
**Date:** 2026-07-30
**Upstream owner:** yg/agentry docs/rfc/vocabulary/RFC-vocabulary.md (ratified)
**Upstream impl:** yg/cascade src/lib.rs `resolve_polarity`
**Depends on:** RFC-013 (ratified) — shares carriers; its Slice A lands first

## §1 — Problem

On a corpus carrying grounding blocks, `graph-specs check` is wrong in
**both** directions (#168, verified against `yg/Bosun` @ `develop`).

A grounding comment sits under a concept heading and carries a
`polarity:` key:

```
## Member
<!-- parent:spec:Unit polarity:forbidden -->
```

`forbidden` means code must **not** bear the name. graph-specs never sees
the field — `specs/dialect.md` lists HTML comments among what the reader
ignores — so it reads the heading as an ordinary obligation:

- **Over-reports.** Seven `missing in code` findings on Bosun for headings
  that have no `pub` item by design. All false; `cascade check` is clean
  at exit 0 on the same tree.
- **Under-reports.** A `pub struct Member` bearing the expelled name
  *satisfies* the heading, so graph-specs falls silent. Reintroducing an
  expelled name makes the tool **quieter**, not louder. Reproduced: with
  the fixture in scope the `missing in code: Member` line disappears and
  nothing replaces it — the tool exits **0**.

The second is the defect that matters.

## §2 — Scope

**Ships:** the reader sees `polarity:`, and the concept pass honours the
three values.

**Does not ship:** any other grounding key (`parent:`, `anchor:`,
`reached_for:`), any provenance or rootedness modelling, any change to
the `--specs` / `--code` CLI surface.

## §3 — Design

### §3.1 — The concept is imported, not defined here

`polarity` is owned upstream: defined in agentry's ratified
`RFC-vocabulary.md`, authored via agentry `specs/dialect.md` and Bosun's
grounding key, realized as `cascade::Polarity`. graph-specs is a
**Conformist** — it tracks that definition and does not fork it. The
three values and their meanings are cited from cascade's
`resolve_polarity` (`cascade/src/lib.rs:350-359`), not re-derived; if
upstream adds a value, that is the seam that changes.

Two scoping corrections:

**"Conformist" here is prose, not a wired relationship.** RFC-001 §3.7's
`ContextPattern::Conformist` is a formal enum wired into
`specs/contexts/*.md` Imports/Exports for *this repo's own* bounded
contexts, and RFC-001 §6 scopes that mechanism single-repo. This RFC does
**not** touch `ContextPattern` / `ContextDecl`, and nothing here should be
read as licence to formalize a cross-repo `ContextImport`.

**`domain::Polarity` gets a `## Polarity` heading in
`specs/concepts/equivalence.md`.** agentry's forest disjointness invariant is scoped to
agentry's own corpus (`RFC-vocabulary.md:341-343`, ruling A2-R1, records
that graph-specs *joining* that forest was raised and **NOT ADOPTED** —
graph-specs is the external gate, not a participant in agentry's
word-homing ledger); and this repo's own self-hosting rule is
unconditional — `specs/concepts/equivalence.md:7-10` requires every
top-level `pub` type in `domain`/`ports` to carry a heading, enforced by
the self-dogfood gate. Omitting it would emit `missing_in_specs: Polarity`
on graph-specs' own tree, contradicting Invariant 1 below.

The heading takes the `## SignatureState` shape (a payload enum hung off
`ConceptNode`) and states the Conformist relationship in its prose,
including that this is the *concept-grounding* sense of the word and not
the vocabulary system's word-polarity — cascade itself disambiguates the
two (`WordPolarity`, `cascade/src/lib.rs:1475-1491`, "Distinct from
`Polarity`"). Declaring the heading is *more* correct Conformist practice
than hiding it: Conformist means faithfully tracking upstream's shape.

### §3.2 — Marker

The `polarity:` token in an HTML comment that is the block **immediately
below** an H2/H3 concept heading. Three values; anything else — absent
comment, absent key, unreadable value — is `declared`, with a
`tracing::warn!` on an unreadable value (the tolerant-skip failure mode
`- verb:` already uses).

The fallback direction is the point: a typo leaves the heading's
obligation **armed**. A marker can only narrow an obligation deliberately
written down.

**On the name.** "Grounding comment" is upstream's term
(`parse_grounding`, `is_grounding_shaped`, "the grounding-block dialect"),
and it is retained deliberately rather than translated — a local rename
would create a second name for something cascade already documents, which
costs more in cross-tool correlation than the clarity it buys. But the term
sits oddly in this RFC, and the oddness is worth naming: *grounding* means
**ancestorship**. Cascade's specs are a refinement forest hung off the RFC,
where every concept declares a parent — an RFC clause (a root) or another
concept (an internal node) — and an ungrounded heading is "an invented,
rootless concept" (`cascade/src/lib.rs:214`). That is the `parent:` key's
job, and rootedness is exactly what §6 non-goals.

So graph-specs does **no** grounding in the sense the name denotes. It
reads one key that happens to be carried in the same comment: `polarity:`
is an independent axis sharing the grounding block's syntax, not part of
its ancestry payload. Reading "graph-specs parses the grounding comment"
as "graph-specs validates ancestry" would be exactly backwards.

**This is not a reversal of RFC-012 §3.2.** That ruling (council unanimous)
rejected issue #144's `<!-- graph-specs:anchor=… -->` because graph-specs
*controlled that authoring surface* and a dialect-consistent bullet form
was available. Here graph-specs owns neither end — cascade owns the value
semantics, Bosun owns the comment encoding, and §6 disclaims any authoring
surface in this repo — so there is no bullet alternative to prefer. Reading
an externally-authored wire format under a Conformist contract is a scoped
exception to comment-skipping, not a new local convention.

**Extraction must be quote-aware.** Cascade makes `anchor:"…"` mandatory
for every `rfc:`-rooted concept (`resolve_parent`,
`cascade/src/lib.rs:361-392`), so most concepts in a real grounded corpus
carry a quoted freeform value in the *same* comment as `polarity:`. A bare
`find("polarity:")` is unsafe against a quoted value containing that
substring as prose — entirely plausible on an architecture-methodology
corpus like Bosun, which carries RFC prose *about* polarity. R14-1 needs a
minimal quote-skipping scan, **not** cascade's full `GroundingTokens` /
`resolve_parent` / `resolve_reached_for` validation (out of scope per §6).

**The adjacency primitive is shared with RFC-013.** "Immediately below the
heading" and RFC-013 §3.1's "first non-blank content line" need the same
mechanism, and `SectionState` (`adapters/markdown/src/section.rs`) has
nothing like it today. Since RFC-013 Slice A lands first, that primitive
(`first_block_since_heading`-shaped) is Slice A's to build and this RFC
reuses it. Building it twice in parallel risks two subtly different
"immediately below" semantics.

### §3.3 — Enforcement

| polarity | code absent | code present |
|---|---|---|
| `declared` (default) | `missing_in_code` (unchanged) | satisfied (unchanged) |
| `forbidden` | clean | `forbidden_concept_reintroduced` |
| `illustrative` | clean | `missing_in_specs` |

The `illustrative` row is upstream's rule, not an invention: cascade
excludes illustrative names from the type-binding set so the marker cannot
launder unspecced public surface past the gate
(`push_type_binding_findings`, `cascade/src/lib.rs:1380-1392`).

**This is a match-attempt gate, not a post-match dispatch.** The `illustrative`
row is not reachable by adding a branch after matching: today
`code_by_name.remove(&spec_node.name)` (`domain/src/diff.rs:112`)
unconditionally consumes the code node on a name hit. An illustrative
concept must **skip that call entirely**, leaving the code node unconsumed
so it falls through to the orphan sweep. This mirrors cascade, which
filters illustrative out of the match set *before* matching.

**Precedence rule.**
`polarity != declared` is evaluated **first and is terminal**, independent
of RFC-013's `marked`; the marker pass (`pending` / `realized`) applies
only when `polarity == declared`.

Without this the two RFCs contradict on shared cells — RFC-013 §3.2 row 4
calls marked+code-present a `realized` record (a success signal meaning
"ratify me"), while this RFC calls forbidden+code-present a violation.
Terminal precedence collapses the 2x3x2 space to a sequential guard
(`match polarity { Forbidden | Illustrative => terminal, Declared =>
RFC-013's marked dispatch }`) and keeps §3.4's "one call site" claim true.

**A marked heading with non-declared polarity never emits a marker record;
the marker vocabulary tracks concept-realization only.** The full 2x3
product, stated so no implementer has to reverse-engineer it from two RFCs
that do not reference each other's tables:

| | `declared` | `forbidden` | `illustrative` |
|---|---|---|---|
| **unmarked** | RFC-013 rows 1/2 unchanged | §3.3 row unchanged | §3.3 row unchanged |
| **marked** | RFC-013 rows 3/4 unchanged (`Pending`/`Realized`) | identical to unmarked — `marked` is inert | identical to unmarked — `marked` is inert |

No cell emits both a marker record and a polarity outcome.

The reason this is principled rather than an arbitrary tiebreak: `marked`
exists to relax the code-existence obligation a `declared` heading carries,
so it does not fire `missing_in_code` while pending ratification.
`forbidden` and `illustrative` carry no code-existence obligation at all —
absence is clean by definition for both. There is no obligation for
`marked` to relax, so it is not out-competed by polarity; it is
structurally inert. This holds regardless of what is *implementable* —
dual emission via two independent passes is perfectly implementable; the
argument is about what `marked` **means**.

**Explicitly ruled out:** a marked+`forbidden` heading does *not* surface
as `Pending` under a "this ban is itself unratified" reading. RFC-013's
vocabulary is textually scoped to concept-realization-in-progress (§1:
"the signal that the heading is realized and ready to ratify"), not to
provisional-heading-state in general — so the "marked might mean something
broader" counter-reading is foreclosed by RFC-013's own problem statement.
If that visibility need proves real it is a small additive RFC on top of a
correct terminal rule, not speculative machinery built now.

Also, as a consequence rather than a third reason: emitting
`realized — ratify` on an expelled name would be an actively wrong
instruction in the upstream worklist — a consumer would read "close this
out" and "actively banned, CI-blocking" off the same heading.

**Uniform obligation-skip.** Every code-obligating check sourced at a
non-`declared` heading is skipped — its edge bullets, `- verb:` anchors and
`- impl:` anchors alike. This adopts RFC-013 §3.4's rule verbatim for the
same reason it exists there. Today `edge::edge_diff` and `verb::verb_pass`
run over snapshots built from *all* `spec_nodes` with no polarity gate
(`domain/src/diff.rs:91-101`; `verb_pass` takes a flat `VerbOwnership`
threaded independently of `spec_nodes`), so a `- verb: foo` bullet under a
`polarity:forbidden` heading would fire `VerbMissingInCode` — contradicting
the heading's own meaning.

**`DanglingAnchor` is polarity-gated (OQ-4 ruled).** A non-`declared`
heading compels nothing, so a `- impl:` anchor under one whose target does
not resolve fires **nothing**. `anchor_pass` (`domain/src/diff.rs:180-192`)
runs unconditionally today over a `Vec<ResolvedAnchor>` joined to
`ConceptNode` only by name. Polarity must **not** be added to
`ConceptAnchor`: that duplicates a fact `ConceptNode` already owns and
reintroduces exactly the side-index split-brain RFC-013 §3.3 retires.

**One shared snapshot, owned keys.** All three polarity-gated consumers —
`anchor_pass`, `edge::edge_diff`, `verb::verb_pass` — read from a single
`HashMap<String, Polarity>` built from `spec_nodes` alongside the existing
`matched_concepts` / `known_concepts` snapshots (`domain/src/diff.rs:91-101`),
not one map per consumer. The keys must be **owned**, not `&str`:
`spec_nodes` is moved by the concept loop at `diff.rs:111`, while
`edge_diff` and `verb_pass` run afterwards (`:141`, `:149`) and key off
owned `String`s of their own (`Edge.source_concept`, `VerbAnchor.concept`),
so a borrowed map cannot survive to reach them. This is why the adjacent
`matched_concepts` / `known_concepts` are already `HashSet<String>` with an
explicit clone.

### §3.4 — Types

- `domain::Polarity` — a three-variant enum, `Declared` by default, marked
  `#[non_exhaustive]` (§3.1 names it as the seam that changes when upstream
  adds a value; same preemptive move as `AnchorKind`). Data only, **no
  predicate methods** — the branch table lives at its one call site. This
  matches upstream exactly: cascade's own `Polarity`
  (`cascade/src/lib.rs:192-197`) has zero methods.
- `ConceptNode` gains `polarity: Polarity` via a
  `with_polarity(self, Polarity) -> Self` builder mirroring
  `with_provenance` — **not** a positional argument on `ConceptNode::new`,
  which deliberately does not derive `Default`. Spec-side only; the code
  side is a fact, not a declaration.
- `Violation::ForbiddenConceptReintroduced { name, spec_source, code_source }`
  — both sites, so the finding names what expelled the name and what
  reintroduced it. Wire: `forbidden_concept_reintroduced`.
  - **Naming (OQ-1 ruled).** Not `…Realized`: RFC-013 §3.4 already uses
    `Realized` for "the pending concept landed" — opposite valence, same
    bounded context, both surfaced by the same `check` subcommand. RFC-013's
    own DDD review set binding precedent by killing the word "report" for
    this exact shape. Invariant 3 binds *behaviour*, not identifier
    spelling. Doc-comment the correspondence:
    `// mirrors cascade::Finding::ForbiddenConceptRealized (cascade src/lib.rs:1363) under a locally-disambiguated name — see RFC-014 OQ-1`.
  - **Sort slot: 15**, appended after `DanglingAnchor` (14) in
    `violation_key` (`domain/src/diff.rs:199-217`). Append-only; existing
    slots are never renumbered and RFC-013's retired slot 13 stays retired.
- Once RFC-013 Slice A lands, the push becomes `outcome.violations.push(...)` once
  `CheckOutcome` exists, and the polarity guard sits *ahead of* the marked
  dispatch inside the same per-node function, not as a parallel `if`.

**Prerequisite (owned by RFC-013 Slice A).** `domain::diff` measures cognitive
complexity 11 today — "Elevated — consider splitting" — before either RFC
lands. The concept-matching loop should be extracted into a
`diff::concept::concept_pass` submodule, mirroring the existing
`edge`/`verb`/`cohesion`/`signature` pattern, as part of Slice A, so this
RFC edits a named function rather than an inline loop body.

### §3.5 — Wire

`forbidden_concept_reintroduced` is an additive `violation` discriminator
— no `schema_version` bump, per `specs/ndjson-output.md` §Schema evolution.
The two narrowings of `missing_in_code` emit no record; like the
`status: draft` suppression before them they change which headings qualify,
not what the discriminator means.

## §4 — Invariants

1. **Unmarked corpora are byte-identical.** Every existing test passes
   unchanged; self-dogfood stays at 0 violations.
2. **Zero-baseline.** Delete the `pub` item and the finding clears;
   re-add it and it returns. No allowlist, no suppression file.
3. **Upstream parity.** On a grounded corpus the two tools agree on this
   axis: cascade's `ForbiddenConceptRealized` and this variant fire on the
   same inputs. Parity is behavioural, not lexical.
4. **Cross-dogfood stays clean** — cfdb at its pinned SHA carries no
   grounding blocks, so the pass must be inert there.

## §5 — Architect lenses

**Unanimous RATIFY sub-points:** the minimal data-only type surface
(verified against cascade's own zero-method `Polarity`), and two
independent `ConceptNode` fields (`marked` + `polarity`) over a fused
carrier — CCP: different upstream sources, different grammars, different
extension seams.

## §6 — Non-goals

- Modelling `parent:` / rootedness. That is cascade's subsumption claim
  (`PLAN.md` check 1); graph-specs does not enter it.
- Replacing cascade on grounded corpora. cascade remains the gate where
  it is deployed (Bosun `scripts/arch-check.sh`). This RFC makes
  graph-specs *correct* when pointed at such a corpus, not authoritative
  over it.
- Any `polarity:` **authoring** surface in this repo's own `specs/`.
- Cascade's full grounding-comment validation (`GroundingTokens`,
  `resolve_parent`, `resolve_reached_for`). Only `polarity:` is read;
  unrecognised keys are skipped, not rejected.

## §7 — Issue decomposition

One slice — the marker is not separable from the behaviour it changes, and
a reader that parses polarity without enforcing it is a producer with no
reader. **Blocked by RFC-013 Slice A**: shared carriers, the shared
adjacency primitive, and the `concept_pass` extraction all land there.

**R14-1 — read `polarity:` and honour the three values.**

```
Tests:
  - Unit: the three values + default parse; unreadable value falls back to
    `declared`; a comment not adjacent to the heading is inert; a decoy
    `polarity:` inside a quoted `anchor:"…"` value is NOT read as the key;
    table-driven 3x2 polarity/presence matrix asserting §3.3 exactly,
    including that the code node is NOT consumed from `code_by_name` for an
    illustrative concept (the match-attempt gate, asserted directly);
    precedence — marked+`forbidden`+code-present fires the violation and NO
    `Realized` record, marked+`forbidden`+code-absent is clean with NO
    `Pending` record, same pair for `illustrative`; a `- verb:`/edge bullet
    under a non-`declared` heading imposes no obligation; a dangling
    `- impl:` anchor under a non-`declared` heading does NOT fire
    `DanglingAnchor`; sort-slot tripwire asserting slot 15 and that slot 13
    stays retired.
  - Integration fixture: synthetic `specs/` with one heading per polarity
    value crossed with code-present/code-absent, through the real markdown
    + rust readers end-to-end, asserting on `Graph`/`Violation` output —
    so the proof is reproducible in CI and not dependent on an external
    tree. forbidden+pub type → exactly one `forbidden_concept_reintroduced`
    and zero `missing_in_code`; illustrative+pub type → exactly one
    `missing_in_specs` and zero `missing_in_code`.
  - Self dogfood (graph-specs on graph-specs): 0 violations, unchanged —
    this repo authors no grounding blocks, so the pass must be inert, and
    the `## Polarity` heading must resolve against `domain::Polarity`
    (the executable proof of Invariant 1).
  - Cross dogfood (graph-specs on cfdb at pinned SHA): 0 findings, unchanged.
  - Target dogfood (on yg/Bosun @ develop): the 7 false `missing in code`
    findings are gone AND `.cfdb/fixtures/bosun-topology-model.rs` in scope
    fires `forbidden concept reintroduced: Member` — the #168 reproduction,
    both directions, as the acceptance proof.
```

The RED for the second bullet of the target dogfood is recorded in #168 and
reproduces on `develop` today: with the fixture in scope the tool exits
**0** with "0 violations".
