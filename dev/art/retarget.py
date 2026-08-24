"""Moves a clip from the generator's own rig onto the prepared one.

Imported by `animate_ranger.py`, and runnable on its own to look at the result:

    dev/art/see_the_retarget.sh                       # walk, in a Blender window
    dev/art/see_the_retarget.sh --clip run

# Why a clip cannot simply be copied

The deliveries in `assets/models/Ranger-*.glb` carry the generator's preset clips, authored
against the generator's own rest pose. `prepare_rig` has moved ours a long way from that: the two
sides were 5.45 cm from mirrored and were mirrored, the legs were bent 17.5 degrees at rest and
were straightened, the arms were put in an A-pose, and the whole pose was baked as the new rest.

A pose bone's rotation is stored RELATIVE TO ITS REST. So the same quaternion means a different
thing on the two rigs, and copying it across produces a body that is wrong by exactly the
difference between the binds - which on this rig is a crouch and a twist. `animate_ranger` has
carried a note saying so since before the preset clips existed: "a clip cannot be copied across a
bind change, only retargeted", and "the honest way is a WORLD-SPACE" one.

So this reads where each bone POINTS IN THE WORLD on the source, and turns the matching bone on
the target until it points there too. What a bone's rest pose happens to be then drops out
entirely, which is the whole idea.

# What is carried, and what is not

ROTATION for every bone the two rigs share by name. TRANSLATION only for the bones that carry the
body through space, because on every other bone a translation is a bone sliding out of its socket
rather than a motion - and the generator's clips do key some of them.

Bones the target has and the source does not - the thirty finger bones - are left alone, at rest.
That is correct rather than a compromise: the preset has nothing to say about fingers, and
`animate_ranger` puts the relaxed curl on them afterwards.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

SCALE = 170.0

# The bones whose translation is real motion rather than a joint coming apart. `animate_ranger`
# keys exactly these, and for the same reason.
CARRIES_THE_BODY = ("Root", "Hip")

# How far a retargeted bone may end up from where the source had it, in degrees, before the
# result is refused. Not zero: the two rigs' rest poses differ, and a joint at the end of a chain
# accumulates its parents' small differences. Anything past this is not accumulation, it is a
# bone that did not track.
TRACKS_WITHIN = 0.75


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def in_hierarchy_order(rig, names):
    """Parents before children.

    Setting `pose_bone.matrix` asks Blender to work out the local rotation that puts the bone
    where you said, and it does that against the parent's CURRENT pose. Do a child first and it
    is solved against a parent that has not moved yet, then the parent moves and takes the child
    with it. The order is not a detail; out of order the whole limb is wrong.
    """
    depth = {}

    def deep(bone):
        if bone.name not in depth:
            depth[bone.name] = 0 if bone.parent is None else deep(bone.parent) + 1
        return depth[bone.name]

    for bone in rig.data.bones:
        deep(bone)
    return sorted(names, key=lambda name: depth.get(name, 0))


# Bones the two rigs call different things, source name to target name.
#
# `dev/art/add_spine.py` put a joint into the middle of the back and renamed the bone above it,
# because that bone carries both clavicles and the neck and is therefore a chest. The delivery
# still calls it Spine02. Matching purely by name would then hand the delivery's CHEST motion to
# our MID-BACK - a plausible-looking clip with the torso bending in the wrong place.
#
# The new mid-back has no counterpart in the delivery and stays at rest, which is fine: the
# clavicles are set from the source's own world orientation, so they land where the source had
# them whatever happens between.
CALLED_SOMETHING_ELSE = {"Spine02": "Chest"}


def shared_bones(source, target):
    both = []
    for bone in source.data.bones:
        name = CALLED_SOMETHING_ELSE.get(bone.name, bone.name)
        if name in target.data.bones:
            both.append((bone.name, name))
    if not both:
        refuse("the two rigs share no bone names at all, so nothing can be matched")
    order = {name: n for n, name in enumerate(in_hierarchy_order(target, [t for _, t in both]))}
    both.sort(key=lambda pair: order[pair[1]])
    return both


def one_cycle_of(source, first, last, talk=True):
    """The frame range of a single gait cycle: one left contact to the next.

    # Why this is needed at all

    The delivered presets are not one cycle. Counting foot plants: the walk has three left
    contacts and two right over 57 frames, the run two and three over 31 - about two and a half
    cycles each. Everything phase-based then fails, and it fails confusingly rather than loudly:
    `verify_gait` compares each frame with the one half a CLIP later, which on two and a half
    cycles is the wrong leg, and it reported the walk limping by 36.84 cm.

    Two and a half cycles also cannot loop. The first and last frames are at different phases, so
    every repetition crosses a step change - measured, 11.88 on the run before the root motion
    came off and 0.03 after.

    A cycle is contact to the NEXT contact of the SAME foot, so that is what is cut.
    """
    lowest = []
    for frame in range(first, last + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        lowest.append(
            (source.matrix_world @ source.pose.bones["L_ToeBase"].tail).z
        )
    floor = min(lowest) + (max(lowest) - min(lowest)) * 0.15
    down, was = [], False
    for n, z in enumerate(lowest):
        now = z <= floor
        if now and not was:
            down.append(first + n)
        was = now
    if len(down) < 2:
        refuse(
            f"only {len(down)} left-foot contact(s) found in frames {first}..{last}, so there is "
            f"no cycle to cut - the clip may not be a walk or a run at all"
        )
    # The LONGEST gap between successive contacts, because a preset often begins or ends
    # mid-stride and the short end pieces are not cycles.
    gaps = [(down[i + 1] - down[i], down[i], down[i + 1]) for i in range(len(down) - 1)]
    span, from_frame, to_frame = max(gaps)
    if talk:
        print(f"    left contacts at {down}; taking {from_frame}..{to_frame} "
              f"({span} frames) as one cycle")
    return from_frame, to_frame


def retarget(source, target, called, talk=True):
    """Bakes the source's current action onto the target as a new action of the same name."""
    if source.animation_data is None or source.animation_data.action is None:
        refuse(f"the source rig has no action to take {called} from")
    clip = source.animation_data.action
    whole_first, whole_last = (int(round(v)) for v in clip.frame_range)
    first, last = one_cycle_of(source, whole_first, whole_last, talk)
    pairs = shared_bones(source, target)
    landed = {t for _, t in pairs}
    missing = [b.name for b in target.data.bones if b.name not in landed]

    if target.animation_data is None:
        target.animation_data_create()
    made = bpy.data.actions.new(called)
    target.animation_data.action = made

    # Where each carrying bone rests, so its translation can be carried as a DIFFERENCE. The two
    # rigs' rests sit in different places - `prepare_rig` centred the skeleton and put the soles
    # on the floor - so copying an absolute position would move the body by the gap between them.
    source_rest = {n: source.data.bones[n].matrix_local.translation.copy()
                   for n in CARRIES_THE_BODY if n in source.data.bones}
    target_rest = {n: target.data.bones[n].matrix_local.translation.copy()
                   for n in CARRIES_THE_BODY if n in target.data.bones}

    # # The travel comes off first
    #
    # These presets carry ROOT MOTION: the body walks forward through the clip, so its last frame
    # is metres from its first. This game moves the character in CODE and plays the clip in place -
    # see docs/animation.md on root motion against in-place - so a clip that also travels is
    # counted twice and never loops. Measured on the delivered run, the first and last frames were
    # 11.88 apart, and verify_gait refused it for exactly that.
    #
    # Removed as a straight-line TREND rather than by zeroing the horizontal outright, because the
    # sway is real and worth keeping: a walking body shifts side to side over its stance foot, and
    # deleting that with the travel would flatten the clip into something that slides.
    travel = {}
    for name in CARRIES_THE_BODY:
        if name not in source.pose.bones or name not in target.pose.bones:
            continue
        held = []
        for frame in range(first, last + 1):
            bpy.context.scene.frame_set(frame)
            bpy.context.view_layer.update()
            held.append(source.pose.bones[name].matrix.translation - source_rest[name])
        # A line through the first and last, so the ends meet and the loop closes.
        drift = held[-1] - held[0]
        span = max(1, len(held) - 1)
        travel[name] = [
            mathutils.Vector((
                one.x - drift.x * (n / span),
                one.y - drift.y * (n / span),
                one.z,
            ))
            for n, one in enumerate(held)
        ]
        print(f"    {name} travelled {drift.length * SCALE:.1f} cm over the clip; "
              f"taken out, its vertical bob kept")

    worst, worst_at = 0.0, ""
    for frame in range(first, last + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()

        # Read the SOURCE first, all of it, before touching the target. Reading and writing in
        # the same pass would have the target's own updates racing the source's evaluation.
        wanted = {}
        for from_name, to_name in pairs:
            posed = source.pose.bones[from_name]
            wanted[to_name] = (source.matrix_world @ posed.matrix).to_3x3().normalized()
        moved = {name: held[frame - first] for name, held in travel.items()}

        for _, name in pairs:
            posed = target.pose.bones[name]
            posed.rotation_mode = "QUATERNION"
            held = posed.matrix.copy()
            where = held.translation.copy()
            if name in moved:
                where = target_rest[name] + moved[name]
            posed.matrix = (
                mathutils.Matrix.Translation(where) @ wanted[name].to_4x4()
            )
            bpy.context.view_layer.update()

        # Check it tracked, on this frame, before writing the key. A retarget that quietly does
        # not track is the failure worth catching: it looks like an animation, just the wrong one.
        for _, name in pairs:
            got = (target.matrix_world @ target.pose.bones[name].matrix).to_3x3().normalized()
            off = math.degrees(
                got.to_quaternion().rotation_difference(wanted[name].to_quaternion()).angle
            )
            if off > worst:
                worst, worst_at = off, f"{name} on frame {frame}"

        for _, name in pairs:
            posed = target.pose.bones[name]
            at = frame - first + 1
            posed.keyframe_insert("rotation_quaternion", frame=at)
            if name in moved:
                posed.keyframe_insert("location", frame=at)

    if talk:
        print(f"  {called}: {last - first + 1} frames, {len(pairs)} bones matched, "
              f"{len(missing)} left at rest")
        print(f"    worst tracking error {worst:.3f} deg ({worst_at})")
    if worst > TRACKS_WITHIN:
        refuse(
            f"{worst_at} ended up {worst:.2f} degrees from where the source had it, past "
            f"{TRACKS_WITHIN}. The pose is not being reproduced, so the clip would be a "
            f"different animation wearing the same name."
        )
    return made


def main():
    import prepare_rig

    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    def flag(name, fallback=None):
        return args[args.index(name) + 1] if name in args else fallback

    root = os.path.dirname(os.path.dirname(ART))
    which = flag("--clip", "walk")
    delivery = flag("--from", os.path.join(
        root, "assets", "models", f"Ranger-{which.capitalize()}.glb"))
    prepared = flag("--onto", os.path.join(ART, "ranger_apose.glb"))
    save_to = flag("--save")

    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)

    bpy.ops.import_scene.gltf(filepath=prepared.replace("\\", "/"))
    target = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    target.name = "prepared"
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(target, mesh)
    prepare_rig.drop_the_widgets(target)

    before = {o for o in bpy.data.objects}
    bpy.ops.import_scene.gltf(filepath=delivery.replace("\\", "/"))
    fresh = [o for o in bpy.data.objects if o not in before]
    source = next(o for o in fresh if o.type == "ARMATURE")
    source.name = "delivered"
    print(f"retargeting {os.path.basename(delivery)} onto {os.path.basename(prepared)}")
    print(f"  source {len(source.data.bones)} bones, target {len(target.data.bones)}")

    made = retarget(source, target, which)
    made.use_fake_user = True

    # The source out of the way, so the window shows the character that matters.
    for thing in fresh:
        thing.hide_viewport = True
        thing.hide_render = True

    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = (int(round(v)) for v in made.frame_range)
    scene.render.fps = 24
    scene.frame_set(scene.frame_start)

    if save_to:
        bpy.ops.wm.save_as_mainfile(filepath=save_to)
        print(f"saved {save_to}")


if __name__ == "__main__":
    main()
