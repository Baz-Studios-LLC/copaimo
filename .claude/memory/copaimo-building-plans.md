---
name: copaimo-building-plans
description: "Copaimo buildings are planned in dev/art/town.py, MEASURED off the built mesh into assets/models/town.txt, and checked against the game by Rust tests — never edit one side alone"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-29T22:01:26.788Z
---

Copaimo buildings (as of 2026-08-29) have a three-part contract, and all three have to
move together:

1. `cottage_plan` in `dev/art/town.py` decides where everything goes before any of it
   is built (entrance → protected route → rooms → furniture). `COTTAGE` holds the
   metrics; `bay_places` owns the wall grid; `fireside` owns where a fire and its stack
   stand.
2. At build time `measure_the_cottage` / `every_doorway` **measure the mesh Blender just
   made** and refuse to write a plan the geometry contradicts.
3. `assets/models/town.txt` carries those measured numbers, and `world::town`'s tests
   check them against what the game does — chiefly `Building::walk_in`, the collision
   gap in `Plot::walls`.

**Why it is split that way:** Blender proves the mesh matches the plan, Rust proves the
plan matches the game. Neither side can pass by comparing a number to the thing that
produced it. See [[validate-the-ruler-first]].

**The bug family this exists to kill:** one fact with two derivations, in two places
nothing ever puts side by side. Each line is fine on its own, so reading either finds
nothing. It has now produced the colour-space bug, the door-orientation bug, a doorway
you could only half walk through, a chimney at the opposite corner from its fire, and a
stud through a front door. When something is wrong and both expressions look right,
search for the second derivation. See [[ask-the-artefact-not-the-arithmetic]] and
[[dry-no-code-reuse]].

Adding a figure or changing a wall means: rebuild with Blender, re-export the `.glb`,
run `cargo test` (full, not `--no-default-features`), and look at it with `--matrix`.
See [[blender-is-the-tool]] and [[test-after-building]].
