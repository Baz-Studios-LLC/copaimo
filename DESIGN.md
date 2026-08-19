# Copaimo — Design

A monster-companion adventure game. You play a warden who raises monsters on a
ranch, travels between cities, and upgrades your Copaimo License by passing the
exam set by each city's Wardens Guild.

Touchstones: **Pokémon** (turn-based battles, a journey structured around gym-like
exams) and **Monster Rancher** (monsters as creatures you *raise*, not just
collect).

> Keep this document current. When a mechanic, tuning value or system changes,
> update the relevant section and add a line to the change log at the bottom.

---

## The mountain pass

The road east runs desert → **mountain pass** → grassland → snow. The pass is a
wall of rock across the whole route with one bore through it, at `pass::AT` on the
desert's eastern edge: you cannot walk round it in any reasonable distance and you
cannot walk over it, so getting east means going through the hole.

**A heightfield has one surface and a tunnel needs two**, so the job is split
between the two things that can each do half:

* the **floor and walls** are the heightfield, carved. Where the passage runs the
  mountain is simply not applied — so the bore's floor is the ordinary ground the
  mountain was raised on, at the height it always had. That is why walking in is
  level and why the mouths need no blending.
* the **rock above** is a mesh (`pass::rock_over_the_bore`) filling the slot the
  carving left, between the tunnel's arched ceiling and the mountain's own skin.

Neither half knows anything the other does not — both are drawn from the same
`ridge` and `bore` over the same ground — so the mesh cannot drift off the terrain
it is plugging, and sculpting the hillside moves both.

**Nothing decides where the mouths are.** The plug's thickness is the mountain's
height above the tunnel's crown; where the mountain is lower than the crown that is
nought. So the roof thins as the ground falls, opens into a cutting, and the
cutting opens onto the plain. A railway looks like this for the same reason, and it
means there is no end cap anywhere to get wrong.

Under the rock nothing grows and the floor paints as stone (`pass::underground`) —
the bore's floor is ordinary walkable ground, which would otherwise have been a
fine place for a wood to grow straight up through the ceiling. The cuttings at each
mouth are open to the sky and keep their grass.

Still to come: branching paths off the bore, and something to make the inside dark
beyond what the roof's own shadow gives.

## Boring tunnels in the terrain tool

The pass above was written in code, and moving it meant reading a screenshot and
guessing which constant it implied. That went wrong three times in an evening — the
wall crossed the desert boundary instead of following it, then it was a mesa, then
it was too thin. Same fault the countries had before they were paintable, same
answer: **the person who can see where a tunnel belongs should be the one putting
it there.**

`B` (or the BORE row) marks one mouth, `B` again marks the other, and the tunnel is
cut between them through whatever is in the way. `Shift+B` fills the nearest one
back in. They live in `assets/world/bores.json` and save with Ctrl+S alongside
every other layer.

A bore only ever cuts **down**: run one over open ground and nothing happens,
because there was no hill to get through. Its floor runs level between its two
mouths, and that floor height is remembered when the bore is laid rather than asked
for later — the carve is part of the terrain's height, so a floor derived from the
carved ground would sink a little further every time the question was asked.

The mountain itself is not part of it. There is already a brush for raising ground;
a bore's job is to make a hole in whatever is there.

## Every action in the tool has a row to press

The palette was clickable but the actions were not — placing a building, picking one
up, turning it, taking it away and boring a tunnel were all keyboard-only, and a key
nobody has been told about is a tool that does not exist as far as anyone can tell.
They are rows now, in DO SOMETHING, each printing its own key.

They are not in the brush palette, and should not be: a brush is dragged over ground
and works wherever it passes. None of these are — they happen at a moment, and two
of them need two moments to say what they mean. A maker who selected PLACE and then
dragged would rightly wonder why nothing happened.

The key and the row raise the same `Asked` event, so the two cannot come to mean
different things — the same arrangement `TOOL_KEYS` keeps for the palette.

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
join the World Copaimo Association
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
extent is derived from the map image's aspect ratio. At the warden's 7 m/s jog,
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

### The kit: ten parts, four of them made OF something

The workbench builds from `Part`: post, rail, wall, floor, beam, roof, cap,
foundation, stairs, bed. Every one is a whole number of modules (1.5 m) on a
sixteenth-metre snap, and **nothing is ever scaled** — a wall dragged from one
module to three is a longer wall, not a wall drawn at three times the size. Its
thickness is the thickness a wall is.

Three of them are more than a box, and they are the reason `Piece::blocks` returns
a list rather than one:

* a **floor** is boarding: eight planks to a module over a solid subfloor, each
  plank cut into boards at three-metre intervals whose ends do not line up with
  the next plank's, and each board laid in three tonal strips along its length.
  The subfloor is what makes a joint a LINE — without it, the joints went all the
  way through and a floor was a duckboard you could see daylight between.
* a **foundation** is two courses of stone in running bond, butted, with the upper
  course offset half a stone from the lower.
* **stairs** are steps, each solid to the ground. It is the one part whose height
  is its length's business: a longer flight is more steps, and more steps reach
  higher. Two modules of flight reach exactly one storey, which is why the storey
  is one number both read.

Every position that decides anything about the boarding or the coursing — which
plank row, where its ends fall, what tone it takes — is measured in WORLD space
along the piece's own axes. Two floors laid edge to edge therefore carry on one
another's pattern instead of each restarting at its own corner and drawing a seam
nobody built.

It costs boxes: about twenty-five to a module of floor. They weld into one mesh,
so the cost lands on the file rather than on the frame, and
`a_module_of_floor_stays_within_its_box_budget` is what says how many is too many.

### A part arrives in its own material

The colour in hand used to follow the maker from part to part, so a foundation came
out oak and so did a flight of stairs — because the last thing placed was. Each part
now names its own material (`Part::natural`): masonry for a plinth, thatch for a
roof and its cap, the darker wood for stairs and furniture, timber for everything
else.

It is a default, not a rule. The swatches overrule it and go on overruling it for as
long as that part stays in hand — what resets it is choosing a **different** part,
because that is the moment the maker has said what they are building next. Both ways
of choosing a part go through `Hand::take`, so the keys and the panel cannot drift.

### Cells, and the joins between them

The module grid has cells. A floor fills one; a **wall stands on the join between
two**, its centre-line the boundary, half its thickness either side. Rails, beams
and foundations run along joins with it; floors, roofs, ridge caps, posts, stairs
and beds sit in cells.

That was always the shape of the kit — `pattern::walls` has placed its walls at
`-MODULE * 0.5` since the day it was written — and the lattice cursor did not know
it. Snapping to whole modules, a wall placed by hand could only land on a cell
CENTRE: three-quarters of a metre in from the floor's edge, clipping through the
boards. **A maker could not build what the generator built**, which says the cursor
was wrong and not them. `Part::off_the_grid` is the one place that knows, and it
follows the piece round — a quarter turn moves the lean from one axis to the other,
or a turned wall lands mid-cell again.

