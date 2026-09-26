---
name: crashout-design-log
description: CRASHOUT has a DESIGN.md (pillars + invariants + change log) — update it after substantive changes; the game is intentionally single-file
metadata: 
  node_type: memory
  type: project
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

The user wants a running design/change log for CRASHOUT to maintain consistency. It lives at `C:\Users\jsull\Desktop\crashout\DESIGN.md` (design pillars, systems + **invariants**, and a reverse-chronological change log).

**How to apply:** After any substantive CRASHOUT change, add a dated entry to the DESIGN.md change log and update the relevant systems/invariants section if the change alters a rule. Treat the invariants there as guardrails (e.g. the quadratic creep ceiling that keeps survival bounded; meter only falls in the break room; outfit notes must state exact trade-offs).

**Single-file decision:** the game is deliberately ONE file (`index.html`, no build step). A split was considered and **declined** — the whole script shares one global scope, so an ES-module split is a huge refactor (hundreds of cross-refs) and an ordered-`<script>` split adds load-order/TDZ fragility for only navigational gain. Keep it single-file unless the user explicitly asks to split. The `window.__crashout` debug hook is intentional test infrastructure, not dead code. Relates to [[ensure-game-balance]] and [[crashout-testing-meter-pause]].
