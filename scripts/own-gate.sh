#!/usr/bin/env bash
# graph-specs' own gate, corpus-wide (keel-harness §7 step 3, §3.2, §8): keel measures
# the deployment level keel.json declares — L3, gated: the cascade pin present, the
# doxa corpus checked out at doxa.rev, the full declared root every run, every
# ##/### heading of specs/concepts carrying a verdict (§3.2: any shortfall is
# silence — refused regardless of how clean the visited subset is), and CI refuses
# on any corpus-wide finding (§8, the L3 row — declared once §7 step 2 reached zero
# ungrounded on this tree, 57/57). What refuses here holds at every level: silence
# (§3.2); a level declared but not running (§8: a repo never claims a level it is
# not running); the corpus absent or unreadable at its pin (keel-harness §3.1 /
# keel-dialect §3.3: unavailable, never a pass). Run-level findings (unclosed
# fence, empty RFC / specs / code) stay out of the node verdicts (keel-dialect §7)
# and refuse here like any other finding (§8, the L3 row).
# Exit 0 = the declared level holds on this tree; anything else refuses.
set -euo pipefail
cd "$(dirname "$0")/.."

CACHE="${GS_BINARY_CACHE:-$HOME/.local/share/graph-specs/binaries}"

resolve_pin() {
  name=$1; bin=$2
  revfile="$name.rev"
  [ -r "$revfile" ] || { echo "FATAL: $revfile is absent — the gate has no rev to resolve $bin against, and a gate that cannot name its pin is not one that passed (keel-harness §6.5)" >&2; return 1; }
  rev=$(tr -d '[:space:]' < "$revfile")
  [ "${#rev}" -eq 40 ] || { echo "FATAL: $revfile holds ${#rev} characters, not a 40-character rev" >&2; return 1; }
  path="$CACHE/$bin-$rev"
  [ -x "$path" ] || { echo "FATAL: $bin @ ${rev:0:12} is absent from $CACHE or not executable — run scripts/provision-instruments.sh; the gate resolves every instrument through this tree's pins and never through PATH, where a bare name is whatever build is nearby (keel-harness §6.5)" >&2; return 1; }
  target=$(readlink -f "$path")
  case "$target" in
    "$(readlink -f "$CACHE")"/*) ;;
    *) echo "FATAL: $bin @ ${rev:0:12} resolves to $target, outside $CACHE — a convenience symlink is not a pinned binary" >&2; return 1 ;;
  esac
  [ -r "$path.sha256" ] || { echo "FATAL: $bin @ ${rev:0:12} carries no digest at $path.sha256 — the file name is a claim the cache does not check; re-run scripts/provision-instruments.sh, which writes the digest as it stages" >&2; return 1; }
  recorded=$(tr -d '[:space:]' < "$path.sha256")
  actual=$(sha256sum "$path" | cut -d' ' -f1)
  [ "$recorded" = "$actual" ] || { echo "FATAL: $bin @ ${rev:0:12} is not the file provisioning staged — recorded ${recorded:0:16}, found ${actual:0:16}; a binary copied under a pin's name answers as that pin until the digest is read" >&2; return 1; }
  RESOLVED="$path"
}

resolve_pin cascade cascade || exit 1
CASCADE_BIN="$RESOLVED"
resolve_pin cascade vocab || exit 1
VOCAB_BIN="$RESOLVED"
resolve_pin keel keel || exit 1
KEEL="$RESOLVED"

if [ -n "${KEEL_OVERRIDE:-}" ]; then
  case "$(readlink -f "$KEEL_OVERRIDE")" in
    "$(readlink -f "$CACHE")"/keel-"$(tr -d '[:space:]' < keel.rev)") KEEL="$KEEL_OVERRIDE" ;;
    *) echo "FATAL: KEEL_OVERRIDE does not point at $CACHE/keel-$(tr -d '[:space:]' < keel.rev) — an override that is not the pinned binary is the PATH problem with a different name" >&2; exit 1 ;;
  esac
fi

BIN_DIR=$(mktemp -d)
trap 'rm -rf "$BIN_DIR"' EXIT
ln -s "$CASCADE_BIN" "$BIN_DIR/cascade"
ln -s "$VOCAB_BIN" "$BIN_DIR/vocab"
DOXA_REV=$(tr -d '[:space:]' < doxa.rev)
DOXA_DIR="${DOXA_DIR:-.doxa}"
if [ ! -d "$DOXA_DIR/.git" ]; then
  git clone -q https://agency.lab:3000/yg/doxa.git "$DOXA_DIR" || { echo "FATAL: doxa could not be cloned — the corpus is unavailable, never a pass (keel-harness §3.1)" >&2; exit 1; }
fi
git -C "$DOXA_DIR" fetch -q origin || true
git -C "$DOXA_DIR" checkout -q "$DOXA_REV" || { echo "FATAL: doxa rev $DOXA_REV not in the clone — the corpus is unreadable at its pin, unavailable, never a pass (keel-harness §3.1; keel-dialect §3.3)" >&2; exit 1; }
[ -f "$DOXA_DIR/index.json" ] || { echo "FATAL: doxa checkout carries no index.json — unreadable at its pin, unavailable (keel-harness §3.1)" >&2; exit 1; }

echo "==> mirror: docs/rfc/*.md is the corpus' own graph-specs-* set at $DOXA_REV, byte-identical — read-only, the corpus is the one source"
python3 scripts/doxa-mirror-check.py --self-test || { echo "FATAL: the mirror check's own positive control does not fire — the check proves nothing, never a pass" >&2; exit 1; }
python3 scripts/doxa-mirror-check.py --doxa "$DOXA_DIR" --prefix graph-specs --path-template 'docs/rfc/{tail}.md' --mirror 'docs/rfc/*.md' || exit 1

echo "==> instruments resolved from this tree's pins into $CACHE, handed to keel as --bin-dir; PATH is not consulted (keel-harness §6.5)"
echo "    cascade @ $(tr -d '[:space:]' < cascade.rev | cut -c1-12) <- $CASCADE_BIN"
echo "    vocab   @ $(tr -d '[:space:]' < cascade.rev | cut -c1-12) <- $VOCAB_BIN"
echo "    keel    @ $(tr -d '[:space:]' < keel.rev | cut -c1-12) <- $KEEL"
echo "==> own gate: keel level --repo . --declaration keel.json --corpus $DOXA_DIR@$DOXA_REV --bin-dir <pinned>"
level_json=$("$KEEL" level --repo . --declaration keel.json --corpus "$DOXA_DIR" --bin-dir "$BIN_DIR" --json 2>/dev/null) && level_rc=0 || level_rc=$?
if [ "$level_rc" -eq 1 ]; then
  echo "FATAL: keel could not measure the declared level (exit 1): $level_json" >&2
  exit 1
fi
python3 - "$level_json" "$level_rc" <<'PY'
import json, sys, collections
level = json.loads(sys.argv[1]); rc = int(sys.argv[2])
errors = []
cov = level.get("coverage")
if cov is None:
    print("ERROR: keel level ran no coverage — the declaration names no instrument or no roots", file=sys.stderr); sys.exit(1)
report = cov["run"]; c = report["counts"]
print(f"    declared {level['declared']} — {'holds' if level['holds'] else 'NOT RUNNING IT'}; cascade pin {'present' if level['instruments'][0]['pin'] else 'MISSING'}, corpus {'at' if level['corpus_pinned'] else 'NOT AT'} its pin")
print(f"    nodes {c['nodes']} (listing {cov['listed']}) grounded {c['grounded']} ungrounded {c['ungrounded']} malformed {c['malformed']} diverged {c['diverged']} findings {c['findings']} run-level {c['run_level']} exit {rc}")
if not level["holds"]:
    for r in level["reasons"]:
        errors.append(f"declared level not running: {r}")
if not cov["covered"]:
    errors.append(f"silence: {cov['silence']} listed heading(s) carry no verdict ({cov['listed']} listed, {cov['verdicts']} verdicts) — refused regardless of how clean the visited subset is (keel-harness §3.2)")
for f in report["findings"]:
    if f["class"] in ("UnclosedFence", "EmptyRfc", "EmptySpecs", "EmptyCode"):
        print(f"    run-level finding (keel-dialect §7) — refuses at L3 (keel-harness §8): {json.dumps(f)}")
classes = collections.Counter(f["class"] for f in report["findings"])
print("    findings (L3 — keel-harness §8: any corpus-wide finding refuses): " + (", ".join(f"{k} {v}" for k, v in sorted(classes.items())) or "none"))
for n in report["nodes"]:
    if n["verdict"] in ("malformed", "diverged"):
        print(f"    visible: {n['file']}:{n['line']} `{n['name']}` {n['verdict']} — {', '.join(n['findings'])}")
ungrounded_docs = sorted({n["file"] for n in report["nodes"] if n["verdict"] == "ungrounded"})
grounded_docs = sorted({n["file"] for n in report["nodes"] if n["verdict"] == "grounded"})
print(f"    ungrounded documents (the live worklist for keel-harness §7 step 2): {len(ungrounded_docs)} carrying {c['ungrounded']} ungrounded heading(s); documents with a grounded heading: {len(grounded_docs)}")
if errors:
    for e in errors:
        print("ERROR:", e, file=sys.stderr)
    print("keel.json declares L3 gated: the full declared root every run, zero silence, any corpus-wide finding refuses (keel-harness §3.2, §8)", file=sys.stderr)
    sys.exit(1)
print("    ok — the declared level holds on this tree, zero silence")
PY
