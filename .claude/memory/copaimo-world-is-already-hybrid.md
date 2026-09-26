---
name: copaimo-world-is-already-hybrid
description: "Copaimo's world is ALREADY a stored/generated hybrid — assets/world holds five stored layers and there is an in-game placement editor; \"nothing is stored\" is only true of settlement layouts, natural scatter and roads"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-09-17T22:43:26.612Z
---

Copaimo's world is not purely generated. As of 2026-09-17 `assets/world/` holds:
`edits.bin` (4 m grid of authored height OFFSETS over generated ground), `surface.bin`
(4 m signed surface bias — worn/paving paint), `country.bin` and `forest.bin` (16 m
painted region and woods-density rasters), `heightmap.png` (base elevation),
`placed.json` (hand-placed buildings with STABLE u32 ids), `world.json` (generator
parameters). `src/editor/mod.rs` is an in-game sculpt + place/carry/turn/remove editor
writing to these. The `trees_in` comment "nothing about a tree is stored anywhere"
is true only of the natural lattice scatter.

What is NOT stored (2026-09-17): settlement layouts (a store now exists in
`src/world/bake.rs`, city 1 first, user widened scope to the whole world), natural
tree/litter instances, country roads.

**Why:** I spent ~30 reads surveying before discovering this; I had designed a store
from scratch when the pattern (painted data the generator consumes) already existed
in five places, with an editor on top.

**How to apply:** Before designing any "make X editable/stored" feature in Copaimo,
read `assets/world/` and `src/editor/mod.rs` first and EXTEND those layers. Scatter
identity is free: a tree/boulder is a pure function of its lattice slot
`(slot_x, slot_z)`, so the slot is the stable id. Grown-street sprout `salt` is a
`salt += 1` counter — NOT a stable id. See [[copaimo-settlements]],
[[copaimo-building-plans]], [[research-proven-solutions-first]].
