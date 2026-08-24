"""Opens the RAW export for you to select the junk, and reads back what you picked.

    dev/art/pick_the_junk.sh          # opens Blender; select, save, close
    dev/art/pick_the_junk.sh --read   # writes what you selected into junk_to_remove.json

# Why this exists

Five heuristics have been written to find the generator's stray geometry by rule - by shell
size, by bone ownership, by face area, by long edges between distant bones, by distance from
the limb axis - and three of them removed real parts of the character: the trouser leg, the
sleeve cuffs, and the shoulder. The rules are not badly tuned. "A long thin face spanning
distant bones" IS a hanging strap, and it is ALSO a shoulder; nothing in the geometry
separates them, so no threshold can.

What separates them is looking at it. So this stops guessing: the junk gets named once, by
eye, and removed by identity on every build afterwards.

# Why the RAW export and not the built asset

`prepare_rig` mirrors, centres, A-poses and bakes, so a vertex is in a different place at the
end than at the start. A list of positions picked off the finished asset would not match
anything at the point where the removal has to happen. Picked off the raw export, the
positions are exactly what the first step of the pipeline sees.

Identity is POSITION, rounded to a hundredth of a millimetre, not index. Indices shift the
moment anything is deleted; positions do not.
"""
import json
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "..", "..", "Ranger_Rig_Idle.glb")
PICKED = os.path.join(HERE, "junk_to_remove.json")
ROUND = 5


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def the_body():
    skinned = [o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups]
    return max(skinned, key=lambda o: len(o.data.vertices)) if skinned else None


def build():
    """A scene holding the raw mesh, ready to select in."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=os.path.abspath(RAW))
    mesh = the_body()
    if mesh is None:
        raise SystemExit("REFUSED: no skinned mesh in the raw export")

    # Anything already on the list starts selected, so a pick can be added to or trimmed
    # rather than started from nothing each time.
    already = set()
    if os.path.exists(PICKED):
        with open(PICKED) as handle:
            already = {tuple(v) for v in json.load(handle)["positions"]}
    for vertex in mesh.data.vertices:
        key = tuple(round(c, ROUND) for c in vertex.co)
        vertex.select = key in already
    print(f"{len(mesh.data.vertices)} vertices; {sum(1 for v in mesh.data.vertices if v.select)} "
          f"already on the list and pre-selected")

    bpy.context.view_layer.objects.active = mesh
    mesh.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="FACE")
    where = os.path.join(HERE, "pick_the_junk.blend")
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.wm.save_as_mainfile(filepath=where)
    print(f"WROTE {where}")


def read():
    """Whatever is selected in pick_the_junk.blend becomes the removal list."""
    mesh = the_body()
    if mesh is None:
        raise SystemExit("REFUSED: no skinned mesh in this file")
    picked = [
        [round(c, ROUND) for c in v.co]
        for v in mesh.data.vertices if v.select
    ]
    if not picked:
        print("nothing is selected, so nothing was written - the list is unchanged")
        return
    with open(PICKED, "w") as handle:
        json.dump({"positions": picked}, handle, indent=1)
    low = min(v[2] for v in picked)
    high = max(v[2] for v in picked)
    print(f"WROTE {PICKED}: {len(picked)} vertices, z {low * 170.0:.1f} to "
          f"{high * 170.0:.1f} cm")


if "--read" in argv():
    read()
else:
    build()
