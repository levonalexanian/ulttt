#!/usr/bin/env bash
# Creates the standard benchmark suites in the database (default data/ulttt.db).
# Suites are generated deterministically from their seed, so every database
# created by this script holds the same positions. Suite `<variant>-pN` holds
# 20 positions after N random moves; smaller N means harder positions.
#
# Usage: scripts/create-suites.sh [path to ulttt binary] [database]
set -euo pipefail
ULTTT="${1:-backend/target/release/ulttt}"
DB="${2:-data/ulttt.db}"
for v in classic open majority; do
  "$ULTTT" --db "$DB" suite-create "$v-p46" -v "$v" --plies 46 -n 20 -s 1
  "$ULTTT" --db "$DB" suite-create "$v-p42" -v "$v" --plies 42 -n 20 -s 2
  "$ULTTT" --db "$DB" suite-create "$v-p38" -v "$v" --plies 38 -n 20 -s 19
  "$ULTTT" --db "$DB" suite-create "$v-p34" -v "$v" --plies 34 -n 20 -s 17
done
