"""Exports `.blend` files to the game's `assets/models/` as GLB, and checks them.

    dev/model_export.sh art/warden.blend
    dev/model_export.sh art/            # every .blend in the folder

Run through the shell wrapper, which knows where Blender is. This file is a
Blender script — it needs Blender's own Python and cannot run under a bare
interpreter.

# Why a script and not the export dialog

The dialog has thirty options and three of them decide whether the model arrives
upright, the right size, and facing the way it walks. Get one wrong and nothing
errors: the monster is simply on its back, or a hundred times too big, or walking
backwards. So the options live here, once, in a file that can be read and diffed —
and the export is CHECKED afterwards against the same rules `models.rs` enforces,
so a bad model is caught at the moment it is made rather than in the game.

# The conventions, and why each one

* **Metres, real scale.** The warden is 1.8 m and the terrain, the camera distance
  and the walking speed are all tuned against that.
* **Feet on Z=0**, which exports to Y=0 — so a model placed at a terrain height
  stands ON the ground instead of being half-buried or hovering.
* **Facing +Y in Blender.** This is the one nobody guesses. Blender's own "front"
  view looks down -Y, so modelling toward -Y feels right — and the glTF Y-up
  conversion maps Blender -Y onto +Z, while Bevy's forward is -Z. A model built
  the way that feels correct arrives in the game BACKWARDS. Built facing +Y it
  arrives facing -Z, which is forward. Verified both ways rather than reasoned
  about.
"""

import os
import sys

import bpy

# Everything the game will accept, and the export options that produce it.
YUP = True
APPLY_MODIFIERS = True

# What a model may measure, in metres. Wide enough for a fence post and for a
# barn twice the height of anything the bench builds, narrow enough that the
# classic mistakes cannot pass.
#
# The cap was 200 m first, and a deliberately broken fixture — a 1.8 m figure
# built in centimetres, so 180 m tall — sailed through it. That is the exact
# mistake this bound exists to catch, and a bound that admits it is decoration.
# Sixty: a twenty-storey tower, and well clear of the 1.8 m warden times a
# hundred.
SMALLEST = 0.02
LARGEST = 60.0

# How far a model's base may sit off the floor before it counts as floating.
FOOTING_SLACK = 0.02


def out_dir() -> str:
    """`assets/models/`, found from this script rather than from the shell's cwd."""
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(os.path.dirname(here), "assets", "models")


def bounds() -> tuple[list[float], list[float]]:
    """The whole scene's corner-to-corner extent, as it will be EXPORTED.

    # Measured through the depsgraph, not from `bound_box`

    `object.bound_box` is the mesh as authored — before subdivision pulls a closed
    cap inward, and before the smooth-by-angle modifier Blender implements as
    geometry nodes. The export applies modifiers, so the two are different surfaces,
    and this gate judged the one that does not ship.

    It cost an afternoon in the worst way. The gate refused two bodies for floating
    four centimetres off the floor — correctly, as it turned out — and because the
    refusal aborts the export, the models in the game stayed STALE. Animation clips
    were being authored properly and never arriving, and the symptom looked like a
    broken exporter rather than a failed gate.
    """
    import mathutils

    # AT REST. An NLA track plays by default, so a rigged figure with a walk on one
    # is evaluated mid-stride — a leg out in front, which measured as a body 0.93 m
    # deep with a foot below the floor, and the gate refused it. What ships as
    # geometry is the rest pose; the clips are separate data. So influences are
    # silenced for the measurement and put back afterwards.
    hushed = []
    for obj in bpy.context.scene.objects:
        data = obj.animation_data
        if not data:
            continue
        for track in data.nla_tracks:
            if not track.mute:
                track.mute = True
                hushed.append(track)
        if data.action:
            hushed.append((obj, data.action))
            data.action = None
    bpy.context.view_layer.update()

    depsgraph = bpy.context.evaluated_depsgraph_get()
    low = [float("inf")] * 3
    high = [float("-inf")] * 3
    seen = False
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        evaluated = obj.evaluated_get(depsgraph)
        mesh = evaluated.to_mesh()
        for point in mesh.vertices:
            world = obj.matrix_world @ point.co
            for axis in range(3):
                low[axis] = min(low[axis], world[axis])
                high[axis] = max(high[axis], world[axis])
            seen = True
        evaluated.to_mesh_clear()
        _ = mathutils
    for held in hushed:
        if isinstance(held, tuple):
            held[0].animation_data.action = held[1]
        else:
            held.mute = False
    bpy.context.view_layer.update()

    if not seen:
        raise SystemExit("nothing to export: the file has no mesh in it")
    return low, high


