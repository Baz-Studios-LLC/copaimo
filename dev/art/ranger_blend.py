"""Writes `dev/art/ranger.blend` — the ranger, rigged and animated, to open by hand.

    dev/art/ranger_blend.sh

# Why this exists

Every piece of work on this character happens in BATCH Blender: a headless process
that imports the GLB, does one job, exports, and exits. Nothing it does ever appears
in an open Blender window, so somebody watching Blender sees an empty scene (or
whatever a live script last built) and reasonably asks where the character is.

The live Blender session cannot import a GLB at all — the add-on that drives it runs
code in a context without `bpy.context.object`, and the glTF importer needs it while
setting up armature display. So the character cannot simply be loaded into the open
window on request.

This closes the gap the other way: it writes a .blend with the model, the rig and all
three clips already in it, ready to open. Rebuilt from the game's own copy, so what
opens is exactly what the game loads.
"""

import os
import sys

import bpy


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(os.path.dirname(here))
    src = os.path.join(root, "assets", "models", "person_ranger.glb")
    out = os.path.join(here, "ranger.blend")
    if not os.path.isfile(src):
        raise SystemExit(f"no {src} — run dev/art/animate_ranger.sh first")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is not None:
        # The walk up front, since that is what anybody opening this wants to scrub.
        walk = bpy.data.actions.get("walk")
        if walk is not None:
            rig.animation_data.action = walk
            low, high = (int(v) for v in walk.frame_range)
            bpy.context.scene.frame_start = low
            bpy.context.scene.frame_end = high
        for action in bpy.data.actions:
            action.use_fake_user = True
        rig.show_in_front = True
        rig.data.display_type = "OCTAHEDRAL"

    # Packed, so the .blend carries its own textures and can be opened anywhere.
    bpy.ops.file.pack_all()
    bpy.ops.wm.save_as_mainfile(filepath=out)
    print(f"WROTE {out}")
    print("clips:", [a.name for a in bpy.data.actions])


main()
