# Troubleshooting

What went wrong, what it turned out to be, and how it was found. Arranged by
**symptom**, because that is how anybody arrives here: something looks wrong on
screen and the question is whether it has happened before.

`DESIGN.md` says what the game IS and why. This says what it did when it was
broken. When a bug listed here comes back, the entry names the test that was
supposed to stop it — start by running that.

---

## The shapes these bugs keep taking

Six patterns account for nearly every hard bug in this project so far. Recognising
the shape is usually faster than diagnosing the instance.

### 1. One question with two answers

The commonest and the most expensive. Two pieces of code each decide the same
thing their own way, both are individually reasonable, and they disagree over some
band of inputs.

* the ground colour blended by one strength while the region boundary used another
  → a hard seam across every biome edge
* the tunnel's faces were wound one way and their normals computed another → the
  whole mountain drawn inside out and invisible
* the terrain carved a mouth open by one rule while the cave mesh decided
  "is this open?" by comparing heights → both drew, a few centimetres apart, over a
  wide flat area → a floor striped in pale bands
* the tool palette held keys in one table and the action rows in another → `B` was
  the BIOME brush *and* BORE A TUNNEL, and one press did both

**The fix is never to make the two agree.** It is to delete one of them: pick the
authority and *derive* the other from it. Normals built out of a winding cannot
contradict it; a guard that asks the carve cannot disagree with the carve.

**Smell:** you find yourself writing "make sure X matches Y".

### 2. Written, but never registered

A system exists, is correct, is covered by a unit test — and is never added to a
schedule, so it never runs. Three times in one session: `leave_editor`, the pass's
roof, and the void mesh.

It is especially nasty because the unit test passes: the test builds its own little
app and registers the system itself.

**The fix:** where it matters, assert the wiring separately —
`leaving_the_tool_is_wired_up_and_not_merely_written` reads the source file and
checks the registration line is there. Blunt, and the only check that costs nothing.

**Smell:** "the code is right, so why is nothing happening?"

### 3. A proxy that stopped being true

A flag means one thing, everything guards on it, and then the flag's meaning
changes. Every guard is now subtly wrong and nothing fails loudly.

`CursorFree` meant "the maker is reaching for a panel", so `if free.0 { return }`
was a sound guard everywhere. When the cursor rule inverted, `free.0` came to mean
*ordinary use* — and every one of those guards silently became "never act".

**The fix:** name the question, not the state. `aiming_at_the_world()` says what the
guards actually mean, so a change of mechanism updates one function.

### 4. Tests that assert the old semantics

When a rule inverts, the tests are part of the change. Several tests here passed
before a fix, failed after it, and were *right to fail* — they encoded the previous
rule. Restating them is the work, not a chore after the work.

Worse variety: a test can be **stricter than the code can be**. The seam test
compared per-vertex when the mesh decides per-cell; the mouth probe used bilinear
`floor_at` where the mesh uses a per-cell flag. Both "failures" were the test
asking the wrong granularity.

**The habit that pays:** after fixing, put the bug back and confirm the test fails —
and check the *message* names the symptom. Every entry below that says
"proven by reintroducing" had this done.

### 5. Invisible by design

A sealed tunnel under a hill shows nothing on the surface. So "it did not work" and
"it worked and I cannot see it" look **identical**, and four rounds of screenshots
could not tell them apart.

**The fix is instruments, not more guessing:**

* a readout with a number in it (`Dug: 2953 cells`)
* the overview shading dug ground, because a passage is only ever visible from above
* an `info!` on the thing you doubt (`the cave: 2953 cells dug, 11968 vertices`)
* and above all: **read the saved data**. `assets/world/dug.bin` answered in one
  minute what screenshots had not in four rounds.

### 6. Granularity mismatches

Two systems sample the same field at different resolutions and disagree at the
boundary. Cell centres versus cell corners (twice); a bilinear read versus a
per-cell flag; a 2 m terrain grid versus a smooth analytic surface.

