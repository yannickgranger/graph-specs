---
title: RFC-003 — OSS readiness + public CI
status: DRAFT (round 1, awaiting architect-team review)
date: 2026-04-21
authors: Claude (session 2026-04-21, EPIC umbrella for OSS + multi-language)
companion: yg/cfdb (publish-paired — see §3.2)
supersedes: —
---

# RFC-003 — OSS readiness + public CI

## §1 — Problem

`graph-specs-rust` is functionally complete at v0.4 (concept, signature, relationship, and bounded-context equivalence) and self-hosts cleanly under its own dual-control gates. The repository is mirrored at `https://github.com/yannickgranger/graph-specs` and dual-licensed MIT-OR-Apache, but the mirror is a straight code copy that **cannot be consumed by an OSS contributor** without access to private infrastructure:

1. **CI is Gitea-Actions-only.** `.gitea/workflows/{ci,cross-bump,cross-loop}.yml` target `runs-on: rust` (a self-hosted runner), use a private Redis sccache (`192.168.1.107:6380`), and clone via `oauth2:${GITEA_TOKEN}` against `agency.lab:3000`. None of these resources exist on github.com.
2. **The companion `cfdb` is private.** `.cfdb/cfdb.rev` and `.cfdb/cross-fixture.toml` both pin commits on `agency.lab:3000/yg/cfdb`. The `cfdb-check` and `cross-dogfood` CI jobs `cargo install --git ${GITHUB_SERVER_URL}/yg/cfdb.git` — a URL that 404s on github.com.
3. **`CLAUDE.md` is methodology-private.** It mixes general RFC-first / dual-control discipline (which is OSS-shareable) with Agent-Zero ceremony (Podman BDD gates, /loop, ScheduleWakeup, A0 council protocols) that is internal to the author's working setup and would confuse an external contributor.
4. **`Cargo.toml.repository` and the README link to `agency.lab:3000`.** Both will produce broken links and surprising 404s for any OSS user.
5. **No `CONTRIBUTING.md`, no `CODE_OF_CONDUCT.md`, no public release cadence.**

The asymmetric-absence finding in `#64` (a concept missing from BOTH spec and code produces zero violations) deserves a documented carve-out before downstream OSS consumers inherit the blind spot silently.

The mission this RFC opens is the umbrella EPIC "OSS-release graph-specs-rust + add PHP and TypeScript adapters." This RFC is **Phase 1**: it makes the public mirror first-class without changing what the tool does. Phases 2–6 (multi-language adapter contract, PHP adapter, TS adapter, cross-language bounded contexts, publishing) are scoped in subsequent RFCs.

## §2 — Scope

**Ships in Phase 1:**

