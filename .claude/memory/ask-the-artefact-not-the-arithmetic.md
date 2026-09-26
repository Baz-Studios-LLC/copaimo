---
name: ask-the-artefact-not-the-arithmetic
description: "\"I want you to always look yourself\" — after ANY visual change, photograph the running game (copaimo --photo x,z) or render in Blender and LOOK before reporting; measurements of the computation agree with me while the artefact is broken"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-29T12:40:46.683Z
---

Standing instruction from the user, 2026-08-29: **always look myself.** Not "measure and infer", not "ask the user to check" — look at the artefact, every time, before saying anything is done.

Copaimo has the tool for it now: `copaimo --photo x,z [--height H --back B --out FILE --settle N]` starts the real game, moves the warden there, waits for streaming, writes a PNG, quits (`src/photo.rs`). For assets, render in Blender ([[blender-is-the-tool]]).

**Why:** four town faults in one session, every one reported by the user looking at their screen while my numbers said fine. The numbers measured the *computation*; the computation was never what was wrong.

- Roads "missing" four times. The paving measured 1,929 vertices, right normals, right height, spawned with a working material — all true. It was **wound inside out**, and a single-sided ribbon wound backwards is *invisible*, not dim. Found in twenty minutes once I could photograph it: lifting it 1.2 m off the ground changed nothing, and nothing buried behaves like that.
- The `.blend` files measured 9.9 m; the `.glb` the game loads were 6.9 m — the batch exporter refuses a bad model **and aborts**, so one stray scratch file staled every model after it alphabetically.
- A landmark stood on the spawn point. The guard walked *other* settlements measuring their distance from the ranch, and never asked what comes up when you stand there.
- `_out` pushed dressings negative always, so on two of four walls "outward" was into the room.

**How to apply:**
- After any visual change: photograph it and Read the PNG. Never report "fixed" on the strength of a passing test alone.
- Write the test that asks the *world*: spawn an app and count entities, open the exported `.glb`, read the pixels. A test of the pure function will agree with me forever.
- `if let Some(resource)` around anything visible is a silent-failure branch — construct on demand.
- Single-sided geometry that "doesn't render" is wound backwards before it is anything else. Same bug hit the roofs.
- Light a verification render **from the camera**; a fixed sun left the wall I was checking in pure shadow.
