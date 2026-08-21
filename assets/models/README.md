# Models

3D models go here. Drop `.glb` / `.gltf` files in and they're loadable by path
relative to `assets/` — e.g. `assets/models/warden.glb` is
`asset_server.load("models/warden.glb#Scene0")`.

Suggested layout as this fills up:

| Folder | Contents |
| --- | --- |
| `models/warden/` | The player character and equipment |
| `models/monsters/` | Monster companions, one folder per species |
| `models/props/` | Ranch pieces, fences, buildings, scatter |
| `models/nature/` | Trees, rocks, grass — anything the world scatters |

Everything the game currently draws is built from Bevy primitives in code, so
each of these is a straight swap when the real asset arrives:

| Placeholder | Lives in |
| --- | --- |
| Blocky warden (body, head, hat) | `src/player.rs`, `spawn_player` |

Keep the warden roughly 1.8 m tall whatever replaces it — the terrain, camera
distance and movement speed are all tuned against that height.

## Getting a model in here

Do not export by hand. Run:

```
dev/model_export.sh art/warden.blend     # or a folder of .blend files
```

It sets the export options once and refuses a model that breaks the conventions:

* **metres, real scale** — the warden is 1.8 m and everything is tuned to it
* **feet on Z=0** — a model is placed by its origin, so its base must be its origin
* **facing +Y in Blender** — the Y-up conversion maps Blender's -Y (its own "front"
  view) onto +Z, and Bevy's forward is -Z, so the axis that feels right exports
  BACKWARDS

`cargo test` checks every `.glb` in this folder against the same rules, so a model
dropped in by hand is caught too. See `src/models.rs` and TROUBLESHOOTING.md.
