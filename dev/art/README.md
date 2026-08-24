# dev/art

Scripts that build the game's art. Each one writes a `.blend` and/or a `.glb`, and none of them
is run by the game — assets are built here and committed.

## The player character is not here

On **2026-08-24** the ranger's mesh, rig, clips, texture and its whole asset pipeline were
deleted, to be rebuilt from new source files. That means these are gone:

    Ranger_Rig_Idle.glb, Ranger-Walk.glb, Ranger-Run.glb, person_ranger.glb,
    ranger_apose.glb, ranger_basecolor.png
    prepare_rig, animate_ranger, verify_gait, retarget, ik_gait, foot_roll,
    add_finger_bones, add_spine, find_the_fingers, unfuse, ranger_texture,
    slim_the_shoes, shoe_form, sneaker_mesh, sneaker_paint, and their viewers

All of it is in git at `ed006b9`, the commit before the deletion — `git show
ed006b9:dev/art/prepare_rig.py` and so on. **Read it as history, not as a starting point.** Its
constants are measurements of a mesh that no longer exists: leaf-bone lengths, a 17.5 degree
crouch correction, strap identities by vertex count, eye boxes by pixel coordinate, shoe
proportions. Applied to a different mesh they are corrections for somebody else's body, and
that is precisely the failure the whole log warns about.

What carries over is in `TROUBLESHOOTING.md` under *"The ranger was replaced"*, and the sourced
industry references are in `docs/`. Read both before writing the first line of the new pipeline.

## What is still here, and what it builds

    props.py    trees.py    ranch.py    cover.py     the world's objects
    people.py   warden.py                            the warden and its parts
    compare_skin.py  ribbon_measure.py               generic diff and strain measurement
    blender.sh                                       finding Blender; sourced, not run
    build.sh                                         builds everything above

`compare_skin.py` and `ribbon_measure.py` take two builds of the same character and diff them.
They are mesh-agnostic and will work on whatever comes next.

## What the new character needs, in the order it will be needed

1. **A rest pose worth binding to**, measured rather than assumed — the two sides mirrored, the
   knees eased, leaf bones given their real lengths.
2. **Mesh work done ONCE and committed.** The single most expensive lesson from the last
   character: re-deriving mesh repairs on every build asks a classifier to make the same
   judgement call forever and never once get it wrong. It cut sleeve cuffs, holed a trouser leg
   and took part of a shoulder. Rig repair is derived per build; sculpting is done once, checked
   by eye, and kept in the asset.
3. **Clips measured off the file**, never described — `covers`, the frame counts, the durations.
4. **A verifier that refuses**, and that cannot be quietly satisfied. Backwards knees shipped
   three times before one existed.
5. **A clay viewer before a textured one.** Paint hides the shape it is painted on.
