#!/usr/bin/env bash
# Shows the GAME's IK solver working on the real rig, in Blender.
#
#   dev/art/see_the_ik.sh            # render the cases
#   dev/art/see_the_ik.sh --open     # ...and open the last one
#
# Two steps, and the split is the point: `src/ik.rs` solves, Blender only draws. There is no
# second implementation of the solver to disagree with the first.
#
# The Blender side then MEASURES BACK what it posed and refuses if the rig does not match what
# the solver said - which tests the part the Rust tests cannot reach, turning solved positions
# into bone rotations on a real armature.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

out="${TMPDIR:-/tmp}/see_the_ik"
mkdir -p "$out"
solved="$out/solved_leg.json"

# Rust solves. `--nocapture` because the test prints rather than asserts, and the markers are
# there because cargo interleaves its own output with the test's.
export PATH="$HOME/.cargo/bin:$PATH"
(cd "$root" && cargo test --quiet solve_a_leg_for_blender -- --ignored --nocapture) \
  | sed -n '/SOLVED_LEG_JSON_BEGIN/,/SOLVED_LEG_JSON_END/p' \
  | sed '1d;$d' > "$solved"

if ! python -c "import json,sys; json.load(open(sys.argv[1]))" "$solved" 2>/dev/null; then
  echo "The solver's output is not valid JSON. First lines:" >&2
  head -5 "$solved" >&2
  exit 1
fi
echo "solved $(python -c "import json,sys; print(len(json.load(open(sys.argv[1]))['cases']))" "$solved") cases -> $solved"

IK_SOLVED="$(win "$solved")" IK_OUT="$(win "$out")" \
  "$blender" --background --python-exit-code 1 --python "$here/see_the_ik.py" 2>&1 \
  | grep -vE "^(Blender [0-9]|Read prefs|Fra:|Saved:|Info:|Warning: )" || exit 1

echo
echo "renders in $out"
if [ "${1:-}" = "--open" ]; then
  last=$(ls -t "$out"/ik_*.png | head -1)
  echo "opening $last"
  start "" "$(win "$last")" 2>/dev/null || open "$last" 2>/dev/null || true
fi
