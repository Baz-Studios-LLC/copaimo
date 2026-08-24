"""Says what is actually inside a .glb, without opening Blender.

    python dev/art/inspect_glb.py assets/character/*.glb

The first thing to run on anything delivered, and deliberately not a Blender script: it reads
the file's own JSON, so what it reports is what the FILE says rather than what an importer made
of it. Those differ more than they should - an importer invents lengths for leaf bones, splits
vertices it thinks need splitting, and renames things.

What it answers, in the order the answers matter:

  Is there one character here or three?      meshes, vertex counts, whether the skins match
  Do the clips share a skeleton?             joint names and their order
  How long is each clip, really?             from the accessor's own min/max time, not a frame
                                             count - glTF stores ABSOLUTE times, and a 24-frame
                                             cycle plus a seam key exports as 25/24 = 1.0417 s
  What will need fixing before it ships?     bone counts, influences per vertex, texture size
"""
import json
import os
import struct
import sys

TYPES = {5120: "b", 5121: "B", 5122: "h", 5123: "H", 5125: "I", 5126: "f"}
COUNTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def chunks_of(raw):
    if raw[:4] != b"glTF":
        raise SystemExit("not a .glb")
    total = struct.unpack("<I", raw[8:12])[0]
    at, out = 12, {}
    while at < total:
        length, kind = struct.unpack("<I4s", raw[at:at + 8])
        out[kind.strip(b"\x00").decode()] = raw[at + 8:at + 8 + length]
        at += 8 + length + ((4 - length % 4) % 4)
    return out


def read(tree, blob, index):
    """One accessor, as a list of tuples. Only what is needed to answer the questions above."""
    acc = tree["accessors"][index]
    view = tree["bufferViews"][acc["bufferView"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    wide = COUNTS[acc["type"]]
    kind = TYPES[acc["componentType"]]
    size = struct.calcsize("<" + kind)
    stride = view.get("byteStride") or wide * size
    out = []
    for i in range(acc["count"]):
        at = start + i * stride
        out.append(struct.unpack_from("<" + kind * wide, blob, at))
    return out


def look(path):
    parts = chunks_of(open(path, "rb").read())
    tree = json.loads(parts["JSON"])
    blob = parts.get("BIN", b"")
    print(f"\n=== {os.path.basename(path)} ({os.path.getsize(path) / 1e6:.1f} MB) ===")

    made = tree.get("asset", {})
    print(f"  made by {made.get('generator', 'unknown')}")

    for index, mesh in enumerate(tree.get("meshes", [])):
        for part in mesh["primitives"]:
            verts = tree["accessors"][part["attributes"]["POSITION"]]["count"]
            tris = tree["accessors"][part["indices"]]["count"] // 3 if "indices" in part else 0
            has = ",".join(sorted(part["attributes"]))
            print(f"  mesh {index} '{mesh.get('name', '')}': {verts} vertices, {tris} triangles")
            print(f"    attributes: {has}")

    for index, skin in enumerate(tree.get("skins", [])):
        joints = skin["joints"]
        names = [tree["nodes"][j].get("name", f"node{j}") for j in joints]
        print(f"  skin {index}: {len(joints)} joints")
        print("    " + ", ".join(names[:12]) + (" ..." if len(names) > 12 else ""))

    for clip in tree.get("animations", []):
        spans = []
        for track in clip["samplers"]:
            acc = tree["accessors"][track["input"]]
            if "min" in acc and "max" in acc:
                spans.append((acc["min"][0], acc["max"][0]))
        first = min(s[0] for s in spans) if spans else 0.0
        last = max(s[1] for s in spans) if spans else 0.0
        moved = {tree["nodes"][c["target"]["node"]].get("name", "?")
                 for c in clip["channels"] if "node" in c["target"]}
        print(f"  clip '{clip.get('name', '')}': {last - first:.4f} s "
              f"({first:.4f} to {last:.4f}), {len(clip['channels'])} channels over "
              f"{len(moved)} bones")

    for index, image in enumerate(tree.get("images", [])):
        view = tree["bufferViews"][image["bufferView"]]
        print(f"  image {index}: {image.get('mimeType')} "
              f"{view['byteLength'] / 1e6:.1f} MB")

    # How many bones actually drive a vertex. glTF carries four per set; anything past the sets
    # present in the file was dropped at export, silently.
    for mesh in tree.get("meshes", []):
        for part in mesh["primitives"]:
            sets = [k for k in part["attributes"] if k.startswith("WEIGHTS_")]
            if not sets:
                continue
            most, spread = 0, 0
            for row in read(tree, blob, part["attributes"][sets[0]]):
                scale = 65535.0 if isinstance(row[0], int) and max(row) > 1 else 1.0
                live = sum(1 for w in row if w / scale > 0.001)
                most = max(most, live)
                spread += 1
            print(f"  skinning: {len(sets)} weight set(s), at most {most} bones drive a vertex")


if __name__ == "__main__":
    for path in sys.argv[1:]:
        look(path)
