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
| `Desktop/terrain-core` | `Baz-Studios-LLC/terrain-core` (public) | **`master`** | — | The world generation and sculpting brush **both** link. No engine named |
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

They share **`terrain-core`** — the world generation, the forest scatter, the
tree growing and the sculpting brush — and otherwise only **files**, documented
in Opificium's `FORMATS.md`. A change to the crate reaches both with
`cargo update -p terrain-core`; anything not in the crate still has to be
written twice.

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

## DONE: the editor is back in the game

**This reversed an earlier decision, and it is finished.** *Shape the World*, on
the main menu, nine brushes on keys 1–9.

The terrain tool started in the game, moved out to Opificium mid-session, and
came back — because that is what studios actually do. Unreal's Landscape and
Unity's terrain are runtime systems the editor wraps tooling around, gated behind
`WITH_EDITOR` / `#if UNITY_EDITOR` and stripped from shipping builds. The editor
is built ON TOP of the game, one codebase. What moves between editor and game is
data, never logic.

How it ended up:

* **`terrain-core`** holds everything about the world *including* the editing
  operations — `sculpt::{Sculpt, Brushing, Stamp}`, the undo stack, `slump`,
  `ramp` — alongside generation, the forest scatter and the tree growing. It
  names no engine and must stay that way.
* **`ranger-game`** has its terrain mode back in `src/editor/`, which is the
  *mode* only: aiming, gestures, the panel, telling chunks to mesh again. It
  shapes nothing itself. `src/world/edit.rs` is the thin adapter that knows where
  this game keeps `edits.bin`, the same shape `world/forest.rs` has.
