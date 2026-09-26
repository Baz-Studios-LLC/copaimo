---
name: research-proven-solutions-first
description: Standing rule across all Baz Studios games — research the PROVEN industry technique and implement it; never report a measured limit as a tradeoff for the user to choose
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-23T14:04:14.168Z
---

The user, 2026-08-23: "I always want industry standard, there is a lot of pushback for AI made games so if the game is high quality people should have nothing or very little to complain about. So when I ask you to research proven solutions and implement them I want that to actually happen. No excuses, no settling. Everything from rigging to mesh issues is available online so we need to utilize this information."

**Why:** AI-made games are held to a higher bar, so the work has to be defensible on craft grounds. And the concrete failure was mine, repeatedly: I hit a measured limit, framed it as a design tradeoff, and handed the choice back — when each one was a solved problem I had not looked up.

**How to apply:**
- When something looks like a hard tradeoff, that is the cue to RESEARCH, not to report. A decision I want to hand the user is usually a signal I have not finished looking.
- Wanting to move a threshold or loosen a guard means the same thing — find the technique instead.
- Mesh, rigging, weighting and garment problems count. Don't defer them as "your sculpting pass" — weight painting, mirror repair and cloth skinning are all documented.
- Reference numbers have a SOURCE and a SCOPE. Mocap constraints do not bind keyframe animation; human cadence bands do not transfer to a different leg length without Froude scaling. Check what a figure is FOR before treating it as a ceiling.
- Runtime correction beats authoring gymnastics. Locomotion examples: foot locking, stride warping, distance matching — all of which dissolve "the clip has one non-sliding speed" rather than tuning toward it.

**Worked examples from the session that produced this:**
- Cadence vs stride reported as a tradeoff → is stride warping / distance matching.
- Foot slide reported as a stride/slide trade → is runtime foot locking.
- Knee fold vs ankle angle vs toe clearance called a three-way conflict → the limits were mocap constraints, not animation ones.
- A limp blamed on the mesh's 4 cm shoe asymmetry FOUR TIMES → was `out[POSES] = out[0]`, a per-POSE seam line left in a function now fed a per-FRAME list. See [[validate-the-ruler-first]] and [[copaimo-troubleshooting-log]].