**The rule:** decide which lattice owns the answer, and sample everything on it.
When a mesh's *vertices* sit on corners, its rules must be evaluated on corners.

---

## Practices worth keeping

**Instrument the running program.** Reasoning found nothing for four builds; a
single `info!` line settled whether the void mesh existed at all. Compiling and
launching without a panic is not verification.

**Read the maker's own data.** Not a fixture — the real `assets/world/*.bin`. Two
tests here do that and skip themselves on a clean checkout.

**Prove the test discriminates.** Reintroduce the bug, watch the test fail, read
the message. A test that has never failed is a guess.

**Coincident surfaces z-fight.** Anything drawn at the same height as the terrain
will stripe. Hold it clear by a few centimetres, or better, do not draw it at all
where the terrain already does.

**The maker can see what the code cannot.** Where the desert belongs, where a
tunnel should run — these went wrong repeatedly while being guessed at from
screenshots, and went right immediately once they were painted or drawn. If a
constant is being tuned from a photograph, the tool is missing.

---

## Tunnels and caves

> **RESOLVED 2026-08-20, by removal.** After the walls, the carve partition, the
> doorframes, the terrain-skin holes and the crater, the maker called it: the whole
> tunnel system was scrapped and the mountain became a **canyon** — a flat-topped
> massif with one winding slot through it, in `world/pass.rs`. The entries below are
> kept as the record of WHY.
>
> **The root cause was never any one bug.** A heightfield has exactly one height at
> every (x, z). Anything under the ground therefore needs a parallel world: a second
> mesh, a second walking rule, a second camera rule, a carve to open the surface, a
> hole cut out of the terrain's own skin (a heightfield cannot represent one), and a
> built landmark to make the hole findable. Each of those was implemented, each
> worked, and the sum still read wrong on screen — five rounds of "still no
> entrance", each a different real fault. The lesson is not "we couldn't fix it";
> it is **choose gates the terrain representation is good at**. A canyon is the same
> gate — can't pass without finding the way, can't see through — built out of walls
> that go up, a floor that stays down, and sky overhead.
>
> If underground spaces ever come back to this game, they need a real answer to the
> heightfield question first (volumetric chunks, portal-stitched interior scenes, or
> holes as first-class mesh features), not another layer of patches.

The longest-running fault in the project while it lived. Reported as **"still not
going through the mountain"** four separate times, and each time the cause was
different — and the code named below is REMOVED; the entries stay because the
mistakes are portable even where the systems are not.

### The passage runs ALONG a hillside instead of into it

**Cause.** Whatever decides *where* to dig was reading the aim, and the aim is a ray
against the ground: **the crosshair can never point inside a mountain.** Sweep it at
a slope and it walks up the face, so the cutting only ever happens where the face
is. Three separate builds hit this — two clicks that computed a tunnel, a floor
re-read from the aim each frame, a head driven blind on a held level.

**Fix.** The route is DRAWN on the surface, where the crosshair works perfectly, and
*lowered* afterwards: the floor is graded between the ground heights at the two
drawn ends. Under the crest that is far below the surface (a tunnel); at each end it
meets the ground it started from (a mouth). Both fall out of one piece of
arithmetic. `editor::lower_the_route`.

**Test.** `a_route_drawn_over_a_mountain_becomes_a_tunnel_under_it` — draws foot to
foot over the pass, checks nothing is dug while drawing, real rock over the middle,
and both ends at the height they started from. Dig at the surface height instead and
it fails with *"only 2 m of rock over the middle — the route was not lowered"*.

### The entrances are covered over — a grey slab where a doorway should be

**Cause.** The cave emitted a wall wherever a *drawn* cell met a cell that was not
drawn — and a cell the carve has opened is **dug but not drawn**. So every mouth got
a wall built straight across it.

