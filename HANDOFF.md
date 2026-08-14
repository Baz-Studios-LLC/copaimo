# Ranger — handoff

Written 2026-08-14. Orientation for picking this up in a fresh session, across
three repositories. Not a transcript: what exists, why it is the way it is, and
what will bite you.

---

## The three repositories

| Folder | Repo | Branch | Version | What it is |
| --- | --- | --- | --- | --- |
| `Desktop/ranger-game` | `Baz-Studios-LLC/ranger-game` (**public**) | `main` | **v0.1.2** released | The game. Rust + **Bevy 0.16**, edition 2021 |
| `Desktop/Opificium` | `Baz-Studios-LLC/Opificium` (public) | `master` | **v0.6.0** released | The studio's maker's bench. Rust + **Bevy 0.19**, edition 2024 |
| `Desktop/baz-studios-launcher` | `Baz-Studios-LLC/baz-studios-launcher` | `main` | — | Tauri launcher; lists both |

**Two different Bevy majors.** Code does not move between them by copy-paste.
0.19 uses `ChildOf(parent)` instead of `with_children`, `Mesh::new(..).with_inserted_attribute(..)`,
`GlobalAmbientLight`, `shadow_maps_enabled`, `BorderColor::all(..)`, `bevy::text::FontSize`.

**Rust is not on PATH.** Use `~/.cargo/bin/cargo` (1.97.1).

**Opificium has other contributors.** `git fetch` and compare before starting;
work on a branch and open a PR. v0.5.2 landed from someone else mid-session and
had to be rebased onto.

---

## The game ↔ bench contract

They **share no code, only files**. Documented in Opificium's `FORMATS.md`.

| File | Direction | What |
| --- | --- | --- |
| `ranger-game/assets/world/heightmap.png` | game → bench | The map the continents are traced from |
| `ranger-game/assets/world/world.json` | game → bench | Every constant that turns that map into ground |
| `ranger-game/assets/world/edits.bin` | bench → game | Hand-sculpted ground, as **signed height offsets** |

⚠️ **Re-export `world.json` whenever a world-shaping constant in
`ranger-game/src/config.rs` changes:**

```bash
cargo test export_world_for_opificium -- --ignored --nocapture
```

A maker sculpts *offsets*, and the game adds them to ground it generates itself.
If the two disagree about what was underneath by a metre, every sculpted hill
sits at the wrong height **and nothing on screen says why**.

Sites, roads and all geometry constants go through this file. Colour does **not**
(`SHORE_FREQ`) — colour cannot put a hill in the wrong place, so drift there is
cosmetic. Both keep the same number anyway.

### How a bench change reaches the game

```
sculpt at Opificium  →  Ctrl+S  →  assets/world/edits.bin  →  next launch
```

Read **once, at startup**; there is no hot-reload. From source, relaunching is
the refresh. An **installed build has its own copy of `assets/`**, so sculpting
the repository does nothing for it until a new release ships — the workflow
packages `assets/` wholesale, so a tag carries whatever is committed then.
`edits.bin` is deliberately **not** gitignored; commit it or the shaping exists
on one machine only.

### A world is NOT an Opificium project

This was got wrong once and rejected. The terrain bench is a **tool you bring
ground to**, like the kiln takes an image: `OPEN A WORLD…` on its shelf picks a
`heightmap.png`, and the folder it sits in is the world. `ranger-game/opificium/opificium.json`
names `"world": "../assets/world"` — a **hint** that saves a walk across the
disk, never a requirement.

---

## Where things are

### ranger-game
- `src/config.rs` — every world constant, and the `world.json` exporter
- `src/world/heightmap.rs` — map image → land mask, signed shore distance
- `src/world/terrain.rs` — the heightfield; `base_height` is the single source of truth
- `src/world/settle.rs` — town/city placement + road network
- `src/world/biome.rs`, `chunk.rs`, `stream.rs`, `water.rs`
- `src/world/edit.rs` — **reader only**. The game never sculpts.
- `DESIGN.md` — the reference. Records approaches tried and rejected. **Read before touching world generation.**

### Opificium
- `src/terrain/` — the whole bench: `mod.rs` (plugin, brush, gestures), `ground.rs`
  (Recipe + heightfield), `settle.rs`, `chunk.rs` (meshing, sea), `edit.rs`
  (sculpt grid, undo, brush ops), `opened.rs` (which world), `shelf.rs` (UI)
- Everything outside `src/terrain/` is ~56 lines across 5 files, all additive or
  gated on `Bench::Terrain`. **Keep it that way.**

---

## Invariants — things that broke and must not break again

1. **Nothing may change height faster than the vertex grid can draw it.** Coastlines
   came out as picket fences of vertical slats twice. Fixed by shelving the coast
   over `BEACH_WIDTH`/`SHELF_WIDTH` instead of the mask blur's few metres.
2. **Never use ridged multifractal noise for mountains.** Tried twice, produced a
   map-wide forest of spikes. Use `1 - |noise|`, low frequency, two octaves,
   modest power (~1.7). Never square it, never stack octaves.
3. **`INLAND_FULL` must sit below the map's deepest interior** or nothing counts
   as inland and mountains **silently never appear**. Current map tops out at
   **820 m** from any coast. The test prints this figure.
4. **Classify water by HUE, not brightness.** Place names, roads and borders are
   all dark; a brightness threshold cuts every label into the terrain as a lake.
   Ocean is blue-dominant (B−R 48–80 on this map); land fills are ≤32.
5. **Screenshots carry UI furniture** — toolbars, scale bars — which is not blue
   and becomes islands. Handled by `MIN_ISLAND_PIXELS` + a border fade.
6. **The world ends in water, never a wall.**
7. **Sand is conditional**, on a gentle shore and a low-frequency field along the
   coast. A uniform sand ring around every continent reads as a drawn map.
