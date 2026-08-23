#!/usr/bin/env bash
# Finding Blender, and handing it a path it will accept. Sourced, not run:
#
#   . "$(dirname "$0")/blender.sh"
#   blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }
#   "$blender" --background --python foo.py -- "$(win "$some_path")"
#
# This exists because the same two functions were pasted into four scripts
# (animate_ranger.sh, build.sh, gait_report.sh, ranger_blend.sh), which means four
# places to fix when a Blender version lands somewhere new. Those four still carry
# their own copies; they should come here too.

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

# Windows Blender wants Windows paths; cygpath is there under Git Bash and absent
# everywhere else, where the path is already right.
win() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else echo "$1"; fi
}