- A public-CI workflow tree at `.github/workflows/` that reproduces the dual-control gates (fmt, clippy, clippy-pedantic, build, test, dogfood, audit) on github.com free-tier runners, with a **public sccache** (`mozilla-actions/sccache-action` + GitHub Actions cache backend) replacing the private Redis.
- A documentation split: `CLAUDE.md` becomes `CONTRIBUTING.md` (public methodology — RFC-first, dual-control, `Tests:` template, vertical-slice doctrine, architect-team protocol) plus `INTERNAL.md` (private working-setup ceremony, kept in `.gitignore` or moved to author's dotfiles repo). A new `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1).
- An `OSS_CFDB_GATES` env-var convention that lets the public CI run with cfdb gates **opt-in**: when set, runs `cfdb-check` + `cross-dogfood`; when unset (default for OSS contributors), skips them with a clear log line. The Gitea-side private CI keeps cfdb gates always-on. The `.cfdb/cross-fixture.toml` pin moves to point at the **public** `github.com/yannickgranger/cfdb` mirror once cfdb's own RFC-033-mirror Phase 1 lands.
- A `Cargo.toml.repository` flip to `https://github.com/yannickgranger/graph-specs` and matching README link updates.
- A new `specs/limits.md` (or `docs/equivalence-limits.md` — see §3.7 OQ-1) that documents the symmetric-absence blind spot, closing `#64`.
- A README rewrite that drops qbot-core-internal references, adds an OSS quick-start path that does NOT require cfdb, and links forward to the multi-language roadmap (RFC-004+).
- A `.github/ISSUE_TEMPLATE/` with three templates: `bug.md`, `rfc-proposal.md`, `feature-request.md`.

**Deferred to later phases:**

- Any change to the equivalence semantics — Phase 1 is documentation, CI plumbing, and policy.
- PHP / TS adapter implementation (RFC-005 / RFC-006).
- Multi-language adapter contract (RFC-004).
- Crates.io / Packagist / npm publishing (RFC-008).

**Out of scope (explicit non-goals):**

- Re-licensing or relaxing the dual MIT/Apache license.
- Changing the RFC-first methodology — `CONTRIBUTING.md` carries it forward verbatim.
- Migrating the private Gitea CI off `.gitea/workflows/` — both CI trees coexist, the Gitea one keeps internal lockstep with cfdb's private gates, the GitHub one is the OSS-facing surface.
- Auto-mirroring source between Gitea and GitHub. The mirror is one-way (Gitea is the canonical write surface; GitHub is the public read mirror, refreshed by an explicit push) until RFC-008 chooses a publishing model.

## §3 — Design

### §3.1 — Hosting target: github.com

The public repository is `github.com/yannickgranger/graph-specs` (already exists, 89 commits as of 2026-04-21, develop branch). The Gitea repository at `agency.lab:3000/yg/graph-specs-rust` remains the **canonical write surface**:

- All RFCs, issues, PRs, and merges happen on Gitea first.
- A documented `git push github main:main && git push github develop:develop` step (or a future cron) refreshes the GitHub mirror from Gitea's develop / main branches.
- Issues filed on GitHub are mirrored back to Gitea for triage; no work is done directly on GitHub branches in Phase 1.
- The GitHub repository's "About" sidebar links back to the Gitea repository with a note that Gitea is canonical.

Rationale: the author's working environment, agent-zero ceremonies, and the cfdb companion all live on Gitea. Inverting the canonical surface would force a much larger migration than this RFC scopes. The GitHub mirror is the OSS-facing read + contribution surface; PRs from external contributors will be cherry-picked or re-pushed to Gitea by the maintainer.

### §3.2 — cfdb companion: publish-paired

`yg/cfdb` will be published in parallel to `github.com/yannickgranger/cfdb` (empty repo already created). cfdb files an equivalent RFC-034 (mirror of this RFC) and the two repos cross-coordinate via an updated `docs/cross-fixture-bump.md` that names the **public** companion URL.

The lockstep window: cfdb's RFC-034 Phase 1 must land **before** graph-specs' R3-4 (cfdb decoupling) closes — otherwise the `cross-fixture.toml` pin can't switch from `agency.lab:3000/yg/cfdb` to `github.com/yannickgranger/cfdb`. Until then, the public CI's cross-dogfood job runs in opt-in mode (off by default; on with `OSS_CFDB_GATES=1` for testing).

After both Phase 1s land, the canonical `cross-fixture.toml.[companion].repo` value flips to `yannickgranger/cfdb` (no `https://github.com/` prefix — the resolution helper in `ci/read-cross-fixture-sha.sh` learns to honor the host implied by `${GITHUB_SERVER_URL}` at CI time, so the same toml works on both Gitea and GitHub).

### §3.3 — Public CI: `.github/workflows/`

Eight jobs, mirroring the existing Gitea ones:

| Job | Image / runner | Replaces |
|---|---|---|
| `fmt` | `rust:1.93` on `ubuntu-latest` | gitea `fmt` |
| `clippy-strict` | `rust:1.93` on `ubuntu-latest` | gitea `clippy` |
| `clippy-pedantic` | `rust:1.93` on `ubuntu-latest` | gitea `clippy-pedantic` |
| `build` | `rust:1.93` on `ubuntu-latest` | gitea `build` |
| `test` | `rust:1.93` on `ubuntu-latest` | gitea `test` |
| `dogfood` | `rust:1.93` on `ubuntu-latest` | gitea `dogfood` |
| `audit` | `rustsec/audit-check@v1` | gitea `audit` |
| `cfdb-gates` (conditional) | `rust:1.93` on `ubuntu-latest`, `if: ${{ vars.OSS_CFDB_GATES == '1' }}` | gitea `cfdb-check` + `cross-dogfood` |

**sccache:** `mozilla-actions/sccache-action@v0.0.5` with `SCCACHE_GHA_ENABLED=true` (GitHub Actions cache backend, free up to 10 GB per repo). No Redis dependency. The Gitea workflow's Redis-backed sccache stays unchanged for the private side.

**Branch coverage:** PRs against `main` and `develop`; pushes on `main`, `develop`, and contributor fork branches via `pull_request` event.

**No `actions/checkout`-related concern:** github.com runners have Node.js, so we use `actions/checkout@v4` and `actions/cache@v4` directly — no need for the manual git-clone workaround the Gitea workflows carry.

### §3.4 — Documentation split

| File | Source | Notes |
|---|---|---|
| `CONTRIBUTING.md` | New, distilled from `CLAUDE.md` §1 (RFC-first), §2 (RFC pipeline), §3 (dual control), §4 (skill selection), §5 (self-hosting discipline), §6 (quick reference), §7 (companion policy) | Public methodology — every external contributor reads this before opening a PR. Carries the architect-team protocol verbatim. |
| `INTERNAL.md` | New, distilled from the private operational chunks of `CLAUDE.md` (kept locally; `.gitignore`-d if the author wants) OR moved entirely to the author's `~/agent-zero/` dotfiles | Agent-Zero gates, Podman BDD, /loop, ScheduleWakeup, council protocols, A0 cron — none of which apply to a Rust OSS project. |
| `CLAUDE.md` | Becomes a **3-line stub** that points to `CONTRIBUTING.md` for public methodology and to `INTERNAL.md` (or the dotfile path) for private setup | Preserves the slot Claude/agent setups expect to find a CLAUDE.md in. |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1, unmodified | Standard OSS expectation. |
| `.github/ISSUE_TEMPLATE/{bug,rfc-proposal,feature-request}.md` | New | Funnels external issues into shapes that map onto the existing RFC pipeline. |

The `CLAUDE.md` 3-line stub matters: agent harnesses (including this author's) auto-load `CLAUDE.md` on session start. Removing it would break the author's workflow; keeping the full content public would leak operational details that don't belong in an OSS repo.

### §3.5 — cfdb decoupling: `OSS_CFDB_GATES` env flag

The eight `.cfdb/queries/*.cypher` ban rules and the cross-dogfood loop are **architecturally sound and worth keeping** — they enforce the unwrap / context-bleed bans that make the codebase predictable. They are also unavailable to OSS contributors who don't have cfdb installed locally. The compromise:

```yaml
# .github/workflows/ci.yml — cfdb-gates job
cfdb-gates:
  if: ${{ vars.OSS_CFDB_GATES == '1' }}
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Install cfdb (pinned)
      run: cargo install --git https://github.com/yannickgranger/cfdb.git --rev "$(cat .cfdb/cfdb.rev)" --locked cfdb-cli --bin cfdb
    - run: cd .cfdb && cfdb extract --workspace .. --db db --keyspace graph-specs
    - run: for r in .cfdb/queries/*.cypher; do cfdb violations --db .cfdb/db --keyspace graph-specs --rule "$r" || exit 1; done
```

The `vars.OSS_CFDB_GATES` is set to `1` at the **repository level** by the maintainer (so the maintainer's own pushes still get the cfdb belt-and-suspenders) and unset by default for forks (so an external contributor's PR runs only fmt / clippy / build / test / dogfood / audit). When a fork's PR lands and the maintainer rebases onto develop, the maintainer's push triggers the cfdb-gates job and catches any cfdb-rule violation before it reaches develop.

`CONTRIBUTING.md` documents:
- the cfdb-gates job is opt-in
- contributors are not expected to install cfdb locally
- the maintainer's pre-merge run catches cfdb regressions
- if a contributor wants to run the gates locally, the install command is one line

This is symmetric to the cfdb-side decision (cfdb's own RFC-034 will define how cfdb decouples graph-specs's reverse cross-dogfood — likely the same flag-based approach).

### §3.6 — README rewrite for OSS audience

The current README is already substantially OSS-friendly (it explicitly invites contributions, names the planned PHP/TS adapters, and documents the "agent-in-the-middle" use case). The Phase 1 rewrite is **scoped, not wholesale**:

- Replace the "Status" paragraph (currently "Concept-level and signature-level checks implemented end-to-end and dogfooded against this repository") with a v0.4-accurate one that names all four equivalence levels as shipped and points at `CHANGELOG.md` for capability history.
- Add a "Quick start" section that walks an external user through `cargo install --git ... graph-specs-cli` (or the post-RFC-008 `cargo install graph-specs`) plus a 5-line example of pointing the tool at their own repo.
- Replace `https://agency.lab:3000/yg/cfdb` with `https://github.com/yannickgranger/cfdb` in the dogfooding paragraph.
- Add a "Roadmap" section that links the umbrella EPIC issue, RFC-004 (multi-language contract), and the per-adapter RFCs (005 PHP, 006 TS).
- Drop the qbot-core-specific references in the existing prose (Study 002 §41 etc) — they don't help an OSS reader. Move them to `docs/downstream-consumers.md` if useful, or delete.

### §3.7 — Limits of equivalence checking (closes #64)

Add a new section to `specs/concepts/core.md` titled **"Limits of equivalence checking"**, after the existing concept entries. Content:

> The diff engine compares two graphs (spec, code) and reports asymmetric drift: a concept on one side without a counterpart on the other fires `MissingInCode` or `MissingInSpecs`. **A concept that is absent from BOTH sides produces zero violations** — there is no anchor to detect its absence.
>
> This is most likely to bite for cross-cutting wire-contract invariants (schema versions, discriminator strings, identifiers) that humans tend to express as string literals in code rather than as named types. The 2026-04-21 self-archaeology that surfaced `schema_version` (RFC-001 §3.3, qbot-core#4034 → graph-specs-rust#63) is the canonical case.
>
> Mitigations are **methodology-side, not tool-side**: every RFC's `§Scope` items must appear either in the RFC's `§Domain types` list **or** in an explicit "deliberately not typed" carve-out. The architect-team review is the gate that catches symmetric absence at ratification time. RFC-001 §5 added this as a permanent lens-checklist item.
>
> Tool-side detection is deferred (see RFC-009 placeholder) — a heuristic that parses RFC `§Scope` blocks and warns when a named scope item appears in neither `specs/concepts/` nor as a `pub` type would close the loop, but at the cost of false positives on intentionally-non-typed concepts.

OQ-1 (resolved below): does this go in `specs/concepts/core.md` (under the gate) or `docs/equivalence-limits.md` (ungated)? Resolved as `specs/concepts/core.md` — it's a property of the equivalence model and belongs in the spec registry; gating it means future tool changes that affect the limit are caught by the dogfood.

## §4 — Invariants

1. **The equivalence semantics do not change.** Phase 1 is plumbing and policy. The `domain::diff` algorithm, the four equivalence levels, the NDJSON wire schema v2 — all unchanged.
2. **Self-dogfood stays green throughout.** Every commit on the work branch passes `graph-specs check --specs specs/ --code .` against the latest tool binary.
3. **Cross-dogfood stays green throughout the lockstep window.** Until cfdb's RFC-034 Phase 1 lands and `cross-fixture.toml` flips to the github.com URL, the existing private cross-dogfood job continues to run on Gitea and must pass.
4. **No deletion of agent-author CLAUDE.md slot.** The 3-line stub preserves the path; agent harnesses that auto-load it continue to work.
5. **The github.com mirror is read-only-canonical:** all writes go through Gitea first. PRs filed on github.com are cherry-picked or re-pushed by the maintainer; no direct merges on github.com until RFC-008 chooses a publishing model.
6. **`OSS_CFDB_GATES=1` is the maintainer's default**, not an external contributor's expectation. CI green for an external PR requires only the seven non-cfdb jobs.
7. **`#64` closes with R3-5.** The "Limits of equivalence checking" section is the documented mitigation; tool-side detection is RFC-009 work, not Phase 1.

## §5 — Architect lenses

(All four return verdicts inline after round 1.)

### §5.1 — Clean architecture (`clean-arch`)

To be filled by the agent-team review.

### §5.2 — Domain-driven design (`ddd-specialist`)

To be filled.

### §5.3 — SOLID + component principles (`solid-architect`)

To be filled.

### §5.4 — Rust systems (`rust-systems`)

To be filled.

## §6 — Non-goals

1. Not auto-mirroring Gitea ↔ GitHub source. One-way, manual push for now.
2. Not changing the equivalence engine.
3. Not publishing to crates.io (deferred to RFC-008).
4. Not migrating the private Gitea CI to GitHub Actions — both coexist.
5. Not enforcing cfdb-gates on external contributors' PRs.
6. Not changing the dual MIT/Apache license.
7. Not addressing PHP / TS adapters (RFC-004 / 005 / 006 territory).
8. Not building the tool-side symmetric-absence detector (RFC-009 placeholder; #64 closes with the documented mitigation per R3-5).

## §7 — Issue decomposition

Each child issue carries the standard `Tests:` template (Unit / Self dogfood / Cross dogfood / Target dogfood). Tests prescription is the architect-team's responsibility per CLAUDE.md §2.5; the table below is the architect's first-cut prescription, refined during the round-1 review.

| ID | Slice | Tests prescription |
|---|---|---|
| **R3-1** | Update `Cargo.toml.repository` to `https://github.com/yannickgranger/graph-specs`. Update README links from `agency.lab:3000` to `github.com/yannickgranger`. Add `.github/ISSUE_TEMPLATE/{bug,rfc-proposal,feature-request}.md`. | Unit: none — text changes. Self dogfood: tool runs unchanged after the URL flip (smoke). Cross dogfood: existing pinned cfdb SHA still resolves correctly via Gitea (no impact yet on the github.com cfdb path). Target dogfood: none. |
| **R3-2** | Doc split: `CLAUDE.md` → `CONTRIBUTING.md` + `INTERNAL.md` + 3-line `CLAUDE.md` stub. Add `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1). | Unit: none — docs only. Self dogfood: 0 violations. Cross dogfood: none. Target dogfood: none — rationale: no executable surface touched. |
| **R3-3** | New `.github/workflows/{ci,audit}.yml` reproducing the seven non-cfdb gates with `mozilla-actions/sccache-action`. Verify on a throwaway PR against the public mirror. | Unit: none. Self dogfood: dogfood job in `.github/workflows/ci.yml` exits 0. Cross dogfood: none in this slice (cfdb-gates job is R3-4). Target dogfood: a green workflow run on a PR against github.com/yannickgranger/graph-specs is the proof. |
| **R3-4** | `cfdb-gates` opt-in job with `${{ vars.OSS_CFDB_GATES }}` guard. cfdb install URL flips to `github.com/yannickgranger/cfdb` AFTER cfdb's RFC-034 Phase 1 lands; until then, the job stays on the Gitea path inside the public CI yaml (with a runtime check that fails fast if `OSS_CFDB_GATES=1` is set on a non-Gitea environment without cfdb access). | Unit: shell script lint on the new workflow yaml. Self dogfood: job exits 0 with `OSS_CFDB_GATES=1` and skips cleanly with `OSS_CFDB_GATES` unset. Cross dogfood: existing private cross-dogfood (Gitea) still passes — no regression. Target dogfood: none. |
| **R3-5** | New "Limits of equivalence checking" section in `specs/concepts/core.md`. Closes #64. | Unit: none. Self dogfood: section is gated as a spec concept, dogfood passes. Cross dogfood: cfdb still passes. Target dogfood: none. |
| **R3-6** | README rewrite per §3.6. | Unit: none. Self dogfood: no impact. Cross dogfood: no impact. Target dogfood: none. |
| **R3-7** | Lockstep with cfdb's RFC-034 Phase 1: once cfdb is published at github.com/yannickgranger/cfdb, flip `.cfdb/cross-fixture.toml.[companion].repo` and the cfdb install URL in the cfdb-gates job to point at the public host. | Unit: `ci/read-cross-fixture-sha.sh` parses the new value correctly. Self dogfood: no impact. Cross dogfood: cfdb-gates with `OSS_CFDB_GATES=1` exits 0 against the github.com cfdb pin. Target dogfood: none. |

R3-1, R3-2, R3-5, R3-6 can ship in parallel after this RFC ratifies. R3-3 is a prerequisite for R3-4. R3-7 depends on cfdb's RFC-034 Phase 1 ratifying and shipping (companion §7).

## §8 — Open questions

| ID | Question | Resolution |
|---|---|---|
| OQ-1 | Limits-doc location: `specs/concepts/core.md` vs `docs/equivalence-limits.md`? | RESOLVED — under the gate, in `specs/concepts/core.md`. |
| OQ-2 | One-way mirror push: manual `git push github` vs scheduled cron? | DEFERRED to RFC-008. |
| OQ-3 | Do we accept external PRs directly on github.com or require they re-file on Gitea? | DEFERRED to round-2 review with input from the maintainer. The tentative answer: accept on github.com, cherry-pick to Gitea on merge. Documented in `CONTRIBUTING.md`. |
| OQ-4 | Should the `cfdb-gates` job ever run on external PRs (e.g., maintainer manually toggles `vars.OSS_CFDB_GATES=1` for a specific PR via `workflow_dispatch`)? | RESOLVED — `workflow_dispatch` with optional `cfdb_gates: bool` input lets the maintainer trigger a one-off cfdb-gates run on a specific commit. Documented. |

## §9 — Forward-looking workflow modes (informational, RFC-004 territory)

This RFC ships only Phase 1 (OSS plumbing). The umbrella EPIC anticipates two distinct authoring workflows that subsequent RFCs (004 multi-language contract, 005 PHP, 006 TS) must accommodate. They are noted here so that Phase 1 documentation (`CONTRIBUTING.md`, README §Quick start, the limits-doc in §3.7) does **not** over-commit to a single mode.

### §9.1 — Greenfield: spec-first, then code

1. Human + agent collaboratively author the markdown spec under `specs/concepts/<concept>.md` — concept name, short prose, optional fenced signature block in the target language.
2. Implementation lands. The class / struct / function is written to satisfy the markdown.
3. For PHP / TS: the new class also carries an inline `#[Spec(...)]` / `@Spec(...)` attribute that re-asserts the shape (implements / extends / signature) inline. Diff has **three sources to reconcile**: markdown spec graph, attribute spec graph, structural code graph.
4. CI fires drift on any pairwise disagreement.

### §9.2 — Legacy / brownfield: archaeology, then lock

1. Agent walks the existing codebase, emits a markdown spec capturing the present-day skeleton (concept names, edges, signatures).
2. Maintainer reviews and **locks** the rebuilt spec as the baseline.
3. Future refactors must spec-back: any change to code structure either updates the markdown or is rejected by the gate.
4. Legacy classes typically have **no** `#[Spec]` / `@Spec`. The markdown spec is the only spec source on the diff. Same shape as Rust today.

### §9.3 — Architectural consequence

Spec sources are **additive per language**. Markdown is universal and mandatory; inline attributes/decorators are an optional augmentation that strengthens the gate when the codebase culture supports it. The diff engine unions all spec sources before comparing against the structural code graph. RFC-004 formalizes the `Vec<Reader>` per side; RFC-005 / RFC-006 ship the language-specific readers.

The blind spot to flag in `CONTRIBUTING.md` (§3.4 deliverable): legacy code without inline attributes can silently lose a class — deleting the class also deletes any inline declaration, so only the markdown spec retains the "this concept must exist" intent. The greenfield mode is naturally protected because the markdown was authored first; the legacy mode depends on the discipline of locking the rebuilt markdown and never deleting concept entries without a spec-back PR. This is a strict superset of the symmetric-absence blind spot in §3.7 — the limits-doc R3-5 ships should mention the workflow-mode dimension.

## §10 — Ratification

Awaiting round-1 architect-team verdicts. RFC ratifies when all four lenses return RATIFY (or reject with documented overrides per CLAUDE.md §2.3).

After ratification, §7 becomes the concrete backlog. Each row is filed as a forge issue with body `Refs: docs/rfc/003-oss-readiness.md`, worked via `/work-issue-lib`, shipped through the dual-control regime (graph-specs check + cfdb violations + the documented opt-in `OSS_CFDB_GATES` shape).
