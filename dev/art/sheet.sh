#!/usr/bin/env bash
# Renders one town figure from four sides, to hold against a concept sheet.
#
#   dev/art/sheet.sh guild_hall
#
# Writes dev/art/shots/sheet_<figure>_{front,side,rear,quarter}.png
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
blender=$("$here/blender.sh" --which 2>/dev/null || echo blender)
"$blender" --background --python-exit-code 1 --python "$here/sheet.py" -- --figure "${1:-guild_hall}"
