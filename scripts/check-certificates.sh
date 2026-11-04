#!/usr/bin/env bash
# Certifies random endgame positions with the Rust solver and checks every
# certificate with the Lean checker (proofs/Check.lean).
#
# Usage: scripts/check-certificates.sh [positions per variant] [seed]
# Expects `backend/target/release/ulttt` and `proofs/.lake/build/bin/ulttt-check`.
set -euo pipefail
COUNT="${1:-5}"
SEED="${2:-1}"
ULTTT=backend/target/release/ulttt
CHECK=proofs/.lake/build/bin/ulttt-check
OUT=$(mktemp -d)
trap 'rm -rf "$OUT"' EXIT
i=0
for v in classic open majority; do
  for pos in $("$ULTTT" random -v "$v" --plies 52 -n "$COUNT" -s "$SEED"); do
    i=$((i + 1))
    "$ULTTT" certify -v "$v" --out "$OUT" --name "$v-$i" "$pos"
  done
done
"$CHECK" "$OUT"/*.cert
