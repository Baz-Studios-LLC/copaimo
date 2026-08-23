"""Opens a gait clip in the Blender GUI so it can be WATCHED, not just measured.

    "$BL" --python dev/art/gait_watch.py -- <glb> [clip] [options]

Defaults to the run clip of assets/models/person_ranger.glb. Options:

    --clip <walk|run|sprint|idle>   which clip (also accepted positionally)
    --rate <game|native>            playback speed; see "The rate" below
    --still                         don't move the ground
    --front | --side | --tq         starting view (default side)

# Why this exists as a script and not "just open the GLB"

glTF stores joint POSITIONS and no bone lengths, so every import invents a length for
every leaf bone. Measured on the current export, that puts `Head` at 2.6 cm on a head
that is 27.8 cm tall, both `Hand`s 8 cm past the fingertips, and each `ToeBase` at
15.9 cm where the geometry it drives is 6.7 - which is exactly the "bone lengths are
messed up again, they should reach the tip of the feet and hands" that keeps coming
back. It comes back because it is a property of the FILE FORMAT, not of the rig: the
repair cannot be exported, so anything that opens the glb has to redo it.

`prepare_rig.reach_the_ends` is that repair, and it is reused here rather than copied.
It is safe on an ANIMATED file for a reason worth stating: it only ever assigns
`bone.length`, which is stored apart from `matrix_local`. Redirecting a bone would
rotate `matrix_local`, and that is the basis the importer already converted the clip's
keys into - so redirecting after import would silently corrupt the pose. Lengthening
along a bone's own axis cannot. The function proves this rather than trusts it: it
re-reads every direction and roll afterwards and refuses if the basis moved at all.

`Root` and `Hip` are the other half of the complaint - the "really long angled bone",
both 85.0 cm, because `Root` sits on the floor and its only child is at the pelvis. They
are shortened here too, but by length ALONE for the same reason, never redirected. Both
drive zero vertices, which is checked below rather than assumed.

# The rate

The clips are authored at 24 fps (`src/motion.rs` FPS), so the run's sixteen frames are
one cycle in 0.667 s - 180 steps a minute. But the game does not play it at its own
rate: it hands `set_speed` a multiple so the clip carries the player's speed, and at
JOG_SPEED against RUN_COVERS that multiple is 1.111, which is 200 steps a minute.

`--rate game` (the default) shows what the game shows. `--rate native` shows what was
authored. The difference is the point: 200 is fast for a jog, and it is worth being able
to see which of the two you are judging.

# The ground

The character stays put and the GROUND moves, at the speed the game drives the player.
That is deliberate: a planted foot should sit still against a moving marker, so any
mismatch between what the clip's feet do and what the game claims they cover shows up
directly as the foot skating across the markers instead of gripping one.
"""

import math
import os
import re
import sys
import time

import bpy
import mathutils

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import prepare_rig

SCALE = 170.0            # centimetres per Blender unit, as the rest of dev/art uses
METRES_PER_UNIT = SCALE / 100.0

FPS = 24.0


def clips_from_the_game():
    """Frames, covers and driven speed, READ from src/motion.rs and src/player.rs.

    This was a hand-kept copy carrying a comment that a change in motion.rs would "show up
    here as a mismatch rather than as a silently wrong cadence". It did not: the covers
    were re-measured three times in one sitting and this table went on reporting 206 steps
    a minute for a clip the game drives at 212. A copy checked only by somebody
    remembering to check it is not a check.
    """
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(os.path.dirname(here))

    def rust(path, pattern):
        text = open(os.path.join(root, path), encoding="utf-8").read()
        found = re.search(pattern, text, re.M)
        if not found:
            raise SystemExit(f"no {pattern!r} in {path}")
        return float(found.group(1))

    out = {}
    for name, speed in (
        ("walk", "WALK_SPEED"), ("run", "JOG_SPEED"), ("sprint", "SPRINT_SPEED")
    ):
        out[name] = (
            rust("src/motion.rs", rf"^const {name.upper()}_FRAMES: f32 = ([0-9.]+);"),
            rust("src/motion.rs", rf"^const {name.upper()}_COVERS: f32 = ([0-9.]+);"),
            rust("src/player.rs", rf"^pub const {speed}: f32 = ([0-9.]+);"),
        )
    return out


