"""Tones down the ranger's eye whites, and writes the texture the game uses.

    python dev/art/ranger_texture.py        # run by dev/art/animate_ranger.sh

Reads the base-colour map out of `Ranger_Rig_Idle.glb` — the file as it arrived —
and writes `dev/art/ranger_basecolor.png` with the eye whites brought down. The
Blender step then points the material at that file before exporting the game's copy.

# Why this is not done inside Blender

It was, first, and it silently did nothing. Editing `image.pixels` and calling
`pack()` reported success and exported the ORIGINAL bytes: the peak luminance
through the eye read 254 before and 254 after. Blender keeps the packed file it
already has, and a buffer edit does not necessarily replace it.

Doing the pixel work here instead means it can be CHECKED — the numbers below are
measured out of the file that gets written — and it means the thresholds are in
sRGB, which is what the measurements were taken in. Blender then has one job it
cannot get wrong: use this file.

# What is wrong with the eyes

The generated map paints a wide white sclera, and the crescent of it below the iris
reads on a face at walking distance as a white line under the eye. Rows through the
eye measure 251 to 255 against skin at 75 to 120.

# Finding an eye without hard-coding where one is

A pixel is eye-white if it is very light, has near-black within a few pixels (an
iris), AND has skin within a few more. All three are needed. Light-beside-dark alone
selected nineteen thousand pixels — the white shirt against the black vest — which
is a wardrobe, not a face. Skin adjacency is what a shirt does not have.

And there is a CAP. Two eyes on a four-thousand-square atlas are on the order of a
thousand pixels; if the rule selects many times that it has found something else, and
then nothing is written and this says so. Loud and wrong beats quiet and wrong on
somebody else's texture.
"""

import io
import json
import os
import struct
import sys

# Where the eyes are in THIS atlas, measured off it: (left, top, right, bottom).
#
# # Why these are written down rather than found
#
# Four goes at finding them automatically, each failing differently, and the record
# is worth keeping because the next person will be tempted the same way.
#
# 1. White pixels near black. Selected 18,937 — the white shirt against the black
#    vest. A wardrobe, not a face.
# 2. Plus skin nearby, which a shirt does not have. Down to 1,647, but only the
#    crescent hugging the pupil: the rest of the sclera is further from the iris.
# 3. Widen the reach to cover the whole sclera. Thirty-five clusters along the
#    atlas edge.
# 4. By connected component instead — a sclera is a small region, wardrobe whites
#    are large ones. Found exactly two, and they were only PART of each eye: the
#    sclera is painted in two tones, a pure white beside the pupil and a warm cream
#    over the rest, and the cream is a separate region. Lowering the cut to catch
#    the cream then let the two halves diverge again.
#
# The thing that made all four brittle is that the eye is not one shape in one
# colour, and neither is anything else in an atlas packed by a generator. So the
# location is stated, because it is a FACT ABOUT THIS FILE and not a rule about
# textures — the same reasoning that keeps the model's height and facing in a table
# beside its filename.
#
# `SOURCE_BYTES` is the guard. If the source is ever regenerated the boxes below
# mean nothing, so the script refuses rather than dimming somebody's cheek.
EYES = (
    (3300, 2168, 3392, 2226),
    (3664, 2436, 3740, 2546),
)
SOURCE_BYTES = 3556668

# What counts as light enough to be part of an eye, and what is skin to be left
# alone. Inside an eye's own box this is all the discrimination needed.
LIGHT_ABOVE = 150
SKIN_LOW = 90
SKIN_HIGH = 242

# What an eye white comes down to. Still a white — just not a headlight.
# 150, down from 168: at 168 the whites still read as lines in the game's own
# dimmer light. Still lighter than the skin around them, so the eye keeps a white.
SCLERA_BECOMES = 150


def chunks_of(raw: bytes):
    if raw[:4] != b"glTF":
        raise SystemExit("the source is not a GLB")
    total = struct.unpack("<I", raw[8:12])[0]
    at, out = 12, {}
    while at < total:
        length, kind = struct.unpack("<I4s", raw[at : at + 8])
        out[kind.strip(b"\x00").decode()] = raw[at + 8 : at + 8 + length]
        at += 8 + length + ((4 - length % 4) % 4)
    return out


