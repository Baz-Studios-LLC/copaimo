# Codex suggestions for Copaimo

This directory is the collaboration boundary between Codex and Claude.

- Codex may read the whole Copaimo repository but only writes inside this directory.
- These documents are suggestions, not instructions to change the game automatically.
- Claude should preserve Copaimo's recorded design decisions and accept, adapt, defer, or
  reject suggestions with a short reason.
- Game code, assets, documentation, commits, and releases remain Claude's responsibility.

## Documents

- [PLAYER_MAP_REVIEW.md](PLAYER_MAP_REVIEW.md) — concrete review of the player-map change.
- [DESIGN_SUGGESTIONS.md](DESIGN_SUGGESTIONS.md) — gameplay and visual implementation ideas.
- [BUILDINGS_TOWNS_CITIES_AND_OUTLINES_RESEARCH.md](BUILDINGS_TOWNS_CITIES_AND_OUTLINES_RESEARCH.md) — production research and a Claude-facing implementation brief for generated settlements, modular architecture, and selective cel-style ink outlines.
- [WORLD_VISUAL_QUALITY_ROADS_AND_OUTLINES_RESEARCH.md](WORLD_VISUAL_QUALITY_ROADS_AND_OUTLINES_RESEARCH.md) — road-first world-art research covering continuous dirt-to-city transitions, settlement approaches, route hierarchy, roadside ecology, selective outlines, atmosphere, weather, and an implementation/validation sequence.
- [ROAD_TRANSITIONS_FOOTWAYS_AND_JUNCTIONS_RESEARCH.md](ROAD_TRANSITIONS_FOOTWAYS_AND_JUNCTIONS_RESEARCH.md) — code-specific production research for dirt-to-city cross-sections, road widening, kerbs, footways, traversal height, junction topology, gateway dressing, performance, and automated proof.
- [AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md](AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md) — a staged proposal for a deterministic bot that drives the real character through roads, kerbs, doorways, interiors, slopes, bridges, and controlled frame-rate tests while producing reproducible evidence.
- [ROADS_SIDEWALKS_PRODUCTION_SPEC.md](ROADS_SIDEWALKS_PRODUCTION_SPEC.md) — a visual-first production specification for road profiles, sidewalk zones, correct curb normals, controlled grading, staged settlement approaches, intersection ownership, road-relative materials, selective outlines, and validation.
- [FOUNDATION_DEEP_DIVE_2026-08-30.md](FOUNDATION_DEEP_DIVE_2026-08-30.md) — a prioritized read-only audit of the current world foundation: shared road-material ownership, staged-transition junctions, glTF material adoption, full road normals, intersection topology, audit readiness/cost, tool feature boundaries, and the next visual-quality passes. It intentionally excludes gameplay-loop and vertical-slice work.
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

The current phase is foundation work, not a push toward playability. Follow the order in
`FOUNDATION_DEEP_DIVE_2026-08-30.md`: close road/material contracts, then surface geometry and
junction ownership, then settlement integration and broader visual polish. Defer gameplay-loop
and vertical-slice work until the user explicitly reopens that scope.