CLIPS = clips_from_the_game()

# Blender's own orthographic view rotations, used as constants rather than derived,
# because `bpy.ops.view3d.view_axis` needs a real 3D region and this has to work with no
# window open at all. The character's forward is +X (five independent derivations agree),
# so Blender's FRONT view - camera at -Y, screen-right along +X - is the one that shows a
# gait in profile with him running to the right.
VIEWS = {
    "--side": (0.7071068, 0.7071068, 0.0, 0.0),      # profile, facing screen-right
    "--front": (0.5, 0.5, 0.5, 0.5),                 # head-on, down the line of travel
    "--top": (1.0, 0.0, 0.0, 0.0),                   # overhead, for foot tracking
}


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def aim_the_viewport(mesh, rotation):
    """Points every saved 3D view at the character, with no window required.

    Written onto `bpy.data.screens` rather than driven through an operator, because this
    runs in --background where there is no window, no area and no region - and the
    screens are part of the .blend, so whatever is set here is what the GUI restores
    when it opens the file. That is the whole reason the build is headless: the same
    scene built during GUI startup could not even import (see main).
    """
    low = mathutils.Vector((1e9, 1e9, 1e9))
    high = mathutils.Vector((-1e9, -1e9, -1e9))
    for corner in mesh.bound_box:
        spot = mesh.matrix_world @ mathutils.Vector(corner)
        for axis in range(3):
            low[axis] = min(low[axis], spot[axis])
            high[axis] = max(high[axis], spot[axis])
    middle = (low + high) * 0.5
    tall = max(high.z - low.z, 0.1)

    aimed = 0
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type != "VIEW_3D":
                continue
            space = area.spaces.active
            space.shading.type = "SOLID"
            space.overlay.show_floor = False
            space.overlay.show_axis_x = False
            space.overlay.show_axis_y = False
            space.region_3d.view_perspective = "ORTHO"
            space.region_3d.view_rotation = mathutils.Quaternion(rotation)
            space.region_3d.view_location = middle
            space.region_3d.view_distance = tall * 1.9
            aimed += 1
    print(f"  aimed {aimed} saved 3D view(s) at the character "
          f"({tall * SCALE:.0f} cm tall, centred at z={middle.z * SCALE:.0f} cm)")
    if not aimed:
        raise SystemExit(
            "REFUSED: no 3D viewport in this startup file to aim, so the saved .blend "
            "would open looking at nothing"
        )


