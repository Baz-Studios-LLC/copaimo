#!/usr/bin/env bash
# Adds walk and run clips to the made ranger. See dev/art/animate_ranger.py.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
find_blender() {
  if command -v blender >/dev/null 2>&1; then command -v blender; return; fi
  for base in "/c/Program Files/Blender Foundation" "/c/Program Files/Blender" \
              "/Applications/Blender.app/Contents/MacOS"; do
    [ -d "$base" ] || continue
    found=$(find "$base" -maxdepth 2 \( -name "blender.exe" -o -name "blender" \) \
            -type f 2>/dev/null | sort -Vr | head -1)
    [ -n "$found" ] && { echo "$found"; return; }
  done
  return 1
}
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
