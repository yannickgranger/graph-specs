#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FIXTURE="$SCRIPT_DIR/fixtures/php-foreign-context"

CFDB_BIN="${CFDB_BIN:-cfdb}"
GS_BIN="${GS_BIN:-$REPO_ROOT/target/release/graph-specs}"

resolved() {
    local bin="$1"
    if [ -x "$bin" ]; then
        ( cd "$(dirname "$bin")" && printf '%s/%s\n' "$(pwd)" "$(basename "$bin")" )
    else
        command -v "$bin" 2>/dev/null
    fi
}

for name in CFDB_BIN GS_BIN; do
    eval "bin=\$$name"
    abs="$(resolved "$bin" || true)"
    [ -n "$abs" ] && [ -x "$abs" ] || {
        echo "php-foreign-context: $bin not found or not executable" >&2
        echo "  hint: CFDB_BIN=<pinned cfdb> GS_BIN=<this tree's graph-specs> $0" >&2
        exit 2
    }
    eval "$name=\$abs"
done

[ -d "$FIXTURE" ] || {
    echo "php-foreign-context: fixture absent at $FIXTURE, so this script would assert nothing" >&2
    exit 2
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp -r "$FIXTURE" "$WORK/tree"
TREE="$WORK/tree"

CONTEXT_FILE="$TREE/specs/contexts/catalogue.md"
IMPORT_LINE='- WidgetId from contracts (PublishedLanguage)'
grep -qF -- "$IMPORT_LINE" "$CONTEXT_FILE" || {
    echo "php-foreign-context: the fixture no longer declares '$IMPORT_LINE', so case 2 would remove nothing and both cases would assert the same tree" >&2
    exit 2
}

run_check() {
    local label="$1" out status
    rm -rf "$TREE/var"
    mkdir -p "$TREE/var"
    ( cd "$TREE" && "$CFDB_BIN" extract --workspace . --db var/db --keyspace fixture ) >/dev/null
    set +e
    out="$( cd "$TREE" && "$GS_BIN" check --specs specs --code . --keyspace var/db/fixture.json 2>&1 )"
    status=$?
    set -e
    echo "==> $label"
    echo "$out" | sed 's/^/    /'
    echo "    exit: $status"
    VERDICT="$out"
    STATUS="$status"
}

run_check "case 1 — the import declared, both instruments answering"
[ "$STATUS" -eq 0 ] || {
    echo "php-foreign-context: FAILED — the declared import is not realised. Either cfdb no longer indexes the installed package that declares a context, or the reader no longer loads its specs as a foreign context." >&2
    exit 1
}
echo "$VERDICT" | grep -qE '^0 violations' || {
    echo "php-foreign-context: FAILED — exit 0 with a verdict that is not '0 violations'; the gate must read the verdict and not the status alone." >&2
    exit 1
}

python3 - "$CONTEXT_FILE" "$IMPORT_LINE" <<'PY'
import sys

path, line = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as handle:
    text = handle.read()
stripped = text.replace(f"\n{line}\n", "\n")
if stripped == text:
    sys.exit(f"php-foreign-context: could not remove {line!r} from {path}")
with open(path, "w", encoding="utf-8") as handle:
    handle.write(stripped)
PY

run_check "case 2 — the same tree with the import line removed"
[ "$STATUS" -ne 0 ] || {
    echo "php-foreign-context: FAILED — an undeclared crossing onto the installed package exits 0. The realisation channel answers but the refusal does not, so the declaration is decorative." >&2
    exit 1
}
echo "$VERDICT" | grep -qF 'cross-context edge unauthorized: Shelf (catalogue) --USES--> WidgetId (contracts)' || {
    echo "php-foreign-context: FAILED — the run refuses, but not with the undeclared-crossing row naming Shelf and WidgetId; something else is red and this gate would have passed on the wrong reason." >&2
    exit 1
}

echo "php-foreign-context OK — the declared import realises, and removing the line refuses the crossing by name"
