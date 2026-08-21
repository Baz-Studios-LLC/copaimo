"""Adds walk and run clips to the made ranger, and writes the game's copy of it.

    dev/art/animate_ranger.sh

Reads `Ranger_Rig_Idle.glb` from the repository root — the file as it arrived, kept
untouched — and writes `assets/models/person_ranger.glb` with its idle plus a walk
and a run. Re-runnable: the source is never modified, so this can be thrown away and
done again, and a new export from the generator drops straight in.

# Rotating a bone you did not build

The rig came from a generator, so nothing about its rest orientations is known: a
bone's local X might point along the limb, across it, or anywhere. Guessing an axis
and looking at the result is how the scripted figure's knees ended up bending
backwards like a bird's.

So nothing here guesses. Every pose is stated in the ARMATURE's own axes — swing
this leg forward, lift these hips — and converted into whatever local axis that
happens to be for the bone in question:

    axis_in_bone = bone.matrix_local.to_3x3().inverted() @ world_axis

That is exact for any rig, and it means this same script would work on a different
skeleton with different rest poses.

# Which way is forward

The model faces +X. Up is +Z (Blender). So a limb swinging forward turns about the
armature's **Y** axis, and the body bobs along **Z**. A positive swing is forward,
which makes a knee's flexion negative and an elbow's positive — the same anatomy the
scripted figure needed, just expressed in axes that hold for any rig.
"""

import math
import os
import sys

import bpy
import mathutils

# How far a thigh reaches at full stride, in degrees, and how far an arm answers it.
WALK_STRIDE = 26.0
WALK_ARM = 20.0
RUN_STRIDE = 42.0
RUN_ARM = 42.0

# How much a knee bends as the leg passes under the body.
WALK_KNEE = 38.0
RUN_KNEE = 74.0

# How far the hips rise and fall, in the model's own units — it stands one unit
# tall, so this is a share of its height rather than metres.
WALK_BOB = 0.014
RUN_BOB = 0.030

# How far the body leans into a run, and how much the spine counter-twists.
RUN_LEAN = 9.0
SPINE_TWIST = 4.0


def swing(rig, bone: str, degrees: float, axis=(0.0, 1.0, 0.0)):
    """Turns a bone about an axis of the ARMATURE, whatever its own rest pose is."""
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    local = rest.inverted() @ mathutils.Vector(axis)
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = mathutils.Quaternion(
        local.normalized(), math.radians(degrees)
    )


def shift(rig, bone: str, along: float, axis=(0.0, 0.0, 1.0)):
    """Moves a bone along an axis of the armature — used for the bob."""
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    posed.location = rest.inverted() @ (mathutils.Vector(axis) * along)


def key(rig, frame: int, bones):
    """Writes the current pose of the named bones at this frame."""
    for bone in bones:
        posed = rig.pose.bones.get(bone)
        if posed is None:
            continue
        posed.keyframe_insert("rotation_quaternion", frame=frame)
        if bone in ("Hip", "Root"):
            posed.keyframe_insert("location", frame=frame)


# Everything a stride touches. Twist bones are left alone: they exist to spread a
# limb's roll and have no business being posed by hand.
DRIVEN = (
    "Hip",
    "Waist",
    "Spine01",
    "Spine02",
    "L_Thigh",
    "R_Thigh",
    "L_Calf",
    "R_Calf",
    "L_Foot",
    "R_Foot",
    "L_Upperarm",
    "R_Upperarm",
    "L_Forearm",
    "R_Forearm",
)


