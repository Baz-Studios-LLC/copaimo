# Models

3D models go here. Drop `.glb` / `.gltf` files in and they're loadable by path
relative to `assets/` — e.g. `assets/models/ranger.glb` is
`asset_server.load("models/ranger.glb#Scene0")`.

Suggested layout as this fills up:

| Folder | Contents |
| --- | --- |
| `models/ranger/` | The player character and equipment |
| `models/monsters/` | Monster companions, one folder per species |
| `models/props/` | Ranch pieces, fences, buildings, scatter |
| `models/nature/` | Trees, rocks, grass — anything the world scatters |

Everything the game currently draws is built from Bevy primitives in code, so
each of these is a straight swap when the real asset arrives:

| Placeholder | Lives in |
| --- | --- |
| Blocky ranger (body, head, hat) | `src/player.rs`, `spawn_player` |

Keep the ranger roughly 1.8 m tall whatever replaces it — the terrain, camera
distance and movement speed are all tuned against that height.