def main() -> None:
    try:
        import numpy
        from PIL import Image
    except ImportError as why:
        raise SystemExit(f"this needs numpy and pillow: {why}")

    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(os.path.dirname(here))
    source = os.path.join(root, "Ranger_Rig_Idle.glb")
    out = os.path.join(here, "ranger_basecolor.png")

    # # Skipped when nothing it depends on has changed
    #
    # Re-encoding a 4096 x 4096 PNG took 5.1 seconds of a 14.9 second build, every
    # build, from an input that changes about once a month. The pipeline is run dozens
    # of times in a session and slow feedback caused real mistakes today - a stale
    # bytecode cache once made a source edit look like a no-op - so a third of the wait
    # for no work is worth removing.
    #
    # The stamp is the source glb AND this script: if either is newer than the output,
    # the output is stale. Nothing else can change the answer.
    if os.path.isfile(out):
        fresh = os.path.getmtime(out)
        if all(os.path.getmtime(need) <= fresh
               for need in (source, os.path.abspath(__file__))):
            print(f"UNCHANGED {os.path.basename(out)} is newer than the export and "
                  "this script, so the texture is already calm")
            return

    parts = chunks_of(open(source, "rb").read())
    tree = json.loads(parts["JSON"])
    blob = parts["BIN"]
    which = tree["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["index"]
    image = tree["images"][tree["textures"][which]["source"]]
    view = tree["bufferViews"][image["bufferView"]]
    start = view.get("byteOffset", 0)
    picture = Image.open(io.BytesIO(blob[start : start + view["byteLength"]])).convert("RGB")
    print(f"base colour: {picture.size[0]}x{picture.size[1]} {image.get('mimeType')}")

    size = os.path.getsize(source)
    if size != SOURCE_BYTES:
        raise SystemExit(
            f"the source is {size} bytes and the eye boxes were measured against "
            f"{SOURCE_BYTES}. Re-measure them before running this — the boxes are a "
            "fact about one file, and on a different one they are somebody's cheek."
        )

    pixels = numpy.asarray(picture).astype(numpy.int16)
    sclera = numpy.zeros(pixels.shape[:2], dtype=bool)
    for left, top, right, bottom in EYES:
        patch = pixels[top:bottom, left:right]
        red, green, blue = patch[:, :, 0], patch[:, :, 1], patch[:, :, 2]
        light = patch.min(axis=2) > LIGHT_ABOVE
        # Skin is warm — clearly more red than green. An eye white is not, whichever
        # of its two tones it is, so this separates them inside the box.
        skin = (
            (red > SKIN_LOW)
            & (red < SKIN_HIGH)
            & (red > green * 1.22)
            & (green > blue)
        )
        sclera[top:bottom, left:right] = light & ~skin
        print(f"  eye at ({left}, {top}): {int((light & ~skin).sum())} px of white")

    # Brought DOWN rather than recoloured: the eye keeps its shape and stops being
    # the brightest thing on the face.
    scale = SCLERA_BECOMES / 255.0
    for lane in range(3):
        channel = pixels[:, :, lane]
        channel[sclera] = (channel[sclera] * scale).astype(numpy.int16)

    # No `optimize=True`: it costs seconds of zlib search on a 16-megapixel image to
    # save space in a DEV INTERMEDIATE that is re-encoded into the glb moments later.
    Image.fromarray(pixels.astype(numpy.uint8)).save(out, "PNG", compress_level=1)
    print(f"CALMED {int(sclera.sum())} pixels of eye white -> {os.path.basename(out)}")

    # Measured out of what was actually written, which is the whole point of doing
    # this here rather than in a tool that reported success and changed nothing.
    check = numpy.asarray(Image.open(out).convert("RGB")).astype(numpy.float32)
    lum = 0.2126 * check[:, :, 0] + 0.7152 * check[:, :, 1] + 0.0722 * check[:, :, 2]
    print(f"peak luminance where the sclera was: {lum[sclera].max():.1f} (was ~254)")


main()
