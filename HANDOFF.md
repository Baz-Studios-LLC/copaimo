# Copaimo — handoff

Written 2026-08-14. Orientation for picking this up in a fresh session, across
three repositories. Not a transcript: what exists, why it is the way it is, and
what will bite you.

---

## The three repositories

| Folder | Repo | Branch | Version | What it is |
| --- | --- | --- | --- | --- |
| `Desktop/copaimo` | `Baz-Studios-LLC/copaimo` (**public**) | `main` | **v0.1.2** released | The game. Rust + **Bevy 0.16**, edition 2021 |
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
| `copaimo/assets/world/heightmap.png` | game → bench | The map the continents are traced from |
| `copaimo/assets/world/world.json` | game → bench | Every constant that turns that map into ground |
| `copaimo/assets/world/edits.bin` | bench → game | Hand-sculpted ground, as **signed height offsets** |

⚠️ **Re-export `world.json` whenever a world-shaping constant in
`copaimo/src/config.rs` changes:**

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
`heightmap.png`, and the folder it sits in is the world. `copaimo/opificium/opificium.json`
names `"world": "../assets/world"` — a **hint** that saves a walk across the
disk, never a requirement.

---

## Where things are

### copaimo
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
* **`copaimo`** has its terrain mode back in `src/editor/`, which is the
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

### The bench gizmo — read this before touching it

Five rounds went into the move arrows and they are still not right. The failures
are worth listing because four of them were the same mistake:

1. clicking an arrow also placed a piece (`place` ran first)
2. reaching for an arrow deselected the piece — a **deadlock**: the ground cursor
   is where the view ray meets the floor, so pointing at an arrow throws it off
   the piece; the selection was dropped, and hovering could never be discovered
   because that needs a selected piece
3. nothing lit up, so a working grab and a dead control looked identical
4. dragging jittered — the drag measured against the arrows' CURRENT position,
   and the arrows sit on the piece the drag is moving
5. still reported: the red arrow "worked once then disappeared"

**The root cause of the class is two sources of truth for where an arrow is.**
`gizmo::show` draws meshes at one place; `ray_against_axis` hit-tests an abstract
line it believes matches. They can disagree, and every symptom above is a version
of them disagreeing.

**The fix is to hit-test the entities that are actually drawn**, which is what
Opificium's `ray_scan` in `src/builder/hand.rs` does: for each candidate it
inverts the object's rotation, brings the ray into local space, and does a slab
test against the box. Its `Hovered { grab: Option<Entity>, build: Option<Hit> }`
is the shape to copy — the IDEA, not the code, which is wired into `Placed`,
`PartKind` and `Slab` and would be a data-model rewrite to lift.

Do not port the file. Port the approach: spawn the arrows with their sizes,
ray-vs-box each one in its own local space, and let the entity you hit BE the
answer. Then there is nothing for a hit test to disagree with.

## Still open in the tool

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
- **No icon for Copaimo.** `packaging/Info.plist` names none on purpose.

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

**Two fields deciding one thing will disagree.** This is the shape of nearly
every fault in the water: a level and the ground; an extent from `bed` and a
depth from `cut`; a drawn surface and a biome; a cut recorded before towns and a
ground levelled after. The fix each time was to delete one of them, not to
reconcile them.

**A test that restates the definition can never fail.** `water - ground <=
RIVER_DEPTH` passed through four versions that all put sheets of water on grass,
because the water was DEFINED as ground plus that. What caught it was measuring
the thing the eye sees — the step at the water's rim — which nothing in the
definition guarantees.

**Ask the question where the answer is used.** The last seven inverted faces
survived two fixes because both decided the winding for a whole TUBE or a whole
QUAD, and the thing that gets culled is a triangle. Deciding on behalf of
something larger than the unit that consumes the decision is the bug.

### Known open

* **Frame rate: shadows are the frame.** Measured at midday, 1600×900, RTX 4060
  laptop: 42 fps / 23.8 ms, of which **16.7 ms is the three shadow cascades** and
  6.2 ms is everything actually on screen. Two things got it there from 30 fps —
  cutting the cascades from four at 900 m to three at 400 m, and halving the leaf
  clump count. Further shadow tuning does NOT help: 200 m was no better than 400,
  a smaller shadow map was no better than a large one, and two cascades were
  worse than three. The per-cascade cost is dominated by re-submitting all the
  tree geometry, so the next real win is **vegetation LOD or a tree draw radius**
  — not another pass over the shadow constants.
  ⚠️ Measure at MIDDAY and let the machine cool between runs. At night the sun is
  behind the world and the cascades catch almost nothing (101 fps), and
  back-to-back runs on a laptop throttle enough to swing the main pass ±40%.

* **Rivers are switched off** (`RIVERS` in `config.rs`). Not broken-and-hidden:
  the slabs on dry ground were fixed and the towns cleared, and then the width
  turned out to be the thing that could not be fixed by tuning. A channel's cut
  spreads three times its own width, so the water does too, and a fifth of the
  land ended up under it. `BANKS`, `RIVER_EDGE` and `NARROWEST` each fix it and
  break one of the other two — see the design log. Turning them back on wants a
  decision about what a river is at this scale, not a pass over the constants.

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

