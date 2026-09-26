---
name: blender-is-the-tool
description: "Copaimo uses Blender for all asset/rig/animation work for the game's whole life — always reach for it rather than reasoning about geometry"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-22T02:12:25.444Z
---

Blender is the tool for Copaimo's asset, rig and animation work **for the remainder of
the game's life** (stated 2026-08-21). Always use it when it can answer the question.

**Why:** every rig fault in this game was found by measuring in Blender and missed by
reasoning about it. The limbs bent backwards for three attempts because a docstring
*described* the axis convention instead of measuring it, and the description was
exactly inverted. The eye-white boxes took four failed heuristics before being
measured off the atlas. Clips were shipped without being rendered and the player
caught every one.

**How to apply:**
- Batch Blender for anything KEPT: `dev/art/*.sh` wrap `blender --background
  --python-exit-code 1 --python <script>` (Blender exits 0 on a traceback without that
  flag). Blender 5.2 lives at `/c/Program Files/Blender Foundation/Blender 5.2/`.
- The live MCP session (`dev/blender_live.py`, port 9876) for FINDING numbers and for
  looking — it imports GLBs fine (see [[preview-tab-hidden-quirk]] for the analogous
  "the tool did nothing" trap). Clear the scene first, and put a camera and light back
  or `--look` reports "the scene has no camera".
- Before believing a pose changed, RENDER it. `dev/art/gait_report.sh` runs every
  instrument at once.
- An assigned action is re-evaluated by the depsgraph and overwrites a pose edit —
  bake the pose (read it, drop the action, write it back) before editing.
- Measure, then name the result for its effect, so a call site cannot restate a sign.
  See [[test-after-building]] and [[ask-when-a-tuning-word-is-ambiguous]].
