---
name: neon-drift-asteroids-core
description: NEON DRIFT design pillar — asteroids are THE core; other hazards must never overshadow them
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

For [[neon-drift-project]], **asteroids are the core of the game and must never be overshadowed** by any other hazard (mines, dense rocks, enemy ships, future bosses/bonus rooms).

**Why:** user's explicit design pillar (2026-07-09). It's a "love letter to Asteroids" — the blue asteroids ARE the game; everything else is a complication layered on top.

**How to apply:**
- Keep the asteroid population the clearly dominant on-screen count at every phase. Secondary hazards (mines, enemies) are capped as a *fraction* of asteroid population (see `mineTarget`/`enemyTarget` in game.js — MINE.maxFraction / ENEMY.maxFraction) so they can never out-number asteroids. Asteroid splits (large→2→4) further multiply the effective count, keeping them the star.
- Dense GREEN and volatile ORANGE rocks are still asteroids — variety *within* the asteroid family is encouraged and does NOT violate the pillar (field is all-green from W6, +orange from W12, per the user; the "keep blue dominant" idea was superseded). The pillar is asteroids vs NON-asteroid hazards, not blue-vs-other.
- **WIREFRAME ONLY — asteroids (and asteroid-bosses) are NEVER a filled core/centre or HP ring.** User rejected, in order: the red devourer's core fill + on-boss HP arc, the octopus HP ring, and the orange volatile's filled core ("asteroids should never have a core"). Signal variety with the OUTLINE instead — colour, concentric wireframe rings (dense), a pulsing/breathing stroke (orange volatiles), jaggedness. Solid fills are out. HP is shown on the screen-space `drawBossBar`, never on the rock.
- Before adding/scaling any hazard, ask: does this make asteroids feel secondary (visually or in playtime)? If yes, don't — dial it back.
- **EVERY BOSS has a mechanic TIED TO ASTEROIDS, and clearing stray asteroids must ALWAYS be an effective strategy against it** (user, 2026-07-16). Corollary: weapons must NOT let you brute-force a boss's core and bypass its asteroid mechanic. Concretely in the Bevy port ([[neon-edge-bevy-port]]): the **chain shot does NOT damage the boss core** — it only shatters rocks/enemies/mines. Against the octopus that means denying it shield material (clearing stray rocks) is the play, not beaming the core. Design future bosses so rock-clearing is the lever.
- **GUARDRAIL (user, 2026-07-10): flag the USER if THEIR OWN direction works against this pillar.** The user explicitly wants to be told. Watch for: a new NON-asteroid enemy/mechanic that would dominate screen-time or attention, a boss that isn't asteroid-centric, bullet-hell that shifts focus off rocks, or weapons/upgrades that make asteroids trivial to ignore. Raise it BEFORE building — explain the risk and offer an asteroid-first alternative. (So far everything REINFORCES the pillar: bosses grab/eat/throw rocks, orange is an asteroid variant, removing mines/mobs is *more* asteroid-focused. Nothing has conflicted yet.)
