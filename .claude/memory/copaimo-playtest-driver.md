---
name: copaimo-playtest-driver
description: Copaimo has `--drive`, a bot that plays the real game; run it after any movement/collision/terrain change, and invert the fix to prove a route catches it
metadata:
  type: project
---

Copaimo has a **deterministic playtest driver**: `cargo run --release -- --drive`.
It presses W. It drives the real warden through the real input (`ButtonInput<KeyCode>`
and `Orbit::yaw`), the real movement, collision and grounding — it never writes the
warden's transform except to place them at a route's start, and it never paths around
an obstacle. Report lands in `dev/evidence/playtest.md` (git-ignored). Built
2026-08-30 from Codex's `AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md`; source `src/drive.rs`.

Routes are authored, resolved from the generated world by NAME (not coordinates), and
each declares its outcome in advance. **A route that expects to be BLOCKED is as
important as one that arrives** — a driver that only checks successful travel approves
a warden who walks through walls.

**Why:** every fault of that week got past 340 unit tests because it only existed once
the systems met — a road drawn 5 m wider than it could be walked, a step rule that was
really a frame-rate rule, a kerb that rendered and refused the controller, a building
placed correctly and floating.

**How to apply:** run it after any movement, collision, road or terrain change. And
prove a new route is worth having by **putting the fault back and watching it fail** —
that is how the canyon routes were shown to be real (with the old step rule restored
the warden walks 20 m up the canyon wall at 60 Hz walking and above). See
[[validate-the-ruler-first]] and [[test-after-building]]; the visual analogue is
[[copaimo-look-at-the-game]].
