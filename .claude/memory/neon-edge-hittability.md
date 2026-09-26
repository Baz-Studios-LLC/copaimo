---
name: neon-edge-hittability
description: "VIOLET EDGE — make small targets hittable via bigger SIZES, not an aim assist (user rejected aim assist)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-28T15:08:43.830Z
---

For VIOLET EDGE, when small targets (small asteroids, mines, mobs) are too hard to hit, the fix is
**bigger hitboxes/sizes — NOT an aim assist.**

The user tried an aim assist across v0.4.1 (a slight snap) and a stronger v0.4.2 attempt (wider cone +
target *leading*), then rejected it outright: **"I actually don't like the aim assist, lets go with bigger
targets instead."** It was ripped out entirely and replaced by enlarging the fiddly targets: small
asteroids 22→30, `MINE_R` 13→18, `ENEMY_R` 14→19 (~35–40%).

**Why:** aiming is meant to be a pure skill in this game — an assist that snaps/leads shots "played the
game for you" and felt like autopilot. The user would rather the objects simply be easier to *see and hit*.

**How to apply:** don't propose or reintroduce aim assist / auto-aim / bullet-homing. Tune hittability at
the target: bump `asteroid_radius` (small tier), `MINE_R`, `ENEMY_R` — these are body/hit/draw radii, safe to
grow without changing danger (a mine's kill is `MINE_BLAST_R`; mobs threaten via bullets, not contact).
Keep it consistent with [[neon-edge-difficulty]] (difficult-but-fair) and [[neon-edge-purple-is-player]].
