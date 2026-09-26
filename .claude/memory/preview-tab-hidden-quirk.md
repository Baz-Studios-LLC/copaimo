---
name: preview-tab-hidden-quirk
description: "Claude Preview: to pick up edits use location.reload(true) (preview_start does NOT reload the already-open tab); reload occasionally hides the tab and freezes screenshots — fall back to eval/server-restart if so"
metadata: 
  node_type: memory
  type: reference
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

When verifying a browser app via the Claude_Preview MCP in this environment, picking up edited source is the tricky part. **`preview_stop`+`preview_start` does NOT reload the already-open preview tab** — it keeps showing the stale, previously-loaded page (confirmed 2026-06-25: after restart the running page still ran the OLD code; functions added in the edit weren't present). To actually load new file content you must navigate the page: `location.reload(true)` via `preview_eval`. In the 2026-06-25 session reload worked perfectly — tab stayed visible, rAF kept running, and `preview_screenshot` succeeded immediately after.

**How to apply:** To pick up file changes, `preview_eval('location.reload(true)')`, then re-`preview_eval` a state getter to confirm the page came back (`window.__crashout` present), then screenshot. Note the game script is wrapped in a closure — only the `window.__crashout` hook object is global, so `typeof someInternalFn` via eval is always `undefined` and can't tell you whether new code loaded; check a NEW debug-hook key or just screenshot instead.

**If reload triggers the hidden-tab freeze** (seen in the originating session, NOT on 2026-06-25 — so it's intermittent): `location.reload()` leaves the tab `document.hidden===true`, suspending `requestAnimationFrame` so the canvas stops updating and `preview_screenshot` times out (~30s, "renderer unresponsive"). Confirm with `({hidden:document.hidden})`. Try `preview_stop`+`preview_start` to get a foregrounded tab; if the panel stays hidden every time (canvas `getBoundingClientRect` 0, screenshots keep timing out), screenshots are unavailable — stop retrying.

**When restart does NOT recover it** (seen in some sessions: the panel stays `hidden` the whole time, canvas `getBoundingClientRect` is 0 so the backing falls back to its default width, and `preview_screenshot` times out ~30s every time): screenshots are simply unavailable — stop retrying. Fall back to `preview_eval`-driven verification. rAF is paused while hidden, so the game loop won't auto-advance; expose debug hooks on `window` (e.g. a state getter, a `tick(dt)` that calls the update fn manually, a `renderOnce()` that runs the draw passes inside try/catch, and getters/mutators for the feature under test) and drive + assert game logic and "does drawing throw" entirely through eval. Verify geometry by arithmetic/construction. Tell the user visual confirmation wasn't possible and ask them to eyeball it.
