"""One scene with the whole character and every clip in it.

    dev/art/see_everything.sh

Built because five separate viewers meant five windows and no way to compare anything against
anything. This is one: the built asset, its skeleton visible, and EVERY clip in the Action Editor
- the authored walk, run, sprint, idle and grip, plus the delivered presets retargeted onto the
same rig under `delivered_walk` and `delivered_run`.

So the authored walk and the delivered walk are two entries in one list on one body, which is the
only way a question like "is the new one better" can actually be looked at.

The reload watcher from gait_watch is installed, so a rebuild reaches this window without anybody
closing anything.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

ROOT = os.path.dirname(os.path.dirname(ART))
SCALE = 170.0


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


args = argv()
SAVE_TO = args[args.index("--save") + 1] if "--save" in args else None
BUILT = os.path.join(ROOT, "assets", "models", "person_ranger.glb")

# NOT read_homefile - see gait_watch.py. Clearing the startup objects by hand avoids a context
# the glTF importer cannot work in.
for stale in list(bpy.data.objects):
    bpy.data.objects.remove(stale, do_unlink=True)

bpy.ops.import_scene.gltf(filepath=BUILT.replace("\\", "/"))
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import gait_watch  # noqa: E402  (the watcher and the stamp; its main() is guarded)
import prepare_rig  # noqa: E402
import retarget  # noqa: E402

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
prepare_rig.drop_the_widgets(rig)
print(f"loaded {os.path.basename(BUILT)}: {len(rig.data.bones)} bones, "
      f"{len(bpy.data.actions)} clips")

# The delivered presets, retargeted onto this same body so they can be compared with the authored
# ones rather than remembered. Named apart so nothing collides, and skipped quietly if the files
# are not there - this is a viewer and it should still open.
for called, file in (("walk", "Ranger-Walk.glb"), ("run", "Ranger-Run.glb")):
    path = os.path.join(ROOT, "assets", "models", file)
    if not os.path.exists(path):
        print(f"  no {file}, so no delivered_{called} to compare against")
        continue
    before = set(bpy.data.objects)
    known = set(bpy.data.actions)
    bpy.ops.import_scene.gltf(filepath=path.replace("\\", "/"))
    fresh = [o for o in bpy.data.objects if o not in before]
    source = next((o for o in fresh if o.type == "ARMATURE"), None)
    if source is None:
        print(f"  no armature in {file}")
        continue
    try:
        made = retarget.retarget(source, rig, f"delivered_{called}")
        print(f"  delivered_{called} retargeted onto this rig")
    except SystemExit as why:
        print(f"  delivered_{called} could not be retargeted: {why}")
        made = None
    for thing in fresh:
        bpy.data.objects.remove(thing, do_unlink=True)
    for spare in [a for a in bpy.data.actions if a not in known and a is not made]:
        bpy.data.actions.remove(spare)

# Every clip kept, or Blender drops the ones nothing points at when the file is saved.
for clip in bpy.data.actions:
    clip.use_fake_user = True
names = sorted(a.name for a in bpy.data.actions)
print(f"  {len(names)} clips in the file: " + ", ".join(names))

# Open on a delivered clip if there is one, since that is the thing most recently asked about.
wanted = next((a for a in bpy.data.actions if a.name == "delivered_walk"),
              next((a for a in bpy.data.actions if a.name == "walk"), None))
if wanted is not None:
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = wanted
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = (int(round(v)) for v in wanted.frame_range)
    scene.frame_set(scene.frame_start)
    print(f"  showing '{wanted.name}', frames {scene.frame_start}..{scene.frame_end}")

scene = bpy.context.scene
scene.render.fps = 24

# The skeleton visible in front of the body, since half of what gets looked at is where the bones
# are. Twist bones hidden - sixteen of them sit on top of the joints that matter.
rig.show_in_front = True
rig.data.display_type = "OCTAHEDRAL"
for bone in rig.data.bones:
    bone.hide = "Twist" in bone.name

# Side on, the whole figure in frame.
low = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
high = max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
aim = mathutils.Vector((0.0, 0.0, (low + high) * 0.5))
across, forward, up = prepare_rig.body_frame(rig)
aimed = 0
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type != "VIEW_3D":
            continue
        space = area.spaces.active
        space.shading.type = "SOLID"
        space.overlay.show_floor = True
        space.region_3d.view_perspective = "ORTHO"
        space.region_3d.view_rotation = mathutils.Vector(across).to_track_quat("Z", "Y")
        space.region_3d.view_location = aim
        space.region_3d.view_distance = (high - low) * 1.6
        aimed += 1
print(f"  aimed {aimed} saved 3D view(s) at the figure, {(high - low) * SCALE:.0f} cm tall")

if SAVE_TO:
    gait_watch.stamp_the_scene(BUILT, "everything")
    gait_watch.install_the_watcher()
    bpy.ops.wm.save_as_mainfile(filepath=SAVE_TO)
    print(f"saved {SAVE_TO}")
