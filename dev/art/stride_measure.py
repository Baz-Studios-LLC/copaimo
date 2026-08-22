"""Measures how far a gait clip actually carries the warden, per cycle.

    blender --background --python dev/art/stride_measure.py -- <glb> <clip> [<clip>..]

# Why this cannot be arithmetic

`motion.rs` divides the warden's speed by how far one stride covers, so the clip
plays at the right cadence and the feet do not skate. That distance was estimated as
`2 * leg * sin(stride angle)` and it was wrong enough to matter: a test comparing the
resulting cadence against a believable one failed at 4.4 strides a second.

The honest number is not about angles. It is how far the PLANTED FOOT travels
backwards relative to the hips while it is on the ground — because that is the
ground the character has covered. So this poses the real rig over the real clip and
measures it.
"""

import sys

import bpy
import mathutils


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def main() -> None:
    args = argv()
    if len(args) < 2:
        raise SystemExit("need <glb> <clip> [<clip>...]")
    src, clips = args[0], args[1:]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    if rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            track.mute = True

    scene = bpy.context.scene
    for name in clips:
        action = bpy.data.actions.get(name)
        if action is None:
            print(f"no clip {name!r}")
            continue
        rig.animation_data.action = action
        low, high = (int(v) for v in action.frame_range)

        # Each foot's position relative to the hips, every frame.
        tracks = {"L_Foot": [], "R_Foot": []}
        for frame in range(low, high + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            hips = rig.matrix_world @ rig.pose.bones["Hip"].head
            for bone in tracks:
                foot = rig.matrix_world @ rig.pose.bones[bone].head
                tracks[bone].append((foot - hips))

        # The stride: how far a foot swings from its furthest forward to its
        # furthest back, along the direction of travel. The model faces +X.
        spans = {}
        for bone, path in tracks.items():
            ahead = [p.x for p in path]
            spans[bone] = max(ahead) - min(ahead)
        stride = sum(spans.values()) / len(spans)

        # And the lift, which says whether the feet leave the ground at all.
        lift = max(max(p.z for p in path) - min(p.z for p in path) for path in tracks.values())
        # A CYCLE is both feet taking one step each, so the body advances by TWICE
        # one foot's swing. Getting that wrong by a factor of two is the difference
        # between a believable cadence and a blur.
        cycle = stride * 2.0 * 1.7
        print(
            f"{name}: foot swing {stride:.3f} units per foot -> a cycle covers "
            f"{cycle:.3f} m at game scale (1.7 m tall); vertical foot travel "
            f"{lift * 1.7:.3f} m"
        )


main()
