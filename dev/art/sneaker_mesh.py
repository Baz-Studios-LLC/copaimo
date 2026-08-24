"""Builds real sneakers and puts them on the character.

    blender --background --python sneaker_mesh.py -- [--dry-run]

# Why the shoe is built rather than reshaped

Counted welded, each delivered shoe is 64 vertices and 113 triangles, and measured through the
ankle the shoe and the leg are one smooth tube with no edge at any height. There is no collar
rim, no tongue, no throat and no sole unit anywhere in it: all of that is PAINTED. Seven passes
tried to reshape it into a sneaker and could not, because the features that make a sneaker were
not there to reshape. See `shoe_form.py`, kept uncalled, and TROUBLESHOOTING.md.

So this generates the shoe: rings of vertices at stations from heel to toe, swept along the
foot's own axis, with a cross-section that has a flat sole, a hard edge where the midsole meets
the upper, and a collar that dips into a throat the ankle comes out of.

# The old shoe becomes the foot inside the new one

It is NOT deleted. Deleting it would leave the leg ending in an open ring at the ankle, which
then has to be found by position (the mesh is split, so its connectivity is not in the file),
extruded downward and capped - three operations that can each go wrong invisibly.

Instead it is drawn IN toward the foot's axis, fading to nothing near the collar so the ankle
stays exactly where the leg meets it. What is left is a foot-shaped shell inside a shoe-shaped
shell, which is how character footwear is normally built anyway. The cost is 226 triangles that
are never seen; the saving is that the leg is never detached from anything.

`the_foot_stays_inside` measures it afterwards rather than trusting the description.

# The atlas had to grow, and the two halves have to agree

Rasterising every UV triangle: the atlas is 90.5% covered and the largest empty square anywhere
is 224 px, which does not change if the old shoe's islands are freed - its 3.7% of UV area is
scattered over the whole sheet rather than sitting in one block.

So `ranger_texture.py` grows the base-colour map to 4096 x 4608 and paints
`sneaker_paint.paint()` into the new rows, and everything already on the sheet is pushed up into
the top 4096 of it. This file applies the MATCHING move to the mesh's UVs. Both read
`STRIP_IS` from one place, because a number written twice is a number that will disagree with
itself.
"""
import math
import os
import sys

import bmesh
import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

import prepare_rig  # noqa: E402
import shoe_form  # noqa: E402  (its last tables and its frame measurement)
import sneaker_paint  # noqa: E402

SCALE = 170.0
UP = mathutils.Vector((0.0, 0.0, 1.0))

# How tall the new strip is as a share of the grown atlas. Everything already on the sheet ends
# up in the top 1 - STRIP_IS of it.
STRIP_IS = sneaker_paint.STRIP[1] / (4096.0 + sneaker_paint.STRIP[1])

# How finely the shoe is swept. 26 stations by 16 points is 416 vertices and about 800 triangles
# a shoe, against 64 and 113 - which is where a game sneaker sits.
STATIONS = 26
AROUND = 16

# The footprint, as shares of standing height and of the shoe's own length.
LONG_AS_A_SHARE_OF_HEIGHT = 0.152
WIDE_AS_A_SHARE_OF_LENGTH = 0.375

# The sole slab, in cm, and where the hard edge at the top of it sits in the cross-section.
SOLE_IS_THICK = 2.0
SOLE_EDGE_AT = 0.19          # share of the way up the normalised cross-section

# THE CROSS-SECTION, normalised: x across (-1 to 1), y up (0 to 1). Read anticlockwise from the
# middle of the sole, so index 0 is under the foot and index AROUND/2 is the top of the instep -
# which is the parameterisation `sneaker_paint` paints against.
#
# The two points at x = +/-1.00 either side of SOLE_EDGE_AT are the midsole's hard edge: a
# vertical wall under a corner, which is what makes a sole read as a separate part rather than
# as the bottom of a sock.
SECTION = (
    (0.00, 0.00), (0.52, 0.00), (0.88, 0.005), (1.00, 0.06), (1.00, 0.19),
    (0.96, 0.33), (0.86, 0.55), (0.60, 0.82), (0.00, 1.00), (-0.60, 0.82),
    (-0.86, 0.55), (-0.96, 0.33), (-1.00, 0.19), (-1.00, 0.06), (-0.88, 0.005),
    (-0.52, 0.00),
)

# THE COLLAR. Over these stations the top of the shoe dips into a throat that the ankle comes out
# of, instead of closing over it. The dip is what puts a RIM on the shoe.
COLLAR_FROM, COLLAR_TO = 0.06, 0.44
COLLAR_DIPS = 0.55           # share of the section's height that the throat drops to
COLLAR_WIDENS = 1.06         # the rim flares slightly, so it stands proud of the ankle

