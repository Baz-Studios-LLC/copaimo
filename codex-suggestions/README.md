# Codex suggestions for Copaimo

This directory is the collaboration boundary between Codex and Claude.

- Codex may read the whole Copaimo repository but only writes inside this directory.
- These documents are suggestions, not instructions to change the game automatically.
- Claude should preserve Copaimo's recorded design decisions and accept, adapt, defer, or
  reject suggestions with a short reason.
- Game code, assets, documentation, commits, and releases remain Claude's responsibility.

## Documents

- [HOVERBOARD_BLENDER_AND_GAMEPLAY_IMPLEMENTATION_SPEC_2026-09-01.md](HOVERBOARD_BLENDER_AND_GAMEPLAY_IMPLEMENTATION_SPEC_2026-09-01.md) — a Copaimo-specific production handoff for the user-requested pre-mount hoverboard: foldable Blender asset, Warden deploy/ride/stow animation, Bevy events, kinematic terrain/collision integration, deck foot planting, camera/audio/VFX, failure handling and proof criteria.
- [HOVERBOARD_CONCEPT_SHEET_2026-09-01.png](HOVERBOARD_CONCEPT_SHEET_2026-09-01.png) — character-matched orthographic, folding, stowed and riding design reference for the board.
- [HOVERBOARD_DEPLOY_STOW_STORYBOARD_2026-09-01.png](HOVERBOARD_DEPLOY_STOW_STORYBOARD_2026-09-01.png) — eight key poses covering backpack pull, unfolding throw, mount, ride, step-off, catch and stow.
- [HOVERBOARD_IMAGEGEN_PROMPTS_2026-09-01.md](HOVERBOARD_IMAGEGEN_PROMPTS_2026-09-01.md) — reference inputs and exact final prompts used to generate both visual sheets.
- [AAA_QUALITY_MASTER_AUDIT_2026-08-31.md](AAA_QUALITY_MASTER_AUDIT_2026-08-31.md) — repository-wide assessment of what is already strong, what still separates the foundation from AAA production quality, and the recommended rendering, world, interior, input, audio, performance, CI, and documentation direction.
- [AAA_EVIDENCE_TEST_AND_PERFORMANCE_MATRIX_2026-08-31.md](AAA_EVIDENCE_TEST_AND_PERFORMANCE_MATRIX_2026-08-31.md) — measurable foundation/vertical-slice/production gates, derived photo views, temporal captures, corrected bot verdict semantics, automated test layers, performance routes, provisional budgets, and CI/release evidence.
- [AAA_ROADMAP_AND_SUGGESTION_LEDGER_2026-08-31.md](AAA_ROADMAP_AND_SUGGESTION_LEDGER_2026-08-31.md) — the compact active queue, explicit dispositions, recently closed work, and staged roadmap. Use this to decide what matters; use the long research files for implementation detail.
- [PLAYER_MAP_REVIEW.md](PLAYER_MAP_REVIEW.md) — concrete review of the player-map change.
- [DESIGN_SUGGESTIONS.md](DESIGN_SUGGESTIONS.md) — gameplay and visual implementation ideas.
- [BUILDINGS_TOWNS_CITIES_AND_OUTLINES_RESEARCH.md](BUILDINGS_TOWNS_CITIES_AND_OUTLINES_RESEARCH.md) — production research and a Claude-facing implementation brief for generated settlements, modular architecture, and selective cel-style ink outlines.
- [WORLD_VISUAL_QUALITY_ROADS_AND_OUTLINES_RESEARCH.md](WORLD_VISUAL_QUALITY_ROADS_AND_OUTLINES_RESEARCH.md) — road-first world-art research covering continuous dirt-to-city transitions, settlement approaches, route hierarchy, roadside ecology, selective outlines, atmosphere, weather, and an implementation/validation sequence.
- [ROAD_TRANSITIONS_FOOTWAYS_AND_JUNCTIONS_RESEARCH.md](ROAD_TRANSITIONS_FOOTWAYS_AND_JUNCTIONS_RESEARCH.md) — code-specific production research for dirt-to-city cross-sections, road widening, kerbs, footways, traversal height, junction topology, gateway dressing, performance, and automated proof.
- [AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md](AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md) — a staged proposal for a deterministic bot that drives the real character through roads, kerbs, doorways, interiors, slopes, bridges, and controlled frame-rate tests while producing reproducible evidence.
- [ROADS_SIDEWALKS_PRODUCTION_SPEC.md](ROADS_SIDEWALKS_PRODUCTION_SPEC.md) — a visual-first production specification for road profiles, sidewalk zones, correct curb normals, controlled grading, staged settlement approaches, intersection ownership, road-relative materials, selective outlines, and validation.
- [FOUNDATION_DEEP_DIVE_2026-08-30.md](FOUNDATION_DEEP_DIVE_2026-08-30.md) — a prioritized read-only audit of the current world foundation: shared road-material ownership, staged-transition junctions, glTF material adoption, full road normals, intersection topology, audit readiness/cost, tool feature boundaries, and the next visual-quality passes. It intentionally excludes gameplay-loop and vertical-slice work.
- [PROCEDURAL_JUNCTION_NODE_RESEARCH_2026-08-31.md](PROCEDURAL_JUNCTION_NODE_RESEARCH_2026-08-31.md) — a production and code-specific contract for the active junction rewrite: planar graph construction, per-arm contacts, curb returns, valid boundary loops, triangulation, shared rendered/traversal height, node material coordinates, deterministic tests, and an AAA-capable staged path.
- [CODEX_REPLY.md](CODEX_REPLY.md) — Codex's latest response to Claude's questions and work.
- [COLLABORATION.md](COLLABORATION.md) — a lightweight Claude ↔ Codex working loop.
- [CLAUDE_REPLY.md](CLAUDE_REPLY.md) — a place for Claude to leave decisions, questions,
  commit references, and requests for another review.

## Current visual reading

The 2026-08-29 screenshots show strong world-scale systems and readable authored buildings,
but settlements and the ranch still read mostly as isolated models and broad road bands on a
large, uniformly green surface. The highest-value visual work is therefore not "more buildings."
It is the connective tissue that makes existing buildings belong to a place: street edges,
yards, gardens, boundaries, entrance sequences, landmarks, material transitions, and props
that imply daily life.

## Suggested priority

The current phase remains foundation work, not a push toward immediate playability. The active source
of truth is `AAA_ROADMAP_AND_SUGGESTION_LEDGER_2026-08-31.md`: finish the active connected-edge junction
contraction, correct the evidence driver's verdict semantics, close the rendered/traversal surface
contract, then establish target-hardware evidence. Gameplay-loop and broad
vertical-slice production remain explicitly deferred until the foundation gate or a user change of phase.
