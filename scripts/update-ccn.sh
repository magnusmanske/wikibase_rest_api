#!/usr/bin/env bash
#
# Regenerate the average cyclomatic complexity (AvgCCN) badge in README.md
# from `lizard`.
#
# Usage: ./scripts/update-ccn.sh
#
# Runs lizard over src/, extracts the total AvgCCN, picks a badge colour from
# that value (lower is better), and rewrites the AvgCCN badge line in README.md.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "Running lizard ..." >&2
# lizard exits non-zero when it reports complexity warnings, so tolerate that.
# The totals block is a "Total nloc ... AvgCCN ..." header, a separator row, then
# the values row whose 3rd column is the AvgCCN.
lizard_out=$(lizard src -C 7 -L 40 2>/dev/null || true)
ccn=$(printf '%s\n' "${lizard_out}" | awk '/^Total nloc/{getline; getline; print $3; exit}')

if [[ -z "${ccn}" ]]; then
  echo "error: could not parse AvgCCN from lizard output" >&2
  exit 1
fi

int=${ccn%.*}   # integer part for colour thresholds (lower is better)

if   (( int <= 2 ));  then colour=brightgreen
elif (( int <= 5 ));  then colour=green
elif (( int <= 8 ));  then colour=yellowgreen
elif (( int <= 12 )); then colour=yellow
else                       colour=red
fi

badge="[![Avg. CCN](https://img.shields.io/badge/avg%20CCN-${ccn}-${colour}?style=flat-square)](README.md)"

if ! grep -q 'img.shields.io/badge/avg%20CCN-' README.md; then
  echo "error: AvgCCN badge line not found in README.md" >&2
  exit 1
fi

perl -i -pe "s{\\[!\\[Avg\\. CCN\\].*}{${badge}}" README.md

echo "Updated AvgCCN badge to ${ccn} (${colour})." >&2
