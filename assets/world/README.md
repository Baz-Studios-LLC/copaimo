# World map

`heightmap.png` in this folder is the authority on the shape of the game world.
The generator reads its brightness as elevation, so the image decides where land
and sea are, where the mountains run, and the world's proportions. If it's
missing, the game falls back to procedural noise and logs a warning — so it will
always run, but it won't be *our* world until the file is here.

## What to export

Any PNG works; brightness is elevation.

* **Darker = lower.** Pixels below `MAP_SEA_THRESHOLD` (20% brightness by
  default, in `src/config.rs`) become sea floor, brighter ones become land.
* **Grayscale heightmap export is ideal** — it already uses the full black-to-
  white range, which is exactly what the terrain wants.
* **A colored political map also works**, just less precisely: brightness gets
  auto-normalized on load, but pastel country fills all sit at similar
  brightness, so the land comes out flatter and the coastline follows the fill
  edges rather than real elevation. Expect to nudge `MAP_SEA_THRESHOLD`.
* **Resolution isn't critical.** The image is sampled bilinearly and procedural
  detail is layered on top, so even ~1000 px across gives crisp ground up close.
  Higher resolution buys finer coastlines and inland lakes, nothing else.
* **Aspect ratio carries through.** The image's proportions set the world's
  north–south extent; `WORLD_WIDTH` sets its scale in meters. A 2:1 map at the
  default 8192 m wide makes a world 8192 × 4096 m.

## Scaling the world

One number, in `src/config.rs`:

```rust
pub const WORLD_WIDTH: f32 = 12_288.0;
```

Everything else — chunk counts, world bounds, the coastline — is derived from it.
At the warden's 7 m/s jog, 12 288 m is about half an hour east to west.

## Later

The same pipeline can take a second image for *regions*: a map where each
political area is a flat unique color becomes a lookup for "which nation is this
point in", which is how cities, guild territory and region-specific monsters get
placed without hand-entering coordinates.

## What else is stored here

The world is a hybrid: generated ground with stored layers over it, and the
generator reads every one of these. Nothing here is a baked scene — it is data the
generator consumes, which is what lets `--audit` and the tests check hand edits the
same way they check generated content.

| file | what | edited by |
|---|---|---|
| `heightmap.png` | base elevation (above) | an image editor |
| `edits.bin` | authored height **offsets** on a 4 m grid, over the generated ground | the in-game sculpt brushes (`src/editor`) |
| `surface.bin` | signed surface bias on a 4 m grid — worn earth and paving paint | in-game paint |
| `country.bin`, `forest.bin` | painted region and woods-density rasters, 16 m | in-game paint |
| `placed.json` | hand-placed buildings and props, each with a stable `u32` id | the in-game place/carry/turn/remove editor |
| `settlement_<name>.json` | **one town per file**: its ways, lots and public places | any text editor, or `--bake` |
| `wild.json` | places the generator may not plant trees or strew boulders | any text editor |
| `world.json` | generator parameters | by hand |

### Settlements

Every settlement has a permanent name (`harbour_city`, `village_1`, `city_2` …)
and a file. The game reads the file **instead of generating** — so editing
`settlement_harbour_city.json` and relaunching is how you move a building or
reroute a street by hand.

Rules the files hold to, and why:

* **Only what was placed is in the file.** Ways, plots (buildings and yards),
  places (squares, parks, markets). The streets between junctions, the junctions
  themselves, the lamps, the retaining walls and the flights of steps are all
  *derived* from those on the way in and never written down. A derived thing in a
  file is one fact with two derivations, and the file's copy wins forever.
* **Every row says where it came from** — `"from": "Generated"` or
  `"from": "Authored"`. Mark a row `Authored` when you edit it. A re-bake
  (`copaimo --bake`) regenerates every `Generated` row from the current generator
  and keeps every `Authored` row untouched, dropping any regenerated row that
  lands within 6 m of one you authored. A row with no `from` counts as
  `Generated`.
* **`--bake` always starts from fresh generation**, never from the file, so
  generator improvements reach stored towns. The file's `stamp` records the site
  position, radius and seed the rows were made from, so a stale file can be seen
  for what it is.

### Deleting something

Don't delete the row — a re-bake would put it straight back, because a generated row
has no name for anything to remember it by. Add a **veto** instead: a place the
generator may not build on.

```json
"vetoes": [
  { "at": [-2553.0, 2251.0], "within": 30.0, "kind": "Plots" }
]
```

`kind` is `All`, `Plots`, `Ways` or `Opens`. That example empties the market square
of its stalls and leaves the paving. A veto says *where*, not *which*, so it keeps
working when the generator moves the building a few metres, and it goes quiet by
itself if the generator stops putting anything there. It is indiscriminate by design:
everything of that kind inside the circle goes.

Vetoes survive every re-bake. Nothing else does, unless you marked it `Authored`.

Coming: ids on authored rows, allocated when you author one, so two authored things
can sit on top of each other without the 6 m rule confusing them.

### The wild

Trees and boulders are not in any file, and putting them there would be the wrong
answer. Every one of them stands on a world-wide lattice and is a pure function of its
slot — about 228,000 trees, all reproducible from nothing. **Adding** things is
already covered by `placed.json` and its stable ids. The only gap was taking one away,
so that is all `wild.json` holds:

```json
{
  "version": 1,
  "vetoes": [
    { "at": [-1020.0, 2040.0], "within": 110.0, "kind": "Trees" }
  ]
}
```

`kind` is `All`, `Trees` or `Props` — props being boulders, logs, stumps and brush.
That example clears a 110 m circle of woodland and leaves the boulders standing in it.

**In the editor**, point the brush at a tree and press `Delete`. If nothing of yours
is under the ring it writes a veto covering the ring instead, and says how many trees
it took — so the usual way to clear a wood is to look at it rather than to work out
its coordinates.

Like a settlement's vetoes this says *where*, not *which*, because a generated tree
has no name. It keeps working when the generator jitters the tree a metre, and it
costs nothing in a world nobody has edited — the file is empty, and the check is one
slice length.