**Fix.** A wall holds back rock. Where the neighbour is *dug* there is no rock,
whether or not the cave bothers to draw it: the rule is about the digging, not the
drawing. `dug::void`, the `rock` test in the wall loop.

**Test.** `a_mouth_is_not_walled_shut` — every wall face must have undug ground
behind it. Put the old rule back and it reports *"32 wall faces stand against dug
ground … which is a doorway with a wall across it"*.

### The floor is ruled into a pale grid; the ceiling is full of ragged holes

**Cause.** Those pale lines are **the sky**. The floor and vault were a quad per
cell, each flat at its own cell's height — and neighbouring cells have slightly
different floors, so no two tiles met. Every cell boundary was a hairline gap.

**Fix.** Build both on the **corner lattice**, one vertex per corner reused by every
quad that touches it. `floor_at` is bilinear, so a corner has one answer however
many cells meet there, and quads share vertices by construction. Vertex count for
the same digging fell from 27,940 to 11,968 — that drop *is* the sharing.

**Test.** `the_floor_and_the_vault_have_no_seams_in_them`. Note it can no longer be
broken by a small edit: the corner lattice makes a duplicated vertex impossible to
write. Designing the class out beats catching it.

### A black ribbon with a sawtooth edge down the whole pass

**Cause.** Two surfaces at the same height. The rock's underside was computed to lie
exactly on the carved terrain — the same surface, twice — and they fought for the
depth buffer.

**Fix.** Hold it clear (`CLEAR_OF_THE_GROUND`), and better, do not draw at all where
the terrain already does.

### Grey terraces strip-mined across a field

**Cause.** The band where digging opens the surface was the full arch height —
**eight metres of cover**. On a mountainside that is a mouth; on rolling grass it is
everywhere, so one test drag opened a whole field.

**Fix.** `DOORWAY = 2.8 m`. Under head height of cover there is no tunnel to be in,
so it opens; anything deeper stays sealed and the vault handles the roof.

### You can see through the mountain into the slot

**Cause.** Both sheets of the rock were wound **backwards** and therefore
backface-culled. The mountain's skin was not drawn at all, so what showed was the
slot cut into the terrain; from inside there was no ceiling either. The normals were
right the whole time — nothing compared them to the winding.

**Fix.** Derive the normals FROM the winding (`settle_the_normals`). A normal built
out of a winding cannot contradict it.

**Note on the test.** The first attempt compared each face to the average at its own
corners — the check terrain-core makes on a tree — and that is the *wrong* check for
a folded sheet, which turns through more than a right angle from wall to crown. What
can be stated is which side each surface is seen from.

### It works but you cannot get in — the tool flies straight over it

**Cause.** The warden and the follow camera used the two-level walk rule; the
**fly camera** clamped to the drawn surface, which inside a passage is the sealed
hilltop. In the terrain tool, where flying is the only way anybody moves, the one
camera the maker actually uses was the one that could never descend into the cave.

**Fix.** The fly camera clamps to `walk_floor` too.

### The passage is a corridor — the camera clips into rock the whole way

**Cause.** 11 m wide by 6.5 m tall fits a walker and nothing else. The follow camera
sits back and above the warden.

**Fix.** 18 m by 10 m. `the_passage_is_wide_and_tall_enough_to_follow_somebody_through`
states it in those terms — including clearance over the warden's shoulders — so it
cannot be shrunk back without meeting the reason it grew.

### A hand-drawn line comes out as a row of beads, or a blob

**Cause.** Points arrive one per frame of a drag: a slow hand piles them up, a fast
one leaves them tens of metres apart, and the crosshair shakes by metres. Dug
straight, that is beads with kinks in it.

**Fix.** `dug::tidy` — resample to fill every gap, smooth to take the shake out,
**ends pinned** because those are the portals. A deliberate bend survives; a wobble
does not. Also: dig at the bore's own width whatever the brush is set to, or a
wobble merges into a hall.