# How far the old shoe is drawn in to become the hidden foot, and the height above which it is
# left alone so the ankle stays joined to the leg.
FOOT_DRAWS_IN_TO = 0.72
FOOT_IS_LEFT_ALONE_ABOVE = 6.5   # cm

# How much of the shoe's length the heel and the toe round off over, and how far the section is
# drawn in at the very end.
#
# Without this the end caps are a flat face the full width and height of the section - the first
# build came out with a squared-off toe like a boot. The sole is NOT lifted by it: only the width
# and the upper come in, because a sprung toe is the mistake the previous attempt made.
ROUNDS_OFF_WITHIN = 0.08
ROUNDS_OFF_TO = 0.34

# What the result has to satisfy.
FOOT_CLEARS_BY = 0.15        # cm the sneaker's surface must stand outside the hidden foot
SNEAKER_IS_LOW = 0.34        # tallest point, as a share of the shoe's own length


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def smoothly(at):
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def make_room_in_the_atlas(mesh):
    """Pushes every existing UV up into the top of the grown atlas.

    The image gains rows at the BOTTOM, so v = 0 stops being the bottom of the old sheet and
    becomes the bottom of the new strip. Everything already mapped has to move up by exactly the
    strip's share and shrink by what is left. Exact arithmetic, and `the_atlas_still_lines_up`
    checks a known point afterwards.
    """
    keeps = 1.0 - STRIP_IS
    for layer in mesh.data.uv_layers:
        for spot in layer.data:
            spot.uv.y = STRIP_IS + spot.uv.y * keeps
    print(f"  every UV moved into the top {keeps * 100:.1f}% of the atlas, leaving the bottom "
          f"{STRIP_IS * 100:.1f}% ({sneaker_paint.STRIP[1]} rows) for the sneakers")


def the_frame(mesh, indices, forward, floor):
    """The foot's own axes and where it sits, measured off the shoe that is there now."""
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, middle = shoe_form.frame_of(spots, forward)
    return long_way, wide_way, length, heel, middle


def profile_at(along, half):
    """How wide and how tall the sneaker is at one station, in cm.

    ONE function, used both to build the shoe and to tuck the old one under it. They were two
    copies of the same arithmetic for one build, the rounding was added to only one of them, and
    the hidden foot came straight out through the toe.
    """
    cap = ROUNDS_OFF_TO + (1.0 - ROUNDS_OFF_TO) * smoothly(
        min(along, 1.0 - along) / ROUNDS_OFF_WITHIN)
    wide = half * shoe_form.along_the_table(shoe_form.WIDTH_OF_THE_SHOE, along) * cap
    top = shoe_form.along_the_table(shoe_form.TOP_OF_THE_SHOE, along) * (0.45 + 0.55 * cap)
    return wide, top


def tuck_the_foot_under(mesh, indices, long_way, wide_way, heel, middle, floor, span, half):
    """Puts the old shoe underneath the new one's own profile, station by station.

    Not a blanket shrink. The first two versions scaled by a constant faded by height, and the
    old shoe still stood 9-10 cm tall at stations where the sneaker is 5 - so the thing meant to
    be hidden was a hand's breadth outside it. Fitted against the SAME tables the sneaker is
    built from, this converges by construction: whatever profile the shoe has, the foot ends up
    under it.

    The exception is the throat. Inside the collar stations and near the centreline, a vertex is
    left where it is - that is the ankle, and it is supposed to come out of the shoe.
    """
    into_mesh = mesh.matrix_world.inverted()
    starts, ends = span
    length = max(ends - starts, 1e-9)
    room = FOOT_CLEARS_BY * 2.0
    tucked = 0
    for index in indices:
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        at = min(1.0, max(0.0, (spot.dot(long_way) - starts) / length))
        above = (spot.z - floor) * SCALE
        across = spot.dot(wide_way) - middle
        wide, top = profile_at(at, half)

        in_the_throat = (COLLAR_FROM <= at <= COLLAR_TO
                         and abs(across) * SCALE < wide * 0.62)
        if not in_the_throat and above > top - room:
            spot.z = floor + max(0.0, top - room) / SCALE * (above / max(above, 1e-9))
            spot.z = floor + max(0.0, top - room) / SCALE
            tucked += 1
        if abs(across) * SCALE > wide - room:
            keep = max(0.0, wide - room) / max(abs(across) * SCALE, 1e-9)
            spot += wide_way * across * (keep - 1.0)
            tucked += 1
        # And inside the shoe's own span, so the heel and toe do not stand out of either end.
        along = spot.dot(long_way)
        if along < starts + room / SCALE:
            spot += long_way * (starts + room / SCALE - along)
        elif along > ends - room / SCALE:
            spot += long_way * (ends - room / SCALE - along)
        mesh.data.vertices[index].co = into_mesh @ spot
    return tucked


