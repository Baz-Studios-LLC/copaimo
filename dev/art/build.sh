#!/usr/bin/env bash
# Builds every scripted figure in this folder and exports it to assets/models/.
#
#   dev/art/build.sh
#
# Two steps on purpose: the script builds a .blend, and the .blend goes through
# the same export gate a hand-made one does. Nothing gets a shortcut into the
# game just because a script made it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"

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

for script in "$here"/*.py; do
  echo "building $(basename "$script")"
  # `--python-exit-code` matters: Blender exits 0 even when the script it ran
  # died on a traceback, so without this a broken generator produced nothing and
  # the build cheerfully carried on to report "no .blend files found".
  "$blender" --background --python-exit-code 1 --python "$script" >/dev/null
done

"$root/dev/model_export.sh" "$here"
