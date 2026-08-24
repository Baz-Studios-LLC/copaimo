"""What the torso's bones are and what each one actually drives.

Read-only. Run before changing the spine, because the hierarchy and the SKINNING are two
different things on this rig and it is the skinning that decides where a new joint belongs.

`docs/rigging.md` puts a standard game spine at five joints - pelvis, three spine, and a chest -
against which this rig has Pelvis, Waist, Spine01 and Spine02. The question this answers is
whether that missing fifth is really a chest, and if so where the line between it and Spine02
falls: not by eye, but by where the vertices Spine02 drives actually sit.
"""
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

GLB = os.environ.get("SPINE_GLB", os.path.join(ART, "ranger_apose.glb").replace("\\", "/"))
SCALE = 170.0
TORSO = ("Root", "Hip", "Pelvis", "Waist", "Spine01", "Spine02",
         "NeckTwist01", "NeckTwist02", "Head", "L_Clavicle", "R_Clavicle")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig  # noqa: E402

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, up = prepare_rig.body_frame(rig)

groups = {g.index: g.name for g in mesh.vertex_groups}
owned = {}
for vertex in mesh.data.vertices:
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    owned.setdefault(who, []).append(mesh.matrix_world @ vertex.co)

print(f"{GLB}\n{len(rig.data.bones)} bones\n")
print(f"  {'bone':<14} {'head z':>8} {'tail z':>8} {'drives':>7} "
      f"{'its skin runs':>22}  parent")
for name in TORSO:
    bone = rig.data.bones.get(name)
    if bone is None:
        print(f"  {name:<14} -- absent --")
        continue
    head = (rig.matrix_world @ bone.matrix_local.translation).z * SCALE
    tail = (rig.matrix_world @ (bone.matrix_local
                                @ mathutils.Vector((0.0, bone.length, 0.0)))).z * SCALE
    mine = owned.get(name, [])
    span = (f"{min(p.z for p in mine) * SCALE:6.1f} to {max(p.z for p in mine) * SCALE:6.1f}"
            if mine else "".rjust(16))
    print(f"  {name:<14} {head:7.1f} {tail:7.1f} {len(mine):7} {span:>22}  "
          f"{bone.parent.name if bone.parent else '-'}")

# Where the arms and neck leave the torso: a chest bone belongs at or just below that line,
# because its job is to carry them.
print()
for name in ("L_Clavicle", "R_Clavicle", "NeckTwist01"):
    bone = rig.data.bones.get(name)
    if bone:
        at = (rig.matrix_world @ bone.matrix_local.translation).z * SCALE
        print(f"  {name} leaves the spine at z {at:.1f} cm, from {bone.parent.name}")

# And how Spine02's own skin is distributed up the body, which is what decides whether it is
# carrying two joints' worth of work.
spine02 = owned.get("Spine02", [])
if spine02:
    zs = sorted(p.z * SCALE for p in spine02)
    marks = [zs[int(len(zs) * f)] for f in (0.0, 0.25, 0.5, 0.75, 0.999)]
    print(f"\n  Spine02 drives {len(zs)} vertices; their heights at the quarters: "
          + ", ".join(f"{m:.1f}" for m in marks))
    tall = zs[-1] - zs[0]
    print(f"  that is a band {tall:.1f} cm tall on a {max(p.z for v in owned.values() for p in v) * SCALE:.1f} cm figure")
