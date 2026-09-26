---
name: neon-edge-photosensitivity
description: "VIOLET EDGE — keep ALL flashing/pulsing ≤3 flashes/sec (photosensitivity); it's a light-heavy game"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-23T17:36:31.455Z
---

VIOLET EDGE is **light-heavy**; the user requires **no strobe/flash hazards** — keep every flashing or pulsing effect at **≤3 flashes per second** (WCAG 2.3.1 seizure guidance), *especially* LARGE-AREA ones (full-screen tints, the background grid, big bosses). Small thin elements (ship flame, chain/well tendrils) are lower-risk but keep them sane too.

**Why:** photosensitive-seizure safety. User: "since the game is very light heavy we should ensure that nothing else could make players sick" + "slow it down so it doesnt make people sick."

**How to apply (do the math before shipping a light effect):**
- `sin`-driven brightness: freq = (arg-rate)/(2π); **`.abs()` DOUBLES the perceived rate**. Pulse fields advance at their `pulse += dt * N` rate (boss=5, phantom=3, slinger/detonator/vessel/pulsar=4, devourer/pickup=5).
- Blink toggles `(x * N) as i32 % 2` flash at **N/2 Hz**.
- **Ramp urgency via AMPLITUDE / how bright-white it gets, NOT frequency.**

Fixed 2026-07-23 (all ≤~3 Hz now): boss-warning **full-screen tint** → slow ~0.7 Hz breath that never fully vanishes; **Devourer** "about to burst" white-hot flash (was up to ~10 Hz) → ≤2.8 Hz; **Phantom** ray + aim telegraphs → ≤2.9 Hz; **warp grid** crackle (large-area!) → ≤2.9 Hz; **death-scene explosion stream** → 2 Hz (`PHANTOM_BOOM_EVERY` ≥0.5, don't lower); ship spawn-blink 3 Hz; armed-mine blink 3 Hz; HUD refill flick 2.9 Hz. See [[neon-edge-scope-cap]], [[neon-edge-design-doc]].
