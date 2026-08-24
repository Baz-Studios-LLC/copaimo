#!/usr/bin/env bash
# Opens the GAME's IK solver, working on the real rig, in a Blender WINDOW.
#
#   dev/art/see_the_ik.sh              # open it - one frame per case, arrow keys to step
#   dev/art/see_the_ik.sh --stills     # render PNGs instead of opening
#   dev/art/see_the_ik.sh --side R     # the right leg
#
# Two steps, and the split is the point: `src/ik.rs` solves, Blender only draws. There is no
# second implementation of the solver to disagree with the first.
#
# The Blender side then MEASURES BACK what it posed and refuses if the rig does not match what
# the solver said, which tests the part the Rust tests cannot reach - turning solved positions
# into bone rotations on a real armature. It also checks that the SKIN moved and not just the
# skeleton, because the leg's skin is on the twist bones and those are a separate question.
#
# Built headless and saved before being opened, for the reason gait_watch.sh documents: the glTF
# importer dies on the context a GUI gets during startup, so a window built at launch comes up
# empty with the failure buried in the console.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

side="L"
stills=""
while [ $# -gt 0 ]; do
  case "$1" in
    --side) side="$2"; shift 2 ;;
    --stills) stills="--stills"; shift ;;
    *) shift ;;
  esac
done

out="${TMPDIR:-/tmp}/see_the_ik"
mkdir -p "$out"
solved="$out/solved_leg.json"
scene="$out/see_the_ik_$side.blend"

# Rust solves. `--nocapture` because the test prints rather than asserts, and the markers are
# there because cargo interleaves its own output with the test's.
export PATH="$HOME/.cargo/bin:$PATH"
(cd "$root" && cargo test --quiet solve_a_leg_for_blender -- --ignored --nocapture) \
  | sed -n '/SOLVED_LEG_JSON_BEGIN/,/SOLVED_LEG_JSON_END/p' \
  | sed '1d;$d' > "$solved"

python - "$solved" <<'PYEOF' || exit 1
import json, sys
cases = json.load(open(sys.argv[1]))["cases"]
print(f"solved {len(cases)} cases -> {sys.argv[1]}")
PYEOF

"$blender" --background --python-exit-code 1 --python "$here/see_the_ik.py" -- \
  --solved "$(win "$solved")" --out "$(win "$out")" --side "$side" \
  --save "$(win "$scene")" $stills 2>&1 \
  | grep -vE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: )" || exit 1

[ -n "$stills" ] && { echo; echo "renders in $out"; exit 0; }
[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

# Prove it is not empty BEFORE handing it over - an empty window with the reason in the console
# is exactly the failure this two-step build exists to prevent.
"$blender" "$(win "$scene")" --background --python-exit-code 1 --python-expr \
"import bpy
rigs=[o for o in bpy.data.objects if o.type=='ARMATURE']
rods=[o for o in bpy.data.objects if 'solved' in o.name or 'asked' in o.name]
posed=[o for o in rigs if o.animation_data and o.animation_data.action]
s=bpy.context.scene
views=[a for sc in bpy.data.screens for a in sc.areas if a.type=='VIEW_3D']
print(f'CHECK rigs={len(rigs)} markers={len(rods)} posed={len(posed)} '
      f'frames={s.frame_start}..{s.frame_end} views={len(views)}')
assert rigs and rods and posed, 'the saved scene has no posed rig or no markers'
assert s.frame_end > s.frame_start, 'only one frame, so there is nothing to step through'" \
  2>&1 | grep -E "^CHECK|Error|Assertion"

echo
echo "opening $scene"
echo "  RIGHT ARROW / LEFT ARROW steps through the cases, one per frame."
echo "  cyan is what src/ik.rs solved, grey is the leg at rest, RED is the target it was asked"
echo "  for - a red ball away from the cyan chain is a miss, and two of the cases are."
"$blender" "$(win "$scene")"
