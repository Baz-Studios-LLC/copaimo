# Claude ↔ Codex collaboration loop

The goal is complementary work, not two agents changing the same repository.

## Roles

**Claude** owns implementation, game files, tests, visual runs, commits, and releases.

**Codex** reads the repository and evidence, reviews active work, proposes implementation ideas,
checks for interactions or missing tests, and writes only inside `codex-suggestions/`.

## Suggested loop

1. Claude reads `README.md` and the relevant suggestion file before starting a related change.
2. Claude records selected items in `CLAUDE_REPLY.md` as `accepted`, `adapted`, `deferred`,
   `rejected`, or `needs review`. A one-line reason is enough.
3. Claude implements one cohesive problem at a time and records the commit plus verification.
4. The user points Codex back to `CLAUDE_REPLY.md` or asks for another read-only review.
5. Codex inspects the current code/evidence and updates only this folder with follow-up findings.

Suggestions are not commands. `DESIGN.md`, measured behavior, and the user's direction outrank
them. A rejected suggestion with a clear reason is successful collaboration.

## Evidence bundle for visual changes

Every visual claim should ideally include:

- a fixed player-height before/after pair;
- the coordinate, facing, time, weather, and build used;
- an overhead view only when layout is part of the question;
- a short performance comparison when geometry, shadows, or draw calls changed;
- a statement of what the screenshot is meant to prove.

The existing `--photo` route is the right foundation. A small named shot matrix would make visual
regressions much easier to compare: ranch gate, village entrance, village node, city entrance,
city landmark, bridge entrance, bridge midpoint, forest edge, and shoreline.

## Review hygiene

- Separate correctness findings from optional design ideas.
- Put the highest-impact item first; do not turn every review into a new roadmap.
- Preserve rejected approaches in the project's existing records when the reason is load-bearing.
- If a suggestion depends on geometry, measure or render before arguing for a number.
- If Claude changes the design materially, update the canonical design record rather than leaving
  the decision only in this collaboration folder.
- Codex should re-read current source and Git state before commenting; this folder may lag behind.

