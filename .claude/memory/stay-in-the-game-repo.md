---
name: stay-in-the-game-repo
description: "Don't expand into Opificium (or other repos) unasked — do the task in the repo the user named, and offer the follow-through rather than doing it"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-15T01:32:22.248Z
---

When the user asks for work **in the game**, do it in the game and stop there. Touching [[opificium-terrain-bench]] as unrequested follow-through drew a pushback: *"Wait why are you messing with opificium... just add the editor to the game"* (2026-08-14).

What happened: bringing the in-game editor back genuinely required moving `Sculpt`/`Brushing`/`Stamp` into `terrain-core` — that part was unavoidable and fine. Then I went further on my own and rewrote Opificium to consume the crate and deleted its ~1,600 duplicated lines. That second step was correct work and still the wrong call to make unprompted.

**Why:** the user tracks effort per repo and Opificium has other contributors, so an unasked change there is a review burden they did not schedule. A standing rule like [[dry-no-code-reuse]] or "the bench and the game move together" explains why the follow-through *matters*; it is not standing permission to do it in the same breath.

**How to apply:** finish the named repo, then say plainly what the sister change would be and offer it — "Opificium still has its own copy; want me to point it at the crate?" If a change to another repo is genuinely load-bearing for the task (a shared crate the build needs), do that part, and say which part was required versus optional. When you have already gone too far, leave it on a branch/PR so `master` is untouched and it costs nothing to drop.
