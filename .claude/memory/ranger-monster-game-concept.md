---
name: ranger-monster-game-concept
description: COPAIMO: THE WARDENS GUILD — monster-companion game; repo copaimo, local folder still ranger-game, Bevy 0.16
metadata:
  node_type: memory
  type: project
  originSessionId: 776b61b8-4294-4ee0-b9e9-04bf61dc40df
  modified: 2026-08-14T16:12:47.129Z
---

**RENAMED 2026-08-18: the game is COPAIMO: THE WARDENS GUILD and the player is a
WARDEN** (was "Ranger" for both). Title art at `assets/Title/Copaimo.png`; the
desktop/window icon is cut from the crest under the wordmark. The GitHub repo is
`Baz-Studios-LLC/copaimo` and the crate/binary is `copaimo` — but the **local
folder is still `Desktop/ranger-game`**, which is the one place the old name
survives, so paths in old notes still work.

Monster-companion adventure game — inspiration: **Pokémon + Monster Rancher**. Player is a Warden who raises monsters on a ranch and travels to cities to upgrade their Warden License by passing each city's Wardens Guild "exam" (gym-battle analogue). Battles are **turn-based**.

**Repo: `Baz-Studios-LLC/copaimo`** (created 2026-08-13, branch `main`). Made **PRIVATE** — every other Baz game repo is public, so flip it with `gh repo edit Baz-Studios-LLC/copaimo --visibility public` if the user wants it to match. `assets/world/edits.bin` (sculpting work) is committed ON PURPOSE — it's authored content and syncing it across machines is why the repo exists.

**Project: `Desktop/ranger-game (folder name unchanged)`** — Rust + **Bevy 0.16**, one plugin per concern. Started fresh **2026-08-13** (the July `copaimo` and `ranch-test` prototypes were gone from disk — nothing salvaged). Reference doc = **`ranger-game/DESIGN.md`** (pillars, world spec, invariants, change log) — keep it updated after substantive changes, same habit as [[crashout-design-log]].

**Premise / core loop:** join the World Ranger Association → base permit → build a ranch outside the village. Two intertwined goals: fix & upgrade the ranch, and travel to earn certifications. Each certification exam is in a **different city**, so progression drives travel. Certifications are the central gate: they expand the ranch AND raise the monster cap (starts at **3**). Monsters are required both to pass exams and to take guild missions posted in cities.

**Build order (2026-08-13):** the **open world FIRST** — "massive but finite", 3D. Models come after the world is in a good place.

