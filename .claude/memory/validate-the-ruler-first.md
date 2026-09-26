---
name: validate-the-ruler-first
description: "\"Stop guessing\" (user, 2026-08-22): before trusting any measurement, validate the instrument; a guard must compare against the SPEC, never its own input"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-22T18:22:40.787Z
---

The user demanded "stop guessing completely" after a day of instrument failures on the Copaimo rig ([[copaimo-rig-pipeline]], [[blender-is-the-tool]]).

**Why:** every wrong conclusion traced to a broken ruler, not a broken rig: a frame derived from one bone made that bone perfect by construction; "nearest bone" was meaningless with an 85 cm Hip; distance-to-vertex can't tell inside from outside; a bake was checked against its own input, so garbage in passed green; a phase float fed to a step-int parameter made every foot "planted" forever and silently ate every fix layered on top.

**How to apply:**
- A constant error is a datum error; a number that ignores its input is a stale read; a metric flagging a correct thing is a broken metric. Fix the instrument before the subject.
- Guards compare against the SPECIFICATION (soles at 0, arms at 45°), never against what was fed in.
- When two fixes in a row don't move the number at all, stop tuning — the number isn't connected to the knob (stale pycache, clamped share, wrong parameter type).
- Trace per-frame/per-item before theorising; the plateau's SHAPE names the cause.
- A guard's REGION must belong to its subject: the right shoe's flatness guard failed three rebuilds at a constant 0.86 cm because its radius reached the LEFT shoe (every offending vert was L_Foot 1.00). Constant residual across code changes = the guard measures something else.
- Segment/marker instruments need a BIND BASELINE subtracted, or sculpted shape (toe spring, rocker soles) reads as animation.
- When a user reports the SAME fault after 3+ measured fixes, stop tuning and question the STRUCTURE: five rounds of foot-pitch tuning all measured true while the pivot joint itself was in the wrong place (see [[copaimo-rig-pipeline]]). Research the anatomy/standard practice at that point instead of iterating.
- A guard's threshold must be in the SAME UNIT as its message: mine printed cm and compared model units, tolerating 3.4 cm while reporting "mirrors".
- SOME FAULTS ARE INVISIBLE TO NUMBERS. Stale normals made the shoes look destroyed while edge lengths, dimensions, weights and contact heights all read perfect. When the user says "worse" and every metric says fine, the metric class is wrong — render it large and LOOK, and isolate by re-running one pipeline step at a time on the source.
- ⭐SHADOWED NAMES IN LONG FUNCTIONS, twice in one day: `reach` (arm swing angle) rebound to a leg length silently killed all forward arm swing; `rise` (limp ratio) rebound to a hip range produced a refusal contradicting itself. Both survived because the code kept running and returned something plausible. Name locals for their SUBJECT (arm_forward / stance_reach / bob_height), not their role.
- Composing two rotations about different axes does NOT add — they couple. Authored degrees stop meaning degrees (14 came out as +2.7, range compressed to 72%). STATE the target direction and turn onto it by the shortest arc; and do it AFTER the parent chain has moved.
- A guard can report success while the thing it guards is destroyed: "the chest is now 73% spine" was true while the jacket tore into triangles. Numeric guards cannot see shading or silhouette — render the pose that exercises the change and LOOK.
- A live session a person is also clicking in is for finding numbers, never a build substrate — builds run from source via a script that re-derives everything.