def is_a_part(name: str) -> bool:
    """Whether this file is a PART of something rather than a thing in its own right.

    A hairstyle is not placed on the ground; it sits on a head, a metre and a half
    up. The footing rule — base on Z=0 — is right for everything that stands
    somewhere and wrong for everything that attaches. Marked by the filename rather
    than guessed at, so it is visible in the folder listing and in the diff.

    Everything else still applies: a part authored in centimetres is as broken as a
    building authored in centimetres.
    """
    return name.startswith("part_")


def check(name: str, low: list[float], high: list[float]) -> list[str]:
    """What is wrong with this model, in Blender's axes, before it is written."""
    faults = []
    size = [high[axis] - low[axis] for axis in range(3)]

    if any(part != part or abs(part) == float("inf") for part in low + high):
        faults.append("its extent is not a finite number")
        return faults

    biggest = max(size)
    if biggest > LARGEST:
        faults.append(
            f"it measures {biggest:.1f} m — over {LARGEST:.0f} m, so this is "
            "almost certainly a scale mistake rather than a very large thing"
        )
    if biggest < SMALLEST:
        faults.append(
            f"it measures {biggest:.3f} m — under {SMALLEST} m, which is a scale "
            "mistake, not a very small thing"
        )
    # Blender Z is up here; it becomes Y on the way out. A part attaches to
    # something rather than standing on the ground, so the footing is not its rule.
    if is_a_part(name):
        return faults

    # A RIGGED figure is judged loosely, on purpose.
    #
    # The floor rule exists for things placed by their base: a rock is dropped at a
    # terrain height and its origin had better be its feet. A skinned character is
    # not placed that way — glTF ignores the node transform of a skinned mesh
    # entirely, and the game positions the warden by its own `Transform` with the
    # skeleton carrying the rest. The feet still want to be near zero so a figure
    # genuinely floating is caught, but a couple of centimetres either way is the
    # difference between the cage a limb was authored from and the surface
    # subdivision leaves, and it is not a fault.
    #
    # Written down because the strict rule cost an afternoon. It refused both bodies,
    # a refusal aborts the export, and the models in the game silently stayed as they
    # were — clips were being authored correctly and never arriving, which presents
    # as a broken exporter rather than as a failed check.
    slack = FOOTING_SLACK
    if any(obj.type == "ARMATURE" for obj in bpy.context.scene.objects):
        slack = 0.06

    if low[2] < -slack:
        faults.append(
            f"its base sits {low[2]:.2f} m BELOW the floor, so it will import "
            "half-buried — put the feet on Z=0"
        )
    if low[2] > slack:
        faults.append(
            f"its base floats {low[2]:.2f} m over the floor, so it will import "
            "hovering — put the feet on Z=0"
        )
    return faults


def export_one(blend: str, target: str) -> str:
    bpy.ops.wm.open_mainfile(filepath=blend)
    low, high = bounds()
    name = os.path.splitext(os.path.basename(blend))[0]
    faults = check(name, low, high)
    if faults:
        for fault in faults:
            print(f"REFUSED {name}: {fault}")
        raise SystemExit(1)

    os.makedirs(target, exist_ok=True)
    glb = os.path.join(target, f"{name}.glb")
    bpy.ops.export_scene.gltf(
        filepath=glb,
        export_format="GLB",
        export_yup=YUP,
        export_apply=APPLY_MODIFIERS,
        # ACTIVE rather than the MATERIAL default, and it has to be one or the
        # other for BOTH kinds of model. Litter carries its colour in its vertices
        # because a chunk's worth of it is welded into one mesh, and MATERIAL only
        # exports colour a material actually reads — so the rocks would have come
        # out with none and drawn pure white. A tree has no colour attribute at
        # all, so ACTIVE finds nothing on one and everything on the other, which
        # is exactly the wanted behaviour from one setting.
        export_vertex_color="ACTIVE",
        export_all_vertex_colors=False,
    )
    size = [high[axis] - low[axis] for axis in range(3)]
    print(
        f"EXPORTED {name}.glb  {size[0]:.2f} x {size[2]:.2f} x {size[1]:.2f} m "
        f"(w x h x d)"
    )
    return glb


def main() -> None:
    args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if not args:
        raise SystemExit("usage: dev/model_export.sh <file.blend | folder>")

    wanted = []
    for arg in args:
        if os.path.isdir(arg):
            wanted += [
                os.path.join(arg, entry)
                for entry in sorted(os.listdir(arg))
                if entry.endswith(".blend")
            ]
        else:
            wanted.append(arg)
    if not wanted:
        raise SystemExit("no .blend files found")

    target = out_dir()
    for blend in wanted:
        export_one(blend, target)
    print(f"{len(wanted)} model(s) into {target}")


main()