# A script stored INSIDE the .blend, registered to run when it loads.
#
# The point is that rebuilding a clip should not cost anyone a keystroke. Blender reads a
# .blend into memory when it opens, so a rebuilt file is invisible to a window already
# showing the old one - which was being dealt with by killing Blender and reopening it,
# and that threw away the viewing angle and the frame every single time.
#
# This watches the .blend's own timestamp and reverts when it changes, so a rebuild just
# appears. The angle survives because it is written to a sidecar file first and read back
# after the reload - the view is stored IN the .blend, so a plain revert would otherwise
# snap back to the authored side-on framing.
#
# It only runs if Blender is started with `--enable-autoexec`, which gait_watch.sh does.
# That flag is per-session, so nothing is changed in anyone's preferences.
WATCHER = '''
import json
import os
import time

import bpy

SIDECAR = bpy.data.filepath + ".view.json"
SETTLE = 1.0
IDLE = 1.5

# What this scene was built from, stamped in at build time. The .blend's own timestamp is
# not enough on its own: it only changes when something rewrites the SCENE, and the failure
# worth catching is the GLB moving on while the scene does not - a build that refused after
# writing the model, or any path that rebuilds the clips without refreshing the viewer.
# Then nothing changes, the watcher has nothing to notice, and the window goes on showing
# work that was superseded hours ago. Which is how already-fixed faults get reported again.
SOURCE = bpy.context.scene.get("built_from", "")
BUILT_AT = bpy.context.scene.get("built_at", 0.0)
CLIP = bpy.context.scene.get("built_clip", "?")


def views():
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type == "VIEW_3D":
                yield area.spaces.active.region_3d


def remember():
    spot = next(views(), None)
    if spot is None:
        return
    try:
        with open(SIDECAR, "w") as handle:
            json.dump({
                "rot": list(spot.view_rotation),
                "loc": list(spot.view_location),
                "dist": spot.view_distance,
                "persp": spot.view_perspective,
            }, handle)
    except OSError:
        pass


def restore():
    try:
        with open(SIDECAR) as handle:
            saved = json.load(handle)
    except (OSError, ValueError):
        return
    for spot in views():
        spot.view_perspective = saved["persp"]
        spot.view_rotation = saved["rot"]
        spot.view_location = saved["loc"]
        spot.view_distance = saved["dist"]


def stamp():
    try:
        return os.path.getmtime(bpy.data.filepath)
    except OSError:
        return None


# [last seen mtime, whether a change is waiting to settle]
state = [stamp(), False]


def tick():
    now = stamp()
    if now is None:
        return IDLE
    if now != state[0]:
        # Noted, but not acted on yet - the file may still be being written.
        state[0], state[1] = now, True
        return SETTLE
    if state[1]:
        state[1] = False
        remember()
        try:
            bpy.ops.wm.revert_mainfile()
        except RuntimeError:
            return IDLE
        return None      # the reloaded file registers its own timer
    return IDLE


def how_stale():
    """Seconds the model is ahead of this scene, or 0 if the scene is current."""
    if not SOURCE:
        return 0.0
    try:
        return max(0.0, os.path.getmtime(SOURCE) - BUILT_AT)
    except OSError:
        return 0.0


def say_so():
    """Draws the verdict over the viewport, because a caption cannot be missed."""
    try:
        import blf
    except ImportError:
        return
    behind = how_stale()
    if behind > 2.0:
        text = (f"STALE - {CLIP} rebuilt {behind / 60.0:.0f} min after this scene. "
                f"Re-run dev/art/animate_ranger.sh")
        colour = (1.0, 0.35, 0.25, 1.0)
        size = 20
    else:
        text = f"{CLIP} - current, built {time.strftime('%H:%M', time.localtime(BUILT_AT))}"
        colour = (0.45, 0.85, 0.5, 0.7)
        size = 13
    try:
        blf.size(0, size)
        blf.color(0, *colour)
        blf.position(0, 22, 22, 0)
        blf.draw(0, text)
    except Exception:
        pass


restore()
try:
    bpy.types.SpaceView3D.draw_handler_add(say_so, (), "WINDOW", "POST_PIXEL")
except Exception:
    pass
bpy.app.timers.register(tick, first_interval=2.0)
'''


def stamp_the_scene(glb: str, clip: str):
    """Records which model this scene was built from, and when.

    Read back by the watcher to decide whether the window is showing current work. Stored
    on the scene rather than in a sidecar so it cannot be separated from the .blend, and so
    a scene copied somewhere else still knows what it came from.
    """
    scene = bpy.context.scene
    scene["built_from"] = os.path.abspath(glb)
    scene["built_at"] = os.path.getmtime(glb)
    scene["built_clip"] = clip
    print(f"  stamped: built from {os.path.basename(glb)} "
          f"({time.strftime('%H:%M:%S', time.localtime(scene['built_at']))}), clip {clip}")


def install_the_watcher():
    """Puts the reload watcher into the .blend and marks it to run on load."""
    name = "gait_watch_reload.py"
    text = bpy.data.texts.get(name) or bpy.data.texts.new(name)
    text.clear()
    text.write(WATCHER)
    text.use_module = True          # this is the "Register" checkbox
    print(f"  installed {name} ({len(WATCHER)} chars), registered to run on load")


