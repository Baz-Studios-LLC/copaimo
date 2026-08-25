#!/usr/bin/env bash
# Authors a gait onto the built character, by moving the feet and letting IK find the legs.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=blender.sh
. "$here/blender.sh"
blender="$(find_blender)"
"$blender" --background --python-exit-code 1 --python "$here/author_gait.py" -- "$@"
