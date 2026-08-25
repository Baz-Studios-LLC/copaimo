"""Builds one Blender scene with the character and every clip in it, ready to open.

    dev/art/see_the_character.sh

Quality control, not a debugging aid: a render is one angle at one instant, and half of what is
wrong with a character is only visible when it moves. Every clip is in the Action Editor on one
body, so switching between them is a dropdown rather than five windows.

# Built headless and saved, then opened

The glTF importer dies on the context a GUI gets during startup, so the scene is assembled in
`--background`, written to a .blend, checked, and only then handed to a window. That two-step
also means the check is real: a window that opens empty with the reason on a console nobody is
reading is the failure this exists to prevent.

# Three things the importer does that have to be undone

It invents lengths for bones the file gives none for, which for `Root` and `Hip` comes out as a
huge spike drawn straight through the body with a joint ball on the end. Reported as "the long
angled bone is back", twice.

It creates an `Icosphere` and assigns it as a CUSTOM SHAPE to every bone, so the skeleton draws
as a bag of spheres instead of as bones. Reported as "no spheres, I want to see the bones".

And from Blender 4.4 an action holds SLOTS - until one is bound the action is attached and
inert, playing nothing while reporting success.
"""
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
if ART not in sys.path:
    sys.path.insert(0, ART)

import ik_gait  # noqa: E402

ART = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(ART))

# Bones the file gives no length for get one invented. A leaf bone is given this share of its
# parent's length instead, which is roughly what a hand or a toe tip is.
A_LEAF_IS = 0.45

# What a bone with nothing to measure against is drawn as, in model units - about 8 cm on a
# 170 cm figure. Enough to see, small enough not to be mistaken for a limb.
A_STUB_IS = 0.05

# Twist bones sit on top of the joints that matter and there are eighteen of them, so they are
# hidden by default. Alt-H in the viewport brings them back.
HIDE = "Twist"

# Whether to carry him forward so a treadmill clip reads as travel. See below.
CARRIES_HIM_FORWARD = True

# How far he goes and which way is MEASURED off the clip now, so there is no number here to go
# stale. There was: a copy of `src/motion.rs`'s covers, and a forward taken from the left toe's
# bind direction. Both were wrong at once - the covers by a factor of two once the delivered clip
# came back, and the toe by 20.6 degrees, because this rig's two bind toes splay 58 degrees apart
# so either one alone is a broken protractor. Together they ran him diagonally off the floor, which
# is what got reported. See `ik_gait.which_way_he_travels`.
#
# How close a sole must be to that foot's own lowest point to count as standing on it, in cm.
STANDS_WITHIN = 2.0


def curves_of(clip):
    """Every fcurve in an action, on Blender 5 and on what came before.

    From 4.4 an action is slots, layers, strips and channelbags rather than a flat
    `action.fcurves`, and reaching for the old attribute raises rather than returning nothing.
    """
    if hasattr(clip, "fcurves"):
        try:
            return list(clip.fcurves)
        except AttributeError:
            pass
    out = []
    for layer in getattr(clip, "layers", []):
        for strip in layer.strips:
            for bag in getattr(strip, "channelbags", []):
                out.extend(bag.fcurves)
    return out


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def flag(name, fallback=None):
    args = argv()
    return args[args.index(name) + 1] if name in args else fallback


