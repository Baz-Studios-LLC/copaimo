---
name: neon-edge-warhead-balance
description: VIOLET EDGE Warhead — RESOLVED 2026-07-30 as a Q-toggled SIEGE weapon (1.3s cadence, on-impact 110px AoE); history of the nerf path
metadata:
  node_type: memory
  type: project
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-30T19:27:07.054Z
---

The **Warhead rounds** powerup (Detonator/boss-4 drop, wave 20) — balance RESOLVED 2026-07-30
(commit 3251e5a). Evolution: permanent passive (flagged too strong 2026-07-21) → Q-cycle toggle
mode (Standard→Mass→Warhead, lever 3) → detonate-ON-IMPACT with a REAL `WARHEAD_BLAST_R`=110 AoE
(round consumed, no more infinite pierce; gold spared, lit pulsers immune, bypasses beacon auras)
→ finally the user's verdict: **"a toggle ability and very slow rate of fire"** →
`WARHEAD_COOLDOWN` 0.28 → **1.3s**. It is now a deliberate SIEGE weapon: toggle to it, aim one
round into a packed lane, wait. Terrible boss DPS by design — you switch back to standard/mass for
cores (a real tactical choice).

**How to apply going forward:** don't re-buff its cadence; if it still dominates, the remaining
levers are shrinking the AoE or making the AoE skip dense/special rocks. If it feels dead, speed
the cooldown toward ~0.9 before touching the radius. See [[neon-edge-powerup-boss-tie]],
[[neon-edge-difficulty]].
