---
name: crashout-chair-leave-original
description: "CRASHOUT's furniture office-chair base should stay the ORIGINAL flat-plank style — don't redesign it"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

For CRASHOUT (`C:\Users\jsull\Desktop\crashout\index.html`), the **furniture office chair** drawn by `drawChair` / `drawSeated` should keep its **original flat base** — do NOT redesign it.

**Why:** Three redesigns of the base were all rejected in a row — a 5-star caster splay (read as scattered/disconnected wheels), a pedestal column (read as a barber chair), and a connected 5-star (still not wanted). The user's final call: "just use the original style they were fine." The original was recovered from the prior session transcript and restored.

**The original base (both `drawChair` and `drawSeated`):**
`px(cx-6,cy-1,12,2,'#2a2e34')` (flat bar) + `px(cx-6,cy,2,2,'#222')` & `px(cx+4,cy,2,2,'#222')` (two castors) + `px(cx-1,cy-6,2,5,'#3a3f47')` (gas post) + `px(cx-5,cy-9,10,3,'#2f3f52')` (seat).

**How to apply:** Leave the furniture chair base as-is unless the user explicitly asks to change it again. (Distinct from the **Office Chair WEAPON** `drawWeaponShape('chair')`, which WAS redesigned from a "tuning fork with wheels" into a side-view chair — that one was wanted.) Relates to [[crashout-design-log]] and [[crashout-text-style]].
