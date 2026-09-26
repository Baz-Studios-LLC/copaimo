---
name: confirm-which-thing-they-see
description: "Before changing anything on a reported visual fault, confirm WHICH mesh/scene/file the user is looking at, and put the change history on one sheet by the second report"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-24T18:22:03.218Z
---

When the user reports a visual fault, establish **which object they are looking at** before
changing any of them, and by the **second** report of the same fault, render every state the
thing has been in onto one labelled contact sheet and ask them to point.

**Why:** four passes were spent slimming and reshaping Copaimo's shoes, which were right as
delivered ("do whatever you did for the old animation, those were perfect"). Two causes, both
avoidable:

- The delivered `Ranger-Walk.glb` / `Ranger-Run.glb` each carry their **own copy of the
  character** with different, shapeless shoes. The build only retargets animation off them, so
  that mesh never ships — but a viewer built from one shows it. One side was reporting on one
  mesh while the other measured a different one, and neither said so.
- Each pass measured the shoe against an anthropometric table (foot = 15% of height, sole =
  7-8% of length, toe spring 1-2 cm). Every number was correct and every one was beside the
  point — see [[ranger-stylised-not-realistic]].

**How to apply:** a fault reported twice means the model of the problem is wrong, not that the
fix was too timid — stop tuning and go wider. Render form in **clay** (`LOOK_CLAY=1` in
`dev/art/look_at_him.py`), never textured: paint hides the shape it is painted on, which is why
a 64-vertex blob read as a trainer in every render taken. Ties to
[[validate-the-ruler-first]] and [[ask-when-a-tuning-word-is-ambiguous]].
