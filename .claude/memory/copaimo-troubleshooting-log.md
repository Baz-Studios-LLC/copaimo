---
name: copaimo-troubleshooting-log
description: "Copaimo keeps TROUBLESHOOTING.md as the durable fix log — update it as ISSUE + SOLUTION pairs after every fix, and read it BEFORE writing a step, not just after hitting a bug"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-23T02:02:20.952Z
---

The user asked "are you not keeping a log of things we're working on? we need to make sure all the troubleshooting we did for the walk is applied to the jog so we dont repeat problems" (2026-08-22), then corrected my format guess with "no no no issue + solution".

**Why:** `TROUBLESHOOTING.md` at the repo root already existed and was actively maintained, and I had stopped feeding it while carrying the history in context instead. It cost real work: the log already documented that `read_homefile(use_empty=True)` breaks the glTF importer, and I wrote that exact line into a new viewer and shipped an empty Blender window. It also already named `ranger_blend.sh` as the scene-keeping tool, so not reading it meant building a duplicate. The log was right both times; I hadn't read it.

**How to apply:**
- Format is **ISSUE + SOLUTION pairs**, not a narrative of what we did. Terse table rows, phrased in the words the fault was reported in.
- Update it after any fix that took more than one attempt, or where symptom and cause were far apart — while the fix is fresh, not at the end of a session.
- Read it before writing a step, not only when stuck. Check "am I about to write something this file warns about".
- The file's own rule: every symbol it names must be verified to still exist when written. Grep them.
- Keep the tuning-vs-structural split visible — on the walk it was ~1/3 tuning and ~2/3 structural, and tuning against a structural fault is what burned the most time every time. See [[validate-the-ruler-first]] and [[copaimo-rig-pipeline]].
- Don't log an unsolved thing as solved: the sculpting items (jacket hem, the ~4 cm shoe asymmetry) are the user's and were deliberately left out.
