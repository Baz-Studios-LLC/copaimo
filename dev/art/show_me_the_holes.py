"""Opens the ranger with the REAL holes selected, ready to fill by hand.

Run through `dev/art/show_me_the_holes.sh`, which opens `ranger.blend` with this.

# Why Blender's own Select Non-Manifold is no use here

glTF encodes hard edges by SPLITTING vertices, so on this mesh 6975 of 10131 edges have
exactly one face and Blender calls all of them boundary. Select Non-Manifold picks every
hard-edge seam on the character - thousands of them - and the three actual holes are lost in
it. Welded by position first, 7062 split vertices are 2302 real ones, and only 140 edges are
genuinely open.

So this welds by position, finds the loops that are really open, and selects only the small
CLOSED ones. The big open chains are left alone because they are meant to be open: a
41-vertex chain on the spine is the jacket's zip, 38 closed vertices at the clavicles are the
collar, 26 on the head are the hairline. Filling any of those would be worse than the holes.

Face select mode and the pivot moved to the selection, so F fills straight away.
"""
import collections
import sys

import bmesh
import bpy

BIGGEST = 8  # a puncture, not a garment opening


def the_body():
    """The biggest skinned mesh - the character rather than the backpack."""
    skinned = [
        o for o in bpy.data.objects
        if o.type == "MESH" and o.vertex_groups
    ]
    return max(skinned, key=lambda o: len(o.data.vertices)) if skinned else None


def main():
    mesh = the_body()
    if mesh is None:
        print("REFUSED: no skinned mesh in this file")
        return

    data = mesh.data
    weld, spot = {}, {}
    for vertex in data.vertices:
        where = (round(vertex.co.x, 5), round(vertex.co.y, 5), round(vertex.co.z, 5))
        spot.setdefault(where, len(spot))
        weld[vertex.index] = spot[where]

    faces = collections.Counter()
    for poly in data.polygons:
        ring = [weld[i] for i in poly.vertices]
        for i in range(len(ring)):
            a, b = ring[i], ring[(i + 1) % len(ring)]
            if a != b:
                faces[tuple(sorted((a, b)))] += 1

    beside = collections.defaultdict(set)
    for pair, count in faces.items():
        if count < 2:
            beside[pair[0]].add(pair[1])
            beside[pair[1]].add(pair[0])

    seen, wanted, holes = set(), set(), 0
    for start in list(beside):
        if start in seen:
            continue
        group, stack = [], [start]
        seen.add(start)
        while stack:
            here = stack.pop()
            group.append(here)
            for there in beside[here]:
                if there not in seen:
                    seen.add(there)
                    stack.append(there)
        if all(len(beside[i]) == 2 for i in group) and len(group) <= BIGGEST:
            holes += 1
            for i in group:
                for j in beside[i]:
                    wanted.add(tuple(sorted((i, j))))

    print(f"{len(data.vertices)} split vertices weld to {len(spot)}; "
          f"{sum(1 for c in faces.values() if c < 2)} edges genuinely open")
    print(f"{holes} small closed holes to fill, {len(wanted)} welded edges")

    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="VERT")
    working = bmesh.from_edit_mesh(data)
    for face in working.faces:
        face.select_set(False)
    for edge in working.edges:
        edge.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    picked = 0
    for edge in working.edges:
        pair = tuple(sorted((weld[edge.verts[0].index], weld[edge.verts[1].index])))
        if pair in wanted:
            edge.select_set(True)
            picked += 1
    bmesh.update_edit_mesh(data)
    print(f"selected {picked} edges of the mesh's own topology")
    print("F fills a selection; Alt-F fills a whole boundary. The zip, the collar and the "
          "hairline are deliberately NOT selected.")

    for area in bpy.context.screen.areas:
        if area.type == "VIEW_3D":
            for region in area.regions:
                if region.type == "WINDOW":
                    with bpy.context.temp_override(area=area, region=region):
                        try:
                            bpy.ops.view3d.view_selected()
                        except RuntimeError:
                            pass


main()