### A piece rests on what it lands on

The cursor's height is the plane you are building on, and a floor laid on that plane
fills the first quarter-metre above it — so a wall placed at the same height had its
foot buried in the floor. `Bench::resting` raises a piece to the top of whatever it
would have clashed with, settling through a stack rather than stepping once.

**Touching is not clashing**, and everything depends on the difference: the kit is
built out of pieces that abut, so a floor laid beside a floor, a wall on a plinth
and a cap set on a ridge all share a face and stay exactly where they were put.

It is the CURSOR's rule and not the kit's — it lives in the bench's placement path,
not in `Bench::add`, because the generators work out exact positions and must not be
second-guessed, and neither must a piece being dragged by its arrows.

The cursor's own height is left alone by aiming, too. It was being rounded to the
module along with x and z, so raising it to clear a floor was undone the moment the
mouse moved.

### A piece is CHOSEN, and it stays chosen

Pointing used to be enough, and that made a piece under another one hard to get at:
reaching across a wall to click the floor it stands on handed the handles to the wall
on the way past, and the wall's arrows then swallowed the click.

Picking one up is a deliberate act — a click, on the work, with an empty hand — and
what is chosen stays chosen until something else is. A click with a part in hand is a
placement and never a selection, which is the rule the rest of the bench already
follows. A line is drawn round the chosen piece, on the handle layer so nothing can
hide it: two boxes a hair apart, the outer dim and the inner bright, because a gizmo
line cannot glow and that is what a glow looks like from a distance.

### Reaching a piece is measured to its BOX

Every "nearest piece" verb — select, paint, turn, remove — measured from the lattice
cursor to the piece's **middle** against a fixed radius. That works only while a
piece is about the size of the radius, and stretching broke it: a four-module floor
has its middle metres from either end, so its ends fell outside the reach and the
piece could not be selected, painted, turned or removed from there. It read as
"once an object is placed I cannot select it again".

`Piece::away_from` measures to the box instead, so a piece of any length is reachable
anywhere along it and the number still means metres.

Selection also tries a **ray** first, and only falls back to the cursor: point at the
top of a wall and the lattice cursor is on the floor several metres behind it, because
that is where the view ray carries on to. What the pointer is on beats what the cursor
is near.

### A floor grows two ways; everything else grows one

`spans` is length along the piece's own X. `across` is width, and only a floor has
it — see `Part::widens`. Every other part's second horizontal dimension is not an
extent: a wall's is its thickness, a beam's is its section, a roof's is the depth
its pitch is measured over. Growing those in whole modules gives a wall a metre
and a half thick, which is a distortion wearing a part's name.

A floor is a surface, and both its horizontal dimensions are real. Before this it
could only be lengthened, so laying a room meant placing a slab per module.

Both grow FORWARD from the foot, so the edge the maker placed stays put and the
far edge moves. The handles that do it stand off to one side of their own axis, in
a pinwheel — four pull-handles offset the same way put the length handle and the
width handle within a metre of each other at the near corner, which is inside the
widest grab there is.

### From the bench to the world, in four steps

The path exists end to end and it runs through a **filename**:

1. **Build it** at the workbench and give it a name — `N`, or the NAME row in the
   panel. `Ctrl+S` bakes it to `assets/buildings/<name>.json` in the same
   `format: 2` a hand-authored building uses, so the live preview is the game's own
   renderer with nothing special in it.
2. The game reads **every** file in that folder into `build::Catalogue` at startup.
3. In the terrain tool, `P` at the brush places the next thing in the catalogue;
   `Delete` takes the nearest away. `Ctrl+S` writes `assets/world/placed.json`.
4. The world reads that sheet and raises each entry on the ground under it, plus its
   `lift` — so a house sits on its hill however the hill is reshaped afterwards.

**Nothing could set the name** until now, so every hand-built work saved as
`untitled.json` and overwrote the last one — the pipeline was real and could carry
exactly one building. Typing a name takes the whole keyboard while it is on (`W`
walks the view, `R` turns the piece, the digits pick parts), including the key that
finishes it: a run condition is asked its question after the typing has been dealt
with, so the ENTER that ended a name found the bench listening again in the same
frame and placed a piece, and ESC walked out of the room.

Placed things can be **moved** afterwards: `G` picks up whatever the brush ring is
over, the thing follows the crosshair, `G` sets it down, `ESC` puts it back. Carried
rather than dragged, because this tool has no pointer to drag with — it aims down the
view ray and the crosshair IS the cursor.

**Only the drawn thing moves until it is set down.** Every placed thing in the world
is despawned and raised again whenever the sheet changes, so writing the sheet each
frame would rebuild a whole street sixty times a second. Carrying moves the raised
entity's own transform (found through `FromSheet`, which was written for exactly this
and unread until now); setting down writes once, which raises everything once;
cancelling writes nothing and touches the sheet, so the truth on file puts it back.

Placed things can be **turned** afterwards: `R` a quarter, `Shift+R` back, on
whatever the brush ring is over — the same rule the tool's other gestures follow.
Quarters, like the kit's own turns, because a building three degrees off its street
is a mistake that reads as one and takes a while to find; the sheet stores radians,
so a boulder that wants a finer angle can still hold one.

Things still go down facing north. That is a starting point rather than an answer
now: guessing at placement — at the camera, say — is a decision somebody has to undo,
and a known heading they can turn in one keypress is not the same as being stuck
with it.

What is still missing from the path, in the order it will hurt: no way to choose
WHICH building `P` places (it cycles), no way to resize one or to lift it off the
ground, and `assets/models/<kind>.glb` is resolved by the same sheet but nothing
generates one yet — that is the kiln's job and it has never been fired.

### Seeing a change without booting the game

`dev/look.py` draws a baked building's boxes to a PNG with a depth buffer:

```
cargo test dump_the_new_parts -- --ignored --nocapture   # prints a scene as JSON
python dev/look.py scene.json out.png --scale 330 --pitch 26
```

