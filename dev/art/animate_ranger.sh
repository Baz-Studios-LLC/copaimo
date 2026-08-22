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

# The texture first, in a tool that can be checked. See ranger_texture.py for why
# this is not done inside Blender.
python "$here/ranger_texture.py"
"$blender" --background --python-exit-code 1 --python "$here/animate_ranger.py"

# And then REFUSE it if the limbs bend the wrong way. Three attempts shipped a walk
# with backwards knees and arms swinging with the legs, and every one of them was
# caught by the person playing the game. See dev/art/verify_gait.py.
"$blender" --background --python-exit-code 1 --python "$here/verify_gait.py" --   "$(cd "$here/../.." && pwd)/assets/models/person_ranger.glb" walk:5 run:3 sprint:2
