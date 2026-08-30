#!/usr/bin/env bash
# Everything measurable about the character, in one run. See dev/art/audit_character.py.
#
#   dev/art/audit_character.sh                       the built asset
#   dev/art/audit_character.sh dev/art/source/character/walk.glb    anything else
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
root="$(cd "$here/../.." && pwd)"
model="${1:-$root/assets/models/person_ranger.glb}"
[ -f "$model" ] || { echo "No $model" >&2; exit 1; }
"$blender" --background --python-exit-code 1 --python "$here/audit_character.py" -- \
  --model "$(win "$model")" 2>&1 \
  | grep -viE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \| INFO)"
