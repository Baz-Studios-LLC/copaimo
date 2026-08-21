#!/usr/bin/env bash
# Exports .blend files to assets/models/ as GLB. See dev/model_export.py.
#
#   dev/model_export.sh art/warden.blend
#   dev/model_export.sh art/
#
# Blender is found rather than assumed: it installs per-version, so the path
# carries a version number that changes under you at every upgrade.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"

find_blender() {
  if command -v blender >/dev/null 2>&1; then command -v blender; return; fi
  # Newest first, so an upgrade is picked up without editing this.
  for base in "/c/Program Files/Blender Foundation" "/c/Program Files/Blender" \
              "/Applications/Blender.app/Contents/MacOS"; do
    [ -d "$base" ] || continue
    # The executable EXACTLY: matching "blender*" picks up the
    # blender_system_info.cmd that ships beside it, which reports on the
    # installation and exports nothing.
    found=$(find "$base" -maxdepth 2 \( -name "blender.exe" -o -name "blender" \)             -type f 2>/dev/null | sort -Vr | head -1)
    [ -n "$found" ] && { echo "$found"; return; }
  done
  return 1
}

blender=$(find_blender) || {
  echo "Blender not found. Install it, or put it on PATH." >&2
  exit 1
}
echo "using $blender"

# `--` separates Blender's own arguments from the script's.
"$blender" --background --python "$here/model_export.py" -- "$@"
