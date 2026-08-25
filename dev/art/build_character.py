"""Puts the three delivered clips onto one character and writes the game's asset.

    blender --background --python build_character.py

Reads `assets/character/*.glb` and writes `assets/models/person_ranger.glb` with the mesh, the
skeleton and three clips named `idle`, `walk` and `run`.

# Why there is no retargeting here

Measured off the files themselves, all three carry the SAME mesh - `tripo_node_eafb5436`, 7844
vertices, 4899 triangles - and the SAME 41-joint skeleton in the same order. A clip cannot be
copied across a bind change, so the first thing this does is prove there is no bind change:
`the_skeletons_match` compares joint names, parents AND rest transforms, and refuses if any of
them differ. If a later delivery breaks that, this stops rather than quietly producing a
character whose arms are in the wrong place.

# What is measured rather than described

The clips are authored at different frame rates - walk's first key lands at 1/24 s and run's at
1/30 - so a frame count is not a shared unit and nothing here uses one. Durations come from the
clip's own range, which is what the animation player will use.

How far the body travels in one cycle is the single most consequential number in movement,
because playback rate is `lasts * speed / covers`. It is measured here and printed. A value
belonging to a different animation is exactly what running through water looks like.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(ART))
SOURCE = os.path.join(ROOT, "assets", "character")
OUT = os.path.join(ROOT, "assets", "models", "person_ranger.glb")

# The delivered file, and what the game calls the clip in it. `lookAround` becomes the idle.
DELIVERED = (
    ("idle.glb", "idle"),
    ("lookAround.glb", "look_around"),
    ("walk.glb", "walk"),
    ("run.glb", "run"),
)

# Which clips are supposed to carry the character somewhere. Everything else is a standing
# motion, and a standing motion with no travel is correct rather than broken - the refusal below
# is there to catch a gait whose channels never bound, which is what an unbound action slot
# looks like from the outside.
TRAVELS = ("walk", "run")

# How far two rest transforms may differ before the skeletons are called different. Tight: this
# asks whether two exports of the same rig agree, not whether two rigs are similar.
RESTS_MATCH_WITHIN = 1e-5

# How far to roll each hand inward, in degrees, and which way that is per side.
#
# The delivered character stands SUPINATED - palms facing out, which no relaxed human does. It is
# in the bind, so every clip inherits it and no clip corrects it: the audit measures bind pose
# and idle frame 1 as identical.
#
# Corrected in the CLIPS rather than in the bind. A bind change invalidates every clip authored
# against it, and these were authored against this one; rolling the hand on each key preserves
# whatever the clip does with the arm and only changes where the hand rests while it does it.
#
# Rolled about the bone's own Y, which is along its length - that is the axis a forearm pronates
# about. Opposite signs per side because pronation is a mirror.
PALMS_ROLL_IN = 90.0
ROLLS = {"L": 1.0, "R": -1.0}

# How the roll is SHARED along the forearm, and why it has to be shared at all.
#
# Rolling only the hand puts the whole ninety degrees into one joint, and the wrist shreds into
# shards - visible in a clay render long before any number complains. The twist bones exist for
# exactly this, but the hierarchy here is not the obvious one:
#
#     L_Forearm -> L_ForearmTwist01 -> L_ForearmTwist02
#     L_Forearm -> L_Hand
#
# The hand is a SIBLING of the twists, not their child, so rolling the twists does not move it
# and rolling it does not twist the forearm. Both are needed.
#
# The shares are cumulative down the chain: a third at Twist01, a third more at Twist02 - which
# rides on Twist01, so it reaches two thirds - and the full amount on the hand, which hangs off
# the forearm and therefore carries no inherited roll. That ramps the skin from nothing at the
# elbow to everything at the wrist, which is what a forearm does.
SHARED_ALONG = (("ForearmTwist01", 1.0 / 3.0), ("ForearmTwist02", 1.0 / 3.0), ("Hand", 1.0))


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def rig_of(objects):
    return next((o for o in objects if o.type == "ARMATURE"), None)


def skeleton_of(rig):
    """Name, parent and rest matrix for every bone, in order - what a clip is authored against."""
    return [
        (bone.name,
         bone.parent.name if bone.parent else None,
         tuple(round(v, 6) for row in bone.matrix_local for v in row))
        for bone in rig.data.bones
    ]


def the_skeletons_match(first, other, called):
    """Refuses unless two rigs are the same skeleton, so clips can simply be moved across."""
    if len(first) != len(other):
        refuse(f"{called} has {len(other)} bones against {len(first)} - not the same skeleton, "
               f"so its clip cannot be copied over without retargeting")
    for mine, theirs in zip(first, other):
        if mine[0] != theirs[0] or mine[1] != theirs[1]:
            refuse(f"{called} has bone {theirs[0]} under {theirs[1]} where the base has "
                   f"{mine[0]} under {mine[1]} - the skeletons differ")
        off = max(abs(x - y) for x, y in zip(mine[2], theirs[2]))
        if off > RESTS_MATCH_WITHIN:
            refuse(f"{called} rests bone {theirs[0]} {off:.6f} away from the base - a clip "
                   f"authored against one bind does not mean the same thing on another")


def play(rig, clip):
    """Assigns a clip so it actually drives the rig.

    Assigning `animation_data.action` alone is not enough from Blender 4.4 on: an action holds
    SLOTS, and until one is bound the action is attached and inert. It reports success and moves
    nothing, which is how this first measured every clip as travelling 0.0 cm - a walk whose feet
    never left the ground, and a number that would have gone straight into `covers`.
    """
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = clip
    slots = getattr(clip, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]
    elif not hasattr(clip, "slots"):
        pass  # older Blender: the action drives the rig on its own


def fcurves_of(clip, slot):
    """Every fcurve in a clip, on Blender 5 and on what came before.

    From 4.4 an action is slots, layers, strips and channelbags rather than a flat
    `action.fcurves`, and reaching for the old attribute finds nothing and raises nothing.
    """
    if hasattr(clip, "fcurves") and len(clip.fcurves):
        return list(clip.fcurves)
    out = []
    for layer in getattr(clip, "layers", []):
        for strip in layer.strips:
            bag = strip.channelbag(slot) if slot else None
            if bag is None and getattr(strip, "channelbags", None):
                bag = strip.channelbags[0]
            if bag is not None:
                out.extend(bag.fcurves)
    return out


def stand_still(rig, clip, scene):
    """Takes the travel out of a clip and leaves the sway in. Returns how far it removed.

    These clips carry ROOT MOTION - the walk moves its root 1.50 units over the clip and the run
    2.81. The game moves the warden in code, so a clip that also translates him would move him
    twice, and the classic symptom is a character skating away from under himself.

    Detrended, not zeroed: a straight line from the first key to the last is subtracted, so the
    travel goes and the side-to-side sway and the bob a real gait has are kept. Zeroing the
    channel outright would take those with it and the walk would go rigid.

    What is subtracted is measured and returned, because it IS `covers` - the distance the clip
    carries him - and that is the number playback rate divides by.
    """
    play(rig, clip)
    slot = rig.animation_data.action_slot if rig.animation_data else None
    first, last = (int(round(v)) for v in clip.frame_range)
    curves = [c for c in fcurves_of(clip, slot)
              if c.data_path.endswith(".location") or c.data_path == "location"]
    if not curves:
        return 0.0, None

    # Whichever channel actually carries the travel, rather than an assumption about which bone
    # or which axis is forward.
    worst, moved = None, 0.0
    for curve in curves:
        keys = [k.co[1] for k in curve.keyframe_points]
        if not keys:
            continue
        drift = abs(keys[-1] - keys[0])
        if drift > moved:
            worst, moved = curve, drift
    if worst is None or moved < 1e-4:
        return 0.0, None

    who = worst.data_path.split('"')[1] if '"' in worst.data_path else "object"
    took = 0.0
    for curve in curves:
        if curve.data_path != worst.data_path:
            continue
        keys = curve.keyframe_points
        if len(keys) < 2:
            continue
        began, ended = keys[0].co[0], keys[-1].co[0]
        low, high = keys[0].co[1], keys[-1].co[1]
        span = max(ended - began, 1e-9)
        took += (high - low) ** 2
        for key in keys:
            slide = low + (high - low) * (key.co[0] - began) / span
            key.co[1] -= slide - low
            key.handle_left[1] -= slide - low
            key.handle_right[1] -= slide - low
        curve.update()
    return took ** 0.5, who


def roll_the_hands(rig, clip, degrees):
    """Rolls each hand inward by a constant on every key, so the palms rest on the thighs.

    Composed onto the keyed rotation rather than replacing it: `keyed * offset` in the bone's
    own space, which leaves the clip's motion exactly as authored and moves only the frame it
    happens in.
    """
    if abs(degrees) < 1e-6:
        return 0
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    turned = 0
    for side, way in ROLLS.items():
        for bone, share in SHARED_ALONG:
            path = f'pose.bones["{side}_{bone}"].rotation_quaternion'
            parts = {c.array_index: c for c in curves if c.data_path == path}
            if len(parts) != 4:
                continue
            offset = mathutils.Quaternion((0.0, 1.0, 0.0),
                                          math.radians(degrees * way * share))
            for at in range(len(parts[0].keyframe_points)):
                keyed = mathutils.Quaternion(
                    [parts[i].keyframe_points[at].co[1] for i in range(4)])
                rolled = keyed @ offset
                for i in range(4):
                    point = parts[i].keyframe_points[at]
                    was = point.co[1]
                    point.co[1] = rolled[i]
                    point.handle_left[1] += rolled[i] - was
                    point.handle_right[1] += rolled[i] - was
            for curve in parts.values():
                curve.update()
            turned += 1
    return turned


def travels(rig, clip, scene):
    """How far the body moves through one cycle, hips and feet separately.

    Two numbers, because they answer different questions. The HIPS moving is root motion, which
    a game either uses or strips. The planted FOOT sliding is how far the character covers when
    the clip is played in place, and that is what playback rate needs.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    scene.frame_set(first)
    bpy.context.view_layer.update()

    def at(name):
        return (rig.matrix_world @ rig.pose.bones[name].head).copy()

    began = {n: at(n) for n in ("Hip", "L_Foot", "R_Foot")}
    hips, feet = 0.0, {"L_Foot": 0.0, "R_Foot": 0.0}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        hips = max(hips, (at("Hip") - began["Hip"]).length)
        for foot in feet:
            feet[foot] = max(feet[foot], (at(foot) - began[foot]).length)
    return hips, max(feet.values())


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)

    base_rig, base_mesh, skeleton = None, None, None
    wanted = {}
    for filename, called in DELIVERED:
        path = os.path.join(SOURCE, filename)
        if not os.path.exists(path):
            refuse(f"{path} is missing")
        before = set(bpy.data.objects)
        known = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=path.replace("\\", "/"))
        fresh = [o for o in bpy.data.objects if o not in before]
        rig = rig_of(fresh)
        if rig is None:
            refuse(f"{filename} has no armature")
        clips = [a for a in bpy.data.actions if a not in known]
        if len(clips) != 1:
            refuse(f"{filename} carries {len(clips)} clips, and this expects exactly one")

        if base_rig is None:
            base_rig = rig
            base_mesh = next(o for o in fresh if o.type == "MESH" and o.vertex_groups)
            skeleton = skeleton_of(rig)
            print(f"  {filename}: the base - {len(rig.data.bones)} bones, "
                  f"{len(base_mesh.data.vertices)} vertices")
        else:
            the_skeletons_match(skeleton, skeleton_of(rig), filename)
            print(f"  {filename}: same skeleton, so its clip moves across unchanged")
            for thing in fresh:
                bpy.data.objects.remove(thing, do_unlink=True)

        clips[0].name = called
        clips[0].use_fake_user = True
        wanted[called] = clips[0]
        play(base_rig, clips[0])
        rolled = roll_the_hands(base_rig, clips[0], PALMS_ROLL_IN)
        if rolled:
            print(f"    rolled {rolled} hand(s) in by {PALMS_ROLL_IN:.0f} deg")

    # Anything else the imports brought in: spare meshes, the widget the importer invents.
    for thing in list(bpy.data.objects):
        if thing not in (base_rig, base_mesh):
            print(f"  dropped {thing.name} ({thing.type})")
            bpy.data.objects.remove(thing, do_unlink=True)
    for spare in [a for a in bpy.data.actions if a not in wanted.values()]:
        bpy.data.actions.remove(spare)

    scene = bpy.context.scene
    if base_rig.animation_data is None:
        base_rig.animation_data_create()
    low = min((base_mesh.matrix_world @ v.co).z for v in base_mesh.data.vertices)
    high = max((base_mesh.matrix_world @ v.co).z for v in base_mesh.data.vertices)
    print(f"\n  a {(high - low) * 100:.1f} cm figure at scene scale")

    print("\n  clips, measured off the file:")
    for _, called in DELIVERED:
        clip = wanted[called]
        first, last = clip.frame_range
        lasts = (last - first) / scene.render.fps
        hips, foot = travels(base_rig, clip, scene)
        # Named, and printed AFTER its own summary. It read the other way round, and a clip with
        # no travel prints no line at all - so every remaining line sat above the clip it was
        # about and the whole column looked shifted by one. It was not; it was unlabelled.
        print(f"    {called:<12s} frames {first:.0f}..{last:.0f}, {lasts:.4f} s at "
              f"{scene.render.fps} fps; hips travel {hips * 100:.1f} cm, "
              f"the furthest foot {foot * 100:.1f} cm")
        covers, who = stand_still(base_rig, clip, scene)
        if covers:
            after, _ = travels(base_rig, clip, scene)
            print(f"    {called:<12s} carried {covers:.4f} units on {who}; taken out, the root "
                  f"moves {after:.4f} -> COVERS = {covers:.4f}")
        elif called in TRAVELS:
            refuse(f"the {called} clip has no travel to take out, which means either it is "
                   f"already in place or the channel carrying it was not found")
        if called in ("walk", "run") and foot < 0.05:
            refuse(f"the {called} clip moves its feet {foot * 100:.1f} cm, which is not a "
                   f"gait - either the clip is empty or it is not driving the rig")

    play(base_rig, wanted["idle"])
    scene.frame_set(int(wanted["idle"].frame_range[0]))

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=OUT, export_format="GLB", use_selection=True, export_yup=True,
        export_apply=False, export_animations=True,
        # NOT resampled. The clips are authored at different rates - the walk's keys land on 24
        # fps and the run's on 30 - and the exporter's default is to bake every action at the
        # SCENE rate. Measured, that cost the run 25 degrees of loop accuracy on its own: its
        # opening and closing poses went from 22.19 degrees apart in the delivered file to 47.13
        # in the export, purely from being resampled onto a grid its keys do not sit on.
        export_force_sampling=False,
    )
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
