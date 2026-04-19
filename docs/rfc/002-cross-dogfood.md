---
title: RFC-002 — Cross-dogfood discipline with cfdb
status: Draft
date: 2026-04-19
authors: graph-specs-architects team (drafted by team-lead)
companion: yg/cfdb RFC-033 (same topic, mirror)
---

# RFC-002 — Cross-dogfood discipline with cfdb

## §1 — Problem

graph-specs-rust and cfdb are a paired toolchain. cfdb is the X-ray (detects existing debt); graph-specs is the vaccine (blocks drift at PR time). Both currently dogfood themselves — graph-specs runs `graph-specs check` against its own `specs/concepts/` on every CI push; cfdb runs `cfdb extract` + `cfdb violations` against its own tree.

The missing invariant: **neither tool runs against the SIBLING tool's tree**. As both repos co-evolve for the qbot-core rescue mission, we risk shipping changes that work on synthetic fixtures but drift from what the sibling produces.

Concrete failure modes this RFC prevents:

1. graph-specs ships a new equivalence level (relationship, bounded-context) that flags false positives on cfdb's `crates/` — discovered only when qbot-core PRs start misfiring.
2. cfdb ships a new fact type or `SchemaVersion` bump; graph-specs' `cfdb-check` job (which vendors cfdb as a pinned git dep) silently uses the old shape. Discovered at rescue time.
3. A new graph-specs violation variant (e.g. `SignatureDrift` sub-kinds) is added without updating the NDJSON output schema documented in `specs/ndjson-output.md` — downstream cfdb / qbot-core consumers see a broken wire.

