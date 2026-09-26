---
name: neon-edge-design-doc
description: "NEON EDGE has a maintained DESIGN.md (waves/spawns/asteroid types/bosses/power-ups/roadmap) at the repo root — keep it updated after any mechanic/spawn/tuning change"
metadata:
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

**ACTIVE doc (2026-07-17):** the Bevy port ([[neon-edge-bevy-port]]) now has its OWN design doc at
`C:\Users\jsull\Desktop\neon-edge-bevy\DESIGN.md` — the canonical roster for the current game. It's
a table of **asteroid types** and **enemies/bosses** with ✅ implemented / 🔷 planned status. Keep it
updated when a type/enemy/boss ships or its behaviour changes. (The old JS doc at
`Desktop/neon-asteroids/DESIGN.md` is now just the legacy reference.)

**Asteroid roster the user specified (2026-07-17):**
- Blue ✅ standard · Green ✅ dense (HP=size) · **Orange 🔷** explosive — area-destroys everything
  nearby incl. other asteroids → CHAIN reactions (and hits the player) · **Red 🔷** grows by
  absorbing nearby asteroids (Glutton-boss mechanic as a field rock; a broken large splits into two
  that can re-absorb and re-grow) · **Pulser 🔷** pulses bright white, INVULNERABLE while lit (shoot
  it on the dark phase).
- Enemies: yellow mob ✅. Bosses: **the Warden** ✅ (boss 1, W5, shield-hoarder) → **the Glutton** ✅
  (boss 2, W10, red seeker/eater). No other enemy types specified yet.

**Why:** the user wants a single authoritative roster/roadmap kept current as the game grows.
**How to apply:** after any substantive mechanic/spawn/boss/pickup/tuning change, update DESIGN.md
in the same turn (like the [[crashout-design-log]] habit). It's committed like code but per
[[neon-edge-bevy-port]] the Bevy repo IS now pushed to GitHub — still don't push unless asked.