**Test.** `a_gappy_wobbly_line_still_comes_out_a_continuous_tunnel` — draws with a
9 m shake and ten steps missing, then requires every point of the alignment to be
dug. Dug raw it fails with *"307 of 1297 points on the alignment were left undug"*.

### Nothing appears to happen at all

Check in this order:

1. **Is the shovel armed?** The DIG row lights gold when it is. With it down, the
   button paints with whatever palette brush is selected — SMOOTH, usually.
2. **Does the `Dug` readout climb?** A number is what tells a passage that was never
   dug from one dug where you cannot see it.
3. **Look at the world overview.** Dug ground is shaded dark there. A passage under a
   hill is invisible from the ground and visible only from above.
4. **Read the file.** `assets/world/dug.bin` — the decoder is four lines of Python
   and it answered in one minute what four rounds of screenshots had not.

## Controls and the tool panel

### A key does two things at once

**Symptom.** One press both picks up a brush and starts something else. Reported as
`B` being BIOME *and* BORE A TUNNEL.

**Cause.** Two tables of keys — `TOOL_KEYS` for the palette, `Act::key` for the
action rows — each correct alone, each printed faithfully on its own rows, and
nothing anywhere compared them to each other. The panel showed `B` twice without a
hint of trouble.

**Fix.** The bore moved to `T`, and `no_key_is_bound_to_two_things` checks the two
tables *against each other*. A second test checks every row prints the key that
actually does it, because a keycap nobody pressed is worse than none: it is believed.
Restore the old binding and the test says *"KeyB is bound to both the BIOME brush and
BORE A TUNNEL"*.

### The panel rows do nothing when clicked

**Symptom.** "This is still not clickable menus." Pressing PLACE, PICK UP, TURN or
TAKE AWAY has no effect at all.

**Cause.** A row can only be *clicked* while ALT holds the pointer free — that is
what ALT is for — and `place_things` guarded **both** its doors with "the pointer is
not out reaching for a row". So the row press arrived and was thrown away on the very
frame it could only have arrived on.

**Fix.** Two doors, two guards. The keyboard is gated on the pointer being on the
world; a row press means the act outright.
`a_row_press_acts_even_though_the_pointer_is_off_the_world`.

**Related.** The rows had no *state* indicator either: DIG is a toggle and nothing lit
up, so a maker with the shovel down would press the button and paint with whatever
palette brush was still selected. `mark_the_shovel` lights it.

### Mouse-look stops working in the game for the rest of the session

**Cause.** `CursorFree` is written only inside the terrain tool, and the game camera
reads it too. Leaving the tool with ALT held left it set.

**Fix.** `leave_editor` on `OnExit(Editing)`. A resource one state writes and two
states read has to be put back by the state that writes it. See also shape #2 — this
system was written and not registered on the first attempt.

### The brush ring and the cursor are in two different places

**Cause.** Two pointers. `apply_cursor` was registered for `OnEnter(Playing)`
**twice** and never for `OnEnter(Editing)`, so entering the tool inherited the menu's
free, visible cursor: the OS arrow wandered the screen while the brush aimed down the
view centre.

### The panel is cut off at the bottom and those rows cannot be reached

**Cause.** Bevy *applies* `ScrollPosition` and clips by `Overflow::scroll_y()`, but
ships **nothing that sets one** — there is no wheel-to-scroll system in the engine.
The bench panel had the overflow and no driver; the editor panel had no height bound
at all and grew past the window.

**Fix.** One `scroll_panels` system for anything marked `Scrolls`, the editor panel
pinned to the window height, and the other wheel consumers yield while the pointer is
over a panel.

### Sliders and the minimap click in the wrong place on a scaled display

**Cause.** `Window::cursor_position()` is **logical** pixels; `ComputedNode::size()`
and UI `GlobalTransform` are **physical**. Compared raw they agree only at 100%
display scale — everything was off by the scale factor at 125% or 150%.

**Fix.** Convert with `inverse_scale_factor()` before comparing.