def bone_lengths_from_the_skeleton(rig):
    """Points every bone at its child, so the skeleton draws as a skeleton.

    glTF carries joint POSITIONS and no lengths, so the importer picks one. For a bone with
    children the honest length is the distance to the first of them; for a leaf it is a share of
    its parent, which is a guess but a small one that stays inside the body.

    # Twist bones are not what a limb points at

    Taking the CLOSEST child drew every limb bone as a speck, reported as "should the bones be so
    small?". `L_Upperarm` has two children - `L_Forearm` at the elbow and `L_UpperarmTwist01`
    whose head sits almost on top of the shoulder - and the closest of those is the twist, so the
    upper arm rendered a few millimetres long. Same for the forearm, both thighs and both calves:
    every bone that matters, and only those, because only limb bones carry twists.

    So twists are skipped when choosing what to aim at. They are hidden in the viewport anyway,
    and a bone whose own length is a rounding error tells a reader nothing about the skeleton.
    """
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    spikes = 0
    for bone in rig.data.edit_bones:
        kids = [b for b in rig.data.edit_bones
                if b.parent is not None and b.parent.name == bone.name and HIDE not in b.name]
        # Children that sit ON this bone's head tell it nothing about its length. `Hip`'s first
        # child is `Pelvis` at the same point, so the closest-child rule measured zero, the
        # "too short to be meaningful" guard below skipped the bone, and it kept the importer's
        # invented length - 84.23 cm on a 170 cm figure, drawn as a cone through the whole torso
        # and reported as "this hip bone is bigger than his body". `Root` was the same.
        apart = [b for b in kids if (b.head - bone.head).length > 1e-4]
        aim = None
        # A bone with a SYMMETRIC PAIR of children is drawn down the middle of them, not at one.
        # `Pelvis` parents both thighs, and pointing it at the nearer one left the other leg
        # looking unattached - asked as "is he missing a hip bone here that should connect to the
        # left leg?". He is not: Root -> Hip -> Pelvis -> both thighs is complete. It was the
        # drawing.
        # A bone that has BOTH a symmetric pair of children and a single centre child continues
        # up the centre one. `Spine02` parents L_Clavicle, R_Clavicle and NeckTwist01, and the
        # nearest of those is a clavicle - so the spine was drawn running into the shoulder, and
        # read as "the spine appears attached to the arm and not the head".
        centre = [b for b in apart if not b.name.startswith(("L_", "R_"))]
        twins = [b for b in apart if b.name.startswith(("L_", "R_"))]
        if len(centre) == 1 and twins:
            bone.tail = centre[0].head
            continue
        if len(twins) == 2 and not centre and twins[0].name[2:] == twins[1].name[2:]:
            middle = (twins[0].head + twins[1].head) * 0.5
            if (middle - bone.head).length > 1e-4:
                bone.tail = middle
                continue
        if apart:
            aim = min(apart, key=lambda b: (b.head - bone.head).length)
            reach = (aim.head - bone.head).length
        elif bone.parent is not None:
            reach = bone.parent.length * A_LEAF_IS
        else:
            # Nothing to measure against at all: a root with everything stacked on it. A short
            # stub says "this is here" without drawing a spear through the character.
            reach = A_STUB_IS
        if reach < 1e-5:
            continue
        if bone.length > reach * 2.0:
            spikes += 1
        # POINTED at the child, not merely shortened toward it. This only ever rescaled the
        # bone along the direction it already had, so once a joint MOVED - the toe joint was
        # taken forward to the ball, lowered and centred - the parent went on pointing where the
        # child used to be and a visible gap opened between them. Reported as "the toes are
        # always offset when they should be attached".
        if aim is not None:
            bone.tail = aim.head
            continue
        along = (bone.tail - bone.head)
        if along.length < 1e-9:
            continue
        bone.tail = bone.head + along.normalized() * reach
    bpy.ops.object.mode_set(mode="OBJECT")
    return spikes


def drop_the_widgets(rig):
    """Takes the importer's Icosphere off every bone and removes it from the file."""
    taken = 0
    for bone in rig.pose.bones:
        if bone.custom_shape is not None:
            bone.custom_shape = None
            taken += 1
    for stray in [o for o in bpy.data.objects if o.name.startswith("Icosphere")]:
        bpy.data.objects.remove(stray, do_unlink=True)
    return taken


def play(rig, clip):
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = clip
    slots = getattr(clip, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]