---

# The next four pieces (asked for 2026-08-17, after v0.1.7)

Four separate projects, in the order they unblock each other. Item 4 is bigger
than everything in v0.1.7 put together and is not a one-session job.

## 1. Cut v0.1.7 — DONE

Tagged and pushed. 36 commits since v0.1.6.

## 2. Biome painter in the terrain tool

**This is the one that matters most, and not because painting is fun.** Five
separate rounds of "the desert is in the wrong place" happened because the only
way to move a region was for me to read a marker off a screenshot, guess which
ellipse it implied, and nudge a number. The maker can point; I cannot see. Hand
them the brush and the loop closes.

The pieces, in order:

* **2a. A painted country layer.** `assets/world/country.bin`, exactly the shape
  `forest.bin` and `surface.bin` already are — a per-cell override where **0
  means "leave the map's own answer alone"**. `terrain_core::painted` already
  does this; it wants a third `Kind`.
* **2b. `region::at` reads the layer first.** Painted wins outright; unpainted
  falls through to the bands and the desert oval, which stay as the default so a
  fresh world still has a world in it.
* **2c. The brush.** One key per country, paint and erase, falloff like every
  other brush, undo through the shared `History`, autosave with the rest.
* **2d. The overview paints from it live**, so the maker sees the country they
  are drawing rather than relaunching to find out.

Once 2b lands, `DESERT_AT`/`DESERT_REACH`/`BANDS` stop being tuning knobs and
become the fallback nobody has to touch.

## 3. Editor controls

Four independent things; the first two are small.

* **3a. No flying under the ground.** Clamp the free-fly camera to
  `drawn_height + clearance`.
* **3b. A heading on the overview.** The map has no north marker and no view
  cone, so it is impossible to tell which way you are facing.
* **3c/3d. SCRATCHED 2026-08-17** by the maker. Left written down because the
  reasoning is worth keeping: props are WELDED per chunk — one mesh, one draw
  call — so there is no per-boulder entity to click. This needs a placed-object
  list (4a) plus a per-cell suppression mask, the same bargain the woods make
  between `natural_density` and `forest.bin`.
  (3d, "move anything", went with it — same dependency on 4a.) The maker wants
  both back **after the workbench is in**, which is the right order: 4a is what
  makes a placed thing a thing the world remembers, and until that exists there
  is nothing to select or drag.

## 4. A prop and building workbench

**Big.** This is a second application, not a feature. Order matters because 4a
unblocks 3c and 3d as well.

* **4a. A placed-object format and loader.** The foundation: a file of
  `{ kind, at, turn, scale }` the world reads at startup and the editor writes.
  `assets/buildings/*.json` format 2 already covers baked SHAPES; this is the
  layer that says where they stand.
* **4b. A kit of parts.** Walls, posts, rails, roof panels, floors — placed piece
  by piece against a snap grid. Fences and houses fall out of the same kit. This
  extends the existing `build::plan` format rather than inventing another.
* **4c. Painting.** Vertex colour on the workbench, since every welded thing in
  this world already carries its colour in its vertices.
* **4d. Reference images.** Import a picture, stand it on a plane, build against
  it. Cheap and it is most of what "add an image" is asking for.
* **4e. Generation. DONE 2026-08-18.** Wired to 3daistudio. **Never tested against
  the live service** — that needs a key and spends credits, so the contract, the
  polling and the failure paths are written from Opificium's kiln and unit-tested,
  and the first real firing is still ahead. See `bench/kiln.rs`. In the workbench, here — Opificium was only
  ever mentioned as a reference for the kind of tool, and nothing goes in it. `G`
  asks for a house, fence, tower or shelter and what arrives is ordinary pieces to
  edit. See `build/pattern.rs`.

## Recommended order

2 → 3a, 3b → 4a → 4b → 4c, 4d. Item 2 pays for itself immediately; 3a and 3b are
an hour; 4a is the keystone.

---

# Copaimo: The Wardens Guild — state at 2026-08-18

## The rename

The game was **Ranger**, which was also what the player was called: one word doing
two jobs. It is **Copaimo: The Wardens Guild**, and the player is a **Warden**.
Two words, and they went to different places — a single blanket rename would have
made the player a Copaimo and the game a Warden.

* repo `Baz-Studios-LLC/copaimo`, crate and binary `copaimo`, folder
  `Desktop/copaimo`
* title art `assets/Title/Copaimo.png`; the icon is the **crest** cut from under
  the wordmark (a wordmark is 3:1 and an icon is square; the big `C` cannot be
  cropped because the C and O interlock)
* three icon mechanisms, all different: the running window (winit, pinned to
  **0.30.13** — a second winit crate compiles and then refuses to talk), a
  resource compiled into the `.exe` by `build.rs`, and an `.icns` the mac runner
  builds with `iconutil`