def section_at(along):
    """The cross-section for one station: the normalised ring, with the collar dipped into it."""
    ring = [mathutils.Vector(p) for p in SECTION]
    if not (COLLAR_FROM <= along <= COLLAR_TO):
        return ring, 1.0
    # A raised-cosine so the throat opens and closes smoothly rather than in two steps.
    deep = 0.5 - 0.5 * math.cos(
        2.0 * math.pi * (along - COLLAR_FROM) / (COLLAR_TO - COLLAR_FROM))
    out = []
    for index, point in enumerate(ring):
        turn = index / len(ring)
        # How far round the ring from the top of the instep, 0 at the top.
        from_top = abs(((turn + 0.5) % 1.0) - 0.5)
        from_top = abs(0.5 - turn)
        near = 1.0 - smoothly(from_top / 0.34)
        drop = deep * near
        out.append(mathutils.Vector((
            point.x * (1.0 + (COLLAR_WIDENS - 1.0) * drop),
            point.y * (1.0 - (1.0 - COLLAR_DIPS) * drop),
        )))
    return out, 1.0


def build_one(name, long_way, wide_way, heel, middle, floor, length, tall, material):
    """Makes one sneaker as its own object: vertices, faces, UVs and a smooth shading basis."""
    wants_long = tall * LONG_AS_A_SHARE_OF_HEIGHT
    half = wants_long * WIDE_AS_A_SHARE_OF_LENGTH * 0.5
    start = heel + (length * SCALE - wants_long) * 0.5 / SCALE

    verts, uvs = [], []
    for s in range(STATIONS):
        along = s / (STATIONS - 1.0)
        ring, _ = section_at(along)
        wide, top = profile_at(along, half)
        # Positions are built from PROJECTIONS on the foot's own axes, not by adding vectors to
        # a scalar. `heel` and `middle` are how far along and across the shoe sits, and
        # long_way / wide_way are horizontal unit vectors, so height is added along Z on its own.
        base = start + along * wants_long / SCALE
        for index, point in enumerate(ring):
            # The sole is a fixed thickness, so the wall under the hard edge does not thin out
            # toward the toe with the rest of the shoe.
            if point.y <= SOLE_EDGE_AT:
                up = SOLE_IS_THICK * point.y / SOLE_EDGE_AT
            else:
                up = SOLE_IS_THICK + (point.y - SOLE_EDGE_AT) / (1.0 - SOLE_EDGE_AT) * \
                    max(top - SOLE_IS_THICK, 0.2)
            spot = (long_way * base
                    + wide_way * (middle + point.x * wide / SCALE))
            spot.z = floor + up / SCALE
            verts.append(spot)
            uvs.append((along, STRIP_IS * (1.0 - index / AROUND)))

    faces = []
    for s in range(STATIONS - 1):
        for index in range(AROUND):
            a = s * AROUND + index
            b = s * AROUND + (index + 1) % AROUND
            faces.append((a, b, b + AROUND, a + AROUND))
    # Capped at both ends, or the shoe is a tube with the heel and the toe open.
    faces.append(tuple(range(AROUND - 1, -1, -1)))
    faces.append(tuple(range((STATIONS - 1) * AROUND, STATIONS * AROUND)))

    made = bpy.data.meshes.new(name)
    made.from_pydata([v for v in verts], [], faces)
    made.update()
    layer = made.uv_layers.new(name="UVMap")
    for poly in made.polygons:
        poly.use_smooth = True
        for loop in poly.loop_indices:
            layer.data[loop].uv = uvs[made.loops[loop].vertex_index]
    if material:
        made.materials.append(material)
    return bpy.data.objects.new(name, made)


def weigh_it(shoe, rig, side, long_way, heel, length):
    """Skins the new shoe: the forefoot to the toe bone, the rest to the foot."""
    foot = shoe.vertex_groups.new(name=f"{side}_Foot")
    toe = shoe.vertex_groups.new(name=f"{side}_ToeBase")
    for index, vertex in enumerate(shoe.data.vertices):
        along = ((shoe.matrix_world @ vertex.co).dot(long_way) - heel) / max(length, 1e-9)
        # Blended across the ball rather than switched at it, so the shoe bends there instead
        # of creasing along one ring of vertices.
        share = smoothly((along - 0.62) / 0.16)
        foot.add([index], 1.0 - share, "REPLACE")
        toe.add([index], share, "REPLACE")


