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

1. Fix correctness and modal-input issues in `PLAYER_MAP_REVIEW.md`.
2. Build one settlement-edge and street-integration pass.
3. Establish player-height proof shots before judging further visual work.
4. Make one short playable vertical slice connecting ranch, companion, journey, and guild.
5. Add broader polish only after that slice shows which spaces the player actually notices.
