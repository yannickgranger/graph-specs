# Contributing to graph-specs

Thanks for your interest in `graph-specs` — a graph-based **equivalence
checker** between markdown specifications and source code. This document is
the contributor-oriented summary of the project's working discipline.

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
Contributions are dual-licensed **MIT OR Apache-2.0** (see `LICENSE-MIT`
and `LICENSE-APACHE`); by submitting a PR you agree your work is offered
under both.

## TL;DR

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# Run the tool on its own specs (it spec-checks itself):
cargo build --release -p application
./target/release/graph-specs check --specs specs/ --code .   # must print "0 violations."
```

A pull request that is `fmt`-clean, `clippy`-clean (`-D warnings`), builds,
and passes `cargo test --workspace` will pass the public CI
(`.github/workflows/contributor-ci.yml`). The maintainer runs the full
dual-control suite (below) on merge.

## How the project is built: spec-first, self-hosting

graph-specs validates its own specs from the first commit. `specs/` holds
markdown specifications for the tool itself; the tool reads those specs
plus its own source and diffs them. **Every change keeps `specs/` and the
code in agreement** across [five equivalence levels](README.md#the-five-levels-of-equivalence):
concept, signature, relationship, bounded-context, and cohesion (the
abstraction ladder). A `pub` type with no spec entry — or a spec entry with
no type — is a violation that blocks the merge.

Practically, this means:

- **Add a `pub struct`/`enum`/`trait`/`type`?** Add a matching `##` entry in
  the owning `specs/concepts/<context>.md`.
- **Add a cross-context dependency?** Declare it in
  `specs/contexts/<context>.md` (`Owns` / `Exports` / `Imports`).
- Run `graph-specs check --specs specs/ --code .` locally; it must report
  `0 violations.`

## What kind of change are you making?

| Change | Path |
|---|---|
| Bug fix (wrong behaviour on existing capability) | Open an issue → PR with a **regression test** that reproduces the bug first |
| New capability (output format, equivalence level, CLI subcommand, violation variant, schema-version bump) | **RFC first** (`docs/rfc/NNN-*.md`) → discussion → issues → PR |
| Docs, CI, chore | PR directly |

**RFC-first** means: for a genuinely new capability, the *shape* of the
solution is negotiated in a short RFC under `docs/rfc/`, not in the PR. This
keeps the design reviewable before code exists. Look at the existing RFCs
(`docs/rfc/001`…`010`) for the format: Problem, Scope, Design, Invariants,
Non-goals, and an issue decomposition with a `Tests:` line per slice.

Open a `rfc-proposal` issue (template provided) to start that conversation.

## Vertical slices

Issues are **vertical slices** — each delivers observable behaviour
end-to-end (domain types → reader/adapter → CLI/output → a test), not one
layer across many features. If a change only touches a single layer with no
exercisable behaviour, it is usually a mechanical refactor (rename / move /
dedup), which is fine on its own but should say so.

## Tests

Tests are mandatory whenever there is a deterministic path to exercise.
**Real infrastructure is preferred over mocks**, in this order:

1. **Dogfood** — run `graph-specs check` against this repo's own
   `specs/` + source and assert on the output (e.g. "still 0 violations",
   "the new record appears in NDJSON"). Strongest signal.
2. **Integration** — build a small real-shaped fixture (a synthetic
   `specs/` dir, a crafted `.rs` file) and run the readers end-to-end.
3. **Unit** — for genuinely pure functions.
4. **Mocks** — last resort; comment why real infra was unavailable.

A new-capability PR carries unit tests for any extracted pure function
**and** a dogfood assertion **and** an integration fixture for the new
surface. A bug-fix PR carries a red→green regression test in the same PR.

## The gates (what must hold)

Public PRs are checked by `contributor-ci.yml` (fmt, clippy `-D warnings`,
build, test). On the maintainer's side, every change additionally passes:

| Gate | Question |
|---|---|
| **Equivalence** — `graph-specs check --specs specs/ --code .` | Do the specs match the code? |
| **Clippy (pedantic + nursery)** | Is the code held to the cleanup-every-PR standard? |
| **Architectural bans** | Does the code avoid forbidden patterns (e.g. `.unwrap()` in `domain`/`ports`)? |
| **Cross-dogfood** | Does the tool still produce zero findings on its companion fixture? |

There is **no baseline file, no ratchet, no allowlist** — violations are
fixed in the same PR that introduces them, or the PR does not land.

## Commit & PR conventions

- Conventional-commit prefixes: `feat:`, `fix:`, `docs:`, `chore:`,
  `refactor:`, `test:`. Reference the issue/RFC (`#123`, `Refs: docs/rfc/010-…`).
- Keep PRs focused — one slice per PR.
- Describe what you changed and how you verified it.

## Getting help

Open an issue (bug / feature-request / rfc-proposal templates are under
`.github/ISSUE_TEMPLATE/`). Questions about the design are welcome as an
`rfc-proposal` or a plain issue.
