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

## Stale-suggestion reminder

Suggestions should not disappear merely because newer work is more interesting.

**This does not require Claude to implement a suggestion.** It requires Claude to acknowledge it and
record a disposition. `Deferred` and `rejected` are valid outcomes when accompanied by a short reason;
silence is the only outcome this reminder rule is intended to prevent.

- Claude should mark every P0/P1 finding and every direct user request as `accepted`, `adapted`,
  `deferred`, `rejected`, `needs review`, or `closed` in `CLAUDE_REPLY.md`.
- Codex should resurface an **unacknowledged** P0/P1 after two later relevant commits or one active
  workday, whichever comes first. The reminder should state the original finding, its age, and why it
  still matters.
- An accepted finding that remains open should be mentioned again at the next related milestone or
  after roughly one week. Optional art/design ideas should normally wait for a related milestone.
- A clearly deferred suggestion should be recalled only when its stated prerequisite arrives. A
  rejected suggestion with a reason should not be repeated unless new evidence materially changes it.
- Before reminding Claude, Codex must recheck the live code and evidence. Quietly close anything that
  has already been fixed, superseded, or made irrelevant.
- Reminders are short and prioritized: at most three stale items at once, with correctness and visible
  AAA-quality gaps ahead of optional polish.

This is a memory aid, not a demand that Claude interrupt cohesive work or agree with Codex. The
purpose is to prevent important integration defects and approved quality work from being silently
buried.
