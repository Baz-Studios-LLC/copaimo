---
name: crashout-release-target
description: "CRASHOUT is targeting a Steam (desktop) release, not mobile; near-ready, remaining work is balance testing + design cleanup"
metadata: 
  node_type: memory
  type: project
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

As of 2026-06-27 the plan for CRASHOUT pivoted: it's targeting a **Steam (desktop) release, not mobile.** The user considers it **nearly ready** — roughly a few more weeks of **balance testing** plus some **design cleanup** remaining.

**Implications to keep in mind (desktop/Steam):**
- Input is keyboard + mouse — the game is already built that way (arrows/E + clickable buttons), so no control rework needed; controller support would be a nice-to-have, not required.
- **Debug hook is now gated by `const DEBUG` (line ~40 of index.html).** It's `true` during dev (hook live for testing); **for the release build, flip `DEBUG=false`** to strip `window.__crashout` (and the `splash`/`__splashHold` test handles) so the console can't be used to cheat.
- A **release checklist** lives in `crashout/DESIGN.md` (Steam build/packaging, options menu, achievements, polish, QA).
- The game is a single HTML file; a Steam release likely needs packaging (e.g., Electron/NW.js wrapper). That's a distribution step, separate from the game code.
- The goals/titles system maps naturally to **Steam achievements**; the leaderboard concept to Steam leaderboards — worth considering if scope allows.
- Steam players expect an **options menu** (volume slider, fullscreen toggle); currently there's just a mute button.

Priorities for the home stretch: balance (keep it bounded-but-skillful, see [[ensure-game-balance]]) and design cleanup tracked in `crashout/DESIGN.md` (see [[crashout-design-log]]).
