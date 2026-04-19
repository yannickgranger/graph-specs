#!/usr/bin/env bash
# cfdb-determinism-check.sh — extract twice, compare sha256.
# Invariant: cfdb extraction over an unchanged tree is byte-deterministic.
# Any drift points to a non-deterministic ordering bug in the extractor.
set -euo pipefail

DB_A="$(mktemp -d)"
DB_B="$(mktemp -d)"
trap 'rm -rf "$DB_A" "$DB_B"' EXIT

cfdb extract --workspace . --db "$DB_A" --keyspace graph-specs > /dev/null
cfdb extract --workspace . --db "$DB_B" --keyspace graph-specs > /dev/null

SHA_A=$(sha256sum "$DB_A/graph-specs.json" | awk '{print $1}')
SHA_B=$(sha256sum "$DB_B/graph-specs.json" | awk '{print $1}')

if [ "$SHA_A" != "$SHA_B" ]; then
  echo "cfdb determinism FAILED:"
  echo "  extract A: $SHA_A"
  echo "  extract B: $SHA_B"
  exit 1
fi

echo "cfdb determinism OK ($SHA_A)"
