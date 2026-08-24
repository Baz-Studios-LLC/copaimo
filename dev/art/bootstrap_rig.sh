#!/usr/bin/env bash
# Re-derives the prepared rig FROM THE ORIGINAL DELIVERY, and overwrites the source asset.
#
#   dev/art/bootstrap_rig.sh
#
# This is a BOOTSTRAP, not a build step. It used to run on every single build, and that
# was the root cause of the mesh damage this pipeline kept taking.
#
# The reason: prepare_rig.py repairs two different KINDS of thing.
#
#   The RIG repairs are measured constants - the two sides 5.45 cm from mirrored, a 17.5
#   degree crouch, leaf bones the importer invented lengths for. Deriving those afresh
#   every run is right. They are deterministic, and the whole file exists because doing
#   them by hand in a live session lost the A-pose halfway through.
#
#   The MESH repairs are SCULPTING - capping holes, deciding a dark shape near an arm is a
#   hanging strap rather than a sleeve cuff, and now adding finger geometry. Re-deriving
#   sculpting on every build means a classifier has to re-decide the same judgement call
#   forever, and get it right forever. It did not: it cut the sleeve cuffs once, faces out
#   of a trouser leg once, and part of a shoulder once. Each time the response was to tune
#   the classifier, which is treating a design fault as a numbers fault.
#
# So the split is now: the rig is derived once, here, and `ranger_apose.glb` is committed
# as the source of truth. Mesh work is done ON that asset, once, verified once, and kept.
#
# RUNNING THIS DISCARDS ANY HAND-SCULPTED MESH WORK in ranger_apose.glb. It is here for one
# reason only - if the character is ever re-delivered from the generator, this is how the
# rig repairs get applied to the new delivery. Check `git log` on ranger_apose.glb first to
# see what you would be throwing away.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

if [ "${1:-}" != "--yes-discard-mesh-work" ]; then
  echo "This overwrites dev/art/ranger_apose.glb, discarding any hand-sculpted mesh work."
  echo "Re-run with --yes-discard-mesh-work if that is what you want."
  echo
  echo "What you would be discarding, most recent first:"
  git -C "$root" log --oneline -8 -- dev/art/ranger_apose.glb | sed 's/^/  /'
  exit 1
fi

"$blender" --background --python-exit-code 1 --python "$here/prepare_rig.py" -- \
  "$root/Ranger_Rig_Idle.glb" "$here/ranger_apose.glb"
echo
echo "Rig re-derived from the original delivery. The mesh is back to as-generated:"
echo "any hole capping, strap removal or finger geometry needs doing again."
