---
name: neon-edge-powerup-boss-tie
description: "VIOLET EDGE design rule — every powerup is thematically derived from the boss whose defeat drops it (chain=Warden grabs/links, mass=red gains mass, drone=enemy ship)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
  modified: 2026-07-28T18:37:11.927Z
---

Design rule for [[neon-edge-bevy-port]] (VIOLET EDGE): **each powerup is thematically tied to the
boss whose defeat drops it** — the reward echoes the mechanic you just beat. Confirmed by the user
(2026-07-17): Chain Shot ← the Warden *linking/grabbing* rocks; Mass Shot ← the red Glutton gaining
*mass*; Drone ← the enemy *ship* (the W15 Slinger). The base Warp weapon is core kit, exempt.
**Nova Shield ✅ BUILT (2026-07-28, user-specced):** the Pulsar (W25) drop = a regenerating ONE-HIT
barrier (player inherits the boss's lit-invuln↔dark-vuln pulse): up→eats one hit→down ~9s→flickers
back on; hit while down = a life. Absorb lives in `kill_ship` (the single death sink); state in
`Run.nova`. This REPLACED the old "Nova pulse" shockwave sketch. Color note: the ORB wears the
boss's hue (pulsar white-cyan), the granted kit itself is player-purple (nova_color) — same split
as Warhead (orange orb → violet blast). Magnet (W30) still undefined/needs re-theme.

**Why:** keeps the reward loop coherent and reinforces each boss's identity.
**How to apply:** when adding or renaming any powerup, derive it from a boss mechanic — see the
"Pickups ↔ boss mapping" table in `neon-edge-bevy/DESIGN.md` for the full 10-boss proposal. Don't
invent unthemed pickups. Sits alongside [[neon-drift-asteroids-core]] (asteroids stay the theme) and
the 50-level boss ladder now in [[neon-edge-design-doc]].