---

## Terrain, biomes and mountains

### Hard seams or choppy bands between biomes

**Cause.** Three faults stacked, each invisible on its own:

1. the boundary speckle read the **nearest cell**, so the nudge jumped at every
   six-metre cell edge and the whole transition rendered as a checkerboard
2. the painted handover flipped category at `TAKES_HOLD` with the painted side still
   carrying half its strength and the natural side picking up at full — a cliff
3. the travelling snowline swept a **fifty-metre** smoothstep window through eleven
   hundred metres of height, so the white cap switched on like a shutter inside an
   otherwise gentle fade

**Fix.** Interpolate the speckle; make both sides reach nought at the handover; blend
the cap *amount* rather than sweeping the window.

**Test.** Transects that walk the boundary at half-metre steps and bound the biggest
colour change per step: 0.805 before, under 0.06 after.

### A green outline around a stroke painted over its own country

**Cause.** A stroke's strength is how much of the neighbourhood voted for it, so at
its rim it is about a half — weak enough that the dither downstream turned it into
grassland. A boundary was being drawn between a country and itself.

**Fix.** Where the stroke agrees with the ground it is laid on, the two claims are the
same claim and the stronger stands.

### The mountain reads as a very long smooth hill, or as a flat-topped table

**Causes, in order of discovery.** The crest was under the treeline so everything was
forested (raise it); the profile was two eased falloffs, which is a berm at any size
(serrate the crest, crease the flanks, mid-flank only); and a shoulder in **both**
directions gives a mesa — across its thickness a ridge should simply peak.

### A wall of rock crosses a biome boundary at an angle

**Cause.** The wall ran due north–south while the region boundary runs on a diagonal
(the region axis is tilted and the world is half as deep as it is wide). One end of
the wall had the wrong country on the wrong flank.

**Fix.** Take the heading from the region's own lean rather than picking it.

### The ground reads as one flat solid colour

**Fix.** Mottle the vertex colours at two scales. Nothing here is textured — the
colour lives in the mesh, so the variation has to as well.

---

## Rendering and performance

### Faint straight lines or streaks all over the ground, worst at night

**Cause.** Shadow acne at grazing light. What a shadow bias must cover grows with the
**cotangent** of the light's elevation, so a bias that is generous at noon runs out
near the horizon and the ground self-shadows in shadow-map texel rows — long thin
parallel streaks along the light azimuth. It showed at night because the moon spends
hours at angles the sun crosses in minutes.

**Fix.** Grow the normal bias as the light drops; park shadows below a floor
elevation; fade the moon toward the horizon so the parking never shows.

**Note.** The old comments claimed the biases were clip-space. They are not: depth
bias is world-space metres and normal bias is texel-scaled, so it also changes meaning
when `SHADOW_DISTANCE` changes.

### Dark blotches drifting across the sea

**Cause.** Cloud shadows applied to water. Physically real, and it reads as stains:
open water is a broad flat blue with nothing for a soft disc to sit on, and half the
view from any coast is sea.

**Fix.** The sea is the one surface that goes without.

### The frame is mostly shadow passes

**Cause.** Every tree in the ~254-chunk streamed disc was re-submitted to all three
cascades every frame — 16.7 ms of a 23.8 ms frame.

**Fix.** Trees stop casting beyond a two-chunk ring, with chunk-level bookkeeping so a
chunk's trees are only walked when it *crosses* the ring.

### Steady frame cost with nothing happening

Three found together: the sea's 26k vertices were rewritten on the CPU and fully
re-uploaded **every frame** (moved into the vertex shader); barren chunks recorded
*nothing* when the answer was "nothing grows here", so they were re-dressed forever;
and the hidden F3 overlay kept sampling the terrain and re-laying-out its text.

### B0004 warnings about inherited visibility

**Cause.** Chunks were spawned with a `Transform` and no `Visibility`, while four
kinds of child hang off them.