The symmetry argument: both tools ARE bounded contexts. graph-specs can check its own `specs/concepts/core.md` equivalence. It should ALSO be able to check cfdb's `specs/concepts/*.md`. If it can't (because cfdb's specs use a graph-specs dialect subset that graph-specs' parser doesn't yet handle), that's a drift we want to discover on every cfdb PR, not at rescue time.

## §2 — Scope

In scope:

1. A `.cross-fixture.toml` file at graph-specs-rust's root pinning a cfdb commit SHA.
2. A CI step that clones cfdb at the pinned SHA and runs `graph-specs check --specs <cfdb>/specs/concepts/ --code <cfdb>/crates/`. Exits 0 if cfdb's specs + code are equivalent under the current graph-specs dialect.
3. A bump protocol (weekly cron + manual PR) for advancing the pinned cfdb SHA, mirroring cfdb's identical protocol for its graph-specs SHA.
4. A coordination rule: cfdb `SchemaVersion` bumps must be absorbed into graph-specs' `cfdb-check` job via a matching `.cross-fixture.toml` bump PR, landed in atomic lockstep with the cfdb PR.
5. A zero-false-positive invariant: every new graph-specs equivalence level and every new graph-specs violation variant must produce zero findings against BOTH repos' trees (graph-specs' own + cfdb's at the pinned SHA) before it ships.
6. Extension to the `Tests:` prescription from CLAUDE.md §2.5: every new-capability issue requires a cross-dogfood assertion as the second test entry (after unit, before qbot-core target).
7. A weekly closed-loop housekeeping job that cross-dogfoods at HEAD (not pinned) and opens an issue if either repo has drifted against the other's develop tip.

Out of scope (explicit non-goals in §6):

- Publishing graph-specs or cfdb to crates.io.
- Bidirectional schema invariants — graph-specs consumes cfdb's `SchemaVersion`, not the reverse.
- Gating cfdb's develop branch on graph-specs' CI (and vice versa).

## §3 — Design

### §3.1 — `.cross-fixture.toml`

File at graph-specs-rust root. Identical format to the companion file in cfdb (RFC-033 §3.1):

```toml
[sibling]
repo    = "yg/cfdb"
branch  = "develop"
sha     = "0000000000000000000000000000000000000000"
bumped_at = "2026-04-19T00:00:00Z"
bumped_by = "initial"
```

Parsed by a single `grep '^sha' | cut -d'"' -f2` in CI — no TOML crate dependency.

### §3.2 — CI cross-dogfood step

Added to `.gitea/workflows/ci.yml` after the existing self-check step:

```yaml
- name: Cross-dogfood — graph-specs on cfdb
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    cd repo
    SIBLING_SHA=$(grep '^sha' .cross-fixture.toml | head -1 | cut -d'"' -f2)
    git config --global url."https://oauth2:${GITHUB_TOKEN}@agency.lab:3000/".insteadOf "https://agency.lab:3000/"
    git clone https://agency.lab:3000/yg/cfdb.git /tmp/cfdb-sibling
    (cd /tmp/cfdb-sibling && git checkout "$SIBLING_SHA")
    ./target/release/graph-specs check \
      --specs /tmp/cfdb-sibling/specs/concepts/ \
      --code /tmp/cfdb-sibling/crates/
```

Exit 0 required. Non-zero = cross-drift, merge blocked.

### §3.3 — Bump protocol

**Weekly automatic bump** (cron, Monday 06:00 UTC):

1. Clone cfdb at current `develop` HEAD.
2. Run `graph-specs check` against it.
3. On success: open a PR updating `.cross-fixture.toml`.
4. On failure: open a `cross-drift` issue with the failing invocation and both SHAs.

**Manual bump:** any PR can bump the pin with a one-line rationale in the PR body. CI must pass.

**Schema-version lockstep:** when cfdb bumps `cfdb_core::SchemaVersion`, the matching graph-specs `.cross-fixture.toml` bump PR lands within minutes of cfdb's bump merging. Neither side merges alone. This is author discipline backed by the zero-false-positive invariant — if graph-specs' `cfdb-check` at the new SHA fails, graph-specs merges nothing until fixed.

### §3.4 — Zero-false-positive invariant

Enforced by the cross-dogfood CI step. A new equivalence level, a new violation variant, or a new ban rule in `.cfdb/queries/` must produce zero findings on the sibling's pinned tree.

Escape hatch: violating PR must (a) fix the finding in the sibling repo in a coordinated prior PR, or (b) narrow the rule/level to exclude the false-positive shape. **No allowlist file.**

### §3.5 — `Tests:` prescription extension

CLAUDE.md §2.5 gets a new row in the prescription template:

```
Tests:
  - Unit: <pure-function assertions>
  - Self dogfood (graph-specs on graph-specs): <assertion shape>
  - Cross dogfood (graph-specs on cfdb): <assertion shape>
  - Target dogfood (on qbot-core at pinned SHA): <assertion shape>
```

The cross-dogfood line is mandatory unless the architect prescribes `Cross dogfood: none — rationale: <why>` with explicit reason (e.g. "this change is internal to graph-specs' markdown adapter and cannot affect cfdb's tree").

### §3.6 — Weekly closed-loop housekeeping

Separate cron job (different schedule from §3.3 to avoid collision). Runs cross-dogfood at BOTH repos' develop HEAD (not pinned). Failure opens a `cross-drift-YYYY-WW` issue; neither repo's next PR merges until resolved.

## §4 — Invariants

- **I1 — NDJSON schema stability.** graph-specs' NDJSON output at `specs/ndjson-output.md` is the authoritative contract for downstream consumers. Changes to that schema are RFC-gated per CLAUDE.md §2; cross-dogfood CI ensures cfdb's consumption path still works at the pinned SHA.
- **I2 — Equivalence-level monotonicity.** Activating a new equivalence level (signature, relationship, bounded-context) may only TIGHTEN the check, never loosen it. Cross-dogfood verifies the tighter check doesn't false-positive on cfdb's tree.
- **I3 — Sibling schema consumption.** When cfdb bumps `SchemaVersion`, graph-specs' `cfdb-check` job absorbs the new shape in lockstep. If graph-specs CI can't absorb, graph-specs doesn't merge the bump PR — forcing cfdb or graph-specs to iterate before both move.
- **I4 — No allowlist.** Same as RFC-033 I4.
- **I5 — Determinism.** Cross-dogfood assertions (e.g. "graph-specs check on cfdb at SHA X returns 0 violations") must be byte-stable; any randomised or environment-dependent output is a bug.

## §5 — Architect lenses

Verdicts captured inline after review.

### §5.1 — Clean architecture (`clean-arch`)

Open question: is graph-specs' `application/` crate the right home for a cross-dogfood integration test, or does that concern belong in its own crate (e.g. `cross-dogfood-check` or similar)?

Open question: should the cross-dogfood CI step use the CLI binary (`./target/release/graph-specs check …`) or call into `application`'s lib surface directly? The first treats graph-specs as a sibling would; the second is more principled architecturally but depends on a lib surface that may not yet be shaped for this.

**Verdict (pending):**

### §5.2 — DDD (`ddd-specialist`)

Open question: the RFC models graph-specs and cfdb as two bounded contexts with a shared kernel (cfdb's fact output schema). Is there instead a third bounded context — "cross-dogfood orchestration" — that owns `.cross-fixture.toml`, the bump protocol, and the housekeeping job? If so, where does it live?

Open question: the term "cross-dogfood" itself — is this a good name? `sibling-check`? `paired-integration`? The RFC sticks with "cross-dogfood" but the question is open.

**Verdict (pending):**

### §5.3 — SOLID (`solid-architect`)

Open question: the `.cross-fixture.toml` file is a single concern (pin the sibling SHA); the bump job has three (clone, test, open PR / issue). SRP violation in the job? Or is the cohesion ("weekly sibling-SHA maintenance") tight enough?

Open question: the zero-false-positive invariant is enforced at CI time. Should there be a local developer tool (`just cross-check` or `scripts/cross-check.sh`) so developers can reproduce the CI check locally before pushing? That would keep the invariant's enforcement mechanism out of CI-only surface.

**Verdict (pending):**

### §5.4 — Rust systems (`rust-systems`)

Open question: `cargo install --git --branch develop` of cfdb on graph-specs' CI is ~60–120s cold. Can it be short-circuited when `.cross-fixture.toml`'s SHA matches a cached `/cache/cargo/bin/cfdb-<sha>` binary? Or is the rebuild-on-every-run simpler and acceptable given sccache warmth?

Open question: the cross-dogfood step clones sibling code (public Rust surface). Is there a circular dependency risk — graph-specs' `cross-fixture` for cfdb points at cfdb SHA Y, and cfdb's `cross-fixture` for graph-specs points at graph-specs SHA Z, and the atomic-lockstep bump has to synchronise both? The RFC assumes the cycle is fine because it's human-mediated (both PRs opened in lockstep). Is there a safer acyclic shape?

**Verdict (pending):**

## §6 — Non-goals

1. Not publishing either tool to crates.io.
2. Not introducing a bidirectional schema (graph-specs does not emit a schema that cfdb consumes).
3. Not gating cfdb's develop on graph-specs' CI (and vice versa).
4. Not requiring the weekly housekeeping job to auto-remediate drift; it opens an issue, humans fix.
5. Not pinning a qbot-core SHA workspace-wide; per-rescue-PR `Tests:` targets pin as needed.

## §7 — Issue decomposition

Mirror of cfdb RFC-033 Group A–F, scoped to graph-specs-rust:

- **Issue B1** (per RFC-033 naming): Add `.cross-fixture.toml` at graph-specs-rust root with initial cfdb SHA.
- **Issue B2**: Wire cross-dogfood CI step in `.gitea/workflows/ci.yml`.
- **Issue C1 mirror**: Weekly cron workflow attempting pin bump to cfdb develop HEAD.
- **Issue C2 mirror**: Document the manual bump protocol in `docs/cross-fixture-bump.md`.
- **Issue D1 mirror**: Document the `cfdb::SchemaVersion` lockstep consumption rule in graph-specs-rust's CLAUDE.md §3 (Dual control).
- **Issue E1 mirror**: Weekly closed-loop housekeeping cron.
- **Issue F1 mirror**: Extend CLAUDE.md §2.5 with the cross-dogfood `Tests:` template row.

Each issue carries the `Tests:` prescription from the architect team after review. Default: unit test for fixture parsing, integration test running the cross-dogfood step on a fresh checkout, dogfood assertion that graph-specs check exits 0 against cfdb at the initial pinned SHA.

Acceptance of this RFC requires all four architect lenses to RATIFY.
