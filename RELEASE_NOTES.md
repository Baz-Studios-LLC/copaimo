## Copaimo: The Wardens Guild

The last release connected the towns and turned the lights on. This one **rebuilds
the first city you will ever walk into** — and hands you the keys to the world.

### A city on a hillside, not a wheel

The city nearest the ranch used to be a wheel: a ring road, spokes off it, and
everything arranged around the hub. It read as drawn, because it was.

Its streets are **grown** now. They start at the market and at the roads coming in,
and they feel their way outward — and on a hillside they do what streets on a
hillside do. The long ones run **along** the slope, holding their level. The short
ones cut **across** it, and where one of those meets a drop it becomes a **flight of
steps**. Nothing was placed to make that happen; it falls out of the ground the town
is standing on.

### Terraces you can see holding the hill up

The city is cut into level platforms that rise toward the middle, and the edge of
every one is a **retaining wall** — coursed stone, battered back into the bank,
coped along the top, with plants growing out of it. Streets run along the top of
them. Where a street needs to get from one level to the next, there is a break in
the wall with a stair in it, and the stair is on the route rather than beside it.

The rings are not circles. That was the first attempt and it looked exactly like
what it was.

### A harbour

The city stands **on the water** now, and there is a **stone quay** along the shore
with bollards to tie up to and a **timber jetty** on posts walking out over the bay.
The town's ground stops at the tide instead of levelling the sea into a field —
which sounds like a technicality and is the difference between a harbour and a
meadow with boats drawn on it.

The bank between the town and the water is terraced down to it, so the harbour is
somewhere you walk to.

### The world is yours to edit

Every settlement is now **a file you can open**. `assets/world/settlement_*.json`
holds one town each: its streets, its buildings, its squares and markets. Change a
line, relaunch, and the town is different. Move a building, reroute a street, delete
something you never liked.

Two things make that safe rather than fragile:

* **Nothing derived is in the file.** The junctions, the lamps, the walls and the
  stairs are worked out fresh every time from what you placed — so moving a street
  moves its lamps and its walls with it, automatically.
* **Every row remembers whether it is yours.** Mark a row `"from": "Authored"` and
  the generator will never touch it again. Run `copaimo --bake` and every row you
  did *not* claim is rebuilt from the current generator, while everything you
  authored survives untouched.

So the world improves underneath your edits instead of overwriting them. There is a
guide in `assets/world/README.md`.

### Also

* Buildings no longer stand on ground that slopes away under them near the shore.
* A settlement beside the sea no longer raises the sea floor to meet itself.
* The forest stopped planting trees in city streets along the waterfront.
* Roads arriving at a town always meet a street, whichever plan drew the town.

---

373 tests, 4,435 streets audited with nothing standing in any of them, and 35 of 35
playtest routes driven by the real game with real collision.
