"""Do the hands hold the same TWIST in the idle as in the gaits?

    blender --background --python roll_match.py -- <glb>

If they do not, the hands snap the moment the warden starts walking, which is a fault
no amount of correct gait makes up for. The idle came with the model and the gaits are
authored from rest, so the two have no reason to agree unless they are checked.

# Twist, not orientation

The first go compared the hands' world orientation at each clip's first frame and
reported the run's right hand 71 degrees off. That was the arm being SWUNG, not rolled
— at a contact pose one arm is forward and the other back, so their hands point
different ways by design, and comparing them says nothing about whether the wrist
agrees.

What matters is the twist about the forearm's own length, which is what
pronation/supination is and what the shipped idle had wrong. So each bone's local
rotation is split by swing-twist decomposition: the component about the bone's axis is
the twist, and everything else is the swing that a gait is entitled to change.
"""

import math
import sys

import bpy
import mathutils

ALONG = mathutils.Vector((0.0, 1.0, 0.0))


def twist_about(turn, axis=ALONG):
    """The part of a rotation that is about the given axis, in degrees.

    Swing-twist decomposition: project the quaternion's vector part onto the axis and
    keep the scalar. What is left over is the swing, which is not this function's
    business.
    """
    vector = mathutils.Vector((turn.x, turn.y, turn.z))
    along = axis.normalized() * vector.dot(axis.normalized())
    twist = mathutils.Quaternion((turn.w, along.x, along.y, along.z))
    if twist.magnitude < 1e-9:
        return 0.0
    twist.normalize()
    if twist.w < 0.0:
        twist = mathutils.Quaternion((-twist.w, -twist.x, -twist.y, -twist.z))
    return math.degrees(twist.angle) * (1.0 if vector.dot(axis) >= 0.0 else -1.0)


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def main():
    source = argv()[0]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    if rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            track.mute = True
    else:
        rig.animation_data_create()

    WATCH = ("L_Forearm", "R_Forearm", "L_Hand", "R_Hand")
    readings = {}
    for name in ("preset:biped:idle", "walk", "run"):
        action = bpy.data.actions.get(name)
        if action is None:
            continue
        rig.animation_data.action = action
        low, high = (int(v) for v in action.frame_range)
        # Averaged over the clip, because a twist should be near-constant: the brief
        # puts pronation through a gait at about 14 degrees, so the whole clip's
        # spread is itself worth seeing.
        got = {bone: [] for bone in WATCH}
        for frame in range(low, high + 1, max(1, (high - low) // 12)):
            bpy.context.scene.frame_set(frame)
            bpy.context.view_layer.update()
            for bone in WATCH:
                posed = rig.pose.bones.get(bone)
                if posed is not None:
                    got[bone].append(twist_about(posed.rotation_quaternion.normalized()))
        readings[name] = {
            bone: (sum(v) / len(v), min(v), max(v)) for bone, v in got.items() if v
        }

    base = "preset:biped:idle"
    print("\ntwist about each bone's own length, in degrees:\n")
    print(f"  {'bone':<12} {'idle':>18} {'walk':>18} {'run':>18}   verdict")
    for bone in WATCH:
        cells, spread = [], {}
        for name in (base, "walk", "run"):
            if name in readings and bone in readings[name]:
                mean, low, high = readings[name][bone]
                spread[name] = mean
                cells.append(f"{mean:+7.1f} ({low:+.0f}..{high:+.0f})")
            else:
                cells.append("-")
        gap = max(
            abs(spread.get(name, spread.get(base, 0.0)) - spread.get(base, 0.0))
            for name in ("walk", "run")
        )
        verdict = "matches" if gap < 25.0 else ("close" if gap < 50.0 else "WILL SNAP")
        print(f"  {bone:<12} {cells[0]:>18} {cells[1]:>18} {cells[2]:>18}   {gap:5.1f} off, {verdict}")


main()
