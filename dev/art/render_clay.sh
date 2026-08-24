#!/usr/bin/env bash
# Renders the character as FORM - no texture. See dev/art/render_clay.py.
#
#   dev/art/render_clay.sh
#   dev/art/render_clay.sh --only feet,hands
#   dev/art/render_clay.sh --clip walk --frame 9
#   dev/art/render_clay.sh --silhouette
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
root="$(cd "$here/../.." && pwd)"
"$blender" --background --python-exit-code 1 --python "$here/render_clay.py" -- \
  --model "$(win "$root/assets/models/person_ranger.glb")" \
  --out "$(win "$root/dev/art/clay")" "$@" 2>&1 \
  | grep -viE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \|)"
