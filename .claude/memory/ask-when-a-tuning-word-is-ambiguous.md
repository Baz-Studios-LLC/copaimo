---
name: ask-when-a-tuning-word-is-ambiguous
description: "On vague visual feedback (\"too thin\", \"too busy\"), ask which thing is meant instead of picking a reading and building on it"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-17T13:37:30.260Z
---

When visual feedback could point at two different things, **ask which** before
building. Do not pick the likelier reading, do the work, and mention the
ambiguity afterwards.

Concrete case: "trees are still a bit too thin" — I read it as foliage density,
spent a pass on leaf clusters and crown height, and noted at the end that I'd
left trunk girth alone. They meant the trunks. The note did not save the pass.

**Why:** a wrong guess spends a whole cycle and the user then has to correct me,
which costs them more than the question would have. Flagging the ambiguity *after*
acting is not the same as resolving it — it reads as covering myself.

**How to apply:** if a word like "thin", "busy", "flat", "heavy" or "too much"
maps onto more than one parameter I can see in the code, name the candidates and
ask which — one short question, then build. If the answer is genuinely
low-stakes and both readings are cheap, do both rather than guessing one.

Related: [[neon-edge-small-steps]], [[favor-emergent-skill-discovery]].
