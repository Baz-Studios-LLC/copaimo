#!/usr/bin/env bash
# Opens a gait clip in the Blender GUI, so it can be watched rather than measured.
#
#   dev/art/gait_watch.sh                 # the jog, at the speed the game plays it
#   dev/art/gait_watch.sh walk            # the walk, for comparison
#   dev/art/gait_watch.sh run --rate native   # the clip's own authored cadence
#   dev/art/gait_watch.sh run --still     # a static floor instead of a treadmill
#   dev/art/gait_watch.sh run --front     # head-on instead of side-on
#   dev/art/gait_watch.sh grip --hands    # close on the hand, skeleton shown
#
# TWO STEPS, and the reason matters. Building the scene from a --python script during
# GUI startup does not work: the glTF importer dies in `armature_display` on a context
# that startup does not provide, so the window comes up EMPTY with the failure buried in
# the console. The identical import is reliable in --background. So the scene is built
# headless, saved as a .blend, checked, and only then opened - which also means the file
# can be reopened later without rebuilding.
#
# See dev/art/gait_watch.py for what it repairs on load and why the ground moves.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"

blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
# Which model. Defaults to the built asset; `--model PATH` points it at anything else, which is
# how a fresh delivery gets looked at before any decision is taken about it. The Python side has
# always taken the path as an argument; only this line was fixed.
glb="$root/assets/models/person_ranger.glb"
prev=""
for token in "$@"; do
  [ "$prev" = "--model" ] && glb="$token"
  prev="$token"
done
case "$glb" in /*|[A-Za-z]:*) ;; *) glb="$root/$glb" ;; esac
[ -f "$glb" ] || { echo "No $glb - run dev/art/animate_ranger.sh first." >&2; exit 1; }

out="${TMPDIR:-/tmp}/gait_watch"
mkdir -p "$out"

# Named per clip, so the walk and the jog can be open in two windows and compared
# rather than one overwriting the other.
named="run"
prev=""
for token in "$@"; do
  case "$token" in
    --*) ;;
    *) if [ "$prev" != "--model" ]; then named="$token"; break; fi ;;
  esac
  prev="$token"
done
# A file name is not a clip name, so a scene per model as well as per clip.
tag="$(basename "$glb" .glb)"

# Everything EXCEPT `--model PATH`, to hand on to the Python. Passing "$@" wholesale fed the model
# path back in as a positional, and the Python takes the second positional as the clip name - so
# it went looking for a clip called `assets/models/Ranger-Walk.glb`.
rest=()
skip=0
for token in "$@"; do
  if [ "$skip" = "1" ]; then skip=0; continue; fi
  if [ "$token" = "--model" ]; then skip=1; continue; fi
  rest+=("$token")
done
scene="$out/gait_watch_${tag}_$named.blend"

"$blender" --background --python-exit-code 1 --python "$here/gait_watch.py" -- \
  "$(win "$glb")" --save "$(win "$scene")" "${rest[@]}" \
  2>&1 | grep -vE "^(Fra:|INFO|Blender [0-9]|Read prefs|Warning: )"

[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

# Prove it is not empty BEFORE handing it over - an empty window with the reason in the
# console is exactly the failure this script exists to prevent.
"$blender" "$(win "$scene")" --background --python-exit-code 1 --python-expr \
"import bpy
rigs=[o for o in bpy.data.objects if o.type=='ARMATURE']
skins=[o for o in bpy.data.objects if o.type=='MESH' and o.vertex_groups]
act=rigs[0].animation_data.action.name if rigs and rigs[0].animation_data else None
s=bpy.context.scene
views=[a for sc in bpy.data.screens for a in sc.areas if a.type=='VIEW_3D']
balls=[b.name for b in rigs[0].pose.bones if b.custom_shape] if rigs else []
print(f'CHECK rigs={len(rigs)} skinned={len(skins)} clip={act} '
      f'frames={s.frame_start}..{s.frame_end} '
      f'fps={s.render.fps/s.render.fps_base:.1f} views={len(views)} '
      f'widgets={len(balls)}')
assert rigs and skins and act, 'the saved scene is missing the character or its clip'
assert not balls, f'{len(balls)} bones still wear sphere widgets'" \
  2>&1 | grep -E "^CHECK|Error|Assertion"

echo
echo "opening $scene - press space to play."
echo "leave it open: it watches the file and reloads itself when a clip is rebuilt."
# --enable-autoexec so the registered reload watcher inside the .blend runs. It is a
# per-session flag and changes nothing in anyone's preferences. See WATCHER in
# gait_watch.py for what it does and why.
"$blender" --enable-autoexec "$(win "$scene")"
