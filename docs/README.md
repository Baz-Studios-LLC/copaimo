# Reference library

Notes on how games get made — rigging, animation, Blender, pipelines, design, and how to work
with an AI on it. Written to be **looked things up in**, not read through.

Started 2026-08-23, after a stretch where the same class of mistake kept costing days: the
character's hands, the run cycle's speed, and a mesh that kept losing pieces to its own build
script. Most of what went wrong was not hard. It was **already solved**, by people who wrote it
down, and we were deriving it from first principles instead.

## The files

| File | What is in it |
|---|---|
| [rigging.md](rigging.md) | Skeletons, bone budgets, naming, skinning, hands and thumbs, bind poses, weights |
| [animation.md](animation.md) | The 12 principles in a game context, locomotion, foot sliding, IK, blending, game feel |
| [blender.md](blender.md) | glTF export rules, `bpy` traps, headless pipelines, auto-riggers |
| [pipeline.md](pipeline.md) | How studios organise asset work, validation, what to commit vs derive |
| [design.md](design.md) | Creature-collection loops, open-world scale, companion AI, feel |
| [working-with-ai.md](working-with-ai.md) | Where an AI is weak on this work, with numbers, and the protocol that fixes it |

## How these are written

Three kinds of statement, and they are kept apart on purpose, because mixing them is how a
guess ends up being treated as a measurement:

- **STANDARD** — what the industry does, with a source. Follow it unless there is a reason not
  to, and write the reason down.
- **MEASURED** — a number from this project, in Blender or in the game. Trustworthy, and
  re-measurable.
- **OPEN** — believed but not established. Flagged so nobody builds on it by accident.

Where something bears directly on Copaimo there is a **→ For Copaimo** line. Those are the ones
worth acting on.

## Related docs at the root

- `DESIGN.md` — what the game is, and the decisions taken
- `TROUBLESHOOTING.md` — the fix log. Bugs hit, and what actually solved them. Read it before
  writing a build step; it has caught a repeat already
- `HANDOFF.md` — current state and what is next

These are the project's record. The library here is the *outside world's* record, which is a
different thing and shorter than it should have been.

## The plan this feeds

`character-pipeline.md` is the ordered work: eleven stages from the character as delivered to
one that ships, each grounded in a measurement of the asset that is actually in the repo rather
than in a general principle. The rest of this folder is the reference it draws on.
