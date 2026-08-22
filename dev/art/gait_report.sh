#!/usr/bin/env bash
# Everything measurable about the warden's gaits, in one command.
#
#   dev/art/gait_report.sh [<clip>...]        # defaults to walk and run
#
# Three instruments over the file the game actually loads:
#
#   verify_gait.py    refuses a wrong SIGN and scores the amplitudes
#   stride_measure.py how far a cycle carries the warden, off the planted foot
#   roll_match.py     whether the hands hold the same twist in every clip
#   gait_look.py      five frames of each clip from the side, to LOOK at
#
# The renders are the part that matters most and the part most easily skipped.
# Three attempts at this walk shipped without anyone looking at a frame of it, and
# every one of them was caught by the person playing the game instead.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
glb="$root/assets/models/person_ranger.glb"
clips=("$@")
[ ${#clips[@]} -eq 0 ] && clips=(walk run)

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

# Windows Blender wants Windows paths; cygpath is there under Git Bash and absent
# everywhere else, where the path is already right.
win() { if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else echo "$1"; fi }

out="${TMPDIR:-/tmp}/gait_report"
mkdir -p "$out"

echo "=============== signs and amplitudes ==============="
"$blender" --background --python-exit-code 1 --python "$here/verify_gait.py" -- \
  "$(win "$glb")" "${clips[@]}" 2>&1 | grep -vE "^(INFO|Blender|[0-9][0-9]:[0-9][0-9])"

echo
echo "=============== what a cycle covers ==============="
"$blender" --background --python-exit-code 1 --python "$here/stride_measure.py" -- \
  "$(win "$glb")" "${clips[@]}" 2>&1 | grep -vE "^(INFO|Blender|[0-9][0-9]:[0-9][0-9])"

echo
echo "=============== do the hands agree between clips ==============="
"$blender" --background --python-exit-code 1 --python "$here/roll_match.py" --   "$(win "$glb")" 2>&1 | grep -vE "^(INFO|Blender|[0-9][0-9]:[0-9][0-9])"

echo
echo "=============== frames to look at ==============="
for clip in "${clips[@]}"; do
  for cam in side tqfront; do
    "$blender" --background --python-exit-code 1 --python "$here/gait_look.py" -- \
      "$(win "$glb")" "$(win "$out/${clip}_${cam}.png")" "$clip" --cam "$cam" \
      >/dev/null 2>&1
    echo "  $out/${clip}_${cam}_0.png .. _4.png"
  done
done
