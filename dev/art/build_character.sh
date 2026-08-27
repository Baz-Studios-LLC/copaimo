#!/usr/bin/env bash
# Builds assets/models/person_ranger.glb from assets/character/*.glb, then PLANTS the delivered
# jog's feet onto the floor - see dev/art/author_gait.py for why the clip is kept and only its
# contact with the ground is solved. Pass --author to author a gait from scratch instead.
# dev/art/audit_character.sh checks the result.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
quiet='^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \|)'
"$blender" --background --python-exit-code 1 --python "$here/build_character.py" 2>&1 \
  | grep -viE "$quiet"
echo
# Both travelling clips, each planted in its own pass. The walk needs it as much as the jog -
# same soles, same floor, same slide - and planting it is also the only exact source of its
# COVERS, since these clips arrive with no root motion to measure it from.
# The plant is OFF. It is the largest thing this pipeline does to a clip - it moves feet,
# legs and toes - and the delivered walk and run are final until each piece of it has been
# shown against them. Nothing below this line runs.
#
# # The walk is not planted yet: its balls measure 0.64 cm a frame against the jog's 14.08, and
# the plant breaks its loop by 12.24 cm - it does not close the way the jog does. Add it back here
# once that is understood.
for clip in jog; do
#   "$blender" --background --python-exit-code 1 --python "$here/author_gait.py" -- \
#     --name "$clip" 2>&1 | grep -viE "$quiet"
# done
#