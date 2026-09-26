---
name: neon-edge-small-steps
description: "VIOLET EDGE 'small steps' = work one cohesive SECTION/area at a time (may be several related changes), not the whole game at once — NOT one-tiny-edit-at-a-time"
metadata:
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

When the user says work in **small steps** on [[neon-edge-bevy-port]], they mean **work on the game in
small SECTIONS** — one cohesive area/feature at a time (e.g. "the menu", "the warp FX", "a boss") — NOT
that every change must be a single tiny edit. Doing several related changes together within a section
is expected and welcome; the thing to avoid is trying to do the whole game at once. (User corrected an
earlier over-literal reading on 2026-07-17: "small steps means we'll work on the game in small sections
not only do small things.")

**Why:** the user solo-tests the window between sections and wants coherent, reviewable chunks.
**How to apply:** scope a section, do the full related work for it, verify (cargo test/clippy), let
them playtest, then move to the next section. Still defer the big 50-level content arc (bosses/
asteroids/powerups) to build section by section — it's written up in `neon-edge-bevy/DESIGN.md`.
