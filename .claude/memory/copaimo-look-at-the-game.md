---
name: copaimo-look-at-the-game
description: Copaimo has a --photo mode that drives the real game and screenshots it; ALWAYS load the game and look at a change before reporting it (user has said so twice)
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-29T15:58:53.255Z
---

Copaimo ships `--photo x,z [--height --back --out --settle --map]` (`src/photo.rs`), which
starts the real game, moves the warden there, waits for streaming, screenshots and quits.
`--map` pulls the in-game map up first. Use it on every visual change before reporting.

**Why:** the user has said it twice, unprompted — *"There has to be a way you can drive the
game and actually see things yourself"*, then *"I want you to always look yourself"*, then
*"Also remember to actually load the game and look, if I need to add it to your rules I
will."* Every long-running visual bug in this project survived because measurements agreed
with me: roads were "invisible" for four rounds while every number about the mesh was
correct (it was wound inside out), and a brown road photographed grey twice before I divided
an observed pixel by the colour that produced it and found the sky light was blue-biased.

**How to apply:** take the photo and READ the image, don't just check the "screenshot saved"
line. Sample pixels only at points confirmed to be the thing in question — a filter that
"finds road-ish pixels" happily returns roofs and reports success. See
[[ask-the-artefact-not-the-arithmetic]] and [[validate-the-ruler-first]]; for Blender-side
work the equivalent is [[blender-is-the-tool]].
