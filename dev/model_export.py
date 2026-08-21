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
    """The whole scene's corner-to-corner extent, in Blender's own axes."""
    low = [float("inf")] * 3
    high = [float("-inf")] * 3
    seen = False
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        seen = True
        for corner in obj.bound_box:
            world = obj.matrix_world @ __import__("mathutils").Vector(corner)
            for axis in range(3):
                low[axis] = min(low[axis], world[axis])
                high[axis] = max(high[axis], world[axis])
    if not seen:
        raise SystemExit("nothing to export: the file has no mesh in it")
    return low, high


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
    # Blender Z is up here; it becomes Y on the way out.
    if low[2] < -FOOTING_SLACK:
        faults.append(
            f"its base sits {low[2]:.2f} m BELOW the floor, so it will import "
            "half-buried — put the feet on Z=0"
        )
    if low[2] > FOOTING_SLACK:
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
