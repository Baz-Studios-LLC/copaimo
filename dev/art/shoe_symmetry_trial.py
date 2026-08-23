"""Would projecting one shoe onto the other's mirrored SURFACE work? Measured, not applied.

Vertex-level mirroring is not available here. The two shoes are differently tessellated -
179 right-side vertices find their nearest mirror partner among only 41 of the left's 190,
so 77% of the mapping collides and a nearest-partner copy would collapse the shoe onto a
fortieth of its detail rather than symmetrise it.

Surface projection has no such problem: each vertex slides onto the mirrored surface of the
other side, so differing tessellation is irrelevant - only the SHAPES have to agree, which
is what reads. Topology, UVs and weights are untouched; only positions move.

What could still go wrong, and is therefore measured here:

* how far vertices have to travel, since a big move is a redesign and not a correction
* what it does to EDGE LENGTHS, because sliding vertices onto a surface can bunch them
* whether the custom split normals still describe the surface afterwards - the pipeline's
  hardest-won rule is that no geometry guard can see shading, so this reports the angle
  each stored normal ends up making with its face

Read-only. Nothing is written and nothing is exported.
"""
import statistics
import sys

import bpy
import mathutils
from mathutils.bvhtree import BVHTree

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
across, _, _ = prepare_rig.body_frame(rig)
lateral = max(range(3), key=lambda i: abs(across[i]))
data = mesh.data

groups = {g.index: g.name for g in mesh.vertex_groups}


def owner(vertex):
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    return who


def region(parts):
    """Vertex indices per side for a set of bone-name fragments."""
    out = {"L": set(), "R": set()}
    for vertex in data.vertices:
        name = owner(vertex)
        if name[:2] in ("L_", "R_") and any(p in name for p in parts):
            out[name[0]].add(vertex.index)
    return out


spots = [mesh.matrix_world @ v.co for v in data.vertices]


def mirrored_surface(indices):
    """A BVH of these vertices' faces, reflected across the midline."""
    faces = [
        p.vertices[:] for p in data.polygons
        if all(i in indices for i in p.vertices)
    ]
    flipped = []
    for p in spots:
        q = mathutils.Vector(p)
        q[lateral] = -q[lateral]
        flipped.append(q)
    return BVHTree.FromPolygons(flipped, faces, all_triangles=False), len(faces)


for parts, label in ((("Foot", "ToeBase"), "shoe"), (("CalfTwist",), "shin")):
    sides = region(parts)
    tree, faces = mirrored_surface(sides["L"])
    if faces == 0:
        print(f"{label}: no whole faces inside the region - cannot project")
        continue

    moves = {}
    for i in sides["R"]:
        hit = tree.find_nearest(spots[i])
        if hit[0] is None:
            continue
        moves[i] = hit[0]

    travelled = sorted((moves[i] - spots[i]).length * SCALE for i in moves)
    print(f"\n{label}: {len(sides['R'])} vertices on the right, {faces} whole faces on the "
          f"left to project onto, {len(moves)} found a target")
    print(f"  travel: median {travelled[len(travelled) // 2]:.2f} cm, "
          f"90th {travelled[int(len(travelled) * 0.9)]:.2f}, worst {travelled[-1]:.2f}")

    # Edge lengths, before and after, for edges wholly inside the moved set.
    changed = []
    for edge in data.edges:
        a, b = edge.vertices
        if a in moves and b in moves:
            was = (spots[a] - spots[b]).length
            now = (moves[a] - moves[b]).length
            if was > 1.0e-9:
                changed.append(now / was)
    if changed:
        changed.sort()
        print(f"  edge lengths: median x{changed[len(changed) // 2]:.3f}, "
              f"shortest x{changed[0]:.3f}, longest x{changed[-1]:.3f} "
              f"({sum(1 for c in changed if c < 0.5 or c > 2.0)} past half or double)")

    # Do the stored normals still describe the surface? Faces wholly inside the moved set.
    if data.has_custom_normals:
        data.calc_normals_split()
        angles = []
        for poly in data.polygons:
            if not all(i in moves for i in poly.vertices):
                continue
            ring = [moves[i] for i in poly.vertices]
            if len(ring) < 3:
                continue
            face = (ring[1] - ring[0]).cross(ring[2] - ring[0])
            if face.length < 1e-9:
                continue
            face.normalize()
            for corner in poly.loop_indices:
                stored = mathutils.Vector(data.loops[corner].normal)
                stored = (mesh.matrix_world.to_3x3() @ stored).normalized()
                angles.append(stored.angle(face))
        if angles:
            worst = max(angles)
            print(f"  stored normals against the new faces: median "
                  f"{statistics.median(angles) * 57.2958:.1f} deg, worst "
                  f"{worst * 57.2958:.1f} deg")
            print("  (a normal far from its own face is the shading fault welding caused - "
                  "if this is large, the projection needs the normals recomputed)")