---

## World data and saves

### Continue starts you at the ranch, and then the save is overwritten

**Cause.** `spawn_player` ran at `Startup`, before the menu had read the save — so
`Progress::from` was always empty, Continue applied nothing, and the thirty-second
autosave wrote the ranch over the real save. New Game mid-session had the mirror
fault: the old warden stayed put and the "fresh" save inherited the position.

**Fix.** Spawn on `OnEnter(Playing)`; a second entry moves the standing warden rather
than doubling them. Pinned by a test that runs the whole flow.

### The whole save is lost after one thing went wrong

**Cause.** `{:.3}` formats a non-finite float as the literal word `NaN`, which is not
JSON — so one bad frame wrote a file the reader threw away entirely.

**Fix.** Refuse to write a non-finite position, and validate on read. Layer files
likewise refuse a non-finite payload under an intact header.

### A format writes something its own reader refuses

**Cause.** `Form::word()` wrote `cut:{low},{high}` while the reader wanted `<a>x<b>` —
latent only because no part emitted a Cut or Hip yet.

**Fix.** Fix the writer, and round-trip **every** variant in a test whether or not
anything emits it today.

### Tests start failing after the maker saves their work

**Cause.** Tests asserting against an *empty* layer. `assets/world/*.bin` carries the
maker's own painting and digging, so those tests pass or fail depending on somebody's
save.

**Fix.** Measure the change the test itself made, not the absolute state.

---

## Build and release

### The release build does not compile, and only the release build

**Cause.** Naming a tools-only item from always-compiled code — `sky.rs` used
`AppState::Bench` in a `run_if` with no `cfg`. Tests build with default features, so
nothing catches it.

**Fix.** `cargo check --no-default-features` before any tag, and a `cfg`-aware
predicate function rather than naming the variant inline.

### A release step reports success while doing nothing

Two of these. `ls a b | head -1` under `set -eo pipefail`: `ls` fails on the missing
path, SIGPIPE, and the step exits 1 having printed nothing. And `set -e` is
**suppressed inside an `if` condition**, so a guard written to stop a silent failure
became one.

**Fix.** Test for each name rather than globbing both, `|| return 1` per step, and
prove a guard works by pointing it at a deliberately broken binary.

### `Access is denied (os error 5)` while building

The game is running and holding the exe. Close it.

---

## The workbench

### The bench shows the world behind it, or a grid with colours in it

**Cause.** The sea was spawned at startup and never taken away — an eight-kilometre
plane at y=0 with the bench floor a centimetre under it. That is z-fighting, and
pieces standing in it went blue because they were *underwater*.

**Fix.** The world's content arrives with the world and leaves with it.

### The gizmo arrows cannot be clicked

Five rounds on this one, and every cause was real: `place` ran first and dropped a
prop instead; pointing at an arrow moved the ground cursor off the piece, which
dropped the selection, so the handles vanished on the way to them; there was no hover
or grab highlight, so working and dead looked identical; and the drag measured against
the arrows' *current* position, which moves with the piece — so it oscillated instead
of sliding.

**The lesson that mattered.** Instrumenting the running program found in one step what
four rounds of reasoning had missed.

---

## Keeping this honest

Add an entry when a bug took **more than one attempt** to fix, or when the symptom
and the cause were far apart. Those are the ones that come back, and the ones where
a future reader saves real time.

An entry is worth writing when it can say all four of these:

* what you **see** — in the words it was reported in, if it was reported
* what it actually **was** — the mechanism, specifically
* what **changed**, and the principle behind it
* the **test** that now guards it, and what its failure message says

If there is no test, say so. An entry with "none" in that slot is a standing
invitation to add one.

Every symbol named here was checked to still exist when it was written. If one has
moved, the entry is stale — fix it rather than leaving it, because a troubleshooting
doc that names things that are gone is worse than no doc: it sends the next reader
looking for code that was renamed for a reason.
