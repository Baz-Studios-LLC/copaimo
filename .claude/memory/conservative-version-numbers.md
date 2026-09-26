---
name: conservative-version-numbers
description: "Be stingy with version numbers — PATCH by default on every project, however big the session's work felt"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-15T01:46:54.167Z
---

**PATCH bumps by default, on every project.** The user, 2026-08-15: *"let's not be so generous with version numbers. This is a genuinely massive game and will take a very long time to perfect. Even if something seems like it warrants a huge version bump it probably doesn't in the grand scheme of things."*

Concretely: Ranger v0.1.3 shipped forests, the ranch, the terrain tool's return to the game AND a building reader — four substantial pieces — and it was still a patch bump.

**Why:** a session's work always feels large from inside it. The version is a statement about the *game*, not about the day — and these are years-long projects, so a number that climbs at the pace of sessions runs out of road long before the game is finished. Save minor bumps for when it is meaningfully a different game; save 1.0 for shipping.

**How to apply:** default to PATCH and don't ask. Only propose MINOR when a whole pillar arrives (monsters exist, battles are playable) — and propose it, don't just take it. Keep `Cargo.toml`'s version in step with the tag by hand where the release workflow reads the version from the tag, since a stale one is only ever misleading. Same rule already recorded for VIOLET EDGE in [[neon-edge-release-cadence]]; this is the general form.