It exists because the tests can measure the geometry and cannot answer the only
question that mattered about the floor, which is whether it looks like wood.

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
- **The warden is ~1.8 m tall.** Terrain scale, camera distance and movement
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
    prop.rs      boulders, bushes, logs — welded per chunk, own draw radius
    heightmap.rs loads and samples the source map image
    terrain.rs   the heightfield — the single source of truth
    biome.rs     height + slope + moisture → surface color
    chunk.rs     chunk mesh construction
    stream.rs    background generation, load and unload
    edit.rs      where edits.bin lives — the brush itself is in terrain-core
    forest.rs    where forest.bin lives — the scatter is in terrain-core too
    water.rs     the sea
  player.rs      the warden and their controller
  camera.rs      orbit follow rig + free-fly
  editor/
    mod.rs       the terrain mode: raycast, gestures, live re-mesh
    theme.rs     its visual language: palette, font, shared fragments
    ui.rs        sidebar, live readouts, confirmation toasts
    minimap.rs   the world overview and camera marker
  sky.rs         sun, moon, stars, clouds, and the player's own clock
  shade.rs       the material everything solid wears, and the cloud shadows on it
  hud.rs         F3 debug overlay
assets/
  shaders/       cloud_shade.wgsl — standard shading plus how much sky a point sees
  world/         heightmap.png (the map), edits.bin (ground), forest.bin (woods)
  models/        3D models, as they're made
  fonts/         Cinzel for the terrain tool, with its licence beside it
