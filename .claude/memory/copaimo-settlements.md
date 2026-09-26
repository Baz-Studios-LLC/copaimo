---
name: copaimo-settlements
description: "Copaimo towns are generated in src/world/town.rs from ten Blender figures; villages = fantasy on dirt, cities = modern towers on stone; size is set by GENRE (village 11 buildings, city 28), not by what fits"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-29T11:13:18.115Z
---

Settlements generate from streets outward — streets, then parcels, then lots — in `src/world/town.rs`, built from ten figures in `dev/art/town.py`. Written up in DESIGN.md's settlement chapter (added 2026-08-29).

**Two ages, deliberately.** Villages and towns are old-school fantasy: half-timber, thatch, packed-earth lanes. Cities are modern: curtain wall, concrete, paved stone, and a skyline whose height falls off from the middle (block 19.7 m → tower 37.5 m → spire 57.1 m). It is the setting's own history showing on the ground, and it's the strongest district cue there is.

**Size is a genre decision, not a capacity one.** A village HAS 11 buildings and a city 28 (`HOUSES_IN_A_VILLAGE` / `HOUSES_IN_A_CITY`). Every attempt to thin towns by geometry — wider frontages, fewer rings, more air — answered "how many can stand here" when the question was "how many should". A Pokémon town is 5–10; Novigrad is a "capital of 30,000" built at the size of a real small town. See [[copaimo-is-stylised-not-realistic]].

**Design frameworks in use:** Kevin Lynch's five (paths, nodes, landmarks, districts — **edges still missing**), and Scott Rogers' hub rules (a "weenie" you can see from outside; a landmark must be a different KIND of thing, not a bigger house). Districts are *shares* of the town, not fixed distances, so they survive a size change.

**How to apply:**
- Buildings are sized for the CAMERA, not the warden — it follows 3–4 m behind, so storeys are 3.6 m and a cottage is 9.9 m across. Threshold is 12 cm, never a plinth: the player walks on terrain, not on a building's floor.
- The ranch is a `Site` with `ranch: true` and is NOT a settlement — nothing may be built on it (guarded).
- `dev/art/see_the_town.sh` opens Blender on the whole kit with a 1.7 m post at each door.
- Footprints in `Building::footprint()` must match the exported `.glb`; a test checks it.