8. **Waves shorter than ~4 vertices are a lie.** The sea spans several times the
   world, so its vertices are >100 m apart; short swell renders as noise. Only
   long swell (900 m, 1800 m) is real.

---

## Verifying world changes

```bash
cargo test -- --nocapture
```

Prints an ASCII map plus land %, low/peak height, the deepest-inland figure, and
the site/road counts. **Faster and more honest than looking at the 3D view.**
It is supersampled — a single-sample preview aliases and lies.

`FLAT_WORLD` in `config.rs` flattens all land to isolate coastline shape.

---

## Running things

```bash
cargo run --release
```

Opificium also has `Desktop/Opificium.cmd` — double-click, runs from source on
whatever branch is checked out, no release or launcher involved.

⚠️ `cargo build` fails with a linker error while the app's window is open (the
.exe is locked). `cargo check` and `cargo test` still work.

---

## Terrain bench controls

`1`–`8` tools · drag applies · right-drag inverts · `[` `]` radius · `-` `=` strength
`Shift`+drag turns the eye · `Shift`+`1`–`6` drafting angles · middle-drag pans · wheel zooms
`Ctrl+Z`/`Ctrl+Y` undo/redo · `Ctrl+S` saves

**Shift is the camera at this bench** because both mouse buttons are tools.
Ramp (`8`) is *clicked*, not dragged: start, far end, right-click abandons.

---

## What is done, and what is not

**Done:** the world — map-driven continents, shelving coasts, varied shorelines,
ruggedness (level plains vs. mountain country), 6 cities + 14 towns on levelled
ground joined by graded roads, a moving sea with a tide, wading limit. The
terrain bench with 8 brushes, undo/redo, live re-meshing, whole-world view.

**The world is deliberately FLAT.** Ranges are 52 m and the inland climb 28 m —
plains and hills you cross, not terrain that stops you. Pokémon-like, by
request. Against that sits **one massif**, 340 m and ~1 km across, standing
wherever the map is furthest from any sea. It is *found, not chosen*, so
redrawing the map moves it to the new heartland; it ignores the ruggedness field
because it is the exception the rest of the world is gentle in order to make;
and it is placed *before* towns so none is levelled onto its flank. Do not
"fix" the flatness — it is the brief.

**Both mouse buttons are tools at the terrain bench**, so the camera moved to
**Shift**+drag and the drafting angles to Shift+1–6. That is why `camera.rs` is
one of the five shared Opificium files this work touches.

**Not started:** monsters, the ranch, battles, guild exams, cities as *places*
(only their ground exists), 3D models (`assets/models/` is where they will go —
everything is Bevy primitives now, each a straight swap).

**Trees: built in Opificium, NOT YET IN THE GAME.** `tree.rs` grows them,
`forest.rs` places them, `chunk.rs` plants them as children of their chunk, and
PLANT on key 9 adds and clears. Ctrl+S keeps `edits.bin` and `forest.bin`
together. 126 tests pass.

**The port to the game is the next job**, and it is mechanical but real: the
game needs its own `tree.rs` and `forest.rs` (Bevy 0.16, so no copy-paste — see
the version note above), the same recipe fields in `config.rs` and its exporter,
and trees spawned per chunk. Both must scatter identically or the forest differs
between bench and game. The recipe already carries `tree_spacing`, `treeline`,
`tree_scale_low/high`.

**Also not done: shelf controls for how trees LOOK.** The knobs exist in the
recipe and bark/leaf take the game's ramps; there is no UI to turn them without
editing `world.json`.

The design, so a fresh session does not redesign it:

* **Auto-placed first, then adjustable.** Both programs work out a *base*
  density from the ground itself — moisture, height under the treeline, gentle
  slope, clear of the beach, and clear of the levelled ground under towns and
  roads. Nobody hand-plants 16 km² of forest.
* **A painted layer on top**, exactly the shape `edits.bin` already has: a grid
  of signed bias in −1..+1 where **0 means leave the automatic answer alone**,
  +1 forces forest and −1 forces cleared. Saved as `assets/world/forest.bin`,
  written by the bench, read by the game. Same reasoning as offsets-not-heights:
  re-tune the automatic placement and hand-painted woods stay where they were put.
* **Scattered deterministically** from a hash of position, so both programs grow
  the same forest with no list of trees passing between them.
* **Geometry is grown, not modelled**: a tapered trunk, branches recursed off it
  from the tree's own seed, then leaf clusters at the tips.
* **A pool of variants, not a mesh per tree.** Thousands of unique meshes is not
  affordable; ~16–24 grown variants, each instanced with its own rotation and
  scale, reads as "every tree different" and costs almost nothing. This is a
  deliberate trade and worth stating to the user rather than implying every
  single tree is unique.

**Next after that, if wanted** (researched, not built): hydraulic erosion,
terrace, stamp, brush falloff control, slope/height masks. Unreal and Unity both
ship these.

---

## Open loose ends

- **Windows long-path bug in Opificium** (pre-existing, untouched): `Project::read`
  canonicalises and gains a `\\?\` prefix while the picker stores the raw path, so
  every project eventually appears **twice** on the opening screen. Fix is in
  `project.rs`.
- **The builder's 14 m floor grid draws at the terrain bench** — a speck inside an
  8 km world that pokes through ground at the origin. Reverted to keep out of
  `stage.rs`; one marker component would hide it.
- **`INLAND_FULL` must stay under the map's real deepest interior** (820 m on the
  current map; the constant is 620 m). Set it above and *nothing counts as
  inland*, so mountains silently never appear — no error, just a world of hills.
  The ASCII map prints the real number; check it after any map swap.
- **No icon for Ranger.** `packaging/Info.plist` names none on purpose.
