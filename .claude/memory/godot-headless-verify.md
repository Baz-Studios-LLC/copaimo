---
name: godot-headless-verify
description: "How to verify a Godot project boots clean from the CLI (import pass first, then run headless)"
metadata: 
  node_type: memory
  type: reference
  originSessionId: 5ca35a6f-ce53-451a-9223-92f4d3d8f972
---

To validate a Godot 4 project without opening the GUI (e.g. catch parse/scene errors after edits):

1. **Import first** — on a fresh project the `class_name` global registry doesn't exist yet, so every cross-class reference falsely errors as "Could not find type X". Build the cache:
   `Godot_console.exe --headless --path <proj> --import`
2. **Then boot** the main scene for a few frames and filter output:
   `Godot_console.exe --headless --path <proj> --quit-after 120 2>&1 | grep -iE "error|script|fail|leak"`

Notes:
- Use the `_console` binary variant on Windows for real stdout.
- `ObjectDB instances leaked at exit` / `resources still in use` after `--quit-after` is USUALLY benign forced-quit noise — BUT it can also flag a real **RefCounted reference cycle** (two RefCounted objects holding each other never free). In Spirit Spire this caught a genuine `PlayerState <-> StateMachine` cycle; fixed by having states reach the machine via a Node reference (`player.sm`) instead of storing the RefCounted `sm` directly. Nodes aren't refcounted, so they don't form the cycle.

Related project: [[spirit-spire-project]].
