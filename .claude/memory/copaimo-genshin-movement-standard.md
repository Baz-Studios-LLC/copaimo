---
name: copaimo-genshin-movement-standard
description: "Copaimo movement is judged against Genshin Impact, and realism is explicitly NOT a constraint — never let a realism band gate a tuning value"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-23T15:25:05.319Z
---

For Copaimo locomotion the user's stated standard is **Genshin Impact** — "Genshin Impact
is the standard for movement and fluidity" — and realism is explicitly off the table:
"Lets drop the realism entirely. This is a fantasy world so I do not care about realism I
just want the movements to feel quick and fluid."

**Why:** I spent several passes tuning gait against human biomechanical bands (cadence
90-140/150-200/220-260 spm, pelvic drop degrees, clinical injury literature). The user
pushed back three separate times that this is a fantasy game about collecting and raising
monsters. Worse, those bands were wired in as *guards*, so the driven speeds were pinned
just under a human cadence ceiling — the speed was never a design choice, it was an output
of a realism gate, and every attempt to raise it got refused by a test.

**How to apply:** Speeds and amplitudes are knobs chosen by feel; realism numbers may
inform a starting point but must never gate one. Keep sanity bounds only wide enough to
catch a genuinely broken measurement (a mis-measured stride showing as 400 spm), never
tight enough to have an opinion about a chosen value. When a tuning value cannot be raised
without a guard refusing it, that value is an output — invert the dependency before tuning
again. Exaggerate past life for the read: see [[ranger-stylised-not-realistic]] and
[[research-proven-solutions-first]], and the fix log in [[copaimo-troubleshooting-log]].

Also from this: **choose from intent, scale by measurement** — Genshin picks the clip from
a discrete movement state and only uses velocity to scale it. Selecting a gait from a
measured velocity made the character jitter.
