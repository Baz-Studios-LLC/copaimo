#!/usr/bin/env bash
# Builds assets/models/person_ranger.glb from assets/character/*.glb, then AUTHORS the jog onto
# it - see dev/art/author_gait.py for why that clip is authored rather than delivered.
# dev/art/audit_character.sh checks the result.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
quiet='^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \|)'
"$blender" --background --python-exit-code 1 --python "$here/build_character.py" 2>&1 \
  | grep -viE "$quiet"
echo
"$blender" --background --python-exit-code 1 --python "$here/author_gait.py" 2>&1 \
  | grep -viE "$quiet"
