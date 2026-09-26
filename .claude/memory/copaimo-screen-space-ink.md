---
name: copaimo-screen-space-ink
description: "Copaimo outlines come from src/ink.rs — a pass over the finished frame reading ViewDepthTexture, NOT inverted hulls and NOT a prepass; hulls remain only on the warden and authored landmarks"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-31T13:54:27.822Z
---

Copaimo's semi-cel outlines are drawn by `src/ink.rs` + `assets/shaders/ink.wgsl`:
one pass after tonemapping that finds where DEPTH breaks and darkens there. Added
2026-08-31. Three decisions in it are load-bearing and easy to undo by accident:

- It reads **`ViewDepthTexture`**, the depth the main pass just wrote, and needs
  `Camera3d::depth_texture_usages` to include `TEXTURE_BINDING` (set on the camera in
  `camera.rs`). Do **not** switch it to a `DepthPrepass` — `CloudShade` deforms grass
  and water in the vertex stage, so a prepass on Bevy's default vertex path puts depth
  where the visible blade is not.
- The planarity test is asked of the **raw reversed depth**, not of metres. Only the
  reciprocal is affine across a projected plane, so converting first makes a flat road
  at a grazing angle read as an edge.
- The line **darkens** (`min(colour * share, charcoal)`) rather than painting a
  colour. Painting charcoal made an unlit wall BRIGHTER — a pale glow round every
  building at dusk.

The inverted hulls came off buildings, props and yards at the same time, halving their
geometry (a hull is a whole second copy of the mesh). `masonry.outline` is still
called by `bridge.py` and `ranch.py` and belongs on the warden — a hull is for a line
somebody sculpts, the pass is for a line the frame detects.

**Why:** a hull is a fixed size in WORLD, so it vanishes at distance and swallows trim
up close, and it can only outline a model — never the terrain, roads or grove.

**How to apply:** judge changes by re-measuring, not by eye alone: the frame carries
~2.6% inked pixels and open grass carries none. See TROUBLESHOOTING.md, and
[[copaimo-look-at-the-game]] for taking the shots.
