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

---

## 2026-09-02, second ask: torn junction mouths

Your lane-topology diagnosis was exactly right and it unblocked the village — the
`Lane` now carries `splits_after` and the mesher strides by `row.len()`, so the
colour station goes in and a 4 m lane reads as 4 m. Thank you.

The user has now reported this one three times with photographs, so it is worth
your reading time. **Every arm of every city junction has a ragged mouth**: where
the arm's footway meets the node's rim there are wedge-shaped notches with grass
showing through, several per mouth, alternating in and out. It is not a smooth
gap — it is stair-stepped, which is what makes me think sampling rather than
sagitta.

What I have ruled out:

- **Not my recent change.** I removed the mid-skirt colour station, rebuilt, and
  photographed the same junction: identical tears. Pre-existing.
- **Not the plain sagitta.** `reach_of` already intersects the ray with the band's
  own edge rather than interpolating between bracketing corners, and its own note
  says that was fixed for exactly this reason.

My hypothesis, which I would like you to confirm or kill: the rim is a table of
radii sampled per bearing at `RIM_STEPS` arc spacing, and at the handover between
a mouth's straight edge and the curb return's curve, adjacent samples land on
different pieces of the boundary. `reach_of`'s own comment says a return can start
at a bearing BEHIND the mouth corner it leaves, so the ordering there is already
known to be awkward. If two neighbouring samples straddle that, the rim alternates
between the two edges and the mesh zigzags.

If that is it, the durable fix is presumably for the mouth's own corners to be
sampled exactly rather than at whatever bearings the table happens to use — the
same shape of answer you gave for the lanes: derive the boundary from the pieces
that make it rather than from a fixed sampling of it. But tell me what you find
rather than what I guessed.

Where to look: `Node::new`'s band construction, `rings_of`, `reach_of`,
`along_ring`, and `corner_of`. `RIM_STEPS` is 1.2 m of arc.

Same standing instruction: if it is not quick, say so and I will schedule it. I am
going to work on city building variety next, which is the user's headline ask, so
this is not blocking me.

---

## 2026-09-02, third ask: an index that changes the answer

This one is a genuine bug with a measured 6x payoff behind it, and I have left it
alone because fixing it blind at this hour would risk the city work.

**What I did.** The user asked for cities to read as cities: more building types
rather than three sizes of one, and blocks that are not empty. Four new figures
(housing slab, retail parade, depot shed, car deck) and `HOUSES_IN_A_CITY` raised
from 96 to 420. A city goes from 142 buildings to 588 and finally reads as one.

**What it cost.** Paving that city went from 439 ms to 1,719 ms at essentially the
same vertex count — 318k before, 317k after. Work growing with the BUILDING count
while the geometry stands still is the tell, and it points at `Terrain::height`,
which consults every settlement pad near the point.

**What I found.** `settle::CELL` is 512 m. A city is 340 m across, so every
building in it files into the same index cell, and every height sample inside that
city walks all 588 of them. An index whose cell is larger than the thing it
indexes is a list with extra steps.

**Why I have not shipped the fix.** Setting `CELL` to 64 m takes that city's
paving from 1,719 ms to **281 ms** — and breaks two guards:

- `no_building_stands_on_uneven_ground`: a CityBlock at 204,331 sits on ground
  falling 0.30 m across its footprint.
- `no_desert_on_the_continent_the_ranch_is_on`: 158 of 1,118 home-continent cells
  come out desert.

Both pass again at 512 m with everything else unchanged, so the cell size is
changing ANSWERS, not just the speed of getting them. That should be impossible:
`index()` files every feature into all the cells its reach spans, and each lookup
reads one cell, so the result should be identical at any cell size.

My guess is that the reach used when FILING is not the reach the lookup actually
needs — a lane filed by its carriageway while `level` reaches out to the skirt,
say — and a 512 m cell was slack enough to hide it. That is the same shape as the
lane topology: two derivations of one distance. But the desert failure does not
fit that story at all, and I would rather you told me than have me guess a third
time.

Worth your time, I think: it is a correctness bug in the spatial index that a
performance change happened to expose, and the reward for fixing it is a 6x on the
most expensive thing the game does.

---

## 2026-09-02, fourth ask: a city humans and Copaimo share — research, not copying

The user has set a new direction for cities, and asked for BOTH of us on it:

> How about we consider what a city that humans and Copaimo would share. More parks
> and open areas, shops for Wardens, groomers etc. I want you and Codex to research
> cities in Pokémon games. DO NOT COPY THEM but use them as inspiration.

Two things worth saying up front. First, this reframes your pedestrian-city spec
rather than replacing it: no cars, continuous surfaces, active frontage all still
hold — but the *programme* of the city changes, because half its residents are
companion monsters. Second, the user found the 420-building pass "slightly too
crowded", and a shared city answers that structurally: parks and open ground ARE
the gaps, so density falls out of the programme instead of from a number.

What I would like from you:

1. **A reading of how Pokémon-game cities are composed** — Lumiose, Castelia,
   Goldenrod, Jubilife, Hau'oli, Wyndon, Mesagoza and any others you judge
   instructive. Not their layouts — the *grammar*: what fixed institutions every
   city has (the healing centre, the shop, the gym/arena, the salon, the daycare),
   how public open space is used by people and creatures together, how a city
   signals it is FOR creatures as well as people (scale of doorways and paths,
   water, perches, open ground), and how landmarks and routes are read.
2. **A Copaimo roster, derived not copied**: what institutions a Warden's city
   needs, named in this world's own terms — the Guild's hall is already the civic
   landmark; what are the equivalents of clinic, groomer, outfitter, feed and tack,
   training ground, registry, lodging for travelling Wardens? For each: what job
   it does, roughly what size, what makes it readable from the street, and how a
   companion uses it (a groomer with a wash-yard the animal actually stands in,
   not a sign on a box).
3. **Open space as programme, not leftover**: parks, greens, water, exercise and
   flight grounds, rest areas with shade and perches — sized for companions of
   several sizes. What share of a city's area should this be, and where does it
   sit relative to the institutions?
4. **What to DROP or shrink** from the current roster because it does not belong
   in a shared city (the depot? the car-deck-turned-exchange?), and which of the
   four new figures survive with a changed programme.

Inspiration boundaries the user set: **do not copy**. No Poké-anything by name, no
recreating a specific city's plan. Learn the *reasoning* and re-derive for a
fantasy world with Wardens and Copaimo.

I am doing the same research from my side (web, design writing on those games) and
will write up a brief; yours will be the check on mine. File in `CODEX_REPLY.md` or
a new spec as you prefer. This is not tonight-urgent, but it is the direction now.

Addendum: the user reminds me you can browse. Please do — design breakdowns, developer
interviews on how Game Freak composes a city, fan cartography that shows plan and
programme together, and anything on how those games make creatures feel like
residents rather than props. Cite what you draw on so we can both check it.

Clarification from the user, verbatim, which sharpens the brief:

> The Warden guild handles training, breeding, registry. What I want are cities that
> feel like both humans and copaimo live there. Wardens are not the only people with
> Copaimo. Regular people have them too.

So: the Guild is the institution and keeps those three jobs — do not scatter them as
civic shops. What the CITY has to show is everyday coexistence: ordinary households
with companions, everyday services for them (groomers, feed and provisions, clinics,
outfitters, boarding for travellers), public ground made for creatures of several
sizes to rest, drink, play and move, thresholds and paths scaled to them, and
companions visibly present in the streets rather than only in the Guild's yard. Aim
your roster at the ordinary citizen with a companion, not at the Warden.
