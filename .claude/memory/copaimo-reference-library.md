---
name: copaimo-reference-library
description: Copaimo has docs/ — a reference library of industry standards (rigging, animation, Blender/glTF, pipeline, design, working-with-AI); READ IT before deriving a technique from scratch
metadata:
  type: reference
---

`~/Desktop/copaimo/docs/` — six files, ~1100 lines, 95 sourced links. Built 2026-08-23 when the
user said to stop and "build a real database we can use to reference how to work efficiently and
effectively."

- `rigging.md` — bone budgets, naming, twist bones, skinning limits, hands/thumbs, bind poses
- `animation.md` — foot sliding and its 3 standard answers, timing, blend trees, foot IK, feel
- `blender.md` — glTF rules that fail SILENTLY, bpy traps with what each cost, headless pipelines
- `pipeline.md` — commit vs derive, validation, golden images, budgets
- `design.md` — creature-collection loops, open-world scale, companion AI
- `working-with-ai.md` — where I'm weak on this work, with published numbers + my own failures

Claims are tagged **STANDARD** (industry, sourced), **MEASURED** (from this project), or **OPEN**.

**Why:** the user's standing rule is [[research-proven-solutions-first]] — too much of what went
wrong was already solved and written down, and I was deriving worse versions. Named things we
lacked: **distance matching**, **stride warping** (Paragon: 60% from stride, 15% from play rate),
**foot IK**, **spring bones** for secondary motion, and the thumb's own axis ~45° off the fingers'.

**How to apply:** check `docs/` BEFORE inventing an approach for rigging, animation, Blender or
pipeline work, and add to it when research turns something up. Root `README.md` links it.
