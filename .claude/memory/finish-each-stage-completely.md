---
name: finish-each-stage-completely
description: A pipeline stage is not done until EVERY bullet and its stated refusal guard are built; never move on with items still open
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-25T13:59:05.257Z
---

When working a staged pipeline (Copaimo's `docs/character-pipeline.md`, and the same applies
anywhere else), a stage is finished only when **every bullet under it is built AND the stage's own
"Refuses when" guard actually exists as a test or an audit measurement**. Do not advance to the
next stage while any item is open, and do not present partial-stage work as the stage being done.

**Why:** the user has said this twice — "lets slow down and make sure each stage is actually done
to completion", then "Remember we need to completely finish each stage". I did Stage 04's
distance matching, reported the step as done, and left the walk↔run blend, turn-in-place, and the
planted-foot slide guard open. The stages are ordered so each one's output is the next one's
foundation (see [[copaimo-rig-pipeline]]), so a half-finished stage silently weakens everything
built on top of it.

**How to apply:** before saying a stage is complete, re-read the stage's bullets and its
*Refuses when* line, and list them off against what was actually built. A stated guard is part of
the deliverable, not documentation — see [[validate-the-ruler-first]]. If something genuinely
cannot be done yet, finish everything else and say explicitly what is left and why, rather than
moving on quietly. Doing several related changes in one pass is fine and wanted
(see [[neon-edge-small-steps]]) — the unit is the stage, not the edit.
