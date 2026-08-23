#!/usr/bin/env bash
# Adds walk and run clips to the made ranger. See dev/art/animate_ranger.py.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# Sourced rather than pasted. This script carried its own copy of `find_blender`, one of
# the four blender.sh was written to replace and whose own header already says they
# "should come here too". Sourcing also brings `win`, which the viewer refresh at the
# bottom of this script needs - and adding a fifth copy of that to get hold of it would
# have been exactly the wrong way round.
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

# The stance shares come from animate_ranger.py, which is where they are decided.
#
# They used to be typed again here as `walk:0.625 run:0.3333 sprint:0.25`, and the day
# RUN_SHARE moved to 7/24 this file went on telling verify_gait 0.3333 - so the verifier
# measured a planted window the clip does not have and reported a duty factor of 0.667 for
# a clip authored at 0.583. A number stated in two places is a number that will disagree
# with itself.
shares() {
  python - "$1" <<'PYEOF'
import re, sys
src = open("dev/art/animate_ranger.py", encoding="utf-8").read()
hit = re.search(rf"^{sys.argv[1]} = (.+)$", src, re.M)
if not hit:
    raise SystemExit(f"no {sys.argv[1]} in animate_ranger.py")
print(f"{eval(hit.group(1)):.6f}")
PYEOF
}

# The texture first, in a tool that can be checked. See ranger_texture.py for why
# this is not done inside Blender.
python "$here/ranger_texture.py"
# The rig is REPAIRED first, into its own file, and the source is never touched. The
# authoring below then has no correction step at all - which is the point: the defects
# were constants of the rest pose (the two sides 5.45 cm from mirrored, a 17.5 degree
# crouch, leaf bones the importer invented lengths for, a mesh in 1440 disconnected
# shells), and correcting a constant per pose is what twisted the feet.
#
# Supersedes straighten_rig.py, which repaired three of those and left the rest.
"$blender" --background --python-exit-code 1 --python "$here/prepare_rig.py" --   "$(cd "$here/../.." && pwd)/Ranger_Rig_Idle.glb" "$here/ranger_apose.glb"
"$blender" --background --python-exit-code 1 --python "$here/animate_ranger.py"

# And then REFUSE it if the limbs bend the wrong way. Three attempts shipped a walk
# with backwards knees and arms swinging with the legs, and every one of them was
# caught by the person playing the game. See dev/art/verify_gait.py.
"$blender" --background --python-exit-code 1 --python "$here/verify_gait.py" --   "$(cd "$here/../.." && pwd)/assets/models/person_ranger.glb" walk:"$(shares WALK_SHARE)" run:"$(shares RUN_SHARE)" sprint:"$(shares SPRINT_SHARE)"

# # And refresh any viewer scene that is already open
#
# `gait_watch.sh` builds a .blend and opens it, and the .blend carries a watcher that
# reverts itself when the file's timestamp changes - so a rebuild is meant to reach an open
# window without anybody closing anything. What was missing is the other half: this script
# rewrites the GLB and nothing rewrote the SCENE, so an open window went on showing whatever
# clip it was built from.
#
# It went unnoticed for a session because the scenes are only stale, never broken. Measured
# at the point it was caught, the viewer scenes were from 10:53 and the GLB from 13:02 - two
# hours and four rounds of changes apart, including the whole arm swing and lean pass. That
# is worse than a bug: a stale scene makes the reports coming back UNRELIABLE, and neither
# side can tell.
#
# Only scenes that already EXIST are rewritten. Building one for a clip nobody has open
# would put a window's worth of work on every run of this script for no reason, and creating
# files nobody asked for is its own surprise.
watch="${TMPDIR:-/tmp}/gait_watch"
for scene in "$watch"/gait_watch_*.blend; do
  [ -f "$scene" ] || continue
  case "$scene" in *.blend1) continue ;; esac
  named="$(basename "$scene" .blend)"
  named="${named#gait_watch_}"
  case "$named" in gait_watch) continue ;; esac
  "$blender" --background --python-exit-code 1 --python "$here/gait_watch.py" -- \
    "$(win "$(cd "$here/../.." && pwd)/assets/models/person_ranger.glb")" "$named" \
    --save "$(win "$scene")" 2>&1 | grep -E "REFUSED|Error|Traceback" && exit 1
  echo "refreshed the $named viewer scene - an open window reloads itself"
done
