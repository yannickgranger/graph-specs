#!/usr/bin/env bash
# ci/cross-dogfood.sh (graph-specs flavour)
#
# RFC-002 §3.2 — runs the locally-built graph-specs binary against the
# companion repo (cfdb) at the SHA pinned in `.cfdb/cross-fixture.toml`.
# Scripted once, invoked by:
#   - PR-time CI job (`.gitea/workflows/ci.yml` → cross-dogfood)
#   - Weekly cross-bump cron (Issue C1 mirror — Monday 06:30 UTC)
#   - Weekly closed-loop cron at companion HEAD (E1 mirror — Tuesday 06:30 UTC)
#
# Differentiated exit codes (rust-systems B2):
#   0  — cross-dogfood pass (graph-specs check clean on companion tree)
#   10 — companion clone or checkout failed (infra problem)
#   20 — `graph-specs check` failed to START (reader panicked, schema-
#        consumption mismatch during an I2 lockstep window — see
#        RFC-002 §3.3 / Invariant I2). Upgrade to differentiate reader-
#        level failures from diff-level findings once graph-specs
#        exposes that distinction.
#   30 — `graph-specs check` returned non-zero exit (one or more
#        violations on the companion tree). Per RFC-002 §3.4 there is
#        NO allowlist: fix in the companion repo (land a fix PR, then
#        bump `.cfdb/cross-fixture.toml`) or scope the violation shape
#        narrower — never add an exemption file.
#
# Two SHA universes (rust-systems C1). The graph-specs binary used
# here is THIS PR's `./target/release/graph-specs`, NOT a pinned-SHA
# binary. The cfdb binary used by this repo's `cfdb-check` CI job is
# installed from `.cfdb/cfdb.rev`. Intentional decoupling:
#   - THIS script answers "does my current graph-specs handle the
#     companion's code?" (test target = companion SOURCE TREE).
#   - The cfdb-check job answers "does this repo's code satisfy a
#     known-good cfdb's ban rules?" (test target = own code, via a
#     pinned companion binary).
# Future maintainers: do NOT unify `.cfdb/cfdb.rev` and
# `.cfdb/cross-fixture.toml`. The questions differ.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

COMPANION_REPO="${COMPANION_REPO:-yg/cfdb}"
COMPANION_URL_BASE="${COMPANION_URL_BASE:-https://agency.lab:3000}"
COMPANION_DIR="${COMPANION_DIR:-$(mktemp -d)}"
GS_BIN="${GS_BIN:-$REPO_ROOT/target/release/graph-specs}"

if [ ! -x "$GS_BIN" ]; then
    echo "cross-dogfood: graph-specs binary not found at $GS_BIN" >&2
    echo "  hint: cargo build -p application --release" >&2
    exit 2
fi

# The pinned SHA is the default; the weekly bump cron (Issue #34,
# ci/cross-bump.sh) overrides via COMPANION_SHA env to test against
# companion develop HEAD. Same script, two universes: PR-time uses the
# pin for reproducibility; the cron uses HEAD to detect pin staleness.
COMPANION_SHA="${COMPANION_SHA:-$("$SCRIPT_DIR/read-cross-fixture-sha.sh")}"

if [ -n "${GITHUB_TOKEN:-}" ]; then
    git config --global url."https://oauth2:${GITHUB_TOKEN}@agency.lab:3000/".insteadOf "https://agency.lab:3000/"
fi
git clone --filter=blob:none "${COMPANION_URL_BASE}/${COMPANION_REPO}.git" "$COMPANION_DIR" \
    || exit 10
(cd "$COMPANION_DIR" && git checkout "$COMPANION_SHA") || exit 10

# graph-specs check exit semantics (v0.4):
#   0 = pass, non-zero = one or more violations reported.
# The distinction between reader-level failure (exit 20 candidate) and
# diff-level finding (exit 30) is not exposed by graph-specs today; for
# now we map all non-zero to 30 and leave a hook for future refinement.
gs_exit=0
"$GS_BIN" check \
    --specs "$COMPANION_DIR/specs/concepts/" \
    --code  "$COMPANION_DIR/crates/" \
    || gs_exit=$?

if [ "$gs_exit" -eq 0 ]; then
    echo "cross-dogfood: 0 violations on ${COMPANION_REPO}@${COMPANION_SHA:0:12}"
    exit 0
fi

echo "cross-dogfood: FAIL — graph-specs check exit=$gs_exit on ${COMPANION_REPO}@${COMPANION_SHA:0:12}" >&2
exit 30
