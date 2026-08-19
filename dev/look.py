"""Draws a baked building's boxes to a PNG, so a change to the kit can be looked at
without booting the game.

A depth buffer rather than painter's order — the first version of this sorted whole
faces by one depth each and the subfloor's top face, being metres across, painted
over the boards in front of it. A z-buffer has no opinion about face size.

    python render_png.py scene.json out.png [--turn 30] [--pitch 34] [--scale 150]
"""
import json
import math
import struct
import sys
import zlib

args = sys.argv[1:]
scene_path, out_path = args[0], args[1]


def flag(name, fallback):
    return float(args[args.index(name) + 1]) if name in args else fallback


TURN = math.radians(flag("--turn", 32.0))
PITCH = math.radians(flag("--pitch", 30.0))
SCALE = flag("--scale", 150.0)
WIDE, HIGH = int(flag("--wide", 980)), int(flag("--high", 740))
BACK = (0x3F, 0x44, 0x50)

boxes = json.load(open(scene_path))["boxes"]

def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def unit(v):
    n = math.sqrt(dot(v, v))
    return (v[0] / n, v[1] / n, v[2] / n)


# The camera: turned about Y, then pitched down. Its own three axes are what a point
# is measured against — right and up give the screen, forward gives the depth.
#
# Built from cross products rather than written out component by component. The
# hand-written version had the sign wrong on two of `up`'s three parts, so the basis
# was not orthogonal — and a skewed basis puts the screen position and the depth of a
# point into disagreement, which showed up as the floor rendering as its own subfloor:
# at most pixels the LOWER surface won the depth test. Nothing about that looks like a
# sign error when you are staring at a picture of a floor.
forward = unit(
    (
        math.sin(TURN) * math.cos(PITCH),
        -math.sin(PITCH),
        math.cos(TURN) * math.cos(PITCH),
    )
)
right = unit(cross((0.0, 1.0, 0.0), forward))
up = cross(forward, right)
assert abs(dot(right, forward)) < 1e-9 and abs(dot(up, forward)) < 1e-9
assert abs(dot(right, up)) < 1e-9


def quat_apply(q, v):
    qx, qy, qz, qw = q
    tx = 2.0 * (qy * v[2] - qz * v[1])
    ty = 2.0 * (qz * v[0] - qx * v[2])
    tz = 2.0 * (qx * v[1] - qy * v[0])
    return (
        v[0] + qw * tx + (qy * tz - qz * ty),
        v[1] + qw * ty + (qz * tx - qx * tz),
        v[2] + qw * tz + (qx * ty - qy * tx),
    )


# Six faces, and the depth buffer decides which are seen. Each is (normal, corners).
FACES = [
    ((0, 1, 0), [(-1, 1, -1), (1, 1, -1), (1, 1, 1), (-1, 1, 1)]),
    ((0, -1, 0), [(-1, -1, -1), (-1, -1, 1), (1, -1, 1), (1, -1, -1)]),
    ((1, 0, 0), [(1, -1, -1), (1, 1, -1), (1, 1, 1), (1, -1, 1)]),
    ((-1, 0, 0), [(-1, -1, -1), (-1, -1, 1), (-1, 1, 1), (-1, 1, -1)]),
    ((0, 0, 1), [(-1, -1, 1), (-1, 1, 1), (1, 1, 1), (1, -1, 1)]),
    ((0, 0, -1), [(-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1)]),
]

# One fixed light, from over the viewer's left shoulder, plus a flat ambient — the
# same bargain the game's own sky makes, at a hundredth of the trouble.
SUN = (-0.42, 0.80, -0.43)
AMBIENT = 0.46

triangles = []
for box in boxes:
    at, size, turn, rgb = box["at"], box["size"], box["turn"], box["rgb"]
    half = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0]
    for normal, corners in FACES:
        facing = quat_apply(turn, normal)
        if dot(facing, forward) > -0.02:
            continue  # pointing away from the camera
        lit = AMBIENT + (1.0 - AMBIENT) * max(0.0, dot(facing, SUN))
        colour = tuple(max(0, min(255, int(c * lit))) for c in rgb)

        screen = []
        for sx, sy, sz in corners:
            spun = quat_apply(turn, (sx * half[0], sy * half[1], sz * half[2]))
            world = (at[0] + spun[0], at[1] + spun[1], at[2] + spun[2])
            screen.append((dot(world, right) * SCALE, -dot(world, up) * SCALE, dot(world, forward)))
        triangles.append((screen[0], screen[1], screen[2], colour))
        triangles.append((screen[0], screen[2], screen[3], colour))

# Centred on what there is to see.
xs = [p[0] for tri in triangles for p in tri[:3]]
ys = [p[1] for tri in triangles for p in tri[:3]]
shift_x = WIDE * 0.5 - (min(xs) + max(xs)) * 0.5
shift_y = HIGH * 0.5 - (min(ys) + max(ys)) * 0.5

pixels = bytearray(BACK * (WIDE * HIGH))
depth = [1.0e30] * (WIDE * HIGH)

for a, b, c, colour in triangles:
    ax, ay, az = a[0] + shift_x, a[1] + shift_y, a[2]
    bx, by, bz = b[0] + shift_x, b[1] + shift_y, b[2]
    cx, cy, cz = c[0] + shift_x, c[1] + shift_y, c[2]
    area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    if abs(area) < 1.0e-9:
        continue
    low_x = max(0, int(min(ax, bx, cx)))
    high_x = min(WIDE - 1, int(max(ax, bx, cx)) + 1)
    low_y = max(0, int(min(ay, by, cy)))
    high_y = min(HIGH - 1, int(max(ay, by, cy)) + 1)
    red, green, blue = colour
    for py in range(low_y, high_y + 1):
        y = py + 0.5
        row = py * WIDE
        for px in range(low_x, high_x + 1):
            x = px + 0.5
            w0 = ((bx - ax) * (y - ay) - (by - ay) * (x - ax)) / area
            w1 = ((x - ax) * (cy - ay) - (y - ay) * (cx - ax)) / area
            if w0 < 0.0 or w1 < 0.0 or w0 + w1 > 1.0:
                continue
            here = az + (cz - az) * w0 + (bz - az) * w1
            at_pixel = row + px
            if here >= depth[at_pixel]:
                continue
            depth[at_pixel] = here
            out = at_pixel * 3
            pixels[out] = red
            pixels[out + 1] = green
            pixels[out + 2] = blue


def png(width, height, rgb_rows):
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        raw += rgb_rows[y * width * 3 : (y + 1) * width * 3]

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
        + chunk(b"IEND", b"")
    )


open(out_path, "wb").write(png(WIDE, HIGH, pixels))
print(f"{len(triangles)} triangles -> {out_path}")
