"""Renders one town figure from four sides, so it can be held against a concept.

    dev/art/sheet.sh guild_hall            writes dev/art/shots/sheet_guild_hall.png

# Why this exists

A concept sheet arrives as front, side and rear elevations. The only honest way to
say a model matches one is to put the same four views next to it - "I built it to the
drawing" is an assertion, and this project has paid for enough of those.

Rendered in Workbench with vertex colour, which is what the game shades from, so the
palette shown here is the palette that ships rather than a lit interpretation of it.
"""
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import masonry  # noqa: E402
import town  # noqa: E402


def argument(name, fallback):
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    for index, item in enumerate(argv):
        if item == name and index + 1 < len(argv):
            return argv[index + 1]
    return fallback


def render(figure: str, out: str) -> None:
    masonry.fresh()
    parts, tall = town.FIGURES[figure]()
    masonry.weld(parts, town.PAINT, tall, name="figure", floor_of=lambda _: 0.0)
    # NO OUTLINE HULL. It is an inverted shell that surrounds the whole figure, so in
    # a flat Workbench render it draws as a solid black object in front of everything
    # - the first sheet came back as a black silhouette with the massing readable and
    # not one colour in it. The game draws it back-face only; this does not.

    # What the figure actually occupies, so the camera frames it rather than guessing.
    low = [1e9, 1e9, 1e9]
    high = [-1e9, -1e9, -1e9]
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        for corner in obj.bound_box:
            point = obj.matrix_world @ __import__("mathutils").Vector(corner)
            for axis in range(3):
                low[axis] = min(low[axis], point[axis])
                high[axis] = max(high[axis], point[axis])
    middle = [(low[a] + high[a]) * 0.5 for a in range(3)]
    span = max(high[a] - low[a] for a in range(3)) * 1.15

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "VERTEX"
    shading.show_shadows = True
    shading.show_cavity = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.resolution_x = 900
    scene.render.resolution_y = 700
    scene.render.film_transparent = False
    scene.world = scene.world or bpy.data.worlds.new("sheet")
    scene.world.color = (1.0, 1.0, 1.0)

    camera_data = bpy.data.cameras.new("sheet")
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = span
    camera = bpy.data.objects.new("sheet", camera_data)
    scene.collection.objects.link(camera)
    scene.camera = camera

    import mathutils

    VIEWS = {
        "front": (0.0, -1.0, 0.0),
        "side": (-1.0, 0.0, 0.0),
        "rear": (0.0, 1.0, 0.0),
        "quarter": (-0.75, -1.0, 0.42),
    }
    written = []
    for name, direction in VIEWS.items():
        away = mathutils.Vector(direction).normalized() * span * 2.0
        camera.location = mathutils.Vector(middle) + away
        camera.rotation_mode = "QUATERNION"
        camera.rotation_quaternion = (-away).to_track_quat("-Z", "Y")
        scene.render.filepath = f"{out}_{name}.png"
        bpy.ops.render.render(write_still=True)
        written.append(scene.render.filepath)
        print(f"RENDERED {scene.render.filepath}")
    return written


# ASKED FOR, never automatic. `build.sh` runs every .py in this folder, so a default
# figure here would re-render a turnaround on every build of the whole art pipeline.
_figure = argument("--figure", None)
if _figure:
    render(_figure, argument("--out", os.path.join(HERE, "shots", f"sheet_{_figure}")))
else:
    print("sheet.py: pass -- --figure <name> to render a turnaround")
