---
name: copaimo-walk-and-jog-only
description: "Copaimo has TWO gaits, walk and jog — the sprint tier was removed 2026-08-25 and run.glb is the jog"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-25T17:25:58.706Z
---

Copaimo's locomotion is **walk and jog only**. There is no sprint/run tier: `SPRINT_SPEED` was
deleted from `src/player.rs` on 2026-08-25 and Shift no longer does anything (Ctrl still walks).

The delivered clip is still the file `assets/character/run.glb`, but the clip it builds into is
called **`jog`**, and `motion.rs` uses `JOG_COVERS` / `JOG_FRAMES`. That is deliberate, not a
slip: measured against `docs/animation.md`, the clip's effective cycle is 23 frames at 130 steps
a minute, where a real run is 12–16 frames at 180–240. It is a jog.

**Why:** the user's call — "we probably dont even need a sprint (run) in this game" — after the
clip measured out as a jog and the sprint tier had been carrying a speed no delivered animation
could serve.

**How to apply:** do not add a third gait or reinstate Shift-to-sprint without being asked. A
real run clip, if one is ever commissioned, comes in *beside* the jog rather than replacing it.
The jog is tuned to read as EASY — trunk leaning about +6° from its own rest and arm swing at
0.45× as delivered; see [[copaimo-rig-pipeline]] and TROUBLESHOOTING.md for why those numbers.
