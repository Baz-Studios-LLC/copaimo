---
name: neon-edge-purple-is-player
description: "NEON EDGE palette rule: purple is reserved for the player ship + its kit; nothing else (hazards, bosses, pickups-for-other-things) may use purple"
metadata:
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

Hard palette rule the user set for the [[neon-drift-project]] game (2026-07-14):
**"everything to do with the player ship [is] purple, purple can't be used by
anything else."**

- **Purple = PLAYER only:** the ship (`COLORS.purple` `#b64bff`), its bullets,
  the chain shot (`#c77dff`), the mass shot (fiery purple), the vortex/black
  hole (`#b06bff`/`#d7a3ff`), and the wingman drone (`DRONE.color =
  COLORS.purple`). Prefer referencing `COLORS.purple` so the family stays unified.
- **Everything else must NOT be purple.** Hazards own distinct hues: blue rocks
  `#24d1ff`, dense green `#39ff88`, orange volatile `#ff7b1a`, crimson mines
  `#ff2e63`, yellow mobs `#ffd633`, magenta octopus `#ff5ad0`, red devourer
  `#ff3b3b`, teal raider `#2ce8cf`. (The drone was originally mint `#5effc8` —
  user rejected it: green reads as a rock, and it wasn't player-purple.)

**How to apply:** when picking ANY new color (new weapon, hazard, boss, pickup,
FX), check this rule first. Player-side → a purple. Anything else → never purple.
Noted in DESIGN.md §1 (Palette rule). The one borderline legacy color is the
octopus's magenta `#ff5ad0` (pink, treated as distinct from purple) — leave it
unless the user says otherwise.
