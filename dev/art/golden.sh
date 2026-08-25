#!/usr/bin/env bash
# Renders a fixed sheet and compares it against the kept copies. See dev/art/golden.py.
#
#   dev/art/golden.sh            render and compare - fails if anything moved
#   dev/art/golden.sh --bless    accept the current render as the new truth
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
python "$here/golden.py" "$@"
