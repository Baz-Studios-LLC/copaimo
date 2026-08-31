#!/usr/bin/env bash
# Renders every tree species side by side, lit and as a flat silhouette.
#
#   dev/art/see_the_trees.sh
#
# Writes dev/art/shots/trees_lit.png and trees_silhouette.png. See the script.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
. "$here/blender.sh"
"$(find_blender)" --background --python "$here/see_the_trees.py"
