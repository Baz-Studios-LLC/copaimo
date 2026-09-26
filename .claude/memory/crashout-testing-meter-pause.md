---
name: crashout-testing-meter-pause
description: "During CRASHOUT testing, pin/freeze the crashout meter so it doesn't auto-trigger rampage/game-over, then restore"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

The user explicitly OK'd pausing/pinning the CRASHOUT meter while testing — "feel free to pause the meter during testing if necessary. Just be sure to set it back after."

**Why:** The meter (`crashout`) climbs in real time via the rAF loop, and it keeps advancing even *between* my tool calls. During multi-step screenshot/verification passes this accumulated to 100 and auto-triggered RAMPAGE (and once, an arrest/"BUSTED"), which snapped the camera back and ruined the shot. Fighting that wasted many screenshots.

**How to apply:** Use `window.__crashout.setCrashout(0)` in the SAME eval as the `renderOnce()` right before a screenshot — the loop only re-climbs a couple % before the shot lands, so it never rampages. A fresh `startGame(false)` also resets the meter to 0 and clears any active rampage. For multi-step DATA checks (not screenshots), do all the ticking + inspection inside ONE eval (the rAF advances state between separate evals), or freeze with `pauseLoop(true)` then `pauseLoop(false)` — but note `pauseLoop(true)` draws a "PAUSED" overlay, so it's NOT usable for screenshots. Always finish by restoring normal play / leaving the game at a clean menu (`setState('menu')`) so no pinned/paused state persists. The camera also clamps to the design frame, so the conference room (bottom-left) and break room (far-left wall) are hard to frame at high zoom; rampage mode uses a tighter follow that can reach them. Relates to [[ensure-game-balance]] and [[preview-tab-hidden-quirk]].
