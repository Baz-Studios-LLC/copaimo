"""Pulls animation clips out of sibling exports of the same character.

Imported by `animate_ranger.py`.

# Why this exists

The character came from a generator that offers preset animations, and the file in the
repository was exported with the IDLE preset selected - which is what
`Ranger_Rig_Idle.glb` is named after. Its one clip is called `preset:biped:idle`, and
that prefix is the generator's, not ours.

So the walk and the run were available at the source the whole time. A locomotion set
was authored by hand instead, which is a great deal of machinery to replace clips that
already existed and were already good. This reads them straight through.

# What has to be true for a clip to be copied

A pose bone's rotation is expressed in a basis built from the bone's REST matrix. Copy a
clip onto a rig whose rest pose differs and every quaternion in it means something else -
the clip will play, and it will play wrong, which is worse than refusing. So the rest
poses are compared bone by bone before anything is taken, and a mismatch is an error
rather than a warning.

That check is also what makes this safe against the obvious mistake: dropping in an
export of a DIFFERENT character, or one the generator re-rigged between exports.
"""

import math
import os

import bpy
import mathutils

# How far two rest matrices may differ and still be called the same rig. Generous
# enough for float32 round-tripping through glTF, tight enough that a genuinely
# different rest pose cannot slip through: 0.5 mm on a figure one unit tall.
SAME_RIG_WITHIN = 5e-4

# Which word in a clip's name says which gait it is. The generator prefixes its own
# presets, so `preset:biped:walk` has to be found by the word rather than the whole.
GAITS = ("idle", "walk", "run", "sprint", "jog")


def sibling_exports(beside: str):
    """Every other export of this character sitting next to the primary one."""
    folder = os.path.dirname(beside)
    primary = os.path.basename(beside).lower()
    found = []
    for name in sorted(os.listdir(folder)):
        if not name.lower().endswith(".glb"):
            continue
        # Anything with the character's name in it, so the export can be called
        # whatever the generator called it - Ranger_Rig_Walk, ranger-walk, Ranger (1),
        # it does not matter. Narrow enough not to sweep up an unrelated asset.
        if name.lower() == primary or "ranger" not in name.lower():
            continue
        found.append(os.path.join(folder, name))
    return found


def rest_differs(one, other) -> float:
    """The worst disagreement between two armatures' rest poses, in model units."""
    mine = {bone.name: bone.matrix_local for bone in one.data.bones}
    theirs = {bone.name: bone.matrix_local for bone in other.data.bones}
    missing = set(mine) ^ set(theirs)
    if missing:
        raise SystemExit(
            f"the rigs do not have the same bones - {sorted(missing)[:6]} differ, "
            f"{len(missing)} in all. A clip cannot be carried between them."
        )
    worst = 0.0
    for name, matrix in mine.items():
        for row in range(4):
            for col in range(4):
                worst = max(worst, abs(matrix[row][col] - theirs[name][row][col]))
    return worst


def which_gait(name: str):
    """Which gait a clip is, by the word in its name, or None."""
    lowered = name.lower()
    for gait in GAITS:
        if gait in lowered:
            return gait
    return None


def already_here(rig):
    """Gaits the PRIMARY export already carries, beyond the idle.

    The generator may put every preset in one file rather than one file per preset, in
    which case there are no siblings to look at and the clips are already loaded. Worth
    checking before going looking, because "no siblings found" would otherwise report
    nothing when everything was already there.
    """
    found = {}
    for action in bpy.data.actions:
        gait = which_gait(action.name)
        if gait is None or gait == "idle":
            continue
        action.use_fake_user = True
        found[gait] = action
        print(f"  the primary export already carries '{action.name}' as the {gait}")
    return found


def take_the_clips(rig, beside: str):
    """Imports each sibling export and moves its clips onto `rig`.

    The imported objects are removed again afterwards, so nothing but the actions
    survives - the exporter writes every object as a node, and a second copy of the
    character would ship inside the first.
    """
    taken = already_here(rig)
    for path in sibling_exports(beside):
        before = set(bpy.data.objects)
        actions_before = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=path)
        fresh = [o for o in bpy.data.objects if o not in before]
        brought = [a for a in bpy.data.actions if a not in actions_before]

        other = next((o for o in fresh if o.type == "ARMATURE"), None)
        if other is None:
            print(f"  {os.path.basename(path)}: no rig in it, skipped")
        else:
            apart = rest_differs(rig, other)
            if apart > SAME_RIG_WITHIN:
                raise SystemExit(
                    f"{os.path.basename(path)} has a rest pose {apart:.5f} away from "
                    f"the primary export, and {SAME_RIG_WITHIN} is the most that can "
                    f"be the same rig. Every quaternion in its clips would mean "
                    f"something else here. Re-export both from the same rig."
                )
            for action in brought:
                gait = which_gait(action.name)
                if gait is None or gait == "idle":
                    continue
                action.use_fake_user = True
                taken[gait] = action
                print(
                    f"  {os.path.basename(path)}: took '{action.name}' as the {gait} "
                    f"(rest poses agree to {apart:.6f})"
                )

        # Drop the imported copy, and any clip that was not wanted.
        for obj in fresh:
            if obj.name in bpy.data.objects:
                bpy.data.objects.remove(obj, do_unlink=True)
        for action in brought:
            if action not in taken.values() and action.users == 0:
                bpy.data.actions.remove(action)

    return taken


def how_long_each_is(taken):
    """Reports each clip's length and key count, since that sets its cadence."""
    for gait, action in sorted(taken.items()):
        low, high = action.frame_range
        print(
            f"  {gait}: '{action.name}' frames {low:.0f} to {high:.0f}, "
            f"{(high - low) / 24.0:.3f} s at 24 fps"
        )
