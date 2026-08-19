"""Turns `dump_the_ground`'s hex rows into a PNG, for looking at the ground.

    cargo test dump_the_ground -- --ignored --nocapture | python dev/ground.py out.png

The ground is coloured per vertex on a two-metre grid, so what a flat field looks
like is decided entirely by `biome::surface_color` — which means it can be judged
without launching anything. One pixel a vertex, scaled up so the blotching is
visible at a glance.
"""

import struct
import sys
import zlib

SCALE = 4


def main() -> None:
    out = sys.argv[1] if len(sys.argv) > 1 else "ground.png"
    wide = high = 0
    rows: list[bytes] = []

    for line in sys.stdin:
        line = line.strip()
        if line.startswith("GROUND "):
            _, w, h = line.split()
            wide, high = int(w), int(h)
            continue
        if wide and len(line) == wide * 6 and all(c in "0123456789abcdef" for c in line):
            rows.append(bytes.fromhex(line))
            if len(rows) == high:
                break

    if not rows:
        sys.exit("no ground in the input — did the test run with --ignored --nocapture?")

    raw = bytearray()
    for row in rows:
        for _ in range(SCALE):
            raw.append(0)
            for x in range(wide):
                raw += row[x * 3 : x * 3 + 3] * SCALE

    def chunk(kind: bytes, body: bytes) -> bytes:
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", wide * SCALE, len(rows) * SCALE, 8, 2, 0, 0, 0)
    with open(out, "wb") as f:
        f.write(
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
            + chunk(b"IEND", b"")
        )
    print(f"{wide}x{len(rows)} vertices -> {out}")


main()
