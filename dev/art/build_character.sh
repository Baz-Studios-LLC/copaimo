#!/usr/bin/env bash
# Builds assets/models/person_ranger.glb from assets/character/*.glb.
# See dev/art/build_character.py, and dev/art/audit_character.sh to check the result.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
"$blender" --background --python-exit-code 1 --python "$here/build_character.py" 2>&1 \
  | grep -viE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \|)"
