---
name: copaimo-asset-is-committed
description: Copaimo's prepared rig (dev/art/ranger_apose.glb) is the COMMITTED source asset; the build only reads it, and mesh/rig sculpting is baked once rather than re-derived per build
metadata:
  type: project
---

`dev/art/ranger_apose.glb` is the source of truth for the Copaimo character, committed to the
repo. `animate_ranger.sh` only READS it. `dev/art/bootstrap_rig.sh` is the only thing that
re-derives it from the original `Ranger_Rig_Idle.glb`, and running that discards any
hand-sculpted mesh work.

Before 2026-08-23 the build ran `prepare_rig.py` on the original delivery on every single run,
re-deriving every mesh fix from scratch.

**Why:** the user asked "How have you not overwritten that original messed up model yet?" and
they were right. Re-deriving *sculpting* every build means a classifier has to re-make the same
judgement call ("is this a sleeve cuff or a hanging strap?") correctly forever. It didn't — it
cut the sleeve cuffs, faces out of a trouser leg, and part of a shoulder — and each time the
response was to tune the threshold, which treats a design fault as a numbers fault.

**How to apply:** split by kind. RIG repair stays procedural (measured constants of the rest
pose; correcting a constant per pose is what twisted the feet three times). MESH work — capping
holes, removing straps, adding finger geometry — is a one-time edit to the committed asset,
verified once and kept. See [[blender-is-the-tool]] and [[validate-the-ruler-first]].