def gait(rig, name: str, stride: float, arm: float, knee: float, bob: float, lean: float, span: int):
    """One cycle: contact, pass, contact, pass, contact.

    A gait is four poses and the fifth is the first again, so it loops. What makes
    it read as walking rather than as legs waving is the BOB — the body falls onto
    each foot and rises over it, twice per cycle — and the arms answering the
    opposite leg.
    """
    action = bpy.data.actions.new(name)
    rig.animation_data.action = action
    quarter = span // 4

    for step in range(5):
        frame = 1 + step * quarter
        phase = step % 4
        # Which leg is forward: +1 means the left.
        lead = 1.0 if phase == 0 else (-1.0 if phase == 2 else 0.0)
        passing = phase in (1, 3)
        # On a pass, the swinging leg is the one that was behind.
        swinging = 1.0 if phase == 1 else -1.0

        for side, hand in (("L", 1.0), ("R", -1.0)):
            if passing:
                forward = swinging * hand
                swing(rig, f"{side}_Thigh", (10.0 if forward > 0 else -14.0))
                swing(rig, f"{side}_Calf", -knee if forward > 0 else -6.0)
                swing(rig, f"{side}_Foot", 12.0 if forward > 0 else -6.0)
                swing(rig, f"{side}_Upperarm", 0.0)
            else:
                forward = lead * hand
                swing(rig, f"{side}_Thigh", stride * forward)
                # The reaching leg is nearly straight; the trailing one keeps a bend
                # because it is pushing off.
                swing(rig, f"{side}_Calf", -8.0 if forward > 0 else -26.0)
                swing(rig, f"{side}_Foot", -10.0 if forward > 0 else 16.0)
                # Arms answer the OPPOSITE leg.
                swing(rig, f"{side}_Upperarm", -arm * forward)
            # A held bend, so the arms are not two planks.
            swing(rig, f"{side}_Forearm", 18.0 if name == "walk" else 62.0)

        # The spine counter-twists against the hips, and leans into a run.
        swing(rig, "Spine01", lean * 0.4, axis=(0.0, 1.0, 0.0))
        swing(rig, "Spine02", SPINE_TWIST * (lead if not passing else 0.0), axis=(0.0, 0.0, 1.0))
        # Lowest at each contact, highest over the standing leg.
        shift(rig, "Hip", 0.0 if not passing else bob)
        key(rig, frame, DRIVEN)

    # Nothing is done about interpolation here. Bezier is Blender's own default for
    # a new key, and reaching for `action.fcurves` to set it does not work on 5.x
    # anyway — actions are LAYERED now, and the curves live under
    # `action.layers[].strips[].channelbag(slot)`. Easing a gait by hand would be
    # the wrong move regardless: a walk wants to slow into each contact, which is
    # what bezier already does.
    return action


def use_the_calmed_texture() -> None:
    """Points the material at the base-colour map with the eye whites brought down.

    The pixel work is `dev/art/ranger_texture.py`, deliberately not here. Doing it in
    Blender — editing `image.pixels` and calling `pack()` — reported success and
    exported the ORIGINAL bytes: peak luminance through the eye read 254 before and
    254 after. Blender keeps the packed file it already has. So the texture is
    prepared outside, where the result can be measured, and this step has one job it
    cannot get wrong.
    """
    here = os.path.dirname(os.path.abspath(__file__))
    calmed = os.path.join(here, "ranger_basecolor.png")
    if not os.path.isfile(calmed):
        print(f"no {os.path.basename(calmed)}; the eyes keep their glare")
        return

    replacement = bpy.data.images.load(calmed, check_existing=True)
    swapped = 0
    for material in bpy.data.materials:
        if not material.use_nodes:
            continue
        for link in material.node_tree.links:
            if link.to_socket.name != "Base Color":
                continue
            node = link.from_node
            if node.type == "TEX_IMAGE":
                node.image = replacement
                swapped += 1
    print(f"USING {os.path.basename(calmed)} on {swapped} material(s)")


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    source = os.path.join(root, "Ranger_Rig_Idle.glb")
    out = os.path.join(root, "assets", "models", "person_ranger.glb")
    if not os.path.isfile(source):
        raise SystemExit(f"the source is not there: {source}")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is None:
        raise SystemExit("no armature in the source")
    print(f"rig '{rig.name}' with {len(rig.data.bones)} bones")
    missing = [b for b in DRIVEN if b not in rig.pose.bones]
    if missing:
        raise SystemExit(f"the rig has no {missing} — the gait cannot be written")

    use_the_calmed_texture()

    idle = rig.animation_data.action if rig.animation_data else None
    if idle:
        print(f"keeping '{idle.name}'")
        idle.use_fake_user = True

    # Twenty-four frames a cycle for a walk, sixteen for a run: a run is the same
    # shape at a quicker cadence, and reads wrong if it is only bigger.
    gait(rig, "walk", WALK_STRIDE, WALK_ARM, WALK_KNEE, WALK_BOB, 0.0, 24).use_fake_user = True
    gait(rig, "run", RUN_STRIDE, RUN_ARM, RUN_KNEE, RUN_BOB, RUN_LEAN, 16).use_fake_user = True

    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        export_yup=True,
        # Every action as its own clip, rather than only whatever is active.
        export_animation_mode="ACTIONS",
        export_animations=True,
        export_skins=True,
        export_morph=False,
        export_apply=False,
    )
    print(f"WROTE {out}")
    print("clips:", [a.name for a in bpy.data.actions])


main()
