## Copaimo: The Wardens Guild

The warden landed last release. This one is about **the world he walks**: it has a
new continent on it, and it is more than twice the size it was.

### There is a fifth landmass

The map has always been the drawn one — a fantasy sheet whose coastlines the game
reads for land and sea. It still is. **Sorrel** was added *to* it rather than by
replacing it with something generated, because the map was already right and only
needed more of itself.

There was no room inside it. The widest stretch of open water anywhere on the sheet
measures 1.5 km across, which is an island and not a continent, so the sheet itself
grew — equally north and south, which is what keeps every existing coastline on the
world coordinates it already had. Nothing you could already walk on moved.

Sorrel is generated rather than drawn, and it took three attempts to stop looking
like it. A coastline is not an ellipse with noise on the rim: the shape has to
wander at a wavelength close to the size of the landmass itself, or the fine detail
just frets the edge of an obvious oval. It also has two inlets that pinch its waist,
and those are structural rather than decorative — ground here rises with its
distance from the coast, so a round continent grows exactly one round ice cap in the
middle of itself, and the first one did.

### The world is 12.3 × 15.3 km

Up from 8.2 × 4.3, with **45 km² of land** — room for the towns, the cities, and the
250 Copaimo the design calls for.

Everything pinned to the old world came along: the ranch, the canyon, the frame the
regions are laid on, and the distance over which land climbs away from its coast.
That last one is why the ranch still stands at **22.9 m**, the height Opificium's
terrain bench measured for it before any of this — scale the world but not the
relief and the same ground reads 28.3 m instead.

### A world map you can print

`dev/art/map/copaimo-world-map.pdf` — A3, 300 dpi, drawn at 4 m a pixel from the
same terrain the game builds, with every city, town, biome and the ranch on it.

### Fixed

- **An island that was a scale bar.** The source map is a screenshot and carries a
  scale bar, a help button and an install box. They sit in the margin the world
  drowns, so they never mattered — until the sheet grew, moved that margin inland,
  and 12,928 pixels of scale bar sailed into the sea off the east coast as a long
  thin island.
- **The desert walking off its own continent.** Regions were positioned as a
  fraction of the map image, so making the image taller moved them. They are pinned
  to the world now, and measured in km² and metres rather than as a share of the
  land — a share moves when a continent is added, and a desert that has not changed
  should not read as having shrunk.
