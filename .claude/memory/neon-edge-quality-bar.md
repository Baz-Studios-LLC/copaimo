---
name: neon-edge-quality-bar
description: "VIOLET EDGE quality bar — look, feel, and sound must read AAA; polish outranks feature count"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-30T16:57:47.099Z
---

For VIOLET EDGE ([[neon-edge-bevy-port]]), the user's standing bar (2026-07-30):
**"I want the quality of the game to look, feel and sound like a AAA game."**

**Why:** the game targets a real Steam release ([[crashout-release-target]] energy — this is a
commercial product, not a toy). Reference class: top-tier neon arcade — Geometry Wars 3,
Resogun, Nex Machina. Every surface players touch (boot → menu → HUD → gameplay → death →
game over) should feel deliberate and cohesive.

**How to apply:** when building ANY feature, spend the extra pass on presentation: transitions
fade instead of cut (cf. the boot splash), every state has an audio identity (main/boss/buildup/
game-over dirge/menu handoff), UI is labeled and fitted, feedback is layered (toast + sfx + HUD
flash). Prefer fewer features executed to this bar over more features without it. Juice must
still respect [[neon-edge-photosensitivity]] (≤3Hz) and the clean-flight pillar in
[[neon-edge-difficulty]] (no assists). When a surface feels flat, propose a polish pass on it —
the user consistently accepts these (splash, boss spectacle, ship fill, HUD labels, toasts).
