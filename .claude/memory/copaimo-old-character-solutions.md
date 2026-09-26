---
name: copaimo-old-character-solutions
description: The deleted character's animation solutions live in git history — search it BEFORE re-deriving anything about gait, arms, lean or feet
metadata:
  type: reference
---

Copaimo's previous character (deleted 2026-08-24, along with `dev/art/animate_ranger.py` and ~52
other scripts) had **most of the gait problems already solved**, with the reasoning in the commit
messages. That work is still in git history and is the first place to look:

```bash
git log --all --oneline -i --grep="pump\|arm swing\|lean\|foot"
LAST=$(git log --all --format=%H -- dev/art/animate_ranger.py | head -1)
git show "$LAST^:dev/art/animate_ranger.py" | grep -n "PUMPS\|ARM_FORWARD\|TUCK_IN"
```

Things already solved there, with numbers: the arm PUMP (a plain cosine cannot pump — an
odd-symmetric power under 1.0 makes the arm dwell at the extremes and snap between them,
`SPRINT_PUMPS = 0.55`); forearms reading "outward" instead of in front (cause: the elbow folding
about a FIXED axis, fix: shoulder internal rotation, `RUN_TUCK_IN = 12`); trunk lean bands (4–12°
real, 15–30 is a 2–4× push); the neck counter-lean so the head stays over the shoulders; wrist
LOCKED not dragged when running.

**Why:** the user, after I re-derived the arm pump from scratch — *"This was a solved problem we
had so its crazy we're going through it again."* They were right.

**How to apply:** on any Copaimo animation question, search git history and TROUBLESHOOTING.md
before deriving. See [[research-proven-solutions-first]] and [[copaimo-reference-library]] — this
is the same rule, and the repo's own history is part of the library.