def lay_the_ground(forward, covers_units, span, low):
    """A floor plus marker bars, which slide backward one stride per cycle.

    Spaced a quarter of a stride apart so the pattern reads as continuous when the
    cycling modifier wraps it, and laid well past the view so nothing runs out.
    """
    floor_mesh = bpy.data.meshes.new("floor")
    floor = bpy.data.objects.new("floor", floor_mesh)
    bpy.context.scene.collection.objects.link(floor)
    verts = [
        (-8.0, -8.0, 0.0), (8.0, -8.0, 0.0), (8.0, 8.0, 0.0), (-8.0, 8.0, 0.0),
    ]
    floor_mesh.from_pydata(verts, [], [(0, 1, 2, 3)])
    floor_mesh.update()
    grey = bpy.data.materials.new("floor")
    grey.node_tree.nodes["Principled BSDF"].inputs[0].default_value = (
        0.16, 0.17, 0.19, 1.0
    )
    floor.data.materials.append(grey)

    # The markers ride a single parent, so one set of keys moves all of them.
    treadmill = bpy.data.objects.new("treadmill", None)
    treadmill.empty_display_size = 0.05
    bpy.context.scene.collection.objects.link(treadmill)

    bright = bpy.data.materials.new("marker")
    bright.node_tree.nodes["Principled BSDF"].inputs[0].default_value = (
        0.55, 0.62, 0.75, 1.0
    )

    step = covers_units / 4.0
    across = mathutils.Vector((forward.y, -forward.x, 0.0)).normalized()
    count = int(12.0 / step) if step > 1e-6 else 0
    for i in range(-count, count + 1):
        bar_mesh = bpy.data.meshes.new(f"bar{i}")
        bar = bpy.data.objects.new(f"bar{i}", bar_mesh)
        half_long, half_wide = 0.006, 0.5
        corners = []
        for sign_a, sign_b in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            spot = forward * (half_long * sign_a) + across * (half_wide * sign_b)
            corners.append((spot.x, spot.y, 0.001))
        bar_mesh.from_pydata(corners, [], [(0, 1, 2, 3)])
        bar_mesh.update()
        bar.data.materials.append(bright)
        # Every fourth bar full width, the rest short, so a stride is countable.
        if i % 4 != 0:
            bar.scale = (1.0, 1.0, 1.0)
            for vertex in bar_mesh.vertices:
                vertex.co = vertex.co * 0.45 + mathutils.Vector((0.0, 0.0, 0.001))
        bar.location = forward * (step * i) + mathutils.Vector((0.0, 0.0, low))
        bpy.context.scene.collection.objects.link(bar)
        bar.parent = treadmill

    # One stride backward per cycle, and then it simply loops with the clip.
    #
    # No cycling modifier and no accumulating travel, and the arithmetic is worth
    # writing down because it is not the obvious "it resets by a whole stride".
    #
    # The keys are at frame 1 and frame 1+span, but only 1..span are PLAYED, so the last
    # frame shown sits at -(span-1)/span of a stride and the wrap jumps it back to 0.
    # For the run that is a jump of +15/16 of a stride, not a whole one. It is invisible
    # anyway: the markers repeat every `covers/4`, so -15/16 of a stride is -3.75
    # periods, which is congruent to +0.25 periods, and the wrap to 0 therefore reads as
    # -0.25 periods = -1/16 of a stride. Exactly the one interval's worth of travel that
    # was owed. Seamless AND correct, with no extrapolation to drift.
    #
    # Blender 5.x has no `action.fcurves` (actions are slots, layers, strips and
    # channelbags), so the interpolation is set as the PREFERENCE before the keys are
    # made rather than edited onto the curves afterwards.
    was = bpy.context.preferences.edit.keyframe_new_interpolation_type
    bpy.context.preferences.edit.keyframe_new_interpolation_type = "LINEAR"
    try:
        treadmill.location = (0.0, 0.0, 0.0)
        treadmill.keyframe_insert("location", frame=1)
        treadmill.location = -forward * covers_units
        treadmill.keyframe_insert("location", frame=1 + span)
    finally:
        bpy.context.preferences.edit.keyframe_new_interpolation_type = was
    return floor, treadmill


