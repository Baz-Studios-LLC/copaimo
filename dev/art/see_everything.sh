#!/usr/bin/env bash
# ONE Blender window with the whole character and every clip in it.
#
#   dev/art/see_everything.sh
#
# Closes any Blender already open first, on purpose: the point of this is that there is one
# window rather than five, and five viewers had accumulated. Anything unsaved in them goes.
#
# In the window: the built character with its skeleton drawn in front, and every clip in the
# Action Editor - the authored walk, run, sprint, idle and grip, plus the delivered presets
# retargeted onto the same rig as `delivered_walk` and `delivered_run`. Switch between them in
# the dope sheet's Action Editor; space plays.
#
# Built headless and saved before opening, because the glTF importer dies on the context a GUI
# gets during startup - see gait_watch.sh, which documents it at length. The scene carries the
# reload watcher, so a later rebuild reaches this window without closing it again.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

glb="$root/assets/models/person_ranger.glb"
[ -f "$glb" ] || { echo "No $glb - run dev/art/animate_ranger.sh first." >&2; exit 1; }

out="${TMPDIR:-/tmp}/see_everything"
mkdir -p "$out"
scene="$out/everything.blend"

"$blender" --background --python-exit-code 1 --python "$here/see_everything.py" -- \
  --save "$(win "$scene")" 2>&1 \
  | grep -vE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: )" || exit 1

[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

# Prove it before handing it over - an empty window with the reason in the console is the failure
# this two-step build exists to prevent.
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
assert not balls, f'{len(balls)} bones still wear sphere widgets'" \
  2>&1 | grep -E "^CHECK|Error|Assertion"

# One window. Closing whatever is open is the whole point of this script.
running=$(tasklist 2>/dev/null | grep -ci "blender.exe" || true)
if [ "${running:-0}" -gt 0 ]; then
  echo "closing $running Blender window(s) so there is only one"
  taskkill //IM blender.exe //F >/dev/null 2>&1 || true
  sleep 2
fi

echo
echo "opening $scene"
echo "  every clip is in the dope sheet's ACTION EDITOR - switch there to compare"
echo "  the authored walk against delivered_walk on the same body. Space plays."
echo "  leave it open: it reloads itself when the asset is rebuilt."
"$blender" --enable-autoexec "$(win "$scene")"