def the_foot_stays_inside(hidden, shell, long_way, wide_way, heel, middle, length, side):
    """Refuses if the hidden foot reaches wider or taller than the sneaker around it.

    Compared station by station rather than by overall bounding box: a foot can fit inside a
    shoe's extents and still poke through its toe, and the box would not notice.
    """
    def bands(points):
        out = {}
        for spot in points:
            at = min(9, max(0, int((spot.dot(long_way) - heel) / max(length, 1e-9) * 10)))
            wide, high = out.setdefault(at, [0.0, 0.0])
            # OFFSETS from the shoe's own centreline, not absolute projections. The first
            # version compared `spot.dot(wide_way)` raw, which is a distance from the world
            # origin and says nothing about whether one shape is inside another.
            out[at] = [max(wide, abs(spot.dot(wide_way) - middle)), max(high, spot.z)]
        return out

    inside, outside = bands(hidden), bands(shell)
    # Only the forefoot. The ankle is SUPPOSED to come out through the collar, so measuring it
    # against a shoe with a hole there would refuse the thing working correctly.
    worst, where = 99.0, None
    for at, (wide, high) in inside.items():
        if at not in outside or at < COLLAR_TO * 10:
            continue
        clear = min(outside[at][0] - wide, outside[at][1] - high) * SCALE
        if clear < worst:
            worst, where = clear, at
    print(f"    {side}: the hidden foot clears the sneaker by {worst:+.2f} cm at its "
          f"tightest (station {where})")
    if worst < FOOT_CLEARS_BY:
        for at in sorted(set(inside) & set(outside)):
            print(f"      station {at}: foot {inside[at][0] * SCALE:5.2f} wide "
                  f"{(inside[at][1]) * SCALE:6.2f} z   shoe {outside[at][0] * SCALE:5.2f} wide "
                  f"{(outside[at][1]) * SCALE:6.2f} z")
        refuse(f"the {side} foot inside the shoe comes within {worst:+.2f} cm of its surface, "
               f"under {FOOT_CLEARS_BY} - it will poke through")


def main():
    every = argv()
    args = [a for a in every if not a.startswith("--")]
    dry = "--dry-run" in every
    source = args[0] if args else os.path.join(ART, "ranger_apose.glb").replace("\\", "/")
    out_path = args[1] if len(args) > 1 else source

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    prepare_rig.drop_the_widgets(rig)
    _, forward, _ = prepare_rig.body_frame(rig)
    print(f"reading {source}")

    mine = shoe_form.shoe_vertices(mesh)
    floor = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    tall = (max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices) - floor) * SCALE

    lowest = min(spot.uv.y for layer in mesh.data.uv_layers for spot in layer.data)
    if lowest >= STRIP_IS - 1e-4:
        refuse(f"the lowest UV is already at v={lowest:.4f}, at or above the strip - the atlas "
               f"has been grown before. Restore the asset from git rather than growing it twice.")
    make_room_in_the_atlas(mesh)

    material = mesh.data.materials[0] if mesh.data.materials else None
    built = []
    for side in "LR":
        long_way, wide_way, length, heel, middle = the_frame(mesh, mine[side], forward, floor)
        print(f"  {side}: the old shoe is {length * SCALE:.1f} cm long; building a "
              f"{tall * LONG_AS_A_SHARE_OF_HEIGHT:.1f} cm sneaker on the same axis")
        wants_long = tall * LONG_AS_A_SHARE_OF_HEIGHT
        starts = heel + (length * SCALE - wants_long) * 0.5 / SCALE
        half = wants_long * WIDE_AS_A_SHARE_OF_LENGTH * 0.5
        tucked = tuck_the_foot_under(
            mesh, mine[side], long_way, wide_way, heel, middle, floor,
            (starts, starts + wants_long / SCALE), half)
        print(f"    {tucked} of the old shoe's vertices tucked under the new one")
        shoe = build_one(f"sneaker_{side}", long_way, wide_way, heel, middle, floor,
                         length, tall, material)
        bpy.context.scene.collection.objects.link(shoe)
        weigh_it(shoe, rig, side, long_way, heel, tall * LONG_AS_A_SHARE_OF_HEIGHT / SCALE)
        the_foot_stays_inside(
            [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]],
            [shoe.matrix_world @ v.co for v in shoe.data.vertices],
            long_way, wide_way, heel, middle, length, side)
        print(f"    {len(shoe.data.vertices)} vertices, {len(shoe.data.polygons)} faces")
        built.append(shoe)

    bpy.ops.object.select_all(action="DESELECT")
    for shoe in built:
        shoe.select_set(True)
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.join()
    print(f"  joined: the body is now {len(mesh.data.vertices)} vertices, "
          f"{len(mesh.data.polygons)} faces")

    prepare_rig.check_the_skin(mesh)
    if dry:
        print("\ndry run, nothing written")
        return
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=out_path, export_format="GLB", use_selection=True,
        export_yup=True, export_apply=False, export_animations=False,
    )
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()
