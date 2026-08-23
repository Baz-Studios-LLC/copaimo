"""Resample the painted world layers when `config::WORLD_WIDTH` changes.

`Sculpt::read` (and the surface/country/forest readers built the same way) REFUSE a file
whose grid does not match the world it is being loaded into:

    if wide != empty.wide || deep != empty.deep || kept_half.distance(half) > 1.0

and `load_from` answers a refusal by falling back to `empty`. So changing the world size
without running this silently discards every painted cell - it logs a warning and carries
on with a blank world, which is the worst possible failure mode for hours of sculpting.

The grid is `ceil(half * 2 / CELL) + 1` cells, and CELL is recovered from the file itself
rather than assumed, so this works for both resolutions in use (4 m for edits/surface,
16 m for country/forest).

Content SQUEEZES with the world. The heightmap is sampled over whatever extent the world
has, so halving `WORLD_WIDTH` shrinks the whole continent rather than cropping it, and the
painted layers have to do the same thing or they would slide out of register with the land
they were painted onto. So the mapping is normalised position to normalised position.

Downsampling sparse data needs care. Only 0.6% of the sculpt grid is non-zero, so an
averaging filter would dilute an isolated sculpted cell toward nothing and quietly flatten
the very features being preserved. Two rules instead, per layer kind:

* HEIGHT offsets take the largest absolute value in the source footprint, so a ridge or a
  pit survives at full amplitude rather than being averaged against the flat ground it
  sits in.
* CATEGORICAL layers - region ids, forest flags - take the most common non-zero value.
  Averaging an id is meaningless, and taking the largest would bias every boundary toward
  whichever id happens to sort highest.

Header layout, read out of the files rather than from a spec: 8 bytes of magic, then
`wide` and `deep` as u32, then `half.x` and `half.y` as f32. Twenty-four bytes, and the
rest is `wide * deep` little-endian f32.
"""
import array
import collections
import math
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
WORLD = os.path.normpath(os.path.join(HERE, "..", "..", "assets", "world"))

# Which rule each layer is resampled by - see the module docstring.
LAYERS = {
    "edits.bin": "height",
    "surface.bin": "category",
    "country.bin": "category",
    "forest.bin": "category",
}


def read(path):
    with open(path, "rb") as handle:
        raw = handle.read()
    magic = raw[:8]
    wide, deep = struct.unpack_from("<II", raw, 8)
    half_x, half_y = struct.unpack_from("<ff", raw, 16)
    cells = array.array("f")
    cells.frombytes(raw[24:24 + wide * deep * 4])
    if len(cells) != wide * deep:
        raise SystemExit(f"{path}: expected {wide * deep} cells, found {len(cells)}")
    return magic, wide, deep, half_x, half_y, cells


def grid_for(half_x, half_y, cell):
    """The same arithmetic as `Sculpt::empty`, so the result is what the game will demand."""
    return (
        int(math.ceil(half_x * 2.0 / cell)) + 1,
        int(math.ceil(half_y * 2.0 / cell)) + 1,
    )


def resample(cells, wide, deep, new_wide, new_deep, rule):
    out = array.array("f", [0.0]) * (new_wide * new_deep)
    for ny in range(new_deep):
        # The source rows this destination row covers. Half-open, and at least one wide,
        # so an UPscale reads a single source cell rather than an empty range.
        y0 = int(math.floor(ny * (deep - 1) / max(1, new_deep - 1)))
        y1 = max(y0 + 1, int(math.ceil((ny + 1) * (deep - 1) / max(1, new_deep - 1))))
        y1 = min(y1, deep)
        for nx in range(new_wide):
            x0 = int(math.floor(nx * (wide - 1) / max(1, new_wide - 1)))
            x1 = max(x0 + 1, int(math.ceil((nx + 1) * (wide - 1) / max(1, new_wide - 1))))
            x1 = min(x1, wide)
            seen = [
                cells[y * wide + x]
                for y in range(y0, y1)
                for x in range(x0, x1)
                if cells[y * wide + x] != 0.0
            ]
            if not seen:
                continue
            if rule == "height":
                out[ny * new_wide + nx] = max(seen, key=abs)
            else:
                out[ny * new_wide + nx] = collections.Counter(seen).most_common(1)[0][0]
    return out


def write(path, magic, wide, deep, half_x, half_y, cells):
    with open(path, "wb") as handle:
        handle.write(magic)
        handle.write(struct.pack("<II", wide, deep))
        handle.write(struct.pack("<ff", half_x, half_y))
        handle.write(cells.tobytes())


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: rescale_world.py <old WORLD_WIDTH> <new WORLD_WIDTH>")
    was, now = float(sys.argv[1]), float(sys.argv[2])
    scale = now / was
    print(f"world {was:.0f} m -> {now:.0f} m, so every painted layer scales by {scale:.4f}\n")

    for name, rule in LAYERS.items():
        path = os.path.join(WORLD, name)
        if not os.path.exists(path):
            print(f"{name:12} absent, nothing to do")
            continue
        magic, wide, deep, half_x, half_y, cells = read(path)
        # CELL from the file, not assumed - the layers do not share a resolution.
        cell = half_x * 2.0 / (wide - 1)
        new_half = (half_x * scale, half_y * scale)
        new_wide, new_deep = grid_for(new_half[0], new_half[1], cell)
        before = sum(1 for v in cells if v != 0.0)
        out = resample(cells, wide, deep, new_wide, new_deep, rule)
        after = sum(1 for v in out if v != 0.0)
        write(path, magic, new_wide, new_deep, new_half[0], new_half[1], out)
        print(
            f"{name:12} {cell:5.1f} m cells  {wide}x{deep} -> {new_wide}x{new_deep}  "
            f"half {half_x:.1f},{half_y:.1f} -> {new_half[0]:.1f},{new_half[1]:.1f}  "
            f"painted {before} -> {after} ({rule})"
        )


if __name__ == "__main__":
    main()
