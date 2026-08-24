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
ROOT = os.path.dirname(os.path.dirname(ART))

# Bones the file gives no length for get one invented. A leaf bone is given this share of its
# parent's length instead, which is roughly what a hand or a toe tip is.
A_LEAF_IS = 0.45

# Twist bones sit on top of the joints that matter and there are eighteen of them, so they are
# hidden by default. Alt-H in the viewport brings them back.
HIDE = "Twist"


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
    """
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    spikes = 0
    for bone in rig.data.edit_bones:
        kids = [b for b in rig.data.edit_bones if b.parent is bone]
        if kids:
            aim = min(kids, key=lambda b: (b.head - bone.head).length)
            reach = (aim.head - bone.head).length
        else:
            reach = (bone.parent.length * A_LEAF_IS) if bone.parent else bone.length
        if reach < 1e-5:
            continue
        if bone.length > reach * 2.0:
            spikes += 1
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
    for bone in rig.data.bones:
        bone.hide = HIDE in bone.name
    print(f"  {sum(1 for b in rig.data.bones if b.hide)} twist bones hidden; alt-H shows them")

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

    if save_to:
        bpy.ops.wm.save_as_mainfile(filepath=save_to)
        print(f"saved {save_to}")


if __name__ == "__main__":
    main()
