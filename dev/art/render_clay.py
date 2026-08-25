"""Renders the character as FORM: no texture, no material, just the shape.

    dev/art/render_clay.sh                       every shot, rest pose
    dev/art/render_clay.sh --only head,hands     just those
    dev/art/render_clay.sh --clip walk --frame 9 posed
    dev/art/render_clay.sh --textured            the same shots with the material on

Stage 00 of `docs/character-pipeline.md`. This is the instrument whose absence was the single
most expensive gap in the last character: a shoe was reported wrong four separate times, and
every render taken of it was TEXTURED. The paint hides the shape it is painted on - a
sixty-four-vertex blob read as a trainer in every one of them, and as a sock the moment the
material came off.

So the default is clay and `--textured` is the exception, which is the way round it should have
been all along.

# The silhouette pass

`--silhouette` renders flat black against white. It is the oldest test in character design and
the cheapest: if the shape does not read at thumbnail size in solid black, no amount of texture
will save it at distance. Worth running on any change to the outline.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

# (name, degrees around the figure, how wide the frame is as a share of standing height,
#  where to aim as a share of standing height)
#
# The close shots are aimed at the parts that carry a character: the face, the hands and the
# feet. Everything else about a stylised figure can be a little loose and still read.
SHOTS = (
    ("front", 0.0, 1.15, 0.50),
    ("side", 90.0, 1.15, 0.50),
    ("back", 180.0, 1.15, 0.50),
    ("quarter", 40.0, 1.15, 0.50),
    ("head", 15.0, 0.22, 0.92),
    ("hands", 0.0, 0.42, 0.52),
    ("hands_side", 90.0, 0.42, 0.52),
    ("feet", 35.0, 0.30, 0.06),
    ("feet_side", 90.0, 0.30, 0.06),
    ("torso", 20.0, 0.55, 0.68),
    # Aimed at the HAND BONE rather than at a height, because a hand hangs at the side of the
    # figure and a shot centred on the body's midline frames it at the very edge - which is what
    # made a first attempt at judging the palms unreadable.
    ("hand_L", 0.0, 0.16, "L_Hand"),
    ("hand_L_side", 90.0, 0.16, "L_Hand"),
    ("hand_R", 0.0, 0.16, "R_Hand"),
    # Under the arm, aimed at the upper-arm bone. This is where a generator webs a limb to the
    # body it was resting against, and it is not visible in any of the shots above.
    ("armpit_L", 25.0, 0.30, "L_Upperarm"),
    ("armpit_L_front", 0.0, 0.30, "L_Upperarm"),
    ("armpit_R", -25.0, 0.30, "R_Upperarm"),
    # The elbows, reported as digging into the mesh in the idle. Aimed at the forearm's root,
    # which is where the joint is, from behind and outside where a crease shows.
    ("elbow_L", 35.0, 0.26, "L_Forearm"),
    ("elbow_R", -35.0, 0.26, "R_Forearm"),
)

WIDE = (700, 900)


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def flag(name, fallback=None):
    args = argv()
    return args[args.index(name) + 1] if name in args else fallback


def mark_the_faults(mesh):
    """The faces worth pointing at: holes, non-manifold edges, and bridges.

    The same three the audit counts, found the same way - welded by position first, because on
    an unwelded glTF mesh every edge looks like a boundary.
    """
    import collections

    grain = 0.00002
    canon, seen = {}, {}
    for vertex in mesh.data.vertices:
        co = vertex.co
        key = (round(co.x / grain), round(co.y / grain), round(co.z / grain))
        canon[vertex.index] = seen.setdefault(key, vertex.index)

    faces_on = collections.defaultdict(list)
    for poly in mesh.data.polygons:
        corners = list(poly.vertices)
        for a, b in zip(corners, corners[1:] + corners[:1]):
            ca, cb = canon[a], canon[b]
            if ca != cb:
                faces_on[(min(ca, cb), max(ca, cb))].append(poly.index)

    lengths = sorted(
        ((mesh.matrix_world @ mesh.data.vertices[e.vertices[0]].co)
         - (mesh.matrix_world @ mesh.data.vertices[e.vertices[1]].co)).length
        for e in mesh.data.edges)
    median = lengths[len(lengths) // 2]

    out = set()
    for _, faces in faces_on.items():
        if len(faces) != 2:
            out.update(faces)
    # A long edge is only a fault if it also spans two body regions. Without that test this
    # picked out the chest panel and the crotch - ordinary large polygons on a low-poly body -
    # and called them bridges. 189 faces, of which almost none were wrong.
    groups = {g.index: g.name for g in mesh.vertex_groups}
    owner = {}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        owner[vertex.index] = who

    def region(name):
        for key, part in (("Thigh", "leg"), ("Calf", "leg"), ("Foot", "leg"), ("Toe", "leg"),
                          ("Upperarm", "arm"), ("Forearm", "arm"), ("Hand", "arm"),
                          ("Clavicle", "trunk"), ("Spine", "trunk"), ("Waist", "trunk"),
                          ("Hip", "trunk"), ("Pelvis", "trunk"),
                          ("Neck", "head"), ("Head", "head")):
            if key in name:
                return part
        return None

    for edge in mesh.data.edges:
        span = ((mesh.matrix_world @ mesh.data.vertices[edge.vertices[0]].co)
                - (mesh.matrix_world @ mesh.data.vertices[edge.vertices[1]].co)).length
        parts = {region(owner.get(edge.vertices[0], "")),
                 region(owner.get(edge.vertices[1], ""))} - {None}
        if span > median * 4.0 and len(parts) > 1:
            pair = (min(canon[edge.vertices[0]], canon[edge.vertices[1]]),
                    max(canon[edge.vertices[0]], canon[edge.vertices[1]]))
            out.update(faces_on.get(pair, []))
    return out


def main():
    args = argv()
    root = os.path.dirname(os.path.dirname(ART))
    model = flag("--model", os.path.join(root, "assets", "models", "person_ranger.glb"))
    out = flag("--out", os.path.join(root, "dev", "art", "clay"))
    # EXACT names, not substrings. `--only front` also matched `armpit_L_front`, and the golden
    # harness then kept whichever file sorted first - so `rest_front.png` in the kept sheet was
    # an armpit close-up. A gate comparing the wrong picture passes for the wrong reason.
    only = [w for w in (flag("--only", "") or "").split(",") if w]
    textured = "--textured" in args
    silhouette = "--silhouette" in args
    # Faces to pick out in red: a comma-separated list of indices, or "faults" to let
    # `mark_the_faults` find them.
    #
    # Nothing gets removed from this mesh without being POINTED AT first. Two removals on the
    # last character took the wrong thing - the sleeve cuffs once, faces out of a trouser leg
    # once - and both times the render that would have shown it was not taken or was misread.
    highlight = flag("--highlight")
    os.makedirs(out, exist_ok=True)

    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)
    bpy.ops.import_scene.gltf(filepath=model.replace("\\", "/"))

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    meshes = [o for o in bpy.data.objects if o.type == "MESH"]
    if not meshes:
        raise SystemExit("REFUSED: nothing to render")
    body = max((o for o in meshes if o.vertex_groups), key=lambda o: len(o.data.vertices))

    # POSED, before anything is measured off it - the camera aims at where the figure actually
    # is, and a clip moves that.
    #
    # And when no clip is asked for, the action is CLEARED. The glTF importer leaves one
    # assigned, so "the rest pose" was quietly frame one of whichever clip the file happened to
    # list first - a render labelled as one thing showing another, which is the whole failure
    # mode this tool exists to prevent.
    wanted = flag("--clip")
    if not wanted and rig is not None and rig.animation_data is not None:
        rig.animation_data.action = None
        bpy.context.view_layer.update()
        print("no clip asked for, so the action was cleared - this is the REST pose")
    if wanted and rig is not None:
        clip = next((a for a in bpy.data.actions if a.name == wanted), None)
        if clip is None:
            raise SystemExit(f"REFUSED: no clip called {wanted}; this file has "
                             + ", ".join(sorted(a.name for a in bpy.data.actions)))
        if rig.animation_data is None:
            rig.animation_data_create()
        rig.animation_data.action = clip
        # Blender 4.4 on: an action holds slots, and until one is bound it is attached and inert.
        slots = getattr(clip, "slots", None)
        if slots:
            rig.animation_data.action_slot = slots[0]
        bpy.context.scene.frame_set(int(flag("--frame", int(clip.frame_range[0]))))
        bpy.context.view_layer.update()
        print(f"posed by {wanted} at frame {bpy.context.scene.frame_current}")

    # Arms raised by so many degrees, on top of whatever pose is showing. The armpit webbing
    # only reads with the arm AWAY from the body, and no delivered clip ever lifts one - which
    # is exactly how it stayed invisible in every render until it was asked about.
    # Curl the fingers, to prove they exist and hinge the right way. `--curl 40` bends every
    # phalanx 40 degrees about its local X - the axis the build aligned so that local Z is the
    # palm normal. `--digit Thumb` curls one digit by name, which is how a naming claim gets
    # checked by eye instead of trusted.
    curl = flag("--curl")
    if curl and rig is not None:
        one = flag("--digit")
        wanted_digits = [one] if one else ["Thumb", "Index", "Middle", "Ring", "Pinky"]
        bent = 0
        for name in list(rig.pose.bones.keys()):
            if any(f"_{d}" in name and name[-1] in "123" for d in wanted_digits):
                bone = rig.pose.bones[name]
                bone.rotation_mode = "QUATERNION"
                bone.rotation_quaternion = (
                    mathutils.Quaternion((1.0, 0.0, 0.0), math.radians(float(curl)))
                    @ bone.rotation_quaternion)
                bent += 1
        bpy.context.view_layer.update()
        print(f"curled {bent} phalanges by {curl} degrees")

    # About local Z, signed per side, because that is what MEASURED as abduction: +70 deg of Z
    # takes the left wrist 0.23 units away from the spine, where X - the assumed axis - is
    # flexion and moves it forward instead. Y is the twist axis; 70 degrees of it moves the
    # wrist a millimetre and shows nothing.
    lift = flag("--lift")
    if lift and rig is not None:
        for name, sign in (("L_Upperarm", 1.0), ("R_Upperarm", -1.0)):
            if name in rig.pose.bones:
                bone = rig.pose.bones[name]
                bone.rotation_mode = "QUATERNION"
                bone.rotation_quaternion = (
                    mathutils.Quaternion((0.0, 0.0, 1.0), math.radians(float(lift) * sign))
                    @ bone.rotation_quaternion)
        bpy.context.view_layer.update()
        print(f"arms lifted {lift} degrees (local Z, the measured abduction axis)")

    marked = set()
    if highlight:
        marked = (mark_the_faults(body) if highlight == "faults"
                  else {int(w) for w in highlight.split(",") if w.strip()})
        print(f"pointing at {len(marked)} faces")

    if not textured:
        clay = bpy.data.materials.new("clay")
        clay.use_nodes = True
        shader = clay.node_tree.nodes["Principled BSDF"]
        flat = (0.02, 0.02, 0.02, 1.0) if silhouette else (0.62, 0.62, 0.62, 1.0)
        shader.inputs["Base Color"].default_value = flat
        shader.inputs["Roughness"].default_value = 1.0 if silhouette else 0.62
        if silhouette:
            shader.inputs["Specular IOR Level"].default_value = 0.0
        for mesh in meshes:
            if not mesh.material_slots:
                mesh.data.materials.append(clay)
            for slot in mesh.material_slots:
                slot.material = clay

    if marked:
        red = bpy.data.materials.new("pointing")
        red.use_nodes = True
        node = red.node_tree.nodes["Principled BSDF"]
        node.inputs["Base Color"].default_value = (0.85, 0.06, 0.05, 1.0)
        node.inputs["Emission Color"].default_value = (0.85, 0.06, 0.05, 1.0)
        node.inputs["Emission Strength"].default_value = 0.7
        body.data.materials.append(red)
        which = len(body.data.materials) - 1
        for face in body.data.polygons:
            if face.index in marked:
                face.material_index = which

    scene = bpy.context.scene
    # Named "BLENDER_EEVEE_NEXT" for two releases and back to "BLENDER_EEVEE" in 5.x. Picked
    # from what this build actually offers rather than from a name that was right once.
    engines = bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items.keys()
    scene.render.engine = "BLENDER_EEVEE" if "BLENDER_EEVEE" in engines else engines[0]
    scene.render.film_transparent = False
    scene.world = bpy.data.worlds.new("w")
    scene.world.use_nodes = True
    lit = 1.0 if silhouette else 0.28
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (lit, lit, lit, 1.0)
    scene.world.node_tree.nodes["Background"].inputs[1].default_value = 1.0

    if not silhouette:
        sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
        scene.collection.objects.link(sun)
        sun.data.energy = 3.2
        sun.rotation_euler = (math.radians(52.0), 0.0, math.radians(38.0))
        fill = bpy.data.objects.new("fill", bpy.data.lights.new("fill", type="SUN"))
        scene.collection.objects.link(fill)
        fill.data.energy = 1.1
        fill.rotation_euler = (math.radians(64.0), 0.0, math.radians(-118.0))

    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    scene.collection.objects.link(camera)
    scene.camera = camera
    camera.data.type = "ORTHO"

    # Measured from the POSED mesh, so a shot aimed at the head finds it wherever the clip put it.
    posed = body.evaluated_get(bpy.context.evaluated_depsgraph_get())
    spots = [posed.matrix_world @ v.co for v in posed.data.vertices]
    low, high = min(p.z for p in spots), max(p.z for p in spots)
    tall = high - low
    middle = mathutils.Vector((
        sum(p.x for p in spots) / len(spots),
        sum(p.y for p in spots) / len(spots),
        0.0,
    ))
    scene.render.resolution_x, scene.render.resolution_y = WIDE
    scene.eevee.taa_render_samples = 48

    # WHICH WAY HE FACES, measured from the toes rather than assumed.
    #
    # A shot called "front" that shows the side is not a naming quibble - it is the instrument
    # reporting the wrong thing, and the previous character's whole shot list was off by ninety
    # degrees for exactly this reason. Toes point forward on every biped, so the toe bone's own
    # axis answers it without a convention.
    facing = 0.0
    if rig is not None and "L_ToeBase" in rig.pose.bones:
        toe = rig.pose.bones["L_ToeBase"]
        along = (rig.matrix_world @ toe.tail) - (rig.matrix_world @ toe.head)
        along.z = 0.0
        if along.length > 1e-6:
            facing = math.degrees(math.atan2(along.x, -along.y))
            print(f"he faces {facing:.1f} deg off the camera rig's zero; shots turn with him")

    made = []
    for name, turn, frame_is, aim_at in SHOTS:
        if only and name not in only:
            continue
        camera.data.ortho_scale = tall * frame_is
        angle = math.radians(turn + facing)
        if isinstance(aim_at, str):
            if rig is None or aim_at not in rig.pose.bones:
                continue
            aim = rig.matrix_world @ rig.pose.bones[aim_at].head
        else:
            aim = mathutils.Vector((middle.x, middle.y, low + tall * aim_at))
        camera.location = (
            aim.x + tall * 4.0 * math.sin(angle),
            aim.y - tall * 4.0 * math.cos(angle),
            aim.z,
        )
        camera.rotation_euler = (math.radians(90.0), 0.0, angle)
        scene.render.filepath = os.path.join(out, f"{name}.png")
        bpy.ops.render.render(write_still=True)
        made.append(name)

    kind = "silhouette" if silhouette else ("textured" if textured else "clay")
    print(f"{len(made)} {kind} shots in {out}: {', '.join(made)}")


if __name__ == "__main__":
    main()
