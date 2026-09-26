---
name: copaimo-hand-rig
description: Copaimo's character has 71 bones incl. 30 finger bones (5 digits x 3 per hand); curl is local X, the closing sign is MEASURED (-1 left, +1 right), and a `grip` clip proves it
metadata:
  type: project
---

The Copaimo character rig is 71 bones: the original 41 plus `{L,R}_{Thumb,Index,Middle,Ring,
Pinky}{1,2,3}`. Built by `dev/art/add_finger_bones.py`, which is re-runnable. A `grip` clip in
`animate_ranger.py` closes and opens both hands; the gaits carry a relaxed curl (`RELAXED`)
instead of flat fingers.

Key facts that are easy to get wrong:

- **Curl is rotation about the bone's LOCAL X** (`curl()`), never `swing()` — five digits point
  five ways, so no armature-space axis flexes them all.
- **The sign that closes a fist differs per hand** (-1 left, +1 right) because the flexion
  reference is a cross product, and mirroring one flips it. `which_way_closes()` MEASURES it by
  curling and checking whether the tips approach the wrist. Never reason about this.
- **A thumb is not the short digit, the splayed one, or the odd one out — the pinky is all
  three.** Four separate discriminators named the pinky. Names are settled on the mesh as
  delivered (the "points across the palm" test, 0.96 vs 0.49) and carried through subdivision by
  tip position.

**How to apply:** verify hand changes by measuring, not looking — this mesh holds its surface
together with duplicate vertices at hard edges, so a seam and a hole render identically under a
glove. Weld by position and check the welded groups stay together when posed. See
[[blender-is-the-tool]] and [[copaimo-rig-pipeline]].