* launcher entry renamed; `"ranger"` is in `RETIRED_SLUGS` and **must stay** —
  a slug is an install folder, and dropping it orphans gigabytes

## Released

**v0.1.9** — both platforms, three assets. v0.1.8 was deleted: it half-published
because of the fault below.

## Two failures worth not repeating

**The tools check killed the release in silence.** `BIN=$(ls a b | head -1)` —
GitHub runs bash with `-eo pipefail`, `ls` on two paths where one exists returns
non-zero, and `-e` ended the step *before its first echo*. A check written to stop
a silent failure became one.

**A guard that lied about succeeding.** Guarding the icon build with
`if build_icon; then` and `set -e` inside it: bash **suppresses `set -e` for any
command whose status is being tested**, so it ran to the end with every `sips`
failing and returned the status of the final `rm`. It reported "icon built" over a
bundle with no icon. Every step is checked by hand now, and the last check is that
the file arrived — `iconutil` can exit 0 having written nothing.

Both were only ever caught by RUNNING the thing. So was the terrain panel's
two-mutable-borrows panic, which compiled clean and died the instant the tool
opened.

## What is in the game now

* **Saves.** `save.rs` — the player's, not the world's. Written whole (temp file
  *beside* the save, then renamed; a rename across filesystems degrades to a copy).
  Continuing takes the position from the file and the HEIGHT from the ground,
  because the world can be resculpted between sittings. Unreadable saves always
  answer "start a new game".
* **Title screen** — full-screen, opaque field, logo art, Continue first and only
  when there is one.
* **Typeface** — `typeface.rs`, always compiled. It was in `tools`, which is
  compiled out of releases, so a *player's* build fell back to a monospace. The
  asset root is worked out once in `main::asset_root()` and given to Bevy —
  absolute, because Bevy joins a relative one onto the executable's folder.

## The workbench

**It shares nothing with the world.** Not the terrain, not the streaming, not even
the material — the game's `Shaded` carries cloud-shadow uniforms and a room with
two lamps has no clouds. The single connection runs the other way: work is **baked
to a file** the game reads as an asset.

* the lattice is a **sixteenth of a metre**: fine, and a power of two, so snapping
  is idempotent and abutting pieces meet on the same coordinate
* the piece in hand is solid; drawn see-through it read as a piece that failed to
  load
* `pattern.rs` generates a house/fence/tower/shelter as **ordinary pieces** you can
  take apart — a generator that hands back a black box is worse than no generator
* `kiln.rs` sends a picture away and gets a GLB back. **It has never fired against
  the live service** — that needs a key and spends credits. If it fails, the likely
  spots are the `task_id` field name and the status strings.

## The launcher

A **separate, shared** repository — `baz-studios-launcher`. Somebody else pushed
to it twice during this session, so **rebase, never force**, and check
`git log HEAD..origin/main` before pushing.

* **v0.1.27** released and installed (verified by reading `ProductVersion` off the
  installed binary, not by assuming the installer took)
* the game's name, tagline and accent are **compiled in**, not fetched — so the
  shelf only learns a new name when the launcher itself is rebuilt. That is why it
  read "Ranger" beside Copaimo release notes: the notes come from GitHub live and
  everything else does not
* hero art is a convention it already had: `src/assets/<slug>.png`, plus
  `<slug>-icon.png` for the tile. Ranger simply never had any, which is why the
  card drew the name as text. Copaimo's wordmark is 2.8:1, which clears the
  launcher's own 2.0 threshold and takes the taller `wordmark` cap
* its `mockInvoke` catalogue is **stale** — it predates several games. Serving
  `src/` in a browser renders that mock, not the real shelf, so patch a scratch
  copy if you want to look at a card
* two version files had drifted (`Cargo.toml` vs `tauri.conf.json`); both read
  0.1.27 now

## Every mention of the old name

Swept case-insensitively across all three repositories. What remains is
deliberate:

* `RELEASE_NOTES.md` in this repo — the v0.1.9 notes explain the rename and have
  to say the old name
* `RETIRED_SLUGS` in the launcher — that string IS the old install folder
* a joystick called "Rockfire Space Ranger" in a third-party gamepad database
  under the launcher's `target/`

One real one was found and fixed: `terrain-core/src/biome.rs` still described the
world as Ranger, and that crate is linked by every tool.

## Loading, measured

The whole world is **never** loaded. The map is ~2,130 chunks; a 9-radius **disc**
(not a square) holds ~254, so about **12%**, nearest-first, with a chunk of
hysteresis on unload. Off-screen chunks are loaded but not drawn — nothing sets
`NoFrustumCulling`. Cover reaches 2 chunks and props 3.

Loading only what faces the camera would make turning round a stall. The lever if
the frame needs it is `VIEW_CHUNKS`, and the cost goes as its square.

## Still open

* the kiln's first real firing
* the terrain and bench panels have never been SEEN by me — they run and measure
  right, but carets, swatches, folding and scrolling want eyes
* 4 pre-existing `B0004` visibility warnings from world spawning
* rivers remain switched off; the reason is width, written up in `config.rs`
