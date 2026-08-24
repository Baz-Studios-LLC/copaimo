#!/usr/bin/env bash
# Moves a delivered preset clip onto the prepared rig and opens it in Blender.
#
#   dev/art/see_the_retarget.sh                # the walk
#   dev/art/see_the_retarget.sh --clip run     # the run
#
# The point of looking is that a retarget can be arithmetically perfect and still wrong: the
# tracking check inside `retarget.py` proves each bone ended up pointing where the source had it,
# which is a different claim from the motion reading correctly on THIS body.
#
# Built headless and saved, then opened - the glTF importer dies on the context a GUI gets during
# startup, so a window built at launch comes up empty with the reason buried in the console. See
# gait_watch.sh, which documents it at length.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

which="walk"
prev=""
for token in "$@"; do
  [ "$prev" = "--clip" ] && which="$token"
  prev="$token"
done

out="${TMPDIR:-/tmp}/see_the_retarget"
mkdir -p "$out"
scene="$out/retarget_$which.blend"

"$blender" --background --python-exit-code 1 --python "$here/retarget.py" -- \
  --clip "$which" --save "$(win "$scene")" "$@" 2>&1 \
  | grep -vE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: )" || exit 1

[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

# Prove it carries the clip before handing it over.
"$blender" "$(win "$scene")" --background --python-exit-code 1 --python-expr \
"import bpy
rigs=[o for o in bpy.data.objects if o.type=='ARMATURE' and not o.hide_viewport]
act=rigs[0].animation_data.action if rigs and rigs[0].animation_data else None
s=bpy.context.scene
print(f'CHECK rigs={len(rigs)} clip={act.name if act else None} '
      f'frames={s.frame_start}..{s.frame_end} bones={len(rigs[0].data.bones) if rigs else 0}')
assert rigs and act, 'the saved scene has no posed rig'
assert s.frame_end > s.frame_start, 'the clip has no length'" \
  2>&1 | grep -E "^CHECK|Error|Assertion"

echo
echo "opening $scene - press space to play."
echo "  this is the DELIVERED $which clip on OUR rig: mirrored sides, straight legs, A-pose bind,"
echo "  71 bones. The delivery's own rig is in the file but hidden."
"$blender" --enable-autoexec "$(win "$scene")"
