#!/usr/bin/env bash
# ONE Blender window with the character and every clip in it. See see_the_character.py.
#
#   dev/art/see_the_character.sh                 opens on the idle
#   dev/art/see_the_character.sh --clip walk     opens on the walk
#   dev/art/see_the_character.sh --in-place      runs him on the spot, to watch the feet land
#
# Closes any Blender already open first, on purpose: the point is that there is ONE window
# showing the CURRENT build. Two windows is how a fault gets reported against a stale one.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
root="$(cd "$here/../.." && pwd)"
glb="$root/assets/models/person_ranger.glb"
[ -f "$glb" ] || { echo "No $glb - run dev/art/build_character.sh first." >&2; exit 1; }

out="${TMPDIR:-/tmp}/copaimo_view"
mkdir -p "$out"
scene="$out/character.blend"

quiet() { grep -viE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: |[0-9]{2}:[0-9]{2}:[0-9]{2} \|)"; }

"$blender" --background --python-exit-code 1 --python "$here/see_the_character.py" -- \
  --model "$(win "$glb")" --save "$(win "$scene")" "$@" 2>&1 | quiet

[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

# Prove it before handing it over. An empty window with the reason on a console nobody reads is
# the failure the two-step build exists to prevent.
"$blender" "$(win "$scene")" --background --python-exit-code 1 --python-expr \
"import bpy
rigs=[o for o in bpy.data.objects if o.type=='ARMATURE']
skins=[o for o in bpy.data.objects if o.type=='MESH' and o.vertex_groups]
act=rigs[0].animation_data.action if rigs and rigs[0].animation_data else None
balls=[b.name for b in rigs[0].pose.bones if b.custom_shape] if rigs else []
s=bpy.context.scene
print(f'CHECK rigs={len(rigs)} skinned={len(skins)} clips={len(bpy.data.actions)} '
      f'showing={act.name if act else None} frames={s.frame_start}..{s.frame_end} '
      f'widgets={len(balls)}')
assert rigs and skins and act, 'the saved scene is missing the character or a clip'
assert not balls, f'{len(balls)} bones still wear sphere widgets'" 2>&1 | grep -E "^CHECK|Error|Assertion"

running=$(tasklist 2>/dev/null | grep -ci "blender.exe" || true)
if [ "${running:-0}" -gt 0 ]; then
  echo "closing $running Blender window(s) so there is only one"
  taskkill //IM blender.exe //F >/dev/null 2>&1 || true
  sleep 2
fi

echo
echo "opening $scene"
echo "  the ACTION EDITOR is open at the bottom: its dropdown lists every clip. Space plays."
echo "  twist bones are hidden; alt-H in the viewport brings them back."
"$blender" "$(win "$scene")" >/dev/null 2>&1 &
