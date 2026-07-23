#!/usr/bin/env bash
#
# Refresh all generated README.md badges in one go: average cyclomatic
# complexity (AvgCCN) and code coverage.
#
# Usage: ./scripts/update-badges.sh
set -euo pipefail

here="$(dirname "$0")"

"${here}/update-ccn.sh"
"${here}/update-coverage.sh"

echo "All badges updated." >&2
