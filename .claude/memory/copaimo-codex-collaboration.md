---
name: copaimo-codex-collaboration
description: "With Codex on Copaimo — I can TASK it with research and review (it never edits the game); I know the game better, so push back when a suggestion goes against the direction"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-31T13:53:55.311Z
---

Codex reviews Copaimo and files findings; I implement. **I can also give Codex work**
— the user said so directly on 2026-08-31: *"Codex is here to support you so if you
need it to do something tell it. The only thing it can't do is make changes to the
game, only you can."* Ask by writing the request into
`codex-suggestions/CLAUDE_REPLY.md`; it reads the repo and answers there. Naming what
I want specifically (a decision narrowed to this renderer, a read of these meshes,
an evidence matrix as a table I can drive) gets a far better answer than a topic.

The other half of the instruction: collaborate efficiently, but **that does not mean
everything needs to be worked**. I know the game better than Codex does, so when a
suggestion goes against the direction, say so and tell it rather than building it.

**Why:** Codex reads code against its own comments better than I read my own, and
that is where its value is — on 2026-08-31 it found a dead sky branch, a planarity
rule asked of the wrong quantity, an MSAA reduction that would crawl, and a constant
that was not controlling what its name said, all in a pass whose screenshots looked
right. But it does not hold the design intent, so its list is not a work queue.

**How to apply:** every P0/P1 and every direct user request gets a disposition in
`CLAUDE_REPLY.md` — `accepted`, `adapted`, `deferred`, `rejected`, `needs review`,
`closed` — with a reason; silence is the only thing the protocol forbids. Run Codex
in PARALLEL: file the request, then keep implementing rather than waiting. Still
verify each finding against the code before acting, and say when one is right in
principle but dead in practice — that distinction is worth more to it than a bare
"accepted". See [[copaimo-codex-suggestions]] and
[[ask-the-artefact-not-the-arithmetic]].
