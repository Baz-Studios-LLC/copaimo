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
pub const WORLD_WIDTH: f32 = 8192.0;
```

Everything else — chunk counts, world bounds, the coastline — is derived from it.
At the ranger's 7 m/s jog, 8192 m is about 20 minutes east to west.

## Later

The same pipeline can take a second image for *regions*: a map where each
political area is a flat unique color becomes a lookup for "which nation is this
point in", which is how cities, guild territory and region-specific monsters get
placed without hand-entering coordinates.
