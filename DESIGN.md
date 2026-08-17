# Ranger — Design

A monster-companion adventure game. You play a ranger who raises monsters on a
ranch, travels between cities, and upgrades your Ranger License by passing the
exam set by each city's Ranger Guild.

Touchstones: **Pokémon** (turn-based battles, a journey structured around gym-like
exams) and **Monster Rancher** (monsters as creatures you *raise*, not just
collect).

> Keep this document current. When a mechanic, tuning value or system changes,
> update the relevant section and add a line to the change log at the bottom.

---

## 1. Pillars

1. **Monsters are companions, not enemies.** They're allies you raise and care
   for. Wild monsters exist to be met and befriended, not farmed.
2. **The journey is the structure.** Progression is geographic — each
   certification is in a different city, so getting stronger means travelling.
3. **The ranch is home.** A place you leave, improve, and come back to. Ranch
   growth and license rank advance together.
4. **A world worth crossing.** The map is big enough that distance means
   something, and varied enough that crossing it is the reward, not the tax.

## 2. Core loop

```
join the World Ranger Association
        ↓
base permit → build a ranch outside the village
        ↓
   ┌──── raise monsters at the ranch ────┐
   │                                     │
travel to a city                   take guild missions
   │                                     │
pass its certification exam ──────────────┘
        ↓
higher license → bigger ranch + higher monster cap → further cities
```

**Certifications are the central gate.** Passing one expands what you can build
on the ranch *and* raises how many monsters you can keep. Monsters are required
both to pass exams and to take guild missions, so the two halves feed each other.

Starting monster cap: **3**, rising with each certification.

## 3. Systems status

| System | State |
| --- | --- |
| Open world terrain | ✅ Built — see §4 |
| Chunk streaming | ✅ Built |
| Main menu | ✅ Built |
| Terrain tool | ✅ Built — see §5 |
| Player controller | ✅ Placeholder body, real movement |
| Camera (follow + free-fly) | ✅ Built |
| Buildings from the bench | ✅ Reader built — see §6. No street layout |
| 3D models | 🔷 Pending — `assets/models/`, swaps in over primitives |
| Cities & towns | 🔷 Ground only. One building per site, no layout |
| The ranch | 🔷 Not started |
| Monsters | 🔷 Not started |
| Turn-based battles | 🔷 Not started |
| Certification exams | 🔷 Not started |
| Guild missions | 🔷 Not started |

---

## 4. The world

### Shape comes from a map image

The landmass is **not** arbitrary noise. `assets/world/heightmap.png` is the
authority: its brightness is elevation, so the continents, seas, island chains
and mountain ranges are all authored, not rolled. Procedural noise only fills in
detail the image is too coarse to describe.

This is the important architectural choice in the world system. It means the map
can be redesigned in a map generator or an image editor and dropped in, with no
code changes — and it means the world is reproducible and *ours*.

See `assets/world/README.md` for export guidance.

### Height is layered

Built in `src/world/terrain.rs`, in order:

1. **The shelving coast** — from the signed distance to the shore. The land
   climbs `BEACH_WIDTH` to reach `COAST_HEIGHT`; the sea floor falls
   `SHELF_WIDTH` to reach `OCEAN_DEPTH`. They meet at zero, the waterline.

   > **Nothing may change height faster than the vertex grid can draw it.** The
   > first version put the whole 76 m drop inside the few metres the mask blur
   > spanned, so neighbouring vertices landed on opposite sides of it and every
   > coastline rendered as a picket fence of vertical slats. If a cliff ever
   > combs again, this is the first thing to check.

2. **Inland rise** — the land climbs away from the sea, by distance from the
   nearest coast. Coastal plains that become uplands.
3. **Mountain ranges** — see below.
4. **Fine detail** — small undulations, damped underwater. This is what stops a
   low-resolution source map from feeling like smooth putty underfoot.

A **domain warp** is applied before the map lookup, so coastlines wander instead
of tracing the source image's pixel grid.

### Mountains are placed by geography, not by noise alone

`Terrain::range_height` needs three factors to agree before any mountain exists:

* **presence** — a very low-frequency field, hard-thresholded, so ranges occupy
  a few regions of the map rather than being its texture.
