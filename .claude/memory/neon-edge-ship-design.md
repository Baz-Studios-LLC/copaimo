---
name: neon-edge-ship-design
description: "VIOLET EDGE ship = ORIGINAL 5-point dart (logo-hull redesigns REJECTED twice); engine = original flame + short Tron light ribbon, NO exhaust spark particles"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-28T19:52:03.005Z
---

The VIOLET EDGE player ship keeps the **ORIGINAL 5-point dart hull** — do not redesign it to match
the logo. Two logo-styled attempts were rejected in one session (2026-07-28): first the barbed
needle-nose arrow ("not sure I like the design"), then a literal solid-arrow-with-two-notches per
the user's own spec ("Nah that's terrible, just go back to the original design").

**Engine visuals (the settled combo):** the original flickering thrust FLAME **plus** a
**Tron-style light ribbon** (`ShipTrail`, ~half a second of path in ship-purple, rooted at the FLAME
(not the ship center), drawn with real width via `draw_light_ribbon`'s 3-stroke band — user: "like
the bikes in Tron just not as long or dangerous"). The old exhaust **spark particles stay deleted** —
they persisted as broken dashes behind the ship and the user flagged them twice.
**The hull is FILLED solid neon purple** (`draw_ship(.., fill=true)`, nested inset outlines fused by
bloom) — the ship and ONLY the ship; every other entity stays wireframe. HUD lives icons stay outline.

**Why:** the logo mark and the in-game ship are separate identities; the classic dart reads better in
play. **How to apply:** treat the hull like the [[crashout-chair-leave-original]] chair — don't touch
it again. `ship_hull()`/`draw_ship()` stay the single shared definition (DRY across ship, lives icons,
finale send-off). Tuning the ribbon (length `SHIP_TRAIL_LEN`, brightness curve) is fine; reshaping the
hull is not.
