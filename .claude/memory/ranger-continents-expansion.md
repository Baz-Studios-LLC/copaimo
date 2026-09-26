---
name: ranger-continents-expansion
description: "Ranger ships new CONTINENTS in later patches — the world and monster roster keep growing, so nothing may assume the map is final"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-17T20:27:59.793Z
---

Ranger is **one game that grows by expansion, never a Part 2**. New continents
and new monsters arrive as patches to the same game — there is no sequel, no
second install, no separate world. Stated by the user on 2026-08-17.

**Why:** it changes what "finite by construction" means and it rules out the
usual escape hatch. A sequel gets to reset everything; an expansion cannot.
Today's landmass is one continent of several to come, today's save has to still
load, and today's monster roster is the start of one list rather than the whole
of it.

**How to apply:** never bake in an assumption that this map is the map, and
never propose "that can wait for the sequel" — there isn't one. Anything
keyed to the world — biome classification, monster habitats, town siting, the
map image and `world.json`, save data, the terrain tool's overview — should keep
working when a second landmass is added rather than needing to be rewritten.
Prefer per-continent data over global constants where it is free to do so, and
flag it when it is not. See [[ranger-monster-game-concept]] for the game itself.
