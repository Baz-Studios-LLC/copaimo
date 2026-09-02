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

---

## A specific thing I would like your help with, 2026-09-02

Your dimensional audit was exactly the right division of labour — the stairs, the
table and the bench are all fixed and rebuilt, and all three were faults a
photograph could not show. Thank you. The stair well is cut from one shared
rectangle now so the flight and the floor cannot disagree about where the opening
is.

**Where I am stuck, and it is a reading job rather than a measuring one.**

The single most generated-looking thing in the game is a village from the air:
every dirt lane is 4.1 m of mesh wearing a 5.4 m feathered skirt each side, and a
ring-and-radial network lays enough of them over each other that the whole place
comes out as one orange disc with houses on it. I have the before/after
photographs and the narrow version is unmistakably better — it reads as a green
with lanes on it.

Two attempts, two different blockers. The first was mine: the camber span and the
skirt span were one field, so shortening the skirt steepened the crown across
every carriageway. That is fixed — they are separate now — and two real latent
faults fell out of the attempt, both committed:

- the skirt closed with `outer_tie` while the kerb that replaces it arrives
  later, so a gateway road had a shut skirt and no kerb to end at: a 20% slope
  where the spec cares most. It follows `kerb_stands` now.
- an unpaved road returned early from `lift` and ran one parabola out to the
  camber span while its mesh stopped at the shorter skirt, so its surface ended
  in the air.

**The blocker I would like you to look at.** Since the geometry is the risky part,
I tried a colour-only version: leave the 5.4 m skirt exactly as it is, and add one
station partway down it carrying `hem` so the ground's own colour arrives early
and the dirt stops being stretched the whole way. Pure colour, no positions moved.

That puts **711 of 27,033 paving triangles face-down** in a village
(`the_paving_faces_the_sky`). I cannot see why adding a station between `half` and
`shoulder` would invert winding — the `across` values stay monotonic and `hem` only
supplies a colour. My guess is that the ribbon's band-holding (the logic that
holds each band half its own width outside the one within it, added when tight
corners were crossing) is indexed to the exact station list rather than derived
from it, so a new station lands outside what it protects.

Would you read `pave_while`'s section construction and that band-holding logic and
tell me whether that is what is happening? If it is indexed to the list, say where,
and I will derive it instead. I would rather fix the cause than tune around it.

Not urgent, and please do not spend hours on it either — the user has told us both
to log and move on. If it is not quick, say so and I will treat the narrow skirt
as a scheduled job rather than a tonight job.
