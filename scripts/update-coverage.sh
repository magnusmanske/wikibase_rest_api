#!/usr/bin/env bash
#
# Regenerate the coverage badge in README.md from `cargo tarpaulin`.
#
# Usage: ./scripts/update-coverage.sh
#
# Runs tarpaulin, extracts the total line-coverage percentage, picks a badge
# colour from that value, and rewrites the coverage badge line in README.md.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Running cargo tarpaulin ..." >&2
summary=$(cargo tarpaulin --skip-clean 2>/dev/null | grep -oE '[0-9]+\.[0-9]+% coverage' | tail -1)

if [[ -z "${summary}" ]]; then
  echo "error: could not parse coverage from tarpaulin output" >&2
  exit 1
fi

pct=${summary%% *}          # e.g. "86.78%"
value=${pct%\%}             # e.g. "86.78"
int=${value%.*}             # integer part for colour thresholds

if   (( int >= 90 )); then colour=brightgreen
elif (( int >= 80 )); then colour=green
elif (( int >= 70 )); then colour=yellowgreen
elif (( int >= 60 )); then colour=yellow
else                       colour=red
fi

badge="[![Coverage](https://img.shields.io/badge/coverage-${value}%25-${colour}?style=flat-square)](README.md)"

# Replace the existing coverage badge line, or fail loudly if the marker is gone.
if ! grep -q 'img.shields.io/badge/coverage-' README.md; then
  echo "error: coverage badge line not found in README.md" >&2
  exit 1
fi

perl -i -pe "s{\\[!\\[Coverage\\].*}{${badge}}" README.md

echo "Updated coverage badge to ${value}% (${colour})." >&2
