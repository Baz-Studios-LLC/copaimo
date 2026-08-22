"""Renders five frames of a named clip from a GLB, so a gait can be LOOKED at.

    "$BL" --background --python-exit-code 1 --python dev/art/gait_look.py -- <glb> <out.png> <clip>

Writes `<out>_0.png` .. `<out>_4.png`: five frames evenly spread over the clip's
own frame range, from a three-quarter front view at chest height. Optional extra
args: `--cam front|tqfront|side|tqback`, `--zoom <factor>`, `--focus <x,y,z>`.

The camera is placed from the MESH's own bounding box, so it frames whatever it is
given rather than assuming the ranger's one-unit height.
"""

import sys
import math
import os

import bpy
import mathutils


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


CAMS = {
    # (azimuth degrees from +X toward +Y, elevation degrees)
    "front": (0.0, 6.0),
    "tqfront": (38.0, 10.0),
    "tqfront_low": (38.0, -4.0),
    # The model's left is +Y, so every `+` azimuth here only ever shows the LEFT
    # hand — and the glove/pocket fault is worse on the RIGHT. Mirrored views, so
    # both sides can be looked at.
    "tqfront_r": (-38.0, 10.0),
    "tqfront_r_low": (-38.0, -4.0),
    "side_r": (-90.0, 6.0),
    "side": (90.0, 6.0),
    "tqback": (140.0, 10.0),
    "top": (38.0, 70.0),
}


def look_at(obj, target):
    direction = target - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def main():
    args = argv()
    if len(args) < 3:
        raise SystemExit("need <glb> <out.png> <clip>")
    src, out, clip = args[0], args[1], args[2]

    cam_name = "tqfront"
    zoom = 1.0
    focus_override = None
    frames_override = None
    i = 3
    while i < len(args):
        if args[i] == "--cam":
            cam_name = args[i + 1]
            i += 2
        elif args[i] == "--zoom":
            zoom = float(args[i + 1])
            i += 2
        elif args[i] == "--focus":
            focus_override = mathutils.Vector([float(v) for v in args[i + 1].split(",")])
            i += 2
        elif args[i] == "--frames":
            frames_override = [int(v) for v in args[i + 1].split(",")]
            i += 2
        else:
            i += 1

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    # Only the SKINNED meshes. The Tripo file carries a stray unskinned, material-less
    # `Icosphere` of radius 1 at the origin; counting it in the bounds pushed the
    # camera back until the character was a twelfth of the frame.
    meshes = [
        o
        for o in bpy.data.objects
        if o.type == "MESH" and (o.vertex_groups or any(m.type == "ARMATURE" for m in o.modifiers))
    ]
    if not meshes:
        meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    if not meshes:
        raise SystemExit("no mesh imported")
    for o in bpy.data.objects:
        if o.type == "MESH" and o not in meshes:
            o.hide_render = True

    # The clip. `None` / `rest` means leave it at rest pose.
    start, end = 1, 1
    if clip.lower() not in ("none", "rest", "-"):
        action = bpy.data.actions.get(clip)
        if action is None:
            raise SystemExit(f"no clip '{clip}'; have {[a.name for a in bpy.data.actions]}")
        if rig is None:
            raise SystemExit("a clip was asked for but there is no armature")
        if rig.animation_data is None:
            rig.animation_data_create()
        rig.animation_data.action = action
        # Blender 5.x: a slot must be assigned for a layered action to evaluate.
        try:
            if rig.animation_data.action_slot is None and action.slots:
                rig.animation_data.action_slot = action.slots[0]
        except AttributeError:
            pass
        rng = action.frame_range
        start, end = int(rng[0]), int(rng[1])
        print(f"clip '{clip}' frames {start}..{end}")
    elif rig is not None and rig.animation_data is not None:
        rig.animation_data.action = None

    frames = frames_override or [
        int(round(start + (end - start) * k / 4.0)) for k in range(5)
    ]

    # World-space bounds of the POSED skin at the first rendered frame, for framing.
    bpy.context.scene.frame_set(frames[0])
    deps = bpy.context.evaluated_depsgraph_get()
    lo = mathutils.Vector((1e9, 1e9, 1e9))
    hi = mathutils.Vector((-1e9, -1e9, -1e9))
    for m in meshes:
        ev = m.evaluated_get(deps)
        mesh = ev.to_mesh()
        for v in mesh.vertices:
            w = ev.matrix_world @ v.co
            for a in range(3):
                lo[a] = min(lo[a], w[a])
                hi[a] = max(hi[a], w[a])
        ev.to_mesh_clear()
    centre = (lo + hi) * 0.5
    radius = max((hi - centre).length, (lo - centre).length) or 1.0
    size = max((hi - lo).x, (hi - lo).y, (hi - lo).z) or 1.0
    focus = focus_override if focus_override is not None else centre
    print("bounds lo", tuple(round(x, 3) for x in lo), "hi", tuple(round(x, 3) for x in hi))

    # Camera and a plain three-light setup.
    cam_data = bpy.data.cameras.new("cam")
    cam_data.lens = 50.0
    cam = bpy.data.objects.new("cam", cam_data)
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam

    az, el = CAMS.get(cam_name, CAMS["tqfront"])
    # Fit the bounding sphere to the narrower half-angle, with a small margin.
    half_fov = math.atan((cam_data.sensor_width * 0.5) / cam_data.lens)
    dist = (radius * 1.12) / math.tan(half_fov) / max(zoom, 1e-3)
    if focus_override is not None:
        dist = (size * 0.62) / math.tan(half_fov) / max(zoom, 1e-3)
    a, e = math.radians(az), math.radians(el)
    cam.location = focus + mathutils.Vector(
        (math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e))
    ) * dist
    look_at(cam, focus)

    for key, (lx, ly, lz, power) in {
        "key": (2.0, 1.4, 2.4, 900.0),
        "fill": (-2.2, 2.0, 1.0, 380.0),
        "rim": (-1.4, -2.4, 2.0, 500.0),
    }.items():
        lamp = bpy.data.lights.new(key, type="AREA")
        lamp.energy = power * (size ** 2)
        lamp.size = size
        ob = bpy.data.objects.new(key, lamp)
        ob.location = focus + mathutils.Vector((lx, ly, lz)) * size
        look_at(ob, focus)
        bpy.context.scene.collection.objects.link(ob)

    world = bpy.data.worlds.new("w")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs[0].default_value = (0.06, 0.07, 0.09, 1.0)
    world.node_tree.nodes["Background"].inputs[1].default_value = 0.6
    bpy.context.scene.world = world

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1000
    scene.render.resolution_y = 1000
    scene.render.film_transparent = False
    scene.render.image_settings.file_format = "PNG"

    stem, ext = os.path.splitext(out)
    for idx, frame in enumerate(frames):
        scene.frame_set(frame)
        scene.render.filepath = f"{stem}_{idx}{ext or '.png'}"
        bpy.ops.render.render(write_still=True)
        print(f"WROTE {scene.render.filepath} (frame {frame})")


main()