* **`Opificium`** keeps its terrain bench, now on the crate too, with its own
  copies deleted (~1,600 lines) — PR
  [Opificium#2](https://github.com/Baz-Studios-LLC/Opificium/pull/2). **Do not
  gut it** — it is released (v0.6.0), other people work in that repo, and it is
  not in the way.

### The one thing that will bite a future move

`terrain-core` asks for glam as a **range** (`>=0.29, <0.33`), never a pin. The
whole crate rests on its `Vec2` being the engine's `Vec2` — Bevy re-exports
glam's — and every Bevy release carries its own glam. Bevy 0.16 has 0.29, Bevy
0.19 has 0.32. Pinned to either, the other program links a second glam and the
compiler spends two dozen errors insisting `Vec2` is not `Vec2`. **Widen the
upper bound when either program moves to a newer Bevy.**

### Still open in the tool

* **Nothing about a tree's look** can be adjusted; the knobs ride in `world.json`
  and no shelf reads them.
* **`settle.rs` is still written twice** — towns, quotas and roads exist in both
  programs and must agree. It is the obvious next thing to move into the crate,
  and the ranch fields (`ranch_x`, `ranch_z`, `ranch_radius`) the game exports
  are a live example: Opificium's `Recipe` does not read them, so the bench does
  not level the ranch shelf.

---

## The working rule: the bench and the game move together

**Anything added to Opificium gets ported to the game in the same pass.** Not
later, not next session. The two are halves of one thing — the bench writes what
the game reads — and a bench feature with no game counterpart is a feature
nobody can play.

This got much easier, and for the part that is now shared it is automatic: the
world generation, the forest scatter, the tree growing and the sculpting brush
all live in `terrain-core` and **both programs link it**. Change the crate, push
it, `cargo update -p terrain-core` on both sides, and they cannot disagree.

It is not automatic for anything else. Different Bevy majors, separate
repositories, and only FILES pass between them otherwise — a bench feature whose
logic is not in the crate still needs writing twice, and `settle.rs` is exactly
that today.

**For anything that stays written twice**, write the pinning test: take literal
numbers OUT of one program and assert them in the other. Do not recompute the
expected values with a second copy of the same arithmetic — it will agree with
itself no matter how wrong it is. And do not invent the numbers, which is how
that pattern was first got wrong here: every "expected" value was a guess, and
every one was wrong. Better still, move the thing into the crate and delete the
question.

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

## Terrain controls

Same nine tools in both places, in the same order, wearing the same colours.

**In the game** — *Shape the World* from the main menu:

`1`–`9` tools · drag applies · right-drag inverts · wheel sizes the brush ·
`[` `]` strength · `Ctrl+Z`/`Ctrl+Y` undo/redo · `Ctrl+S` saves ground **and**
woods · `Esc` back to the menu. The camera is free-fly and mouse-look, the way
the rest of the game flies.

**At the bench** (Opificium):

`1`–`9` tools · drag applies · right-drag inverts · `[` `]` radius · `-` `=` strength
`Shift`+drag turns the eye · `Shift`+`1`–`6` drafting angles · middle-drag pans · wheel zooms
`Ctrl+Z`/`Ctrl+Y` undo/redo · `Ctrl+S` saves

**Shift is the camera at the bench** because both mouse buttons are tools; the
game has no such problem, since mouse-look needs no button.

Ramp (`8`) is *clicked* in both, not dragged: start, far end, right-click
abandons. Plant (`9`) paints woods and never moves earth — right button clears.

---

## What is done, and what is not

**Done:** the world — map-driven continents, shelving coasts, varied shorelines,
ruggedness (level plains vs. mountain country), 6 cities + 14 towns on levelled
ground joined by graded roads, a moving sea with a tide, wading limit,
procedurally grown woods. The terrain tool with 9 brushes, undo/redo, live
re-meshing and whole-world view — **in the game and at the bench**, both driving
the same brush out of `terrain-core`.

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

**Trees: in both, and grown by the same code.** `terrain_core::tree` grows them,
`terrain_core::forest` places them, and each program plants them as children of
their chunk. PLANT on key 9 adds and clears in either place. `Ctrl+S` keeps
`edits.bin` and `forest.bin` together.

### What used to be the sharp edge, and no longer is

Where trees stand was written twice, and its failure was silent: both programs
worked the forest out from scratch, never exchanged a list of trees, and a digit
out of place in the hash gave the bench one forest and the game another with
nothing to point at. It was held together by a page of things that had to match
exactly — the scatter multipliers, the six salts *in order*, the world-wide slot
lattice, the order `Draw` draws a tree's numbers in, every rejection rule — and
by tests pinning literal values copied between the programs.

**That page is now `terrain-core`.** One implementation, so there is nothing to
keep in step. The constants are still load-bearing — changing `chance()` moves
every wood in every world already planted — and the crate guards them with
`the_scatter_is_what_it_has_always_been`, but it guards them against *accident*
rather than against the other program.

What still has to agree is the RECIPE, because each side reads its own
`world.json`: `tree_spacing` 14.0, `treeline` 150.0, `tree_scale_low` 0.75,
`tree_scale_high` 1.35. Re-export it whenever a world-shaping constant changes.

**Not done: shelf controls for how trees LOOK.** The knobs exist in the recipe
and bark/leaf take the game's ramps; there is no UI to turn them without editing
`world.json`.

The design, so a fresh session does not redesign it:

* **Auto-placed first, then adjustable.** The base density comes from the ground
  itself — moisture, height under the treeline, gentle slope, clear of the beach,
  and clear of the levelled ground under towns and roads. Nobody hand-plants
  16 km² of forest.
* **A painted layer on top**, exactly the shape `edits.bin` has: signed bias in
  −1..+1 where **0 means leave the automatic answer alone**. Same reasoning as
  offsets-not-heights: re-tune the automatic placement and hand-painted woods
  stay where they were put.
* **Scattered deterministically** from a hash of position, so no list of trees
  passes between the programs.
* **Geometry is grown, not modelled**: a tapered trunk, branches recursed off it
  from the tree's own seed, then leaf clusters at the tips.
* **A pool of variants, not a mesh per tree.** Thousands of unique meshes is not
  affordable; 20 grown variants, each instanced with its own rotation and scale,
  reads as "every tree different" and costs almost nothing. This is a deliberate
  trade and worth stating to the user rather than implying every single tree is
  unique.

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

---

## Where it got to, 2026-08-17

The world is the whole of what exists. No monsters, battles, ranch or exams.
Released through **v0.1.6**; `main` is ahead of it by a run of fixes.

### What the world has

Biomes (`terrain-core/src/biome.rs`) answer what kind of place any point is —
water, shore, grass, forest, desert, rock, snow, settled — and everything hangs
off that: seven tree species growing where they belong, grass and flowers and
scrub, and later which monsters live where. Rivers are FOUND by flow
accumulation, not placed. Day and night follow the player's own clock. Roads are
authored with PATH; the generator no longer lays them (`LINK_TOWNS_WITH_ROADS`).

### The three lessons that keep repeating

**Measure the world, not a fixture.** Every real fault this week was found by a
test that loaded the actual world and counted something — 23 cells of channel,
0 vertices of cover, a 13.8 m lake, 1,119 water samples down to 181. Fixture
tests passed throughout and found none of them.

**A shape that reads wrong is usually the wrong shape, not the wrong number.**
Clouds and leaves were jittered octahedra; no amount of scaling or recolouring
was going to fix a shard. Same for stars, which were literally triangles.

**Flat worlds break assumptions.** This world is deliberately flat, and that is
what broke drainage (no cell had a lower neighbour), river surfaces (bank ground
sits at bed height), and the tide (a smoothstep beach has zero slope at the
waterline). Anything reasoning about slope needs checking against flat ground.

**Ask the question where the answer is used.** The last seven inverted faces
survived two fixes because both decided the winding for a whole TUBE or a whole
QUAD, and the thing that gets culled is a triangle. Deciding on behalf of
something larger than the unit that consumes the decision is the bug.

### Known open

* **Trees may still be floating.** Reported twice. `drawn_height` was meant to
  fix it — trees now sit on the bilinear surface the renderer draws rather than
  the true height — and the last screenshots still looked wrong, though the
  open-ended limbs were confusing the picture. Verify before doing more.
* **B0004 on chunk children.** `stream.rs` spawns a chunk with `Transform` and
  no `Visibility`, then hangs meshes off it — Bevy warns that inherited
  visibility is inconsistent. Harmless while nothing hides a chunk; it will not
  stay harmless. Pre-existing, unrelated to anything recent.
* **`assets/world/edits.bin`** is modified in the working tree — the maker's own
  sculpting, left uncommitted for them to judge.

### Working rules

PATCH versions only, however big a session felt. Push the crate before the game
and `cargo update -p terrain-core`. Never tag with red tests — check the exit
code, do not pipe through `tail`. The shell's working directory drifts between
the two repos; `cd` explicitly in every command.
