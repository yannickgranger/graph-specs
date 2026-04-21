---
title: RFC-003 — OSS readiness + public CI
status: DRAFT (round 1 r2 — collapses dual-CI to mirror + tiny contributor-CI; supersedes the as-merged r1 inline)
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

- A **Gitea→GitHub one-way mirror** (Gitea Actions on push to `develop` / `main` / tags runs `git push --mirror github`).
- A **single GitHub Actions workflow** (`.github/workflows/contributor-ci.yml`, ~50 lines) on `pull_request` only — fmt + clippy-strict + build + test. No cfdb, no dogfood, no sccache, no env-flag dance. The eight authoritative Gitea jobs stay where they are.
- A documentation split: `CLAUDE.md` becomes `CONTRIBUTING.md` (public methodology — RFC-first, dual-control, `Tests:` template, vertical-slice doctrine, architect-team protocol, dual-surface model explained) plus `INTERNAL.md` (private working-setup ceremony, kept in `.gitignore` or moved to author's dotfiles repo). A new `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1).
- A `Cargo.toml.repository` flip to `https://github.com/yannickgranger/graph-specs` and matching README link updates.
- A new "Limits of equivalence checking" section in `specs/concepts/core.md` (per §3.7 OQ-1) documenting the symmetric-absence blind spot, closing `#64`.
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

### §3.1 — Hosting target: github.com (mirror, not dual-CI)

The public repository is `github.com/yannickgranger/graph-specs` (already exists, 89 commits as of 2026-04-21, develop branch). The Gitea repository at `agency.lab:3000/yg/graph-specs-rust` remains the **canonical write surface AND the canonical CI surface**:

- All RFCs, issues, PRs, merges, and authoritative CI runs (the eight Gitea workflow jobs incl. cfdb gates + cross-dogfood) happen on Gitea.
- A Gitea Actions workflow on push-to-`develop`/`main`/tags runs `git push --mirror github` so the GitHub repo is a near-instant reflection of Gitea's branches and tags. Mirror direction is one-way — Gitea→GitHub, never the reverse.
- The GitHub repo provides three things: code visibility (browse, search, fork, star), issue tracker (mirrored back to Gitea for triage by a tiny webhook handler — see R3-3), and a **minimal contributor-feedback CI** (§3.3) that fires only on PRs filed against the GitHub mirror.
- The GitHub repo's "About" sidebar links back to the Gitea repo with a note that Gitea is canonical and that PRs filed on GitHub are cherry-picked or re-pushed to Gitea by the maintainer for the canonical CI run before merge.

Rationale (revised from r1): translating eight Gitea jobs into `.github/workflows/` to "achieve dual CI" doubles the maintenance surface for marginal contributor benefit — the maintainer's pre-merge cycle on Gitea catches everything that matters. A pure mirror plus a tiny contributor-feedback CI gives 90% of the OSS value at 10% of the work and removes the `OSS_CFDB_GATES` env-flag dance the r1 draft contemplated.

### §3.2 — cfdb companion: publish-paired (for visibility, not CI)

`yg/cfdb` is published in parallel to `github.com/yannickgranger/cfdb` (empty repo already created) using the same Gitea→GitHub mirror pattern from §3.1. cfdb files an equivalent RFC-034 (mirror of this RFC).

cfdb's gates (`cfdb-check`, `cross-dogfood`) remain **Gitea-only**. They run on the canonical CI; they do NOT run on the GitHub contributor-feedback CI. The maintainer's pre-merge cycle on Gitea is the gate that protects develop. Forking the cfdb gates to GitHub would require duplicating the Redis/sccache setup, the cfdb pinning protocol, and the cross-fixture lockstep — none of which give an external contributor useful feedback they can act on (they don't have cfdb installed locally either).

`docs/cross-fixture-bump.md` is updated only to mention `github.com/yannickgranger/cfdb` as the public-visibility mirror; the canonical `[companion].repo` value in `.cfdb/cross-fixture.toml` stays the Gitea path. **No env-flag, no opt-in, no lockstep window** between graph-specs' and cfdb's Phase 1 RFCs — both can ship independently.

### §3.3 — Contributor-feedback CI: one workflow, one event

A single GitHub Actions workflow at `.github/workflows/contributor-ci.yml` runs only on `pull_request` events against the GitHub mirror. The mirror cron's pushes to `develop` / `main` do NOT trigger it (the workflow subscribes to `pull_request: [opened, synchronize, reopened]` only — mirror pushes use the default `push` event which the workflow does not listen for). This avoids redundant CI on every mirror sync.

Four jobs, all on `ubuntu-latest` with the public `rust:1.93` Docker image:

| Job | What | Notes |
|---|---|---|
| `fmt` | `cargo fmt --check --all` | shape-mirror of gitea `fmt` |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` (strict only — no pedantic, since pedantic-only failures from contributors should be the maintainer's job to fix during cherry-pick) | shape-mirror of gitea `clippy` |
| `build` | `cargo build --workspace` | shape-mirror of gitea `build` |
| `test` | `cargo test --workspace` | shape-mirror of gitea `test` |

**Not reproduced** on the contributor CI:
- `clippy-pedantic` — too noisy for first-time contributors; maintainer applies during cherry-pick
- `dogfood` (`graph-specs check`) — runs on Gitea where the binary is built canonically; if the contributor's PR introduced spec drift, the Gitea run catches it before merge
- `audit` (cargo-audit) — runs weekly on Gitea cron; no need on every contributor PR
- `cfdb-check`, `cross-dogfood` — Gitea-only per §3.2

**No sccache.** The workspace is small (5 crates, ~1.5k LOC); cold builds on free-tier runners complete in 60–90s. Adding `mozilla-actions/sccache-action` is premature optimisation; revisit if cold-build time exceeds 3 min.

**Workflow yaml is ~50 lines total**, single file. CONTRIBUTING.md (R3-2 deliverable) tells external contributors:

> Open your PR on github.com/yannickgranger/graph-specs. The contributor-feedback CI will run fmt + clippy + build + test on your branch. The maintainer cherry-picks accepted PRs to the Gitea canonical write surface, where the full gate suite (incl. dogfood, cfdb, cross-dogfood) runs before merge. Merged commits flow back to the GitHub mirror via cron.

Honest about the dual-surface model; sets contributor expectations correctly.

### §3.4 — Documentation split

| File | Source | Notes |
|---|---|---|
| `CONTRIBUTING.md` | New, distilled from `CLAUDE.md` §1 (RFC-first), §2 (RFC pipeline), §3 (dual control), §4 (skill selection), §5 (self-hosting discipline), §6 (quick reference), §7 (companion policy) | Public methodology — every external contributor reads this before opening a PR. Carries the architect-team protocol verbatim. |
| `INTERNAL.md` | New, distilled from the private operational chunks of `CLAUDE.md` (kept locally; `.gitignore`-d if the author wants) OR moved entirely to the author's `~/agent-zero/` dotfiles | Agent-Zero gates, Podman BDD, /loop, ScheduleWakeup, council protocols, A0 cron — none of which apply to a Rust OSS project. |
| `CLAUDE.md` | Becomes a **3-line stub** that points to `CONTRIBUTING.md` for public methodology and to `INTERNAL.md` (or the dotfile path) for private setup | Preserves the slot Claude/agent setups expect to find a CLAUDE.md in. |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1, unmodified | Standard OSS expectation. |
| `.github/ISSUE_TEMPLATE/{bug,rfc-proposal,feature-request}.md` | New | Funnels external issues into shapes that map onto the existing RFC pipeline. |

The `CLAUDE.md` 3-line stub matters: agent harnesses (including this author's) auto-load `CLAUDE.md` on session start. Removing it would break the author's workflow; keeping the full content public would leak operational details that don't belong in an OSS repo.

### §3.5 — cfdb stays Gitea-only (no decoupling needed)

Folded into §3.2. cfdb gates run on the Gitea canonical CI; the GitHub contributor-feedback CI does not attempt to install cfdb. No env-flag, no opt-in, no lockstep PR with cfdb's RFC-034 Phase 1.

The eight `.cfdb/queries/*.cypher` ban rules and the cross-dogfood loop remain unchanged — they are the maintainer's pre-merge belt-and-suspenders on Gitea. CONTRIBUTING.md notes that the contributor-feedback CI does not run cfdb gates; if the maintainer's Gitea cycle finds a cfdb-rule violation in a cherry-picked PR, the maintainer either fixes inline (≤15 min per CLAUDE.md §7) or files a follow-up issue against the contributor's branch.

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
2. **Canonical CI is Gitea.** All eight Gitea jobs (fmt, clippy-strict, clippy-pedantic, build, test, dogfood, audit, cfdb-check + cross-dogfood) continue to run unchanged. The mirror workflow (R3-3a) only adds outbound `git push --mirror github`; it does not modify any existing Gitea job.
3. **Self-dogfood stays green throughout.** Every commit on the work branch passes `graph-specs check --specs specs/ --code .` against the latest tool binary on Gitea.
4. **Cross-dogfood stays green throughout.** The existing Gitea cross-dogfood job continues to pass against the pinned cfdb SHA; nothing in Phase 1 perturbs the cross-fixture protocol.
5. **No deletion of agent-author CLAUDE.md slot.** The 3-line stub preserves the path; agent harnesses that auto-load it continue to work.
6. **The github.com mirror is read-only-canonical from Gitea's perspective:** all writes go through Gitea first. PRs filed on github.com are cherry-picked or re-pushed by the maintainer; no direct merges on github.com until RFC-008 chooses a publishing model.
7. **Contributor-feedback CI is informational.** A green run on github.com does NOT authorize merge to develop. The maintainer's Gitea cycle is the gate.
8. **`#64` closes with R3-4.** The "Limits of equivalence checking" section is the documented mitigation; tool-side detection is RFC-009 work, not Phase 1.

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

1. **Not translating the eight Gitea jobs into `.github/workflows/`.** The contributor-feedback CI (§3.3) reproduces only four (fmt / clippy-strict / build / test); the canonical CI stays on Gitea.
2. **Not running cfdb gates on the GitHub side.** cfdb stays Gitea-only (§3.2). No env-flag, no opt-in.
3. Not building a bidirectional mirror. Gitea→GitHub only; GitHub→Gitea is the maintainer's manual cherry-pick on accepted PRs.
4. Not changing the equivalence engine.
5. Not publishing to crates.io (deferred to RFC-008).
6. Not changing the dual MIT/Apache license.
7. Not addressing PHP / TS adapters (RFC-004 / 005 / 006 territory).
8. Not building the tool-side symmetric-absence detector (RFC-009 placeholder; #64 closes with the documented mitigation per R3-4).

## §7 — Issue decomposition

Each child issue carries the standard `Tests:` template (Unit / Self dogfood / Cross dogfood / Target dogfood). Tests prescription is the architect-team's responsibility per CLAUDE.md §2.5; the table below is the architect's first-cut prescription, refined during the round-1 review.

| ID | Slice | Tests prescription |
|---|---|---|
| **R3-1** | Update `Cargo.toml.repository` to `https://github.com/yannickgranger/graph-specs`. Update README links from `agency.lab:3000` to `github.com/yannickgranger`. Add `.github/ISSUE_TEMPLATE/{bug,rfc-proposal,feature-request}.md`. | Unit: none — text changes. Self dogfood: tool runs unchanged after the URL flip (smoke). Cross dogfood: existing pinned cfdb SHA still resolves correctly via Gitea. Target dogfood: none. |
| **R3-2** | Doc split: `CLAUDE.md` → `CONTRIBUTING.md` + `INTERNAL.md` + 3-line `CLAUDE.md` stub. Add `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1). `CONTRIBUTING.md` documents the dual-surface model (Gitea canonical, GitHub mirror, contributor-feedback CI vs canonical CI distinction). | Unit: none — docs only. Self dogfood: 0 violations. Cross dogfood: none. Target dogfood: none — rationale: no executable surface touched. |
| **R3-3** | **Gitea→GitHub mirror + contributor-feedback CI.** Two artifacts in one slice: (a) `.gitea/workflows/mirror-to-github.yml` — runs on push to `develop` / `main` / tags, executes `git push --mirror github` using a `GITHUB_MIRROR_PAT` secret; (b) `.github/workflows/contributor-ci.yml` — fires only on `pull_request: [opened, synchronize, reopened]`, four jobs (fmt / clippy-strict / build / test) on `ubuntu-latest` with `rust:1.93` image, no sccache. Plus a tiny optional issue-mirror webhook (Gitea side, OQ-3 resolved) that copies new GitHub issues to Gitea. | Unit: yaml-lint + shellcheck on both workflow files. Self dogfood: a throwaway PR against the GitHub mirror runs all four jobs green. Cross dogfood: the new mirror workflow on Gitea pushes successfully without breaking existing Gitea jobs. Target dogfood: a green run on `github.com/yannickgranger/graph-specs/pulls/<n>` is the proof. |
| **R3-4** | New "Limits of equivalence checking" section in `specs/concepts/core.md`. Closes #64. Also notes the workflow-mode dimension surfaced in §9 (legacy archaeology has the same blind-spot shape, amplified for codebases without inline attributes). | Unit: none. Self dogfood: section is gated as a spec concept, dogfood passes. Cross dogfood: cfdb still passes. Target dogfood: none. |
| **R3-5** | README rewrite per §3.6. | Unit: none. Self dogfood: no impact. Cross dogfood: no impact. Target dogfood: none. |

All five slices can ship in parallel after this RFC ratifies — there are no internal dependencies. R3-3 can land before or after the doc split (R3-2); the mirror works regardless of CONTRIBUTING.md state, and contributor-feedback CI is independent of doc layout.

**Slices removed from the round-1 draft** (folded into §3.2 + §3.3 simplifications):
- ~~R3-4 r1 (cfdb decoupling via `OSS_CFDB_GATES` env-flag)~~ — DROPPED, cfdb stays Gitea-only
- ~~R3-7 r1 (cfdb URL flip lockstep with RFC-034 Phase 1)~~ — DROPPED, no canonical-URL change needed; the github cfdb mirror is purely for visibility

## §8 — Open questions

| ID | Question | Resolution |
|---|---|---|
| OQ-1 | Limits-doc location: `specs/concepts/core.md` vs `docs/equivalence-limits.md`? | RESOLVED — under the gate, in `specs/concepts/core.md`. |
| OQ-2 | One-way mirror trigger: Gitea Actions on `push` to develop/main/tags vs scheduled cron? | RESOLVED — Gitea Actions `on: push` for near-instant mirror; cron not needed. |
| OQ-3 | Do we accept external PRs directly on github.com or require they re-file on Gitea? | RESOLVED — accept on github.com, cherry-pick to Gitea on merge. CONTRIBUTING.md (R3-2) documents the dual-surface model. |
| ~~OQ-4~~ | ~~Should the `cfdb-gates` job ever run on external PRs?~~ | DROPPED — no `cfdb-gates` job exists in the revised design (cfdb stays Gitea-only per §3.2). |

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

The blind spot to flag in `CONTRIBUTING.md` (R3-2 deliverable): legacy code without inline attributes can silently lose a class — deleting the class also deletes any inline declaration, so only the markdown spec retains the "this concept must exist" intent. The greenfield mode is naturally protected because the markdown was authored first; the legacy mode depends on the discipline of locking the rebuilt markdown and never deleting concept entries without a spec-back PR. This is a strict superset of the symmetric-absence blind spot in §3.7 — the limits-doc (R3-4) should mention the workflow-mode dimension.

The blind spot to flag in `CONTRIBUTING.md` (§3.4 deliverable): legacy code without inline attributes can silently lose a class — deleting the class also deletes any inline declaration, so only the markdown spec retains the "this concept must exist" intent. The greenfield mode is naturally protected because the markdown was authored first; the legacy mode depends on the discipline of locking the rebuilt markdown and never deleting concept entries without a spec-back PR. This is a strict superset of the symmetric-absence blind spot in §3.7 — the limits-doc R3-5 ships should mention the workflow-mode dimension.

## §10 — Ratification

Awaiting round-1 architect-team verdicts. RFC ratifies when all four lenses return RATIFY (or reject with documented overrides per CLAUDE.md §2.3).

After ratification, §7 becomes the concrete backlog. Each row is filed as a forge issue with body `Refs: docs/rfc/003-oss-readiness.md`, worked via `/work-issue-lib`, shipped through the dual-control regime (graph-specs check + cfdb violations + the documented opt-in `OSS_CFDB_GATES` shape).
