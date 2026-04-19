#!/usr/bin/env bash
# ci/read-cross-fixture-sha-test.sh
#
# Unit tests for ci/read-cross-fixture-sha.sh per RFC-002 Issue B1 Tests:
#   - Unit: parser echoes valid 40-char SHA; rejects malformed inputs
#     (missing quote, unanchored match, placeholder zeros, absent field).
#   - Self dogfood: parser succeeds on the checked-in fixture.
#
# Mirror of cfdb's ci/read-cross-fixture-sha-test.sh. Kept byte-identical
# in structure so divergence is obvious to a diff-based reviewer; only the
# repo name in the `valid.toml` fixture is graph-specs-specific, and even
# that is incidental (the parser does not inspect the repo field).
#
# No test framework — plain assertions so this runs in the CI setup step
# before cargo is warm. Exits non-zero on any failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PARSER="$SCRIPT_DIR/read-cross-fixture-sha.sh"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0
pass=0

assert_pass() {
    local name="$1" fixture="$2" expected="$3"
    local got
    if got="$("$PARSER" "$fixture" 2>/dev/null)" && [ "$got" = "$expected" ]; then
        pass=$((pass + 1))
        echo "PASS: $name"
    else
        fail=$((fail + 1))
        echo "FAIL: $name — expected '$expected', got '$got'" >&2
    fi
}

assert_fail() {
    local name="$1" fixture="$2" want_exit="$3"
    local got_exit=0
    "$PARSER" "$fixture" >/dev/null 2>&1 || got_exit=$?
    if [ "$got_exit" -eq "$want_exit" ]; then
        pass=$((pass + 1))
        echo "PASS: $name (exit $got_exit)"
    else
        fail=$((fail + 1))
        echo "FAIL: $name — expected exit $want_exit, got $got_exit" >&2
    fi
}

# Happy path — valid 40-char lowercase hex.
cat > "$TMP/valid.toml" <<'EOF'
[companion]
repo = "yg/cfdb"
sha  = "b39bf622e3baee48f2a1383177507b3e3952ae58"
EOF
assert_pass "valid sha extracted" "$TMP/valid.toml" "b39bf622e3baee48f2a1383177507b3e3952ae58"

# Anchored-grep check — a commented-out sha must not match (§3.1 RC3).
cat > "$TMP/commented.toml" <<'EOF'
[companion]
# sha = "ffffffffffffffffffffffffffffffffffffffff"
sha = "b39bf622e3baee48f2a1383177507b3e3952ae58"
EOF
assert_pass "commented sha ignored, real sha returned" \
    "$TMP/commented.toml" "b39bf622e3baee48f2a1383177507b3e3952ae58"

# Only a commented-out sha — nothing real to return.
cat > "$TMP/only-commented.toml" <<'EOF'
[companion]
# sha = "b39bf622e3baee48f2a1383177507b3e3952ae58"
EOF
assert_fail "only-commented sha rejected" "$TMP/only-commented.toml" 2

# No sha field at all.
cat > "$TMP/missing.toml" <<'EOF'
[companion]
repo = "yg/cfdb"
EOF
assert_fail "missing sha rejected" "$TMP/missing.toml" 2

# Placeholder all-zeros sentinel.
cat > "$TMP/zeros.toml" <<'EOF'
[companion]
sha = "0000000000000000000000000000000000000000"
EOF
assert_fail "placeholder zeros rejected" "$TMP/zeros.toml" 3

# Wrong length.
cat > "$TMP/short.toml" <<'EOF'
[companion]
sha = "b39bf622"
EOF
assert_fail "short sha rejected" "$TMP/short.toml" 3

# Uppercase hex — reject (git outputs lowercase).
cat > "$TMP/upper.toml" <<'EOF'
[companion]
sha = "B39BF622E3BAEE48F2A1383177507B3E3952AE58"
EOF
assert_fail "uppercase sha rejected" "$TMP/upper.toml" 3

# Missing fixture file.
assert_fail "missing fixture rejected" "$TMP/does-not-exist.toml" 1

# Self-dogfood — the checked-in fixture parses.
CHECKED_IN="$REPO_ROOT/.cfdb/cross-fixture.toml"
if [ -f "$CHECKED_IN" ]; then
    got="$("$PARSER" "$CHECKED_IN" 2>/dev/null)" || {
        fail=$((fail + 1))
        echo "FAIL: self-dogfood — checked-in fixture $CHECKED_IN did not parse" >&2
        got=""
    }
    if printf '%s' "$got" | grep -Eq '^[0-9a-f]{40}$'; then
        pass=$((pass + 1))
        echo "PASS: self-dogfood — checked-in fixture yields 40-char lowercase hex"
    elif [ -n "$got" ]; then
        fail=$((fail + 1))
        echo "FAIL: self-dogfood — checked-in fixture returned '$got'" >&2
    fi
else
    echo "SKIP: self-dogfood — $CHECKED_IN absent (run tests after B1 ships the file)"
fi

echo
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