* **inland** — distance from the nearest coast, computed once at load by a
  breadth-first sweep out from the water. Mountains are not allowed near the
  shore; beaches and plains belong there, and a range rising straight out of the
  sea reads as a mistake.
* **ridge** — `1 - |noise|`. The crease where the noise crosses zero becomes a
  crest, and at this frequency that crest runs for kilometers.

> **Do not use ridged multifractal noise here, and do not stack octaves or
> square the crest.** Tried and rejected twice. Ridged multifractal creases at
> every zero crossing; masking it by `land²` and squaring narrowed those creases
> into a map-wide forest of isolated spikes. Two octaves and a modest power
> (~1.7) on the crest gives ranges you walk over rather than teeth you walk
> between.

> **`INLAND_FULL` must be checked against the map.** Every mountain threshold is
> a fraction of it. Set it above the map's actual deepest interior and nothing
> ever counts as inland, so the mountains silently never appear — no error, just
> a world of hills. `cargo test -- --nocapture` prints the furthest any point on
> the current map gets from a coast; keep `INLAND_FULL` below it. On the current
> map that number is 820 m, and 1100 m produced exactly this failure.

### Cleaning the map's line work

Real maps are covered in things that aren't terrain: region borders, rivers,
roads, place names, and — in a screenshot — the tool's own buttons and scale
bar. Read at face value they carve trenches and islands across the continents,
which then alias against the 2 m vertex grid into rows of spikes.

Land/sea therefore comes from a **cleaned mask** built in four stages:

1. **Classify by hue, not brightness.** This is the important one. Brightness
   cannot tell open water from a black place name, a road, or a dashed border —
   all of them are dark, so a brightness threshold cuts *every label on the map*
   into the terrain as a lake. Water is the one thing on a political map that is
   distinctly **blue**, so the test is blue meaningfully greater than red
   (`MAP_SEA_BLUE_MARGIN`). Labels and borders are neutral or warm and stay land.
   Measured on the current map: ocean sits at 48–80, every land fill at 32 or
   below. A genuinely grayscale heightmap has no hue to test, so it is detected
   automatically and thresholded on brightness instead.
2. **Majority filter.** Each pixel becomes whatever most of its neighbourhood is.
   A river or border a few pixels wide is outvoted by the land around it and
   disappears; a coastline has land on one side and sea on the other all the way
   along, so it holds its position exactly.
3. **Drop small islands.** Any land blob under `MIN_ISLAND_PIXELS` is deleted as
   furniture rather than geography — this is what removes a screenshot's buttons
   and scale bar. Real islands are far larger. Cropping the source is still the
   cleaner fix; this makes an uncropped one usable.
4. **Sweep for distance to the coast**, twice: once from the sea inward, once
   from the land outward. Subtracted, they give a **signed distance** — positive
   inland, negative out to sea — that crosses zero exactly at the shoreline.
   That single number is what the whole landscape is built on, and it is what
   lets the coast shelve (see §4, *Height is layered*).

### Flat mode

`FLAT_WORLD` in `config.rs` puts all land at one height and all sea at one
depth, with no generated relief at all. It's the shape-checking mode — the only
thing visible is the outline of the continents. **Currently off**; turn it on
whenever the coastlines need checking after a map swap or a mask change.

It's also the natural companion to the sculpting tool: the map gives you the
continents, and every hill and mountain on them is one you put there. Hand edits
still apply on top, so a flat world is a canvas, not a locked one.

### What the map image does and doesn't carry

A **grayscale heightmap export** carries real elevation, and relief lands where
the map says it does. A **colored political map** does not — its brightness is
region fill colors and means nothing as terrain. It defines the *coastline*
perfectly and nothing else.

The loader detects which it has and says so in the log. On a colored map the
brightness channel is ignored entirely and all relief comes from the inland rise
and the mountain layer; `MAP_SEA_THRESHOLD` is unused, since hue does the
classifying. On a grayscale map, brightness drives both the waterline
(`MAP_SEA_THRESHOLD`, around 0.20) and `BASE_ELEVATION` on top of everything
else.

Brightness is normalized on load between the 0.5th and 99.5th percentile, not
the true min and max — map exports carry outliers that aren't terrain (label
text, scale bars, UI chrome in a screenshot) and a single black pixel would
otherwise anchor the range and flatten everything real.