**World is heightmap-driven, not noise-driven.** User supplied a fantasy world map (looks like Azgaar's Fantasy Map Generator — wide ~2:1 multi-continent sprawl, ~20 political regions; said **"ignore the names but let's go with this"**). `assets/world/heightmap.png` brightness = elevation and is the authority on landmass shape; procedural noise only adds close-up detail. Missing file → procedural fallback + warning, so it always runs.

- Scale = ONE knob, `WORLD_WIDTH` in `src/config.rs` (currently 8192 m); N–S extent derived from the map image's aspect ratio.
- World ends in **open ocean, never a wall**.
- Region-colored map image can later be a second lookup for placing cities/nations.

⭐ **TERRAIN TOOL NOW LIVES IN OPIFICIUM, NOT HERE (moved 2026-08-14).** See [[opificium-terrain-bench]]. `ranger-game/src/editor/` is DELETED; `src/world/edit.rs` is a READER only (no brushes). The game reads `assets/world/edits.bin`; Opificium writes it. Don't rebuild a sculpting tool in the game.

Wiring on the game side is JUST `assets/world/{heightmap.png, world.json, edits.bin}` — **no `opificium.json`, this game is NOT an Opificium project** (tried, rejected; see [[opificium-terrain-bench]]). To sculpt: open Opificium → BENCH → THE TERRAIN → OPEN A WORLD… → pick `assets/world/heightmap.png`. ⚠️ **Re-run `cargo test export_world_for_opificium -- --ignored --nocapture` whenever a world-shaping constant in `config.rs` changes**, or the bench and game disagree about the generated ground and every sculpted hill sits at the wrong height with NO on-screen sign. F3 overlay shows how many sculpted cells actually loaded (a mismatched edits.bin is refused, not applied).
- ⚠️ **Bevy's built-in font is an ASCII subset** — `·` `—` `→` render as TOFU BOXES. Keep ALL UI strings plain ASCII; build structure from layout (rule nodes, meter bars, boxed keycaps), not punctuation. Optional override: `assets/fonts/ui.ttf` (must be permissively licensed — NOT Windows system fonts).
- ⚠️ `cargo build` fails with linker error 1104 while the user has the game window open (exe locked). `cargo check`/`cargo test` still work — use those, and DON'T kill the process (may hold unsaved sculpting).

**World-gen gotchas learned the hard way (all in DESIGN.md):**
- ⚠️ NEVER use ridged-multifractal noise for mountains — produced a map-wide forest of spikes ("tooth like mountains") TWICE. Use `1 - |noise|` at low frequency, 2 octaves, modest power (~1.7); never square it or stack octaves.
- ⚠️ **Classify water by HUE, not brightness** (fixed 2026-08-14). A political map's labels, roads and dashed borders are all DARK, so a brightness threshold cut every place name into the terrain as a lake — and the majority filter's few-pixel reach is nowhere near a label stroke's thickness. Water is the one thing that's distinctly BLUE → test blue vs. red (`MAP_SEA_BLUE_MARGIN`, ocean 48–80 vs. every land fill ≤32 on this map). Grayscale heightmaps have no hue → auto-detected, thresholded on brightness (`MAP_SEA_THRESHOLD` ~0.20; it is UNUSED for colored maps now).
- ⚠️ A screenshot carries **UI furniture** (toolbar, scale bar) that isn't blue → became islands. Fixed by dropping land blobs under `MIN_ISLAND_PIXELS`, plus a border fade guaranteeing ocean at the world edge whatever the image shows there. Cropping the source is still the cleaner fix.
- ⚠️ **`INLAND_FULL` must sit BELOW the map's actual deepest interior** or nothing counts as inland and mountains **silently never appear** — no error, just a world of hills. This map tops out at **820 m** from any coast; 1100 m caused exactly that. The test prints the measured figure.
- Mountains are placed by GEOGRAPHY: a BFS from the water at load gives distance-from-coast, and ranges need presence + inland distance + a ridge line to all agree. Plains stay by the sea.
- `FLAT_WORLD` now **false** (mountains are in). Flip it true to re-check continent SHAPE after any map swap or mask change.
- ⚠️ **NO distance fog** — user had it removed 2026-08-13 (obscured the land's shape). Don't re-add it. Consequence: `VIEW_CHUNKS` (now 9, ~1150 m) IS the horizon; distance-based mesh LOD is the unbuilt real fix for seeing further.
- Verify world changes with `cargo test -- --nocapture` → prints an ASCII map (supersampled; single-sample preview aliases and lies).

**Conventions:**
- **`assets/models/`** is where 3D models go — user will make them; pull from and save generated ones there. Everything drawn now is Bevy primitives, each a straight swap (see that folder's README for the placeholder→file map).
- Ranger placeholder is **~1.8 m tall** — terrain scale, camera distance and move speed are all tuned against it; keep the height when art lands.
- I compile/verify; **user runs the window** (same as [[neon-edge-bevy-port]]).
- Shared logic factored per [[dry-no-code-reuse]]: `util.rs` math, `WorldBounds` extents, `Terrain::height` as the single source of truth for ground height.

**Why:** the monsters here are ALLIES/companions — never write wild monsters as threats to fight off.

**How to apply:** to reshape the world, swap the map image, not the code. To resize it, change `WORLD_WIDTH` only.
