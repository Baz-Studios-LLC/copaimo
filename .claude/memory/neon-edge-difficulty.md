---
name: neon-edge-difficulty
description: "NEON EDGE difficulty target — difficult but NOT impossible; manageable chaos is fine/desired"
metadata:
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
  modified: 2026-07-29T14:30:34.231Z
---

For [[neon-edge-bevy-port]] (and NEON EDGE generally), the user's difficulty target:
**"difficult but not impossible — manageable chaos is fine."**

**Why:** the game is a hectic neon Asteroids love-letter; the fun is in busy,
readable pressure, not unfair walls. The user explicitly welcomes on-screen chaos
(lots of hazards at once) as long as it stays *dodgeable/manageable*.

**How to apply:** when tuning boss/mine/enemy/asteroid density, throw rates, speeds,
and spawn caps, bias toward "a lot going on but survivable with skill." Don't nerf
mechanics for fear of chaos, but keep everything telegraphed and escapable (e.g.
dodgeable projectile speeds, gaps to exploit, brief invuln windows). If a change
could create an unavoidable/instant-death situation, that's the line — pull back
there (cf. the empty-field fix: rocks drift in from edges so none spawns on the
player). Mirrors the [[ensure-game-balance]] spirit from CRASHOUT, for this game.

**Asteroid escalation (2026-07-20):** each NEW asteroid type SHOULD be dangerous and
should get progressively harder to deal with than the last — orange explosives are
*meant* to be lethal, not a nuisance. The intent is that players must LEARN to manage
each type (spacing, order of engagement, when to shoot). So when a new asteroid feels
weak/harmless, that's a bug — lean INTO the threat (e.g. orange blast radius bumped
150→250 so it truly engulfs neighbours). Pair with the "learn the toy before the boss
made of it" progression in DESIGN.md. Still bounded by the instant-death line above.

**Arcade permadeath + anti-give-up (2026-07-29):** DECIDED — game over = full reset,
NO upgrades persist, no continues ("the idea is those old school arcade games"); a
CONTINUE system was floated and declined. The user's worry is players *quitting over
difficulty*, answered with persistent RECORDS not power: best-wave stat + nearest-grind
ticker on the Game Over screen, lifetime achievements, permanent Pilot Log decrypts.
**Pillar: "flying and shooting as clean as possible — skilled play should be rewarded"**
— control feel/readability issues outrank difficulty nerfs; never fix difficulty by
adding player power. **Release bar: the game isn't ready until the user can beat it
WITHOUT dev mode** — expect continued balance passes until then.