### Finite by construction

The world ends in **open ocean**, never a wall. Outside the map image everything
reads as deep sea, and the water plane extends four times the world's longest
axis so the horizon past any coast is water. `WorldBounds` stops the player
walking off the edge, but they should reach open sea long before they feel it.

A **border fade** (`COAST_FADE_START`) pulls land under water in the outermost
few percent of the map, whatever the source image shows there. This is not
cosmetic: it's what keeps the invariant true when the map is a screenshot whose
margins contain a toolbar. It's kept tight to the border so it trims furniture
rather than real coastline. The generation test asserts all four corners are
open sea, and has already caught this exact failure once.

### Scale

One knob: `WORLD_WIDTH` in `src/config.rs`, currently **8192 m**. North–south
extent is derived from the map image's aspect ratio. At the ranger's 7 m/s jog,
that's roughly **20 minutes** east to west on a 2:1 map.

Emptiness is not a concern at this stage — smaller towns between larger cities,
trainers, terrain and puzzles fill it in later.

### Biomes

Classified per vertex from **height, slope and moisture** and baked into the mesh
as vertex colors (`src/world/biome.rs`). No textures yet; the same classification
picks textures later.

| Band | Reads as |
| --- | --- |
| Deep water | Dark silt |
| Shallows / shoreline | Sand, visible through the water |
| Low, dry | Dry grass and plains |
| Low, wet | Grassland → dense forest |
| High | Bare alpine ground |
| Above the snow line (210 m) | Snow |
| **Steep, any band** | **Rock** — cliffs read as stone, never vertical lawn |

### Streaming

Terrain exists as **128 m chunks** on a 2 m vertex grid, meshed on background
threads and kept within a **9-chunk disc** (~1150 m) of the camera.

There is **no distance fog**. It was there to hide the streaming boundary, but
haze across the whole view is the wrong trade when the point is reading the
shape of the land. So the view radius *is* the horizon — terrain stops at the
edge of it. Raising `VIEW_CHUNKS` pushes that edge back at a cost that grows
with the square of the radius; distance-based mesh LOD is the real answer to
seeing further, and is not built yet.

Chunks stitch seamlessly because both height and normals are computed from world
coordinates alone — neighbours sampling a shared edge get identical answers, so
there is no crack and no lighting seam.

---

## 5. The terrain tool

**Shape the World**, from the main menu. Nine brushes on keys 1–9: raise, lower,
smooth, flatten, path, roughen, erode, ramp, plant. Left button applies, right
inverts, the wheel sizes the brush, `[` and `]` set its strength, `Ctrl+Z` and
`Ctrl+Y` take strokes back and put them again, `Ctrl+S` writes ground and woods
together. Chunks under the brush re-mesh live and replant as they do.

