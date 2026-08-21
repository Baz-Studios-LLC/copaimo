#!/usr/bin/env bash
# Renders the patch of stamped ground cover that the dump test writes.
#
#   cargo test dump_a_patch_of_cover -- --ignored --nocapture
#   dev/cover_look.sh out.png
#
# What the templates look like can be seen in Blender any time; what cannot is
# what the STAMP makes of them — the fan, the lean, the greens, a flower among the
# grass. This draws the real geometry the chunk dresser would weld.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
out="${1:-$root/cover_patch.png}"

[ -f "$root/cover_patch.ply" ] || {
  echo "no cover_patch.ply — run the dump test first" >&2
  exit 1
}

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

"$blender" --background --python-exit-code 1 --python "$here/cover_look.py" -- "$out" "$root/cover_patch.ply"
