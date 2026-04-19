# graph-specs-rust — CLAUDE.md

Repo-local rules. Extends the global `~/.claude/CLAUDE.md`, does not replace it.

## §1 — Core methodology

**New code is RFC-first. After RFC, issues. Dual control on every PR.**

| Work type | Path |
|---|---|
| New capability (output format, equivalence level, CLI subcommand, violation variant, schema-version bump) | RFC → architect review → issues → dual-control PRs |
| Bug fix (wrong behavior on existing capability) | Issue → `/work-issue-lib` → dual-control PR |
| Mechanical (rename, file split, dedup) | Issue → `/fix-mechanical` → dual-control PR |
| Docs, CI config, chore | Issue → direct PR → dual-control PR |

"RFC-first" means: no implementation issue is filed until the RFC is ratified. Writing implementation without a ratified RFC is a methodology violation — the shape of the solution must be negotiated in the RFC, not in the PR.

## §2 — RFC pipeline

### §2.1 — Where RFCs live

`docs/rfc/NNN-<kebab-title>.md` — numbered sequentially.

### §2.2 — RFC contents

Every RFC answers, in this order:

1. **Problem.** What user-visible or methodology problem does this solve? Cite the concrete session, issue, or upstream request that prompted the RFC.
2. **Scope.** Exact deliverables — what ships, what does not.
3. **Design.** Types, CLI surface, wire format, schema additions, exit codes.
4. **Invariants.** What must still hold after the change — test corpus, stable wire schema, backward compatibility.
5. **Architect lenses.** Dedicated subsections for each architect perspective (see §2.3). Architects' verdicts are captured inline.
6. **Non-goals.** What this RFC explicitly does not address.
7. **Issue decomposition.** Vertical slices, one issue each. Each entry carries an explicit `Tests:` line naming the test surface per §2.5 — architects prescribe, implementers execute.

Schema-v1 of the NDJSON output at `specs/ndjson-output.md` is the model for how an RFC graduates into an authoritative specification once ratified.

### §2.3 — Architect review via agent teams

Reference: https://code.claude.com/docs/en/agent-teams.

Every RFC is reviewed by a team of architect sub-agents, one teammate per lens:

| Lens | Subagent type | Question answered |
|---|---|---|
| Clean architecture | `clean-arch` | Dependency direction, port purity, screaming architecture |
| Domain-driven design | `ddd-specialist` | Bounded contexts, homonym detection, aggregate boundaries |
| SOLID + component principles | `solid-architect` | SRP, ISP, CCP, main-sequence distance |
| Rust systems | `rust-systems` | Crate granularity, feature flags, trait object safety, orphan rules |

Invocation is via `Agent(subagent_type=...)` or agent teams — whichever affords the review. Each lens returns a verdict (RATIFY / REJECT / REQUEST CHANGES) with evidence. The RFC is not ratified until all four verdicts are RATIFY, or a single author-documented override is recorded inline.

