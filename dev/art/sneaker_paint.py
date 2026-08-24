"""Paints a sneaker into the strip that gets added to the bottom of the atlas.

    python dev/art/sneaker_paint.py preview.png     # look at it on its own

Imported by `ranger_texture.py`, which grows the base-colour map from 4096x4096 to
4096x4608 and drops this into the new rows. `shoe_form.py` writes the matching UVs.

# Why the atlas had to grow at all

Measured by rasterising every UV triangle: the atlas is 90.5% covered, and the largest
genuinely empty square anywhere in it is 224 px. Freeing the old shoe's islands only
takes that to 224 as well - the shoe's 3.7% of UV area is scattered across the whole
sheet rather than sitting in one block, so there is nothing to reclaim in one piece.

Growing keeps ONE texture and ONE material, which is what putting it in the atlas
means. The alternative - shrinking the existing content into 3584 rows to free a strip
without changing the image size - costs 12.5% of the resolution on everything else to
save a non-power-of-two dimension that no modern GPU minds.

# The parameterisation, which the geometry has to agree with

The shoe is a swept mesh: rings of vertices at stations from heel to toe. So it
unwraps to a rectangle, and both shoes share the island - they are mirror images, and
sharing doubles the resolution each one gets.

    u   along the shoe, 0 at the back of the heel, 1 at the tip of the toe
    v   around the ring, 0 at the middle of the sole, a quarter at the outer side,
        a half at the top of the instep, three quarters at the inner side

So v tells you which PART you are painting, and that is what everything below is
written in terms of. `shoe_form.RING_STARTS_UNDER` says the same thing from the
geometry's side; if one moves the other has to.
"""
import math
import sys

# The strip's size in pixels. 512 rows on a 4096-wide atlas gives the shoe about
# 4096 x 512 to itself, which at 26 cm long is roughly 150 px per cm around the ring -
# more than the body gets, and it is what is being looked at closely.
STRIP = (4096, 512)

# Where each part of the shoe sits around the ring. See the parameterisation above.
SOLE_UNDER = 0.11        # |v| below this is the underside of the sole
MIDSOLE_UPTO = 0.215     # and up to here is the white midsole wall
PANEL_UPTO = 0.38        # then the side panels
# the rest, toward v = 0.5, is the instep: tongue and lacing

# The colours, taken off the character's existing palette so the new shoe belongs to
# the same outfit: the jacket's olive, its black, and the orange trim.
OLIVE = (86, 92, 58)
BLACK = (34, 35, 38)
WHITE = (222, 220, 212)
GREY = (54, 55, 58)
ORANGE = (214, 96, 30)
LACE = (198, 196, 188)


def smoothstep(numpy, edge0, edge1, at):
    """Elementwise, because `at` here is the whole strip rather than one number."""
    t = numpy.clip((at - edge0) / max(edge1 - edge0, 1e-9), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def paint(numpy):
    """Returns the strip as a (rows, columns, 3) uint8 array, bottom-of-image first."""
    wide, tall = STRIP
    # u along the columns, v up the rows. Row 0 is the TOP of the strip in image terms,
    # and the strip sits at the BOTTOM of the atlas, so v = 1 - row/tall.
    u = numpy.linspace(0.0, 1.0, wide, dtype=numpy.float32)[None, :]
    v = 1.0 - numpy.linspace(0.0, 1.0, tall, dtype=numpy.float32)[:, None]
    u = numpy.repeat(u, tall, axis=0)
    v = numpy.repeat(v, wide, axis=1)

    # How far round the ring from the middle of the sole, 0 at the sole and 0.5 at the
    # instep, so left and right of the shoe are painted by one expression.
    round_from_sole = numpy.abs(((v + 0.5) % 1.0) - 0.5)

    out = numpy.zeros((tall, wide, 3), dtype=numpy.float32)

    def lay(mask, colour):
        for lane in range(3):
            out[:, :, lane] = numpy.where(mask, colour[lane], out[:, :, lane])

    # THE UPPER, olive with a black heel counter and a black toe cap - the panelling
    # the old shoe had painted on it, kept so the character still reads as himself.
    lay(numpy.ones_like(u, dtype=bool), OLIVE)
    lay(u < 0.22, BLACK)                       # heel counter
    lay(u > 0.80, BLACK)                       # toe cap
    # A stripe sweeping from the midsole at the toe up toward the collar at the back,
    # which is the one shape that says "sneaker" more than any other.
    stripe = numpy.abs(round_from_sole - (0.20 + 0.24 * (1.0 - u))) < 0.045
    lay(stripe & (u > 0.24) & (u < 0.86), WHITE)

    # THE INSTEP: a tongue up the middle with lacing either side of it.
    tongue = (round_from_sole > 0.44) & (u > 0.30) & (u < 0.66)
    lay(tongue, BLACK)
    rungs = (numpy.abs(numpy.sin(u * math.pi * 26.0)) > 0.86)
    laces = rungs & (round_from_sole > 0.385) & (round_from_sole < 0.474) \
        & (u > 0.30) & (u < 0.68)
    lay(laces, LACE)
    # The collar rim itself, a band of orange trim where the foot goes in.
    lay((round_from_sole > 0.47) & (u > 0.06) & (u < 0.30), ORANGE)

    # THE SOLE, last, so it covers anything that ran into it. A white midsole wall
    # under a hard line, and a dark tread underneath.
    lay(round_from_sole < MIDSOLE_UPTO, WHITE)
    lay(round_from_sole < SOLE_UNDER, GREY)
    # Tread bars across the underside, and a darker pad at the heel and the ball.
    bars = (numpy.abs(numpy.sin(u * math.pi * 44.0)) > 0.55) & \
        (round_from_sole < SOLE_UNDER * 0.92)
    lay(bars, BLACK)

    # A little shading round the ring so the shoe is not flat: darker toward the sole,
    # lighter across the top where light lands.
    shade = 0.86 + 0.24 * smoothstep(numpy, 0.0, 0.5, round_from_sole)
    out *= shade[:, :, None]

    return numpy.clip(out, 0, 255).astype(numpy.uint8)


if __name__ == "__main__":
    import numpy
    from PIL import Image
    where = sys.argv[1] if len(sys.argv) > 1 else "sneaker_strip.png"
    Image.fromarray(paint(numpy)).save(where)
    print(f"wrote {where} at {STRIP[0]}x{STRIP[1]}")