def main():
    model = flag("--model", os.path.join(ROOT, "assets", "models", "person_ranger.glb"))
    save_to = flag("--save")
    opens_on = flag("--clip", "idle")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)
    bpy.ops.import_scene.gltf(filepath=model.replace("\\", "/"))

    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = max((o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups),
               key=lambda o: len(o.data.vertices))
    print(f"loaded {os.path.basename(model)}: {len(rig.data.bones)} bones, "
          f"{len(mesh.data.vertices)} vertices, {len(bpy.data.actions)} clips")

    spikes = bone_lengths_from_the_skeleton(rig)
    print(f"  bone lengths taken from the skeleton; {spikes} were drawn more than twice as long")
    print(f"  dropped {drop_the_widgets(rig)} sphere widgets")

    # Kept, or Blender drops the clips nothing points at when the file is saved.
    for clip in bpy.data.actions:
        clip.use_fake_user = True
    names = sorted(a.name for a in bpy.data.actions)
    print(f"  clips in the file: {', '.join(names)}")
    for clip in bpy.data.actions:
        first, last = (int(round(v)) for v in clip.frame_range)
        print(f"    {clip.name:<12s} frames {first}..{last}, "
              f"{(last - first) / 24.0:.2f} s")

    wanted = next((a for a in bpy.data.actions if a.name == opens_on),
                  next(iter(bpy.data.actions), None))
    scene = bpy.context.scene
    scene.render.fps = 24
    if wanted is not None:
        play(rig, wanted)
        scene.frame_start, scene.frame_end = (int(round(v)) for v in wanted.frame_range)
        scene.frame_set(scene.frame_start)
        print(f"  opening on '{wanted.name}', frames "
              f"{scene.frame_start}..{scene.frame_end}")

    # The skeleton drawn in front of the body, because half of what gets looked at is where the
    # bones are rather than where the surface is.
    rig.show_in_front = True
    rig.data.display_type = "OCTAHEDRAL"
    # Twists are hidden because there are eighteen of them sitting on top of the joints that
    # matter - but NOT one that is the only route to a bone worth seeing. `NeckTwist01` and
    # `NeckTwist02` are named like roll helpers and are actually the neck: they carry 25 and 65
    # vertices and the Head hangs off them. Hiding them cut the head loose from the skeleton.
    def leads_somewhere(bone):
        for kid in rig.data.bones:
            if kid.parent is not None and kid.parent.name == bone.name:
                if HIDE not in kid.name or leads_somewhere(kid):
                    return True
        return False

    for bone in rig.data.bones:
        bone.hide = HIDE in bone.name and not leads_somewhere(bone)
    print(f"  {sum(1 for b in rig.data.bones if b.hide)} twist bones hidden; alt-H shows them")

    # # The clip list, visible on open
    #
    # The default layout shows a TIMELINE, which scrubs the current clip and gives no hint that
    # others exist - "you're talking about the run but I only see the idle". The timeline area
    # becomes a dope sheet in ACTION EDITOR mode, where every clip in the file is one dropdown
    # away and the list is on screen rather than hidden.
    swapped = 0
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type != "DOPESHEET_EDITOR":
                continue
            area.spaces.active.mode = "ACTION"
            swapped += 1
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type == "TIMELINE":
                area.type = "DOPESHEET_EDITOR"
                area.spaces.active.mode = "ACTION"
                swapped += 1
    print(f"  {swapped} editor(s) set to the Action Editor, so every clip is one click away")

    low = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    high = max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    aim = mathutils.Vector((0.0, 0.0, (low + high) * 0.5))
    aimed = 0
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type != "VIEW_3D":
                continue
            space = area.spaces.active
            space.shading.type = "SOLID"
            space.overlay.show_floor = True
            space.region_3d.view_perspective = "ORTHO"
            # Side on: the view that shows a gait. Front-on hides everything a leg does.
            space.region_3d.view_rotation = mathutils.Vector((1.0, 0.0, 0.0)).to_track_quat(
                "Z", "Y")
            space.region_3d.view_location = aim
            space.region_3d.view_distance = (high - low) * 1.5
            aimed += 1
    print(f"  aimed {aimed} saved view(s) at a {(high - low) * 170.0:.0f} cm figure")

    # # He is driven FORWARD, or a treadmill reads as ice skating
    #
    # Every clip is a treadmill: the hips end 0.0 cm from where they began and the planted foot
    # slides backward at the speed the character is meant to be travelling. That is correct -
    # the game moves the warden and `motion::strides_over` drives the clip's phase from the
    # ground he covers - but with nothing moving underneath him in a viewer it looks exactly
    # like skating, and was reported as exactly that.
    #
    # So the scene carries him forward at the speed his own stride implies, and the planted foot
    # then stands still against the floor the way it will in game. The speed comes from the clip
    # itself - the distance one cycle covers over how long it lasts - so it is right for whatever
    # clip is showing rather than a number typed here.
    if wanted is not None and rig is not None and CARRIES_HIM_FORWARD and mesh is not None:
        first, last = (int(round(v)) for v in wanted.frame_range)
        play(rig, wanted)
        feet = ik_gait.which_vertices_are_feet(mesh)
        forward, each, _ = ik_gait.which_way_he_travels(
            rig, mesh, feet, wanted, bpy.context.scene, STANDS_WITHIN)
        if forward is not None and each > 1e-9:
            carries = each * (last - first)
            spot = rig.location.copy()
            for frame in (first, last):
                rig.location = spot + forward * (carries * (frame - first) / (last - first))
                rig.keyframe_insert("location", frame=frame)
            for curve in curves_of(rig.animation_data.action):
                if curve.data_path != "location":
                    continue
                for point in curve.keyframe_points:
                    # Linear, or the ramp eases in and out and he skates at both ends of the
                    # cycle - which is the very thing this exists to stop.
                    point.interpolation = "LINEAR"
            bpy.context.scene.frame_set(first)
            print(f"  carried him {carries * 170.0:.0f} cm along ({forward.x:+.3f}, "
                  f"{forward.y:+.3f}), measured off his own planted feet, so the planted foot "
                  f"stands still instead of skating")

    if save_to:
        bpy.ops.wm.save_as_mainfile(filepath=save_to)
        print(f"saved {save_to}")
        # Said out loud because it has already cost a round trip: this writes to a FIXED path,
        # so a Blender window that is already open is holding the PREVIOUS build and will keep
        # showing it however many times this is re-run. Reported as "the blender still shows the
        # idle" after the scene had been rebuilt on the run.
        print("  NOTE close any Blender window already open on this file first - it is holding "
              "the previous build, not this one.")


if __name__ == "__main__":
    main()