**Architects also prescribe tests** (§2.5). The verdict is not complete until each issue in the decomposition carries a named test surface — unit, integration, dogfood assertion (`graph-specs check` against this repo's own `specs/concepts/`), or a documented `Tests: none` rationale. Implementers do not choose the test shape; they deliver against the prescription.

### §2.4 — Ratification → issues

Once ratified, the RFC's "Issue decomposition" section becomes the concrete backlog. Each vertical slice is filed as a `forge_create_issue` with body linking back to the RFC (`Refs: docs/rfc/NNN-...md`) and carrying the prescribed `Tests:` section from the RFC verbatim. Issues are worked via `/work-issue-lib`. A PR against an issue without the prescribed test is not merged.

### §2.5 — Tests and real infra

**Tests are always mandatory when possible.** "When possible" = there is an executable path the change touches that can be exercised deterministically. "Mandatory" = the PR implementing the issue lands the prescribed test; a PR without it is not merged. Architects prescribe in the RFC + issue body (§2.3 + §2.4); implementers pass.

**Real infra is always preferred over mocks.** The hierarchy:

1. **Dogfood / self-integration.** Exercise the change against this repo's own source tree via `graph-specs check --specs specs/concepts/ --code .` and assert on the observable output (e.g. "the new output variant appears in NDJSON", or "violations count is still zero after the refactor"). Strongest signal — real data, real pipeline.
2. **Integration against real inputs.** Construct a small real-shaped fixture (a synthetic `specs/` directory, a crafted Rust source file) and run the markdown and rust readers end-to-end. Assert on the resulting `Graph` / `Violation` output.
3. **Unit tests on pure functions.** Fine when the function is genuinely pure. Do not stub out reader I/O that could be exercised via option 2.
4. **Mocks / doubles.** Last resort. Must carry an inline comment naming why real infra was unavailable.

**Prescribed test categories by work type:**

| Work type | Required test |
|---|---|
| New capability (output variant, equivalence level, CLI subcommand, schema bump) | Dogfood against this repo's own specs/code **AND** unit tests for extracted pure functions **AND** integration fixture covering the new surface |
| Bug fix | Regression test that reproduces the bug first (red → green in the same PR) |
| Mechanical refactor | No new tests; the existing suite must pass byte-identically |
| Docs / CI / chore | No test required; the change is its own verification surface |

**Escape hatch.** An issue that is genuinely untestable carries `Tests: none — rationale: <why>` in its body. "I didn't bother" is not a valid rationale.

## §3 — Dual control

Every PR passes both gates. CI enforces them (`.gitea/workflows/ci.yml` jobs `dogfood` and `cfdb-check`).

| Gate | Tool | Question answered | Failure mode |
|---|---|---|---|
| Equivalence | `graph-specs check --specs specs/concepts/ --code .` | "Do the markdown specs match the code?" | Adding a `pub` type without a spec entry, or changing a signature without updating the spec |
| Architectural bans | `cfdb violations` over `.cfdb/queries/*.cypher` | "Does the code use forbidden patterns?" | Introducing `.unwrap()` in `domain/` or `ports/`; future rules added per new RFC |

**Adding a new ban rule is an RFC-gated change.** The rule goes into the same PR as the code motivating it, with `schema_version: 1` proof that develop is zero-violation before the rule lands.

**Adding a new concept / trait / output variant is specs-gated.** The spec entry goes in `specs/concepts/` in the same PR as the code.

## §4 — Skill selection

| Scenario | Skill |
|---|---|
| New vertical slice derived from a ratified RFC | `/work-issue-lib` |
| Bug fix on existing behavior | `/work-issue-lib` (or `/fix-issue` if framing is regression-first) |
| Rename / move / dedup / file split | `/fix-mechanical` |
| N parallel mechanical refactors | `/sweep-epic` |
| Pre-push | `/ship` — the only authorized push + PR path |

The full `/work-issue` orchestrator (with Podman, BDD, bounded-context raid) is overkill for this repo — it is a pure library with no external infrastructure.

## §5 — Self-hosting discipline

graph-specs-rust dogfoods itself from day zero:

- `specs/concepts/core.md` describes graph-specs-rust's own public API.
- `specs/ndjson-output.md` is the authoritative NDJSON v1 schema contract referenced by downstream consumers (qbot-core Study 002 v4.2 Phase A1).
- `specs/dialect.md` documents the markdown + Rust reader rules.

The tool's own `check` runs against `specs/concepts/` + `.` on every CI push. A new concept in code without a spec entry blocks the PR — this is the REUSE / CREATE test; no sub-agent discovery is needed for this codebase.

## §6 — Quick reference

```bash
# Local dual-control check before pushing
cargo build --release -p application
./target/release/graph-specs check --specs specs/concepts/ --code .
mkdir -p .cfdb/db && cfdb extract --workspace . --db .cfdb/db --keyspace graph-specs
for r in .cfdb/queries/*.cypher; do cfdb violations --db .cfdb/db --keyspace graph-specs --rule "$r"; done

# Ship
/ship <issue> agency:yg/graph-specs-rust --workspace <path>
```

## §7 — Companion policy

The same RFC-first + architect-review methodology applies to `yg/cfdb`. See that repo's `CLAUDE.md`.
