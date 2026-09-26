---
name: crashout-balance-lever
description: "For CRASHOUT, ease difficulty at the SOURCE (event/task inflow) — never by slowing the meter rise or buffing relief"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

For CRASHOUT (`C:\Users\jsull\Desktop\crashout\index.html`), when the CRASHOUT meter / difficulty feels too punishing ("the run is essentially over once the meter is high"), fix it at the **source — reduce how often events/tasks fire** (event recurrence gap, the multi-event burst, crunch cooldown, the task cap). Do **NOT** slow the meter's rise (a mid-band creep "ease"/`easeBand`) and do **NOT** buff break-room relief (a high-meter heal bonus). Both were tried and explicitly rejected.

**Why:** The rise must stay **honest** at every level — a high meter should be genuinely dangerous, made recoverable by controlling the inflow, not by artificial level-dependent assists. The user corrected this twice: first "I wasn't saying slow down the meter at 70%, just that it should still be recoverable," then "don't make it easier by scaling break room relief either — the problem is the crazy amount of events that fire rapidly."

**How to apply:** Tune the office-event cadence (`gap = clamp(...)` in the scheduler, the `burst` probabilities in `officeEvent`), `crunchCool`, and `TASK_CAP`. Leave the creep formula and the flat per-station relief amounts alone. Relates to [[ensure-game-balance]] and [[crashout-design-log]].