def main():
    args = argv()
    glb = None
    clip = None
    rate = "game"
    ground = True
    view = VIEWS["--side"]
    save_to = None
    i = 0
    while i < len(args):
        token = args[i]
        if token == "--clip":
            clip = args[i + 1]
            i += 2
        elif token == "--save":
            save_to = args[i + 1]
            i += 2
        elif token == "--rate":
            rate = args[i + 1]
            i += 2
        elif token == "--still":
            ground = False
            i += 1
        elif token in VIEWS:
            view = VIEWS[token]
            i += 1
        elif glb is None and not token.startswith("--"):
            glb = token
            i += 1
        elif clip is None and not token.startswith("--"):
            clip = token
            i += 1
        else:
            i += 1

    glb = glb or os.path.join(HERE, "..", "..", "assets", "models", "person_ranger.glb")
    clip = clip or "run"
    glb = os.path.abspath(glb)

    # NOT read_homefile(use_empty=True).
    #
    # Calling it from a --python script during GUI startup leaves a context the glTF
    # importer cannot work in: it died in `armature_display` on
    # `bpy.data.collections[BLENDER_GLTF_SPECIAL_COLLECTION].objects.link(
    # bpy.context.object)`, so NOTHING was imported and the window came up empty. The
    # same import is fine in --background, which is why this builds headless and saves a
    # .blend for the GUI to open. Clearing the startup objects by hand is enough.
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)
    bpy.ops.import_scene.gltf(filepath=glb)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is None:
        raise SystemExit("no armature in that file")
    # The BODY, not whichever skinned mesh the importer happened to list first - the
    # backpack is its own object now, and it is 370 vertices against the body's 7261.
    mesh = prepare_rig.the_body()

    print(f"\nopening {os.path.basename(glb)}, clip '{clip}'")
    print("repairing what the importer invented:")
    # Before the ground exists: this deletes unskinned meshes to dispose of the
    # importer's Icosphere, so a floor laid first would go with it.
    prepare_rig.make_the_import_readable(rig, mesh)

    action = bpy.data.actions.get(clip)
    if action is None:
        raise SystemExit(
            f"no clip '{clip}'; this file has {[a.name for a in bpy.data.actions]}"
        )
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = action
    try:
        if rig.animation_data.action_slot is None and action.slots:
            rig.animation_data.action_slot = action.slots[0]
    except AttributeError:
        pass

    first, last = (int(round(v)) for v in action.frame_range)
    frames, covers, drives_at = CLIPS.get(clip, (float(last - first), 0.0, 0.0))
    span = int(round(frames))
    # The LAST frame is the same instant as the first - that is what closes the cycle -
    # so playing both back to back would hold the seam pose for two frames every time
    # the loop wrapped. The game samples a duration and wraps continuously, which does
    # not double it, so the preview must not either.
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = first, first + span - 1
    if last != first + span:
        print(f"  note: '{clip}' is frames {first}..{last} but motion.rs calls it "
              f"{span} frames; playing {scene.frame_start}..{scene.frame_end}")

    natively = covers * FPS / frames if frames else 0.0
    multiple = (drives_at / natively) if natively else 1.0
    if rate == "native":
        multiple = 1.0
    effective = FPS * multiple
    scene.render.fps = int(round(FPS))
    scene.render.fps_base = FPS / effective if effective else 1.0
    cadence = effective / frames * 120.0 if frames else 0.0
    print(
        f"\n  {span} frames authored at {FPS:.0f} fps = {natively:.2f} m/s natively"
        f"\n  the game drives it at {drives_at:.2f} m/s, a {multiple:.3f}x multiple"
        f"\n  playing at {effective:.1f} fps -> {cadence:.0f} steps a minute"
        f" ({rate})"
    )

    _, forward, _ = prepare_rig.body_frame(rig)
    forward = mathutils.Vector((forward.x, forward.y, 0.0)).normalized()
    if ground and covers:
        # The ground travels what the GAME says a cycle carries, not what the clip's
        # feet do - so the gap between the two is what you see.
        lay_the_ground(forward, covers / METRES_PER_UNIT, span, 0.0)
        print(f"  the ground slides {covers:.3f} m a cycle "
              "(a planted foot should hold one marker)")

    scene.frame_set(scene.frame_start)
    aim_the_viewport(mesh, view)
    stamp_the_scene(glb, clip)
    install_the_watcher()

    if save_to:
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(save_to))
        print(f"\nwrote {os.path.abspath(save_to)}")
        print("open it and press space to play.")


main()
