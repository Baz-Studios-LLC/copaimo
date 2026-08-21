"""Talks to the Blender MCP add-on's socket, so a live session can be driven.

    python dev/blender_live.py "result = bpy.app.version_string"
    python dev/blender_live.py --file some_script.py
    python dev/blender_live.py --look out.png

# Why this exists beside the batch pipeline

`dev/art/*.py` runs under `blender --background`: a fresh Blender per attempt, a
whole rebuild for every change. That is right for the ASSETS — a rock or a tree
should come out the same every time from the same script, and batch mode is what
guarantees it.

It is wrong for figuring out a shape. Working out why a figure reads as a doll took
four rebuild-and-render cycles, each of them starting Blender again from nothing,
and every cycle threw away the scene that would have let the next question be
answered in a second.

The add-on (Blender's own, `blender.org/lab/mcp-server`) opens a socket into a
RUNNING Blender. So a shape can be nudged and looked at and nudged again against
one scene. What comes out of that is then written into `dev/art/` as a script, so
the asset stays reproducible — this is for finding the numbers, not for keeping them.

# The protocol

Null-byte-delimited JSON, one request per connection:

    {"type": "execute", "code": "<python>", "strict_json": true}

and the reply is `{"status": "ok", "result": ...}` or `{"status": "error", ...}`.
Assign to `result` in the code to get something back. `bpy` is already imported in
the add-on's namespace, but importing it again is free and clearer.
"""

import argparse
import json
import os
import socket
import sys

HOST = "localhost"
PORT = 9876

# Long enough for a render, which is the slowest thing anybody asks for here.
PATIENCE = 180.0


def ask(code: str, timeout: float = PATIENCE) -> dict:
    """Runs `code` in the live Blender and returns its reply."""
    try:
        link = socket.create_connection((HOST, PORT), timeout=timeout)
    except OSError as why:
        raise SystemExit(
            f"nothing is listening on {HOST}:{PORT} ({why}).\n"
            "Open Blender, enable the MCP add-on, and start its server — or run\n"
            "  blender --background file.blend --command blender_mcp\n"
            "for a headless one."
        ) from why
    try:
        link.sendall(
            (json.dumps({"type": "execute", "code": code, "strict_json": True}) + "\0").encode()
        )
        buf = b""
        while not buf.endswith(b"\0"):
            chunk = link.recv(1 << 16)
            if not chunk:
                break
            buf += chunk
    finally:
        link.close()
    if not buf:
        raise SystemExit("Blender closed the connection without answering")
    return json.loads(buf.rstrip(b"\0").decode("utf-8"))


# A render of whatever the live scene currently is, written where it can be looked
# at. Kept here rather than typed out each time, because "show me" is the whole
# reason for talking to a live session at all.
LOOK = '''
import bpy, os
scene = bpy.context.scene
scene.render.filepath = {out!r}
scene.render.resolution_x, scene.render.resolution_y = {wide}, {high}
scene.render.image_settings.file_format = "PNG"
if scene.camera is None:
    result = {{"drawn": False, "why": "the scene has no camera"}}
else:
    bpy.ops.render.render(write_still=True)
    result = {{"drawn": os.path.exists({out!r}), "at": {out!r}}}
'''


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("code", nargs="?", help="Python to run in the live Blender.")
    parser.add_argument("--file", help="Run a script file instead.")
    parser.add_argument("--look", help="Render the live scene to this PNG.")
    parser.add_argument("--size", default="1100x700", help="Render size, WxH.")
    args = parser.parse_args()

    if args.look:
        wide, _, high = args.size.partition("x")
        out = os.path.abspath(args.look)
        code = LOOK.format(out=out, wide=int(wide), high=int(high))
    elif args.file:
        with open(args.file, encoding="utf-8") as script:
            code = script.read()
    elif args.code:
        code = args.code
    else:
        parser.error("give some code, --file, or --look")

    reply = ask(code)
    print(json.dumps(reply, indent=1)[:4000])
    # A non-zero exit on error, so a shell chain stops rather than carrying on
    # against a scene that is not what it thinks it is.
    if reply.get("status") != "ok":
        sys.exit(1)


main()
