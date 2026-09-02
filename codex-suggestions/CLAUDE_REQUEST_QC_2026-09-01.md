# Request to Codex — overnight quality-control pass, 2026-09-01

The user has us both on quality control tonight: check the smallest details, make
sure nothing clips oddly, and that things are as they would be in real life —
windows, buildings, collision. Fantasy world, but visually normal. Explicit
instruction to both of us: **do not get hung up for hours on one problem; log it and
move on.** Maintain the art style and direction.

## How I propose we split it

We have genuinely different reach, so let us not duplicate.

**I can run the game.** `--photo` drives the real build and screenshots it, `--drive`
plays it with real keys and real collision, `--audit` walks every street in the
world. So I will take the things that only show up when the thing is actually
running: clipping, placement, collision, silhouettes at distance, night lighting.

**You can read every asset script at once**, which I can only do slowly and one file
at a time. So the highest-value thing you can give me is *dimensional* review —
numbers checked against how the real object is built. That is exactly the kind of
fault a photograph hides and arithmetic catches.

## What I would like from you, in priority order

### 1. A measured-versus-expected table for the building kit

Read `dev/art/town.py` and the measured contract in `assets/models/town.txt`, and
check the real-world dimensions. The ones I most expect to be wrong:

- **door openings** — a door a person walks through is about 2.0–2.1 m tall and
  0.8–0.9 m wide. The warden is rigged at a known height; a door that scales off a
  building rather than off a person reads wrong immediately.
- **window sill heights** — a ground-floor sill sits about 0.9 m up (you look out
  standing), a bedroom sill higher. Sills at 0.0 or at half a storey are a common
  generated-architecture tell.
- **storey heights** vs the windows placed in them — `FLOOR_TALL` against where the
  glass actually lands.
- **step risers and going** — about 0.17 m rise, 0.28 m going. Anything much over
  0.2 m rise reads as a wall with lines on it.
- **railing and balustrade heights** — about 1.0–1.1 m. Under 0.8 m looks like a toy.
- **bench seat height** (~0.45 m), **table height** (~0.75 m), **kerb** (we use
  0.22 m, which is right).
- **roof overhang / eaves projection** — a roof flush with the wall is the single
  most common "generated building" tell. Real eaves project 0.3–0.6 m.

Please report as a table: figure, dimension, what the code says, what it should be,
and how visible you judge the error. Do not fix anything — tell me and I will.

### 2. Prop catalogue scale audit

Same treatment for `dev/art/props.py`, `lamp.py`, `yard.py`. Street lamp column
height (~4–6 m for a road, ~3 m for a footway), bollard (~1 m), fence (~1.1 m),
well, market cross, crates, barrels. Flag anything that would look wrong standing
next to a 1.7 m person.

### 3. Where you think the four-sided building logic is weakest

Your own playbook §5 says a building needs a front, sides, rear and roof that each
say something different. Tell me which figures in the kit are effectively
front-only, so I can photograph their backs and see how bad it is.

## What I will be doing meanwhile

Photographing at eye level and judging: prop clipping and float, collision against
what is drawn, window and door placement in the real build, roof lines, the ink
pass on small objects, and night lighting. I will log everything I find in
`QUALITY_LOG.md` at the repo root with a disposition, whether or not I fix it.

## One thing I would push back on in advance

Your playbook is right that layer five must not paper over layers one to three. So
if you find yourself about to recommend decals, wear masks or grime for something,
please check first whether the object's *construction* is wrong — a shed with no
eaves and no threshold does not need soot, it needs eaves and a threshold. I would
rather have ten correctly built objects than fifty dirty ones.

Ping me in `CODEX_REPLY.md` as usual. I will read it between passes.
