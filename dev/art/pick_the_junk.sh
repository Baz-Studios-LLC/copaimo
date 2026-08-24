#!/usr/bin/env bash
# Pick the generator's stray geometry by hand, once. See pick_the_junk.py.
#
#   dev/art/pick_the_junk.sh          build the scene and open it
#   dev/art/pick_the_junk.sh --read   read the selection out of the saved scene
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
scene="$here/pick_the_junk.blend"

if [ "${1:-}" = "--read" ]; then
  [ -f "$scene" ] || { echo "No $scene - run without --read first." >&2; exit 1; }
  "$blender" "$(win "$scene")" --background --python-exit-code 1 \
    --python "$(win "$here/pick_the_junk.py")" -- --read
  exit 0
fi

"$blender" --background --python-exit-code 1 --python "$here/pick_the_junk.py" \
  2>&1 | grep -vE "^(Fra:|INFO|Blender [0-9]|Read prefs|Warning: )" || true
[ -f "$scene" ] || { echo "The scene was not written." >&2; exit 1; }

cat <<'HOW'

  Select the junk in FACE mode, then save with Ctrl-S and close Blender.
  Anything already on the list comes up pre-selected, so you can add or trim.

  Then run:  dev/art/pick_the_junk.sh --read

HOW
"$blender" "$(win "$scene")"