The **same tool is also a bench in [Opificium](https://github.com/Baz-Studios-LLC/Opificium)**,
the studio's maker's bench, for shaping a world without opening the game.

### The brush belongs to neither of them

It lives in **[`terrain-core`](https://github.com/Baz-Studios-LLC/terrain-core)**,
a crate with no engine in it, which both programs link. So does the world
generation, the forest scatter and the tree growing. That is the whole
arrangement: two programs, one answer about what the ground is.

It was not always. The generation was written twice and the copies had to agree
exactly — a digit out of place in a hash gave the bench one world and the game
another, with no error and nothing failing. It was held together by tests pinning
literal numbers copied from one program into the other. Written once, they cannot
disagree at all.

This is what the studios do: the editor is built **on top of the game's own
runtime**, not beside it, and the world code exists once. `src/editor/` here is
the *mode* — aiming, gestures, the panel, telling chunks to mesh again. None of
it shapes ground; it drives something that does.

### What passes between the two programs

Files, and only files. The layout of each is written down in Opificium's
`FORMATS.md`.

| File | Direction | What it is |
| --- | --- | --- |
| `assets/world/heightmap.png` | game → bench | The map the world is drawn from |
| `assets/world/world.json` | game → bench | The recipe: every number in `config.rs` that shapes the ground |
| `assets/world/edits.bin` | both ways | Sculpted ground, as signed height offsets |
| `assets/world/forest.bin` | both ways | Painted woods, as signed bias — zero leaves the ground's own answer alone |

`world.json` is exported by a test in `config.rs`:

```bash
cargo test export_world_for_opificium -- --ignored --nocapture
```

> **Run it whenever a world-shaping constant changes.** The bench and the game
> must agree about the *generated* ground exactly. A maker sculpts offsets — how
> far the ground moved — and the game adds those to ground it generates itself.
> If the two disagree about what was underneath by so much as a metre, every
> hill placed at the bench sits at the wrong height here, and nothing on screen
> says why.

### How a change at the bench reaches the game

```
sculpt at Opificium  →  Ctrl+S  →  assets/world/edits.bin  →  next launch
```

Sculpting *here* needs no such trip — the mode writes the same file the game read
at startup, and the ground under the brush is already the ground you are standing
on. The bench route is for shaping a world without opening the game at all.

The game reads `edits.bin` **once, at startup**. So:

* **Running from source** (`cargo run`) — sculpt, save, relaunch, it's there.
* **An installed build from the launcher** has its *own* copy of `assets/`, so
  sculpting this repository changes nothing for it until a **new release ships**.
  The release workflow packages `assets/` wholesale, so a tag carries whatever
  `edits.bin` is committed at that moment.

`edits.bin` is **not gitignored**, on purpose — it's authored content, not build
output. Commit it like any other work, or the shaping is on one machine only and
never reaches a build.

There is no hot-reload: the bench and the game are separate processes and the
game does not watch the file. Relaunching is the refresh.

`edits.bin` whose grid or world size doesn't match is **refused, not stretched** —
offsets landing in the wrong places would be worse than none. The F3 overlay
shows how many sculpted cells actually loaded, so a refusal is visible without
reading the log.

### Why it left, and why it came back

It was built here first, as a mode behind this game's main menu. It moved out
because a sculpting tool only one game can run is one every other game has to
rebuild — Opificium is where the studio's authoring lives.

That was right about the tool and wrong about the *code*. Moving the mode moved
the world generation with it, and the world generation is what the game is made
of, so it existed twice: two programs on two Bevy versions, kept in step by hand.

`terrain-core` is the answer to that, and once it existed the mode could come
home. Shaping ground you are standing in, at the height you will walk it, beats
shaping it in another program and relaunching to see. Both are still true at
once — the bench is there when a world wants shaping on its own.

### Undo reaches into both layers

The ground and the woods keep separate histories and neither can know about the
other, so the mode remembers the **order** strokes landed in and sends `Ctrl+Z`
to whichever layer the last one touched. Undo means "take back the last thing I
did" or it means nothing.

Clearing what you planted is not a substitute, which is why this was worth
building rather than documenting away: clearing *writes* negative bias, forcing
bare ground and holding it bare. Zero — no decision, the ground answering for
itself — is only reachable by undoing.

### Not done yet

* **Nothing about a tree's *look*** can be changed here. The knobs are exported in
  `world.json`; no shelf reads them.
* **`world/settle.rs` is still written twice**, once here and once at the bench.
  Towns, quotas and roads have to agree, and nothing enforces it. It is the
  obvious next thing to move into `terrain-core`.

## 6. Buildings

Houses, signs and bridges are the same thing: **boxes**. They are drawn at
Opificium's builder on a sixteenth-metre lattice, painted from this game's own
palette ramps, and baked to `assets/buildings/<name>.json` with
`cargo test bake_the_works -- --ignored`. `src/build/` reads that; nothing here
draws a building.

Trees are *not* buildings — they are grown in `terrain-core` from a hash of
position, twenty varieties, no files. Drawing them at the bench would make them
heavier, fewer and all alike.

### One mesh per building

A house is fifty-odd boxes and four colours. Given a mesh each that would be
fifty draws for one building and hundreds for a street, so every box is welded
into one mesh with its colour carried per vertex — the same bargain the terrain
makes, and the shared white material lets the bench's colours through exactly as
drawn. Glass gets a second mesh, because what lets light through has to be drawn
after what is behind it and one mesh can only be one or the other.

### The four shapes, and the thing to watch

`box`, `wedge` (a gable's prism), `ridge` (the same turned to run lengthwise) and
`cut:<low>x<high>` (a face cut back at each end), plus `hip:<x>x<z>` for a hip
roof with a deck. `cut`'s runs are **signed**: positive takes the top back,
negative the bottom, and that is the whole trick — top at one end and bottom at
the other leaves the ends parallel, which is what a diagonal brace is.

**Opificium draws these from its own code and shares none of it.** A shape is
only the same shape in both because it is written out twice, which is exactly the
arrangement `terrain-core` exists to kill. It has not been done for buildings.
The shapes are pure geometry over vectors and would move cleanly, and the day one
of these disagrees is the day to move them.

Every face is wound by comparing where it sits against the middle of its box,
rather than by hand. A wrong winding is invisible until something is lit from the
wrong side, and there are twenty-six of them across the four shapes.

### An unknown form refuses the building

Not a box — a beam the two programs disagree about. Refusing the file and naming
the form says so; substituting a cuboid would put a solid block where a cut brace
belongs and read as a fault in the drawing. One bad file costs its own building
and no others.

### Where they stand

One per town site, at its middle, on the height the site was levelled to. **That
is not a village** — laying out a street is its own job and is not started — but
the sites already exist as levelled ground with a centre and a size, so a drawing
reaches the world the moment it is baked. A building reaching past its site's
levelled ground is counted and warned about once, because one end on a hillside
reads as a broken building rather than as a site too small for it.

`assets/buildings/house-cottage.json` is **hand-written, not baked** — a stand-in
that uses all four shapes so the geometry has something to prove itself against
before any real drawing exists. Replace it with real bakes.

Read but not yet used: `stage` (what a box IS, enough to raise a building in
order), `levels` and `phases` (upgrades, and the steps of raising one),
`half_w`/`half_d`/`high` (the plot a village clears), and the `marks` that say a
place has a door. The marks are spawned as child entities regardless, so whatever
comes to use them asks the world rather than re-reading the files.

## 7. Invariants

Things that must stay true. Breaking one is a bug, not a tuning choice.

- **One source of truth for ground height.** Everything — meshing, the player's
  feet, camera clearance, spawn search, the brush's raycast — calls
  `Terrain::height`. The ground drawn and the ground walked on can never
  disagree. `Terrain::base_height` exists *only* for the brush's Smooth and
  Flatten, which run holding the edit lock and so must not read back through it.
- **Hand edits are offsets on top of generation, never a replacement for it.**
  Regenerating the world must never move sculpted terrain.
- **The world ends in water, not a wall.**
- **The map image is the authority on shape.** Noise adds detail; it never
  decides where land is.
- **Chunk seams are invisible.** Normals stay analytic from the heightfield, not
  averaged from triangles.
- **The ranger is ~1.8 m tall.** Terrain scale, camera distance and movement
  speed are all tuned against that. Replacement art keeps the height.
- **Monsters are allies.** Nothing in the world is built as a threat to fight
  off. Wild monsters are met, not repelled.
- **No copy-paste.** Shared logic lives in one place — `util.rs` for math,
  `WorldBounds` for extents, `Terrain` for the heightfield.

## 8. Controls

| Input | Action |
| --- | --- |
| `WASD` / arrows | Move (relative to the camera) |
| `Shift` | Sprint |
| Mouse | Look |
| Wheel | Zoom |
| `F` | Toggle free-fly — `Space` / `Ctrl` for up and down, `Shift` to boost |
| `Esc` | Back to the main menu |
| `F3` | Toggle the debug overlay |

In the terrain tool:

| Input | Action |
| --- | --- |
| Left mouse | Apply the current tool |
| Right mouse | Apply it inverted (raise ↔ lower) |
| Wheel | Brush radius (proportional, 4–500 m) |
| `[` / `]` | Brush strength |
| `1`–`6` | Pick a tool |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+S` | Save edits to disk |
| `Esc` | Back to the main menu |

The cursor is captured in both modes and released in the menu, driven by the
state transition rather than a key — so it can never end up grabbed while a
menu is asking to be clicked.

Free-fly is a development tool for reading the map from above. It streams real
terrain rather than a special case, so what you inspect is what ships.

## 9. Project layout

```
src/
  main.rs        wiring: every concern is a Bevy plugin
  config.rs      every world tuning knob, WORLD_WIDTH above all
  states.rs      menu / playing / editing, and the cursor policy
  menu.rs        the main menu
  util.rs        shared math (smoothstep, facing)
  world/
    mod.rs       WorldPlugin, WorldBounds
    heightmap.rs loads and samples the source map image
    terrain.rs   the heightfield — the single source of truth
    biome.rs     height + slope + moisture → surface color
    chunk.rs     chunk mesh construction
    stream.rs    background generation, load and unload
    edit.rs      where edits.bin lives — the brush itself is in terrain-core
    forest.rs    where forest.bin lives — the scatter is in terrain-core too
    water.rs     the sea
  player.rs      the ranger and their controller
  camera.rs      orbit follow rig + free-fly
  editor/
    mod.rs       the terrain mode: raycast, gestures, live re-mesh
    theme.rs     its visual language: palette, font, shared fragments
    ui.rs        sidebar, live readouts, confirmation toasts
    minimap.rs   the world overview and camera marker
  sky.rs         sun, ambient, fog constants
  hud.rs         F3 debug overlay
assets/
  world/         heightmap.png (the map), edits.bin (ground), forest.bin (woods)
  models/        3D models, as they're made
  fonts/         Cinzel for the terrain tool, with its licence beside it
```

---

## Change log

**2026-08-15** — **The world knows what kind of place it is.** Eight biomes —
water, shore, grassland, forest, desert, rock, snow, settled — classified in
`terrain-core` from five signals the game answers for: height, slope, moisture,
distance to the coast, and how much a settlement has levelled the ground. Both
the game and Opificium's bench ask the same question and get the same answer,
which is what makes "this monster lives in forests" mean one thing.

The **order** of the questions is the rule, most-physical first. Under the
waterline nothing else is worth asking, so a drowned forest is water. A levelled
town on a hillside is a town — asked before the slope, or wild things would live
in the middle of a settlement. A beach is measured from the coast and not from its
height, because a clifftop ten metres up is not a beach and a sandbar is.

`Biome::of` answers with ONE kind, because habitat is a yes or no. `confidence`
answers *how strongly*, for everything downstream that should fade rather than
switch. Both read the same thresholds, so a boundary you can see is the boundary
that decides what lives there. Those thresholds are a `Climate` — a world's own
numbers, exported in `world.json` — not constants, because the same generation
with the desert threshold moved is a wetter continent.

Measuring it caught two faults immediately. Above the treeline read as **grass**,
so the world contained *no rock at all* and anything meant to live in the
mountains had nowhere to be. And the graded skirt around a town counted as
settled, which made a fifth of the land somebody's.

What the world is made of now, sampled on a 160 × 160 grid:

| | share of world | share of land |
| --- | --- | --- |
| Water | 63.0% | — |
| Grassland | 9.9% | 27% |
| Forest | 8.5% | 23% |
| Settled | 7.8% | 21% |
| Desert | 4.8% | 13% |
| Shore | 4.4% | 12% |
| Rock | 1.0% | 2.7% |
| Snow | 0.8% | 2.2% |

Shown on the F3 overlay and in the terrain tool's panel, both with the confidence,
because tuning a climate means standing in one and reading the number.

**Not done:** rivers. `Water` covers them the moment they exist, but nothing
generates them — that needs downhill flow, and it is its own job.

**2026-08-14** — **Trees that look like trees.** Three faults, and the middle one
was a real bug.

*Branches all pointed up.* Limb directions were built in **world** space — the
lean measured from Y — so every sub-branch re-aimed at vertical however its
parent was heading. A limb growing sideways had children that turned straight
back up, which is why a canopy came out as a fan of parallel canes. Directions
are built in the parent's own frame now, and each limb is drawn in two lengths
with a sweep back toward the light, because a straight limb reads as a spoke.

*Trunks were canes.* Girth was absolute and tapered to a fifth of itself over the
whole height in one tube, so a twelve-metre tree stood on something the width of
a broom handle. Girth comes from height, the taper leaves a third at the crown,
and the trunk is drawn in segments so it holds its girth low and leans as it
climbs. It stops at 78% and lets the crown take over instead of standing out of
the leaves as a bare pole.

*Every tree looked the same.* Spread is drawn first and the limb count and length
follow from it, so the pool keeps spires **and** spreading trees rather than
averaging into twenty of one tree — 2.2 m to 12.4 m across, on trunks from 0.23 m
to 0.95 m. Leaf clusters are half the size with three per limb end, since a
canopy is read by its edge and one boulder at a tip has almost none. And each
tree draws its own place in a leaf-colour range: **one material for the whole
forest** was doing more to flatten a wood than any of the shaping.

**2026-08-14** — **Buildings can come in from the bench** (§6). `src/build/`
reads a baked `assets/buildings/<name>.json` — the boxes Opificium's builder
resolves a drawing down to, colours already looked up — welds them into one mesh
with the colour per vertex, and stands one at each town site. Houses, signs and
bridges are all the same thing to it; only `kind` tells them apart.

All five shapes are drawn here: `box`, `wedge`, `ridge`, `cut` and `hip`.
Opificium draws them from its own code and shares none of it, so **this is
written twice on purpose and knowingly** — the note in §6 says when to move them
into a crate. Windings are decided from the geometry rather than by hand, since
there are twenty-six of them and a wrong one is invisible until something is lit
from the wrong side.

`house-cottage.json` ships hand-written, using all four forms, so the geometry
has something to prove itself against before a real drawing exists. Writing it
caught two things: a bounding sphere per box buried a cottage two hundred
millimetres underground (its nine-metre ridge cap's sphere reaches below the
ground it sits four metres above), and the cap itself was longer than the roof it
sat on.

**2026-08-14** — **The terrain tool is back in the game**, as *Shape the World*
on the main menu (§5) — and the brush it drives is now
[`terrain-core`](https://github.com/Baz-Studios-LLC/terrain-core)'s, the same one
Opificium's bench drives. Nine tools rather than the six it left with: **erode**
(thermal, material moves and is never invented or lost), **ramp** (click two
points for a graded run) and **plant** (paint woods, right button clears) came
back with it, wearing the bench's colours and its Cinzel-on-near-black look.

The blockage was the crate move, and the crate is the point of it: `Sculpt`,
`Brushing` and `Stamp` moved out of Opificium into `terrain-core` with the engine
taken out — bytes in and out instead of paths, no logging, and a pair of corners
where a `Rect` used to be. `src/world/edit.rs` here is now the thin adapter that
knows where this game keeps its file, the same shape `world/forest.rs` already
had. **This is the AAA arrangement**: the editor is built on top of the runtime
and the world code exists once.

Two things fell out of it. Chunks re-mesh under the brush, which they never did
before, so `collect_chunks` now **clears a chunk's trees before replanting** —
without it every stroke doubled the wood and left the old trees hanging at the
height the hill used to be. And `Ctrl+S` saves **ground and woods together**,
because they are one afternoon's work and a maker should not have to know there
are two files.

Then **planting learned to be taken back**. It shipped in the pass above with no
history at all, so `Ctrl+Z` after growing a wood either did nothing or reached
past it and took back a hillside. The ground and the woods want the *same* undo,
so it is written once as `terrain_core`'s `History` and both layers own one;
`Sculpt` lost about forty lines to it. The mode remembers which layer each stroke
went to, so the key means "the last thing I did" rather than "the last thing I
did to the ground".

**2026-08-14** — **Level ground, places, roads and moving water.** Three things:

*Somewhere level to put anything.* A very low-frequency **ruggedness** field now
scales both the mountains and the fine detail, so most of the world is plain
enough for forest, farmland and walking and the rough country is somewhere in
particular rather than everywhere at once. Before this every square metre of the
map was equally lumpy, which leaves nowhere for anything to happen.

*Places, and the roads between them.* `world/settle.rs` plans **6 cities and 14
towns** from the seed, rejecting anywhere that is on the beach, up a mountain,
already a hillside, or too close to a place already placed. Each gets level
ground with a skirt easing back into the land. They are then joined by a
**minimum spanning tree** — the smallest set of roads that still leaves every
place reachable from every other — and each road is *graded*, climbing steadily
from one end's height to the other's so it can be walked and carted. No
buildings: this is ground, prepared.

*The sea moves.* A slow **tide** (±0.55 m over 26 s) plus three layers of swell
running at different angles. The tide is the important half — on a coast that
shelves over hundreds of metres, half a metre of vertical travel walks the
waterline a long way up the beach and back, so the water visibly approaches and
recedes. `WADE_DEPTH` now blocks the *step* into deep water as well as clamping
standing height, so the ranger can paddle at a beach and is turned back by the
sea rather than by an invisible wall. Only the step *into* deeper water is
refused, so anyone who ends up out there can always walk home.

**2026-08-14** — **Coastlines now shelve.** The whole drop from land to sea floor
used to happen across the width of the mask's blur — a few metres — which no
vertex grid can draw: neighbouring vertices landed on opposite sides of it and
every cliff face came out as a fence of vertical slats. Replaced the blurred
coverage field with a **signed distance to the coast** (a second breadth-first
sweep, from the land out to sea, alongside the existing one), so the land climbs
a beach's width and the floor falls a shelf's width, each at its own rate,
meeting at the waterline. `MASK_BLUR_RADIUS` is gone; `BEACH_WIDTH` and
`SHELF_WIDTH` replace it. Ported from Opificium's terrain bench and re-exported,
so the two agree.

**2026-08-14** — **The terrain tool moved out of this repository** into
Opificium's new terrain bench (§5). The game keeps only the ability to *read*
`edits.bin`; `src/editor/` is gone and `src/world/edit.rs` is now a reader with
no brushes in it. Added `opificium/opificium.json` so the bench can open this
game, and an exporter for `assets/world/world.json` so the two programs cannot
drift about how the ground underneath is generated. The F3 overlay now reports
how many sculpted cells actually loaded, since a mismatched `edits.bin` is
refused rather than applied.

**2026-08-13** — Switched the land/sea test from brightness to **blue channel
dominance**, which fixed the map's place names being cut into the terrain as
lakes — brightness cannot separate open water from a black label, and the
majority filter's reach was far short of a label stroke's thickness. Added
small-island removal so a screenshot's toolbar and scale bar stop becoming
islands, and a border fade so the world ends in ocean whatever the source shows
at its margins (the corner assertion caught this). Turned `FLAT_WORLD` off and
added real mountains, placed by distance from the coast so ranges sit inland
with plains between them and the sea. Found and documented that `INLAND_FULL`
must sit below the map's actual deepest interior — at 1100 m against a map whose
interior tops out at 820 m, the mountains silently never appeared.

**2026-08-13** — Removed distance fog; raised `VIEW_CHUNKS` to 9 and the shadow
cascade bound to 900 m to compensate for the now-visible streaming edge. Fixed
the world overview's title and scale label running together.

**2026-08-13** — Rebuilt the terrain tool's interface to production standard,
since it's intended for use across projects: sectioned sidebar, logarithmic
brush meters, live cursor readout, unsaved indicator, confirmation toasts, and a
background-rendered world overview with a camera marker. Moved styling into
`editor/theme.rs` and added optional `assets/fonts/ui.ttf` support. Replaced all
non-ASCII UI characters, which were rendering as empty boxes in Bevy's subset
font.

**2026-08-13** — Split the terrain tool out as its own mode behind a main menu,
with app states driving the cursor policy. Added Path and Roughen brushes, undo
and redo, and the tool's own palette UI. Fixed the map's line work (borders,
rivers, roads) being read as trenches, which was aliasing into spike rows —
land/sea now comes from a majority-filtered mask. Corrected the sea threshold to
0.74, the real gap between the map's ocean and land brightness. Added
`FLAT_WORLD` for checking continent shape without any generated relief.

**2026-08-13** — Added the in-game terrain sculpting tool (§5): raise, lower,
smooth and flatten brushes writing to a signed-offset edit layer, live chunk
re-meshing, and a save format that refuses files from a differently-sized world.

**2026-08-13** — Replaced the ridged-multifractal mountain layer with broad
rounded fBm ranges; the ridged version produced a map-wide forest of spikes.
Raised `MAP_SEA_THRESHOLD` to 0.50 for the supplied political map — at 0.20 the
oceans were reading as land. Switched map normalization to percentile clipping
so label text no longer punches pits in the terrain.

**2026-08-13** — Project started. Built the open world: heightmap-driven terrain
generation, background chunk streaming, height/slope/moisture biome coloring,
sea, sun and fog, a placeholder ranger with a ground-following controller, an
orbit camera with free-fly, and the F3 debug overlay. World scale set to 8192 m
wide, sourced from a supplied fantasy map.
