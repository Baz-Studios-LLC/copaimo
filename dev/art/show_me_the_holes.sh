#!/usr/bin/env bash
# Opens dev/art/ranger.blend with the real holes selected. See show_me_the_holes.py.
#
# Rebuilds the .blend first, so what opens is the current asset rather than whatever was
# left from last time - the viewer going stale without saying so has cost a session already.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

"$here/ranger_blend.sh" >/dev/null
[ -f "$here/ranger.blend" ] || { echo "ranger.blend was not written." >&2; exit 1; }

echo "opening $here/ranger.blend with the holes selected"
# --enable-autoexec so the selection script runs; it is per-session and changes no preference.
"$blender" --enable-autoexec "$(win "$here/ranger.blend")" \
  --python "$(win "$here/show_me_the_holes.py")"