```

---

## Change log

**2026-08-18** — **The kiln, and the tools out of releases.**

**An image in, a model out.** `F5` on the workbench sends the picture on the wall
to 3daistudio and keeps the GLB that comes back, in `assets/models` where the
placed sheet can stand it by name. Mirrors Opificium's kiln, contract and hard-won
lessons both: the download is **streamed** (a textured GLB goes past a ten-megabyte
in-memory cap, and failing there fails *after* the model is paid for), a
FINISHED-with-nothing-attached answer is told apart from a failure, and there is a
global timeout so a stalled line cannot leave it fetching for ever.

It happens **on a press and never on its own**. No retries, no polling ahead, one
job at a time — every firing spends credits and uploads a picture to a third party.
The key comes from `COPAIMO_3DAI_KEY` or the maker's own home folder, never this
repository: a key committed once has to be rotated, and the commit that did it is
usually the one nobody looked at.

**None of the tools ship.** `--no-default-features` takes out the terrain brush,
the workbench, the kiln, every layer's *writer*, and `ureq` with them. A player's
build should not carry a way to break a save, and it certainly should not carry
code that can spend somebody's credits.

Gated rather than hidden, and **proved rather than trusted**: the release workflow
greps the built binary for the service the kiln talks to and fails if it is there.
Measured — **2 occurrences in a maker build, 0 in a release**. A dropped flag is a
silent failure otherwise, and a release that ships a brush looks exactly like one
that does not.


**2026-08-18** — **The placed sheet can stand a generated model.**

Corrected after reading Opificium's **kiln**, which is the tool this was always
being compared to: an image goes to a generation service and a **GLB comes back**.
Nothing is traced. The reference-picture-to-trace-against was the wrong feature
for the wrong job.

The lesson worth taking whole, and it is in the kiln's own notes: **a generated
mesh is not a part.** A part is a name that resolves to boxes on a lattice painted
from a shelf; a model is arbitrary triangles carrying their own PBR materials. It
cannot be painted, snapped to the lattice, or written into a building's `boxes`,
and pretending otherwise breaks the brush and the bake at once. So a model stays a
**file** and is carried whole.

`placed.json`'s `kind` now resolves against two places in order: a building the
bench baked, then `assets/models/<kind>.glb`. One field for both, because almost
nothing is shared between them except the only thing that file cares about —
somebody decided one of them stands here, facing this way, at this size.

Running it found a real bug: the raiser bailed when the *catalogue* was empty, so a
world furnished entirely with generated models raised nothing and said nothing
about why. Only an empty sheet stops it now.


**2026-08-18** — **The bench gets a mouse, a paint mode and a reference picture.**

**The mouse proposes; the lattice disposes.** A ray through the pointer, met
against the plane the cursor is on, then *snapped* — and that last word is the
whole design. Letting the mouse place freely would look like precision and would
throw away what makes the kit work: every part is a multiple of the snap, so
pieces abut exactly and a wall meets a floor without anybody measuring. Free
placement gives you walls a centimetre apart and hairline gaps you cannot see
while building and cannot miss in the finished thing. Keys still nudge; whichever
moved last wins, since a mouse that overrode the keys every frame would make them
useless.

The bench also stops capturing the cursor, which it should never have done. It is
a *pointing* tool — aim at a cell, click it — and grabbing the pointer in order to
place a fence rail fights the one input the job actually wants.

**Painting is a mode, not a held key.** Going round a roof takes a minute, and
holding a key for a minute is worse than pressing one twice. Painting a piece the
colour it already is changes nothing and marks nothing unsaved.

**A picture to build against, at a stated size in metres.** The one thing that
would make a reference worse than useless is an unknown scale: trace a cottage off
an image sized to whatever the loader felt like and you get a cottage of no
particular dimensions, found out later standing beside something built properly.
Sized in module steps, because "four modules wide" is what a maker wants and it
puts every wall in the picture on the lattice. Upright it is an elevation; flat it
is a plan. Stepping through the pictures passes through *none*, so putting one
away is one press rather than as many as there are files.

`assets/reference/` — drop a png or jpg in and press `I`.


**2026-08-18** — **Ask the bench for one.**

`G` fills the bench with a house, a fence, a tower or a shelter; `Shift+G` moves to
a different kind. A new seed every press, because most of what anybody does with a
generator is press it until they like what came out — and repeatable, so "that one,
but wider" is a thing you can actually do.

**It hands you pieces, not a building.** That is the whole design constraint. The
point is not to produce a finished house; it is to skip the boring half of making
one — laying eight floor slabs and fourteen wall panels, every one of which goes
exactly where the last one implies. The interesting part is what happens next: take
a wall out for a wider door, drop the roof a storey, put a lean-to on the back. So
what arrives is *ordinary pieces*, indistinguishable from placed ones, and there is
a test that takes a generated house apart with the same calls that would have built
it. If it could not be edited it would be a black box with a building inside it.

Nothing structural is left to the seed: a house has a door, a roof covers its
footprint, and nothing generates below the floor. Proportions, storeys and
materials vary; whether there is a way in does not. A test checks the doorway
across a dozen seeds — and caught its own first version, which counted the
quarter-turned side walls as part of the face it was measuring.


**2026-08-19** — **The floor became wood, and the kit grew three parts.**

Reported from the bench: the floor's planks had gaps between them, no texture, and
stretched only one way. All three were the same shortfall — a floor was five
30 cm slats laid at the floor's full thickness, which is a boardwalk. It is
boarding over a subfloor now, at flooring width, cut into boards whose ends
stagger; the gap between two of them is a centimetre of shadow rather than a slot
through to the world. See **The kit** above.

**Stretching gained a second axis**, because a floor has two and every other part
has one. And three parts arrived with it — **foundation**, **stairs**, **bed** —
which took the kit past nine, so the digits ran out and the panel's habit of
numbering its own rows became a lie. There is one key table now, read by the input
and by the panel both. That is the second time this codebase has learned that
lesson; the first was the terrain tool's eleventh tool wearing the first one's key.

**A corner is not a floor.** The resting rule read two walls meeting at a right
angle — which share half a thickness, because that is what a corner IS — as one
standing on the other, and lifted the second a storey into the air the moment a room
got its second side. Being underneath is now about how much of a piece's FOOTPRINT is
covered: a floor under a wall on its edge covers half, a corner covers a twelfth, and
the threshold sits at a quarter. The first guess was a half, which is exactly the
number it cannot be.

**Walls stood in the floor rather than on it**, and could not be put on its edge at
all — one fault in the cursor's lattice and one in its height. See **Cells, and the
joins between them** and **A piece rests on what it lands on**.

**Two faults the same afternoon**, both reported from the bench and both the same
shape as each other: a part took whatever colour was in hand rather than its own
material, and reaching a piece was measured to its middle rather than to its body.
See **A part arrives in its own material** and **Reaching a piece** above.

**A renderer for looking at it**: `dev/look.py`. The floor took four goes to get
right and none of them could be judged from a test, because "does this look like
wood" is not a number. Two of the four goes were wrong about my own renderer
rather than about the floor — a camera basis that was not orthogonal made the
lower surface win the depth test, so the floor rendered as its own subfloor and
looked like a bug in the kit.

**2026-08-18** — **A workbench, and a kit of parts.**

**Pieces, not shapes.** A building could be authored as arbitrary boxes at
arbitrary sizes and it would be worse. Everything in a real structure is a repeat
of a few members — post, rail, wall panel, floor slab — because that is how things
get built out of stock lengths, and it is why buildings look like buildings. Free
boxes give you the freedom to make every wall a slightly different thickness,
which is a freedom nobody wants and every eye notices.

Seven parts to begin with — ten now — at fixed sizes, all multiples of a
sixteenth-metre snap on a 1.5 m module. **A fence and a house come out of the same
kit** — posts and rails, versus floor, walls
and roof — and there is a test that builds both, because that is the check on
whether the parts were chosen well or invented one building at a time.

**It writes the format that already exists.** The bench builds a `Plan`, which is
what a baked building reads as, so the live preview is the game's own renderer with
nothing special in it and what it saves lands in the buildings folder beside
anything else. Two formats for one idea is two readers, two writers, and a
fortnight of finding out which one a bug is in. A test writes a hut and reads it
back through the game's own reader.

**Quarter turns and a lattice, on purpose.** A wall three degrees off is a mistake
that reads as one and takes a while to find, and no house anybody would build has
one. When something genuinely wants an angle it wants a *part* for it — a brace —
not free rotation on a wall.

**Keys, not the mouse.** A lattice is what a keyboard is good at: press once, move
one snap, know where you are. Aiming a mouse at a 25 cm cell from across a room is
a fight, and every builder offering both ends up with people using the keys.

`Workbench` on the main menu. Verified end to end: built a hut on the bench, saved
it, the catalogue read it, the placed sheet stood it in the world.


**2026-08-18** — **A placed-object sheet: the keystone.**

`assets/buildings/*.json` says what a building IS. Nothing said where any building
stood — the world raised one at the middle of each levelled site, cycling the
catalogue, which is a stand-in and behaved like one: it could not be told where to
put anything, two towns got the same house, and the only way to change any of it
was to add a file to a folder.

`assets/world/placed.json` is the other half: *this thing, here, turned this way,
this big*. Read at startup, written by the editor, and **it comes before the
workbench, moving props and removing debris because all three stand on it** — a
workbench needs somewhere to put what it makes, moving a thing needs the world to
remember where it was, and taking a boulder out needs the world to have an opinion
about that boulder.

Three decisions worth the argument:

* **JSON, where the other layers are binary.** Those are dense grids of millions
  of cells that nobody reads. This is tens of entries, each a decision somebody
  made — it should be legible, diffable and fixable in a text editor, and at this
  size that costs nothing.
* **On the ground, not at a height.** `at` is x and z; height comes from the
  ground plus `lift`. An absolute height is simpler and wrong: sculpt the hill and
  every building placed beforehand is buried or standing on air with nothing to say
  which. A bridge over a gorge is placed on the gorge floor with a lift.
* **Things carry names, not list positions.** Delete the third of five and every
  index after it shifts, so a selection or an undo entry would point at a
  different object with nothing to say it had moved.

The site loop is **gone**, not kept alongside — two systems both spawning
buildings would put two on every site the moment anybody placed one deliberately.
Rebuilds are despawn-and-raise rather than a diff: a placed world is tens of
buildings, and a diff that is subtly wrong leaves a building that cannot be
deleted.

`P` places the next thing in the catalogue under the brush, `Delete` takes back
whatever is nearest, and both save with everything else. That is deliberately not
the workbench — no gizmo, no snapping, no shelf — but a format nothing can write
to is a plan rather than a feature.


**2026-08-18** — **Two editor controls.**

**No flying under the ground.** Under the map is not a place: the world is a
single surface with no underside, so a camera beneath it sees the backs of hills
and the sea from inside. What makes it worth a guard rather than a note is that
nothing about the view says "you are underneath" — it simply looks broken, and
the world gets blamed. The free-fly camera is held above the *drawn* height, or
above the sea where that is higher, so the floor is the surface actually on screen.

The clearance is 2 m and the test is why. I guessed 1.6, and it failed: the grass
reaches **1.66 m** since the meadows pass. It reads `cover::tallest()` from the
crate rather than a number copied here, because tall grass grew twice in one
evening and will grow again.

**A heading on the overview.** A dot says where you are and nothing about where you
are pointing, which on a map with no landmarks yet is half the information missing
— you can see you are on the north coast and not whether you are facing it. A
needle now turns with the camera, rotated about the marker rather than about
itself: a bar rotated about its own middle swings around a point halfway up itself
and reads as an axis through you rather than a direction from you. The caption
says N is up.


**2026-08-18** — **A biome brush, because I cannot see and the maker cannot edit
constants.**

Where the countries were was decided in code — a band across the east, an oval in
the middle — and moving one meant reading a marker's position off a screenshot,
guessing which constant that implied, and nudging it. That went wrong five times
in one evening. Not carelessness: the person who can SEE where a desert belongs
and the person who can edit the number were not the same person, and a picture is
not a coordinate.

`country.bin` is a fourth painted layer, as coarse as the woods because a country
is kilometres across. **Painted ground overrules the generated regions; where
nothing is painted the code still answers**, so a fresh world still has continents
with character rather than one green sheet.

It needed two things the other layers did not, and both come from the same fact:
**a country is a NAME, not an amount.**

* `stamp` writes an exact value instead of adding to what is there. You cannot
  accumulate your way from grass to snow, and the clamp to ±1 that keeps a bias
  sane would refuse to store a third option at all.
* `choice` reads by VOTE instead of by blend. Blending four cells turns a two
  beside a four into a three — grass beside snow reading as desert, a country
  nobody painted appearing along every boundary between two that somebody did.
  The corners vote, the winner takes the point, and the share of weight it carried
  gives a painted edge its soft side without inventing anything.

The same fact caught a live bug through a test: the right button originally
*faded* like every other layer's eraser, which walks a mark down through the other
countries' marks. A snowfield being cleared read as desert, then as grassland,
then as nothing. It stamps zero now, and a test asserts no country nobody chose
ever appears between two that somebody did.

`B` picks the brush and cycles which country it lays; the caption says which,
since one swatch cannot show three. The overview watches this layer as well as the
ground — it is the only place a whole region can be seen at once, so it must not
go stale while a region is being drawn.


**2026-08-17 (last)** — **Stop guessing at boundaries; fill the continent and
count.**

"The entire western continent should have zero desert" was said four separate
ways, with marked-up screenshots, and four times I moved a number without fixing
it — because I was reading a marker's position off a picture and guessing which
ellipse to nudge. Guessing at a boundary is not a method.

A continent is something the world already knows: it is the land you can walk to
from the ranch without getting your feet wet. Flood-fill it and count the desert
cells on it. **It was 57. It is 0**, and there is a test that says so, so it
cannot quietly come back.

That also surfaced how wrong my mental picture was — the home continent reaches
east to **u=0.529**, far further than I had assumed, which is why every nudge kept
missing. The desert now sits on the northern landmass only, a small band
north-west to south-east.

This matters past looking right: monsters will be placed by biome, so a desert
species turning up on the home continent is not a colour being slightly off — it
is the wrong creature in the starting area.

Standing at: grass 32.4%, settled 17.8%, snow 17.5%, desert 10.2%, shore 9.2%,
forest 6.7%, rock 6.2%. Nearest desert to the ranch: 1,640 m.

**2026-08-17** — **The desert's two numbers do different jobs.** Asked to
reach further south-east, it went *west*: the width was grown alongside the
length and the rim tightened, both of which push the western edge out into the
green world. Correcting that by sliding the middle south then lost the northern
end and left a finger of sand on the western continent.

`DESERT_REACH.y` is **length**, along the lean, north-west to south-east.
`DESERT_REACH.x` is **width**, across it. Only the first does the asked-for job,
and the file says so now. One number, changed on its own — desert 12.8%, and its
near edge 1,280 m from the ranch.

**2026-08-17** — **The snow is a band; the desert is a place.** A band was
the wrong shape for the desert in the *other* direction. The snow is a whole end
of the world and earns one — it has to hold from the north coast of its island to
the south without stopping short, which is exactly what an ellipse could never
do. The desert sits in the middle of the green world with grassland on every side
of it, and a band gave it a northern and a southern coastline it was never meant
to have. It is an ellipse again, measured along the same tilted axis so it leans
with the continents, and laid only where the bands have not already spoken.

Two assertions of mine went with it. One constrained what a region does out at
sea — `Biome::of` answers Water before it ever asks which country a point is in,
so that constrained nothing and was a trap for whoever next moved the ellipse.
The other walked a square box around the ranch while claiming a radius, so its
corners reached 1.4× as far as its sides and it failed on ground that was never
inside the claim.

Standing at: grass 31.6%, settled 17.8%, snow 17.5%, desert 11.0%, shore 9.2%,
forest 6.7%, rock 6.2%.

**2026-08-17** — **Snow country is white down to the water.** Its low
ground painted conifer green, because that is the biome down there — and the
result was a ring of green around every white island, which reads as snow
stopping before the shoreline. It doesn't: a snowy forest is conifers standing
*on* snow. The ground goes white and the trees are still planted, which is both
what it looks like and what was actually wanted from "we could have trees in
snowy areas".

Band edges moved with it: the desert's western boundary was still landing on the
western continent in the south, and the snow began east of its own island's
western half.

Standing at: grass 27.1%, settled 17.8%, snow 17.5%, desert 15.5%, shore 9.2%,
forest 6.7%, rock 6.2%.

**2026-08-17** — **Bands, not blobs.** Regions were ellipses, and an
ellipse is the wrong shape for the job: a blob has a rim *everywhere*, so it
always stopped short of something — the desert before the north coast, the snow
before its own shoreline — and each time the answer was to grow the blob until it
covered the land, which squeezed whatever was beside it. Every one of those was
the same fault wearing a different number.

The map is divided by **lines** now. Each band runs coast to coast by
construction, so "the whole of this section is desert" is something the model can
*express* rather than something it has to be tuned toward. The lines are tilted,
because the continents are.

The ragged edge moved inside `region::at`: how far along the axis a point counts
as being is nudged by a fine speckle, so every question about that point gets the
same broken boundary. Each caller dithering for itself is exactly how the painter
and the classifier came to draw two different lines.

**And moisture is gone entirely** — the field, the frequency constant, the green
ramp, all of it. The green world has no climate. What decides a wood is the ground
(not too steep, too high, the beach, or somebody's yard) or a person with the
Plant brush. *Consequence worth knowing:* ordinary country is now uniformly
`Grass` with trees scattered on it, so `Forest` only occurs as snow country's
conifers. If distinct woods are wanted, forest should become a named band like
the others.

Standing at: grass 26.0%, desert 21.9%, settled 17.8%, snow 14.4%, shore 9.8%,
forest 5.4%, rock 4.7%.

**2026-08-17** — **A hard choice still needs a soft edge.**

Naming countries instead of inferring them fixed the class of bug that had been
eating the session, and immediately introduced its opposite: a country is a hard
choice, and a hard choice drawn straight across the map is a **line**. Grass on
one side, snow on the other, nothing between.

How firmly somewhere belongs to its region now races the local noise. Deep inside,
belonging wins everywhere and the region is solid; at the rim it wins only where
the noise is low — so the boundary breaks into fingers of one country reaching
into the other. One comparison, no new field, and it lives in `region::holding`
where *every* path can reach it. What a place is and what it looks like are
decided in different files, and the three times they were each given the chance to
answer that question separately they answered it differently.

Two more that came with it. **Trees filled the deserts** the moment moisture
stopped meaning anything — a noise field centred on a half says half the slots
everywhere, sand included; trees are a fact about the map now. And **snow country
had beaches**, a ring of sand around its own coastline, which is a beach nobody
would sunbathe on.

Standing at: grass 19.0%, settled 17.8%, forest 17.5%, snow 17.3%, desert 13.2%,
shore 9.7%, rock 5.5%.

**2026-08-17** — **A region NAMES a country; it does not describe a climate.**

This began as two physical fields — how dry, how cold — with the biome inferred
from them by threshold. That is how a simulation does it, and it was a steady
source of bugs in a game that is not one. The moisture ramp, the treeline and the
snowline all pushed each other about: lowering the snow to reach a coast closed
the bare-rock band, widening the desert to reach a town squeezed out the grassland
behind it, and every one of those was a consequence arrived at by arithmetic from
two numbers nobody wanted to think in.

Nobody needs a humidity model to say "the northern desert". A region is a
**country** now — ordinary, desert, or snow — and the map says which. Height and
slope are left with what they genuinely decide: where snow sits on a mountain,
which faces are too steep to hold anything.

Retired with it: `DESERT_MOISTURE`, `TEMPERATE_MOISTURE`, `ARID_MOISTURE`,
`LOCAL_MOISTURE`, `CHILL_TREELINE`, `CHILL_SNOWLINE`, and the whole per-point
`climate_at`. One `COLD_SNOWLINE` replaces four, and the rock band is *derived*
from it rather than given a number of its own — two independent lines could walk
past each other and close the band, a fraction of one cannot.

`moisture` survives as `wooded`, deciding only meadow against wood *within* the
green world. It never decides which country somewhere is in.

The arrangement is a test now: **grass/forest, desert, grass/forest, snow**, read
west to east along the middle of the map.

Standing at: desert 17.8%, snow 17.8%, settled 17.8%, grass 14.4%, forest 13.9%,
shore 12.0%, rock 6.3%.

**2026-08-17** — **Trees were the one path that never asked which region it was
in.** Planting was keyed to the *global* treeline, so trees grew to 150 m
everywhere — including snow country, where the treeline is ten. That is why the
snowfields had a forest standing on them: the ground was classified snow, painted
snow, and planted as though it were a temperate hillside.

Desert grown south and east again. It now measures 16.8% of the land and its near
edge sits **1,400 m from the ranch** — about three minutes at a jog.

The region test's stray-check moved with it. It was a box over the whole
south-west, which reached onto the middle landmass and started failing the moment
that landmass was asked to be desert: the claim was right and the box was drawn
around the wrong thing. It is anchored to the **ranch** now, because the corner of
a map is arbitrary and the ranch is the thing that must not wake up in a desert.

Standing at: snow 24.9%, grass 17.9%, settled 17.8%, desert 16.8%, shore 12.0%,
rock 6.1%, forest 4.6%.

**2026-08-17** — **The regions grew to the areas they were drawn over.**
Both had to be pushed out twice, for the same reason each time: **a zone's rim is
not its region's rim.** The falloff leaves the outer band merely dry, or merely
cool, so the region lands well inside the ellipse that produced it — the desert
stopped short of the town below it, and the snow country was a ring round the
peak with forest behind the mountain. Measure the world, not the zone.

Snow country had a second cause: even at full chill the treeline stood at 57 m,
so every part of an island lower than that grew a forest, which is most of an
island. The lines come down almost to the water now.

And the **treeline** drops harder than the snowline, which is the opposite of how
it first read. Drop the snowline further and snow starts below where trees stop,
closing the bare-rock band between them — a mountain goes wood straight to white
with no mountain in it. What gave it away was bare rock measuring **0.0%** of the
world; it is 5.2% now.

Standing at: grass 22.7%, snow 22.7%, settled 17.8%, desert 14.0%, shore 12.0%,
forest 5.6%, rock 5.2%.

**2026-08-17** — **The ground looks like the region it is in.** Deciding
what a place *is* and drawing what it *looks like* are two separate paths here,
and only the first one had been told about the regions. So the northern desert
was classified desert and painted dry grassland — sand existed in the palette but
was reachable only from the shoreline band — and snow country was classified snow
and painted green until 200 m up.

They were also reading two different snow lines: **165 m for deciding, 210 m for
painting**. One idea, two numbers, and forty-five metres of world where the ground
disagreed with itself about what it was. There is one number now and both paths
read it, along with the same treeline and the same chill.

Guarded by a test that averages the painted colour over every desert and every
snow tile of the real world: desert has to come out warm with red over green
(dry grass is olive, so the old behaviour fails it), snow has to come out bright
with no colour cast.

**2026-08-17** — **The world has regions now, not scattered biomes.**

What kind of ground a point carried was decided entirely by that point: a
moisture field said dry here and wet there, and desert appeared wherever the
noise happened to dip. Correct at every point and wrong everywhere — desert
patches inside grassland, a stripe of wood across a dune, no two hundred metres
the same as the next. A player can say "the northern desert" or "the snow country
in the east"; they can say nothing at all about a place whose character changes
every time they walk a field's width, and a monster that belongs to the desert
needs a desert to belong to.

Three hand-placed zones in normalised map coordinates — read straight off a
picture of the world with the areas drawn on it — say how *dry* and how *cold*
somewhere is, with soft edges a day's walk wide. The local noise is demoted to
what it should always have been: variation within a place, not the thing that
decides which place it is. Cold regions work by bringing the treeline and the
snowline **down to meet the ground**, so snow country is snow country rather than
high ground that happens to be white.

Measured on the generated world, not on a fixture: **desert 8.8% of land centred
north-central, snow 15.2% centred on the eastern island, grass 35%**, and *no*
desert or snow anywhere in the south-west where the game starts. Towns still
overrule everything — a site in the desert is still a town.

**2026-08-17** — **Grass grows only where grass grows.** Sand, rock, snow,
desert and the ground a town stands on each carried a thin scatter, on the
reasoning that nowhere real is completely bare. True, and the wrong call: it made
five different places look like one place with different ground paint. What they
needed was things that belong in *them*, and they have those now — driftwood,
scree, cactus, dead brush. Open country and the wood floor keep it; nowhere else
grows a blade. A town is somebody's, besides.

Ribbons thickened by half at the same time, which cost what the width model said
it would: fragments 4.56 M → 7.18 M, main pass 6.86 → 7.81 ms. The frame held at
**48–51 fps** anyway, because it is shadows that own the frame and not that pass.
Width remains the dial to reach for if headroom is ever wanted back.

**2026-08-17** — **A blade of grass is a ribbon, not a wedge.**

Twice the shape was the complaint and both times it was one fault: the blade was
drawn as a wedge — wide at the foot, needle-pointed, aimed somewhere. That is an
agave leaf, and a ring of them from above is a black starfish, which is what it
looked like. Bending it in the middle helped from the side and not at all from
overhead, because it was still a wedge with a point on it.

It is swept along an arc now: one narrow width for almost its whole length,
tapering only at the end, each step turning further from upright than the last so
it curves rather than kinks, and past a right angle the tip is falling. A
centimetre across rather than four. The length grew to compensate — a blade that
arches reaches about three fifths of its length into the air.

**And it turns out grass was never vertex-bound.** The ribbon put the count up a
fifth and the frame cost *down*: fragments 6.53 M → 4.56 M, main pass unchanged.
A meadow of 4 cm wedges overdraws itself many times over. Width costs more than
vertices do, which is worth knowing before anybody tunes this again.

**2026-08-17** — **Every tuft was the same tuft, and the parting snapped.**

The tufts fanned through a whole circle, which is a rosette by construction —
turn one and you get the same object back, so jittering the angles inside it can
never make two of them differ. They fan through half a turn to four fifths now,
so a clump has a front and a back and the turn it was already planted at starts
doing visible work. The clump leans bodily as well, and the blade count varies.

**The snap was a 180° flip.** The push points away from whatever is standing
there, so it points the opposite way on either side — walk over a blade and its
lean reversed in the width of a boot. The sideways push now fades out toward the
middle and a downward *press* fades in to replace it: what is directly underfoot
is trodden, not shoved aside, which is both what happens and the shape with no
discontinuity in it. Plus a lag, so the disturbance follows you rather than being
pinned to you — one lerp on the CPU doing what per-blade state would have done.

**A blade of grass is a blade, and it parts as you walk through it.**

The tufts read as crowns, and the construction *was* a crown: blades at even
steps right round a circle, all the same length, all leaning the same amount, all
rising from one point. A stem under a fan of spikes is a coronet. So the even step
is jittered by most of the gap between blades, the blades rise from a patch of
ground rather than a point, and their lengths differ by better than half.

**And a blade bends.** It was one triangle — straight, evenly tapered, pointing
wherever it was aimed. Two segments now, the fewest that can curve: up steeply,
arcing over at the tip. That curve does almost all the work of reading as grass
rather than as geometry.

**Grass moves for whatever walks through it.** The world material grew a vertex
stage — Bevy's own, copied, with one call inserted — that pushes a blade away
from anything wearing `Wades`. The foot never moves; the bend goes as the square
of how far up the blade a vertex sits, which the mesh carries in its spare U
coordinate, because a vertex's height is the ground's plus the blade's and only
the second may move. A component rather than a query for the player, because
grass that parts for you and stands still for what is stalking you would be worse
than grass that never moved.

Costed nothing measurable: **50.2 fps**, and one material written per frame,
since the whole world's cover shares one.

**2026-08-17** — **Tall grass, and it is tall on purpose.** A patch core
comes up past the knee, three times the height of its own edge, and grows fuller
and deeper in colour the further in it stands. Grass you can see over is scenery;
this is meant to be somewhere a wild monster can be without being seen, and
somewhere you can tell from across a field where it starts and stops. A blade is
one triangle, which makes thickness the cheapest kind of detail in the world.

Thickened once more after that — spacing to a metre and two more blades on every
tuft deep in a patch, since once every slot carries a tuft the only levers left
are closer slots and fuller ones. Half as much grass again for **48.7 fps against
48.3**, and a main pass that got *faster*: this machine is not vertex-bound at
this scale, and grass is no longer where the frame goes.

Costed 48 fps against 49, because grass stopped casting shadows first. That is
the whole trade: it used to be submitted five times over — main pass plus four
cascades — to show a smudge under something a hand tall, and spending that
five-times-over budget once instead bought a thicket.

**2026-08-17** — **Grass grows in meadows now.** Cover spread evenly is
the same thin stubble on every field in the world, which is what it looked like.
The same amount of grass is *gathered* instead: two octaves of value noise
squared off into patches with middles, edges and gaps, so a meadow core is solid
and taller and the ground between is nearly bare. Tuft spacing came down from
2.6 m to 1.7 m at the same time — a tuft is a hand's width, and one every two and
a half metres is not a sward however many there are, because the eye reads the
gaps.

**Desert is why patching takes a biome at all.** Dry scrub is sporadic *by
nature* — that is what makes it read as dry — so gathering it into lush patches
would be inventing oases. Rock and snow the same. Grass, forest, shore and
trodden ground clump; the rest stay as they were.

**Grass casts no shadow, and that is what pays for it.** A meadow is by far the
heaviest thing in the world by triangle count and every caster is submitted again
for each shadow cascade, so a chunk of grass was drawn five times to show a smudge
under something a hand tall. It still *receives* — grass under a tree or a cloud
goes dark with the ground it stands on. Measured: **49 fps against 44**, denser
grass and a faster frame both.

**2026-08-17 (last)** — **The world has things lying about in it.** Eight kinds
of natural object — boulders, scree, bushes, stumps, fallen logs, dead standing
snags, cactus and dry brush — three sizes of each, keyed to the biome they belong
in. A wood gets its floor of wreckage, bare rock sheds stone, dry country grows
cactus and dead sticks, the shore gets driftwood. A landscape of ground and trees
reads as a golf course; the litter is what tells you where you are, and later
what tells a monster where it lives.

**Welded, not planted, and that is the whole of why it is free.** A tree is
spawned as its own entity because it wears one material for bark and another for
leaves. A prop carries its colour in its vertices instead, so a chunk's worth of
them is stamped into ONE mesh — fifty objects in one draw call rather than fifty,
which matters twice over because every caster is submitted again for every shadow
cascade. Measured: 43.9 fps with them against 42.1 without, and **nineteen extra
entities** in the whole radius.

**Its own draw radius**, three chunks, about 450 m — shorter than the horizon and
longer than the grass, because a metre-wide boulder is sub-pixel well before the
terrain under it runs out. `PROP_CHUNKS` is the first knob if the frame rate ever
wants headroom back; the cost goes as its square.

**2026-08-17 (later still)** — **The frame rate was the shadows.** Four cascades
reaching nine hundred metres meant every one of them redrew the whole visible
world, so a frame spent twenty-five milliseconds building shadow maps and seven
drawing anything. Three cascades at four hundred metres, which is as far as a
shadow's actual job reaches — sitting an object on the ground it stands on —
takes that to sixteen. Past that a shadow is texture on a hillside, which the
terrain's own shading already gives.

**And the leaves.** A leaf clump is eighteen vertices and an oak carried fifteen
hundred of them: twenty-seven thousand vertices of foliage on one tree, drawn
thousands of times and then again for every cascade. The budget moved from count
to size — half the clumps at 1.4× the radius cover the same crown, since area
goes as the square — for half the vertices.

Together: **30 fps to 42** at midday, measured back to back on the same view.

What did NOT help, so nobody spends the afternoon on it again: a smaller shadow
map (the passes are geometry-bound, not fill-bound), a shorter shadow distance
than 400 m, and fewer than three cascades (two is worse than three, because the
far one then has to cover everything). The remaining sixteen milliseconds is the
cost of submitting the tree geometry once per cascade, and the answer to that is
vegetation LOD, not another constant.

**2026-08-17 (later)** — **Rivers are switched off.** `RIVERS` in `config.rs`,
the same bargain the roads between towns get: the machinery is written, tested
and shared with the bench, and none of it is in the way while it is not running.
Nothing is carved, no surface is drawn, no ground calls itself flooded, and no
town has a river to avoid — one switch at the one place rivers come from, and
everything downstream falls out of it.

**What killed them was width, and every lever pulls against the others.** A
channel's cut spreads over three times its own width because banks do, so water
filled to any useful depth spreads about that far too: an eighteen-metre channel
arrives on screen sixty metres across. Over the whole network that came to **a
fifth of the land under water** — not rivers through a landscape, a landscape
with a lake on it.

The three levers and why none of them is a tuning pass: `BANKS` makes the cut
spread, and narrowing it narrows every valley in the world as well; `RIVER_EDGE`
is where the waterline sits on that spread, and tightening it leaves too little
room for the surface's edge to feather, so the rim goes ragged again;
`NARROWEST` sets the smallest channel, and it was raised to eighteen metres in
the first place because anything less could not be drawn at all — recorded on a
twenty-metre grid, a seven-metre creek came out as disconnected rectangles lying
in fields. Any two of those can be satisfied at once. That is the actual problem,
and it is a design question about what a river IS here rather than a number to
adjust.

Everything below still stands and still runs if the switch goes back on.

**2026-08-17** — **Water fills a channel; it does not sit at a height.** The
random slabs of river lying about in fields are gone, and the reason they kept
coming back is worth writing down: a water LEVEL is a flat thing and the ground
it covers is not, so the two disagree, and every disagreement between them is a
sheet of water hanging over a hillside. Four attempts moved the slabs somewhere
else — held level, capped level, level masked to a bed, fixed depth above one.

There is no level any more. The water fills the channel that is still cut into
the ground at that point, three quarters of the way up, and a fraction of a hole
cannot be outside the hole. It also gives a big river deeper water than a creek,
which a fixed depth never did.

**What made it stubborn was the towns.** The rivers are carved first and a town
levels its site on top — which fills the channel back in. The ground went up; the
record of the cut did not, so the water carried on being drawn at the depth of a
channel that was no longer there. **787 of the 804 slabs in the world were sitting
on a town's flat field.** Levelling raises the ground by exactly the cut it
covers, so what is left of a channel is now worked out from what is left of the
cut, and a levelled site has none.

**And no town is built on a river.** Seventeen of the twenty-one were. Siting
already asked how high, how steep, how far from the sea and how far from its
neighbours; it now also asks whether water runs through the ground, and turns it
down if it does. The ranch is pinned by hand and keeps its spot — its levelling
erases the channel under it.

**A river's three edges all feather away to nothing**, and each had to be found
separately: the BANK where the bed gives way to the rise beside it, the SHALLOWS
where the channel runs out of depth, and the MOUTH where the river reaches a sea
already drawn at its own level. An edge that simply stops leaves a step, and a
step of water with nothing under it is what a slab actually is. Half the river's
rim now meets the ground within four centimetres and ninety-nine in a hundred
within twelve.

**Nothing is cut narrower than the world can draw.** The narrowest channel goes
from seven metres to eighteen. Nothing about a seven-metre channel survived the
journey to the screen — recorded on a grid sampled every twenty metres and drawn
on a mesh coarser than it was, it came out as disconnected rectangles of water
lying in fields, one per grid cell it happened to land on. This adds no rivers:
the same cells drain the same way. The river surface is also drawn at the ground's
own resolution now rather than a quarter of it, so a bank is a bank and not a
flight of eight-metre stairs.

**One answer, everywhere.** The drawn surface and the biome that calls a place
water were two claims from two fields with two thresholds, and they disagreed
wherever the fields did. Both ask `river_depth` now. The old held water level is
deleted from `terrain-core` outright — with it went the downstream sort of every
cell on the map, which existed only to keep that level from climbing.

Cloud shadows went from a third to nearly half strength, with a solid middle
rather than all soft rim: at a third a small cloud never reached full strength
anywhere and half the sky cast shade you could barely see.

**2026-08-17** — **Cloud shadows, cast by the actual clouds.** The clouds
overhead now put shade on the ground, and it is theirs: one soft disc per cloud,
placed where the sun's own line through that cloud strikes the land. Stand in a
patch of shade, look up, and the cloud casting it is there. The usual way to do
this is scrolling noise, which looks right until somebody checks.

They are not cast by the engine's shadow pass and cannot be — a caster at 165 m
would need the cascades stretched past anything useful for the world underneath.
They are laid on in the material instead, which is why there now IS one material:
`Shaded`, worn by the ground, the grass, the trunks, the leaves, the water, the
walls and the warden alike. A cloud shadow that stopped at the edge of the grass
would be worse than none.

Almost nothing is sent to the GPU for it. A cloud's drift is a speed times the
clock, so the shader works out where each one is for itself; the only thing ever
rewritten is the sun's slant, a few times a minute as it climbs. The sky wraps
around the viewer, which makes it a tile repeated in every direction — so the
copy of a cloud that shades a point is whichever one it is nearest to, found by
rounding the gap away in whole tiles rather than by checking nine neighbours.

Shadows fade out through the morning and evening. A cloud two hundred metres up
with the sun near the horizon casts more than a kilometre sideways, which is
true and useless: the shade over your head would belong to a cloud you cannot
see. It is also the hour when the light is too flat to read a shadow by.

They shade a sixth of the ground, which is a clear day with weather crossing it,
and a test measures that off the real sky rather than a fixture — cloud count,
scale and ceiling have all been tuned by eye for how they look UP there, and
every one of them moves the ground too.

**Every triangle in a tree is now wound on its own account.** The last seven
inverted faces of a spruce — and twelve of a willow — were the second triangle of
a quad that the first triangle had decided for. A quad is two triangles, and
where a limb turns through an elbow they do not face the same way. The winding
lives in one place now and every triangle goes through it, walls, caps and leaves
alike, so there is nowhere left for the question to be answered on something
else's behalf. The guard drops from "under one in two hundred" to none.

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
standing height, so the warden can paddle at a beach and is turned back by the
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
sea, sun and fog, a placeholder warden with a ground-following controller, an
orbit camera with free-fly, and the F3 debug overlay. World scale set to 8192 m
wide, sourced from a supplied fantasy map.
