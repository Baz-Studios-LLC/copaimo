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

### The packaged build opens a world nobody made

**Symptom.** The game runs, the window is fine, and the LAND IS WRONG — no
sculpting, no painted countries, nothing placed. Only in a packaged build, and on
macOS in particular; never from the repository.

**Cause.** The world's layers are read with plain `std::fs`, so
`"assets/world/edits.bin"` resolves against the **working directory**. From the
repository that is the crate root and everything is found. macOS launches a `.app`
from `/`, which is also how the studio launcher starts it — and from there every
one of those paths misses. Nothing errors: the heightmap logs a warning and falls
back to procedural, and each layer file "not existing" is the ordinary case for a
world nobody has painted, so all of them load empty. Shape #5 — invisible by
design. The failure looks exactly like a fresh world.

**Fix.** `crate::asset_file` — working directory if it has an `assets` folder,
otherwise beside the binary. `asset_root` (Bevy's own server) and `wear_the_icon`
each already carried their own copy of this rule; the hand-read files were the set
that did not, so now all three answer it the same way.

**How it was confirmed.** Staged a bundle (binary + `assets/`) and ran it from `/`.
Before: the relative names do not resolve from there at all. After: the log names
absolute paths beside the binary and reports the real map (2478×1290, 36% land),
8642 sculpted cells and 6042 painted ones. Run the packaged build from a foreign
working directory — never only from the repo — or this class of fault cannot show
up at all.

**The decision is testable on purpose.** `which_asset_file` takes "does the cwd
have assets" and "where is the binary" as arguments, because both are process-wide
and a test cannot change either without disturbing every test beside it.

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

## Models from Blender

### A model arrives on its back, or backwards, or filling the sky

**Symptom.** The GLB loads with no error and looks wrong: lying down, walking
backwards, a hundred times too big, half-buried, or hovering.

**Cause.** Blender and Bevy disagree about which way is up and which way is
forward, and the glTF exporter's own conversion only fixes the first one.

* **Up.** Blender is Z-up, glTF and Bevy are Y-up. `export_yup=True` handles it.
* **Forward.** This is the one nobody guesses. Blender's own *front* view looks
  down **-Y**, so modelling toward -Y feels correct — and the Y-up conversion maps
  Blender -Y onto **+Z**, while Bevy's forward is **-Z**. So the axis that feels
  right exports backwards. **Build facing +Y in Blender.** Verified both ways
  empirically rather than reasoned about: -Y gave +Z, +Y gave -Z.
* **Scale.** A figure authored in centimetres is a hundred times too big, and
  nothing objects.
* **Origin.** Feet on Z=0 in Blender, or the model imports sunk or floating —
  placing it at a terrain height puts its ORIGIN there.

**Fix.** Export through `dev/model_export.sh`, which sets the options once and
refuses a model that breaks the rules at the moment it is made. `models.rs` asks
the same questions of whatever is in `assets/models/` under `cargo test`, so a
model dropped in by hand is caught too.

**Two gates, one set of numbers.** The bounds live in both files, and
`the_two_gates_agree_about_what_they_allow` reads the Python from the Rust test
and fails if they have drifted — this project's most frequent bug shape is one
question with two answers.

**And bound it tightly enough to catch the thing it is for.** The size cap was
first written at 200 m. A deliberately broken fixture — a 1.8 m figure authored in
centimetres, so 180 m tall — passed it. A bound that admits the exact mistake it
exists to stop is decoration. It is 60 m now, and there is a test with the 180 m
figure in it.

It was 90 for a while, widened to admit an 80.5 m guild hall built to out-top a
city's spire. That hall was replaced by a 12.7 m one and the bound stayed at 90 —
which is the failure mode worth naming: **a guardrail widened for one asset does not
narrow again when the asset goes.** It sits there describing a world that has been
deleted, and the only thing that noticed was a review reading this file against the
code. The tallest thing the game ships is the spire at 57.1 m, so 60 has its
headroom back.

**A known limit, stated.** The Z-up diagnosis is a *hint* layered on the
base-on-the-floor rule, not a general detector: it fires when the model is sunk
below the floor and is also wider and deeper than it is tall. A Z-up export that
happens to sit entirely above Y=0 is caught as "floating" instead, which is the
right refusal for the wrong reason. "Tallest axis must be Y" cannot be the rule —
a fence rail and a carpet are legitimately not tallest in Y.

### A Blender script fails and the build says everything is fine

**Symptom.** `dev/art/build.sh` reports no error and then the exporter says "no
.blend files found".

**Cause.** **Blender exits 0 even when the script it ran died on a traceback.** The
traceback goes to stderr and the exit status says success, so a shell with
`set -e` sails straight past it.

**Fix.** `blender --background --python-exit-code 1 --python script.py`. Without
that flag every generator in this project can fail silently.

### A headless glTF load never finishes

**Symptom.** A test that loads a `.glb` through `AssetServer` sits at
`LoadState::Loading` forever and the test times out or asserts.

**Cause, as far as it was chased.** A hand-assembled plugin set — `TaskPoolPlugin`,
`AssetPlugin`, `ScenePlugin`, `GltfPlugin`, assets registered by hand — does not
complete a glTF load. It is not a timing problem: it was given several seconds of
real `thread::sleep` between updates and every file stayed `Loading`. Tight
`app.update()` loops with no sleep are *also* wrong for this (six hundred of them
finish in forty milliseconds), so check that first, but sleeping does not fix it.

**What was done instead.** The loading is Bevy's and is left to Bevy. What is
tested here is the part written here — see
`a_species_claims_its_own_varieties_and_replaces_what_they_draw` — plus the file
contract, read straight out of the GLB's own JSON: two meshes, named `wood` and
`leaves`, one primitive each.

**Do not spend another afternoon on it** without a reason to think the plugin set
is the problem.

### An authored model replaces nothing, and the world looks normal

**Cause.** Swapping the HANDLE in a pool instead of the ASSET the handle points
at. Everything planted before the swap keeps its old handle, and for a chunk that
has already streamed in, nothing ever replants it — so the new shape only appears
in places nobody has been yet.

**Fix.** `Assets::insert(&existing_handle, new_mesh)`, which changes what every
instance draws at once. Pinned by a test that reads back through the handle a
planted tree would be holding, and confirmed by mutating the code to do it the
wrong way and watching the test fail.

## Litter, cover and steep ground

### Rocks and dead sticks poke sideways out of a cliff face

**Symptom.** The canyon walls are studded with scree, boulders and snags standing
out of near-vertical rock.

**Cause.** Steep ground reads as `Biome::Rock`, which carries the MOST litter of
any country and the right kinds for a mountainside — and a seventy-degree wall is
steep ground. Ground cover never had this problem because the biome does the work
for cover: nothing grows on sheer rock. Litter is the opposite case, so nothing was
stopping it.

**Fix.** `LIES_UPTO` in `prop.rs`: litter stops where a WALKER stops. Taken from
`player::CLIMB_LIMIT` rather than picked, and
`litter_lies_where_a_walker_could_stand` checks the arithmetic between the two so
they cannot drift.

### The world feels too busy, and one biome worse than the rest

**Cause.** `prop::density` says how much litter a biome carries and `belongs` says
which kinds may stand in it — and between them they cannot say "plenty of cactus
but few boulders", because a biome's kinds are picked EVENLY. Three kinds at 0.30
density means a third of the desert is boulders.

**Fix.** `keeps(biome, kind)` in `prop.rs`, thinning by kind in the game rather
than in the shared crate, where the density and the belonging are other worlds'
business too. Measured per chunk before and after, per kind, so a cut aimed at
boulders can be seen not to have taken the cacti with it.

---

## Steep terrain in a heightfield

### A fine comb along the top and bottom edge of a wall

**Symptom.** The rim of a sheer wall is a row of vertical teeth, and the faces
below them are streaked.

**Cause, and it is arithmetic rather than a bug.** The rim's position is displaced
by noise. Moving the rim sideways by a metre moves the ground up or down by the
WALL'S OWN GRADIENT — about 4.6 m per metre for a seventy-degree wall — so a rim
that wanders half a metre between two vertices steps the ground by two. Vertices
are two metres apart. The old fine octave wandered nearly two metres per metre,
which is seventeen metres of step between neighbours.

**Fix.** A wander of A metres over L needs **L greater than about 26·A** to keep a
step under a metre. `JAG_BROAD` went 22 m over 90 to 7 over 380; `JAG_FINE` 7 over
24 to 1.5 over 130; the slot's chip 8 over 50 to 3.5 over 150.

**The trade, stated.** Sheer, jagged, and a heightfield: pick two. The walls stay
sheer, so the rim line has to be gentle, and the canyon's shape comes from the way
through winding two hundred metres side to side instead. This is the same family as
the tunnel: the heightfield has limits, and arguing with them costs days.

### Smoothing a wall shortens it

Gentling the rim's wander cost the massif about twenty metres at each end — the old
wander had been pushing the rim outward there, and the gate test caught it (a
straight crossing climbing 101 m against a threshold of 102). `WALL_LONG` was
lengthened to give it back. Worth expecting whenever a rim's roughness is reduced.

### A measurement reports the same number after the fix

**Symptom.** A probe said 41.5 m of step before a roughness fix and 40.7 m after,
which reads as "the fix did nothing".

**Cause.** The probe was wrong. It walked a fixed offset from a FIXED centreline
while the canyon's real centreline swings up to nearly three metres for every metre
travelled — so it was cutting ACROSS the wall, and a wall is meant to change height
when you cross it. The measurement was dominated by the thing it was supposed to
hold constant.

**Fix.** Follow the wander. Corrected, the same probe reported the average step
falling from 3.01 m to 1.60 m high on the wall and 2.53 m to 0.70 m low on it —
which is the comb going away. **When a fix changes nothing, suspect the ruler**
before suspecting the fix.

## Towns, roads and levelled ground

### A raised section the brush cannot fully smooth out

**Symptom.** A lip or raised shelf in the ground. The smooth brush takes some of it
and never all of it, however many passes.

**Cause.** `Settlements::level` returned the strongest claim's TARGET as well as its
strength. Where two claims cross — a road running into a town, two roads meeting —
the pulls are equal and the targets are not, so the winning height snapped from one
to the other between two vertices while the pull carried on smoothly. Measured at
**8.6 m of step between neighbours two metres apart**: a pull of 0.47 times about
eighteen metres of disagreement.

The brush could not fix it for two reasons at once. The sculpt layer is **four-metre
cells**, so it cannot express the inverse of a step that sharp; and the generator
re-applies the step underneath every frame regardless of what is painted over it.

**Fix.** The strongest claim decides HOW MUCH; all of them decide WHAT. The height
is blended across every claim weighted by the **cube** of each pull, and only the
strength is the strongest claim. Cubed because the original note was right that a
road meeting a town should join the town's level rather than splitting the
difference — at any real distance the dominant claim is overwhelming, and the blend
only shows in the narrow band where two pulls are comparable, which is exactly where
a step must not be. Levelling roughness at the reported spot went 8.58 m → 0.22 m.

**Third time for this shape.** The biome boundary did it — the category flipped at
the threshold while the strength carried on — and so did the painted country. **A
thing that flips cannot be the thing that varies.**

**Why no test caught it.** Every road and town test passed throughout. They ask
about gradients ALONG a road and about the width of its cutting — real questions,
none of which walks across the seam BETWEEN two features, which is the only place
this shows.

### Measuring it: two mistakes worth not repeating

**Measure what the layer ADDS, not what the ground ends up as.** The first version
of the regression test bounded the total height step near a town, and failed on a
mountainside 240 m away where no settlement had any claim at all — ordinary terrain
is allowed to be a cliff. What must not have a step in it is `(target - dry) * pull`,
which is zero where nothing claims the ground and cannot be confounded by whatever
the ground was doing already.

**Sample finer than the thing you are trying to rule out.** At the terrain's own
two-metre spacing a step and a steep ramp look identical, and a road cut into a
hillside legitimately grades two metres over two. At a quarter of a metre a ramp
shrinks in proportion and a discontinuity does not. The same measurement reads
0.26 m now and 4.45 m with the old code put back.

### A generated character's hands drag the trousers about

**Symptom.** A ribbon of surface stretches from the glove to the top of the thigh
pocket whenever a limb moves, as if the fingers were stitched to the cloth.

**Cause.** **Reciprocal cross-limb weight bleed between two shells that touch at
bind pose.** The generator parks the hands ON the pockets — glove and trouser come
within 0.003 of touching — and its radius-based auto-skin reached across the gap in
BOTH directions: trouser vertices picked up `*_Hand` weight (up to 0.728) and glove
vertices picked up `*_ThighTwist01/02`. Around 480 vertices ended up roughly half
hand and half thigh, so both surfaces swim to the average of two diverging
transforms.

**Fix.** `unfuse_the_gloves_from_the_pockets` in `dev/art/animate_ranger.py`: weld
vertices by position into pieces, let each piece choose ONE limb chain by majority
weight, delete any weight naming the other chain, renormalise. It only removes
weights the generator should not have authored — nothing is invented — and it is
seam-free because a piece of cloth has no interior boundary to leave a
discontinuity at. 562 weights off 481 vertices; walk's worst edge growth 0.0884 ->
0.0323, edges over 2x 11 -> 0.

**Four wrong turns worth recording, because each looked reasonable:**

1. **"Strip the arm share from every vertex holding both."** Measured: **sixteen
   times WORSE** (triangle stretch 2.64x -> 42.77x). It strips arm weight from
   GLOVE vertices too, so a glove corner rides the thigh outright. Partial and
   thresholded versions are also worse than nothing, because they leave a bigger
   weight discontinuity at the boundary. **The repair has to be all-or-nothing per
   piece.**
2. **"There are no hand-to-leg edges, so there is nothing to cut."** Testing the
   wrong thing. Dominance FLIPPING across an edge was never the requirement — a
   0.50/0.50 vertex beside a 0.79/0.21 vertex is already a 4x tear with no flip at
   all.
3. **"The mesh is 432 disconnected fragments, so weighting cannot reach it."** A
   measurement artifact of counting connected components in INDEX space. Weld
   coincident positions and it is **19 clean pieces**. This one wasted the most
   time, and it was my own number.
4. **"Re-skin by proximity to the nearest bone."** Reproduces the original bug
   exactly: the `R_Hand` bone segment runs THROUGH the pocket volume — pocket vertex
   v887 sits 0.0204 from the hand bone and 0.0662 from the thigh, so the wrong limb
   is 3.2x nearer. **Proximity decides how much; piece identity decides whose.**

**And the filter that found nothing was looking for the wrong words.** An early
repair searched vertex-group names for `"arm"` and `"leg"`. This rig's hands are
`L_Hand`/`R_Hand` — no `"arm"` in them — and **no bone in the rig has `"leg"` in its
name at all**. It found nothing where the fault actually was and reported that there
was nothing to fix.

### Rendering one side and calling it verified

The fault was worse on the RIGHT hand and every camera in the render helper had a
positive azimuth, which on this model only ever shows the LEFT. Mirrored views are
in `CAMS` now (`tqfront_r`, `side_r`). **If a model is symmetric, the render set has
to be too.**

### Ranking stretch by ratio finds phantoms

A 0.0017-unit edge at 15x is invisible; a 0.09-unit edge at 1.5x is the fault.
`dev/art/ribbon_measure.py` ranks by absolute growth AND ratio, over every frame of
a clip, with each edge attributed to its welded piece — which is how the remaining
offenders were identified as the jacket hem rather than the hands.

### Movement is far too fast, and one clip never plays at all

**Symptom.** "Movement is extremely fast." Also: judgements about how the walk looks
that never actually described the walk.

**Cause — three numbers, all wrong together.**

* `WALK_SPEED` was **7.0 m/s** and `SPRINT_SPEED` **15.0** on a figure 1.7 m tall. A
  real walk is 1.4 m/s. Seven is a 2:23 kilometre — faster than the world record —
  and fifteen beats Usain Bolt's peak.
* `BREAKS_INTO_A_RUN` was **6.5**, which is BELOW the walking speed. So every step
  the warden had ever taken played the RUN clip. The walk clip sat in the file
  unused, and every opinion formed about "the walk" was formed about the run played
  at five cycles a second.
* `STRIDE_COVERS` and `RUN_COVERS` — how far a cycle carries the warden, which the
  playback rate is divided by — were estimated as `2 * leg * sin(stride angle)`.

**Fix.** 1.8 and 3.6 m/s, threshold 3.0 between them, and the coverage MEASURED
rather than derived: `dev/art/stride_measure.py` poses the real rig over the real
clip and measures how far a foot travels front-to-back relative to the hips. 0.451
units per foot walking, 0.478 running.

**And a factor of two, which the test caught.** A cycle is both feet taking one step,
so the body advances by TWICE one foot's swing — 1.53 m walking, not 0.77. Getting
that wrong is the difference between a believable cadence and a blur.

**Two tests now hold the set together**, because these numbers are only correct
relative to each other: the gait threshold must lie strictly between the two speeds,
and each clip must play between 0.6 and 2.5 cycles a second at its speed.

**A consequence worth stating.** Believable speeds mean the 8 km map takes about
thirty-seven minutes to cross at a run. That is a design question — mounts, roads,
fast travel — not a bug to tune away by making the warden superhuman again.

### "Why can I not see the character in Blender?"

Because none of the authoring happens in the open Blender window. Every asset step is
BATCH Blender: a headless process that imports, does one job, exports and exits.
Nothing it does touches a running instance, so the open window keeps showing whatever
was last put in it.

**What was actually in the window** was the wreckage of one failed import: an
`Armature`, an `Icosphere` — the sphere the glTF importer makes to draw bones with —
and no character mesh between them. It looked like an empty rig because it *was* an
empty rig.

**And a correction worth keeping, because it was written into this file as a rule.**
This section previously said the live session *cannot* import a GLB: that the add-on
runs code in a context without `bpy.context.object`, which the importer needs while
setting up armature display. That failure was real, but it was a STATE, not a
property. Clearing the scene's objects and importing again worked first try — mesh,
rig and all three clips. The original failure came from a script that began with
`read_homefile(use_empty=True)`, which leaves no active object for the importer to
reach for.

The lesson is the one this file keeps relearning from the other direction: **one
failure is a data point, not a rule.** Promoting it to a rule cost more than the bug
did, because a written-down impossibility stops anyone trying again.

**And then it happened again, with this entry already written.** A new viewer
(`gait_watch.py`) was written starting with `read_homefile(use_empty=True)`, hit exactly
the failure described above — `armature_display` dying on
`bpy.data.collections[...].objects.link(bpy.context.object)` — and opened an empty
window, reported as "there is nothing in blender that I can see to verify". The entry
was correct and simply had not been read. Two things follow. Check this file for the
step you are about to write, not only for the bug you already have. And prefer the
shape that cannot fail: build the scene in `--background`, save a .blend, and open
that — which is what `ranger_blend.sh` and `gait_watch.sh` both now do, each verifying
the saved file before handing it over.

**Two ways in, then.**

* `dev/art/ranger_blend.sh` writes `dev/art/ranger.blend` — the model, the rig and all
  three clips, textures packed, the walk loaded and the frame range set — built from
  the game's own copy, so what opens is exactly what the game loads. Use this to
  KEEP a scene.
* The live session imports fine. Use it to LOOK at one, and to find numbers: it is
  how the eye boxes in `dev/art/ranger_texture.py` were measured. Clear the scene
  first, and put a camera and a light back afterwards — `--look` renders through the
  scene camera and reports "the scene has no camera" if the clear took it.

### A clip plays too fast, and the cadence test passes anyway

**Symptom.** The feet skate. The warden's ground speed is believable, the stride
distance is measured, the cadence test passes — and the run still looks sped up.

**Cause.** `set_speed` is a **multiple of a clip's natural rate**, not a rate. A
clip's natural rate is one cycle over its authored duration, so cycles a second is
`set_speed / duration`. `motion.rs` handed it `speed / covers`, which is already
cycles a second — correct only for a clip lasting exactly one second.

The walk lasts **1.042 s**, near enough to one that nothing looked wrong. The run is
authored over sixteen frames rather than twenty-four, lasts **0.708 s**, and therefore
played at **3.16 cycles a second where 2.24 was wanted — 41% too fast**.

**Why no test saw it.** The cadence test asserted `speed / covers` and passed
throughout, because that is the number the code was ASKING for. A test that checks the
request rather than the result is a test that agrees with the code about something
they are both wrong about. This is the second time that shape of mistake has appeared
in this file; the first was a probe that walked a fixed offset down a centreline that
swings, and reported "no change" for a fix that worked. **When a fix changes nothing,
or a test never fails, suspect the ruler.**

**Fix.** `playback_rate(speed, covers, clip_lasts)` includes the duration, and
`Motions` carries both clips' lengths, read off the clips as they load.
`models.rs::inspect` now reports each animation's length straight out of the header —
glTF requires an animation sampler's input accessor to carry `min` and `max`, so no
buffer decoding is needed.

**And the test that can actually catch it, which is the point.** Not a value — a
PROPERTY: *the cadence must come out the same whatever the clip's length.* Any check
of the number `speed / covers` produces will pass while being wrong by exactly the
duration, so the only way to see the fault is to vary the duration and demand the
answer stay put. Verified by reintroducing the bug: it fails, naming 3.16 cycles a
second.

Plus one that a clip is authored somewhere near the time its stride takes, since the
fix removes the pressure to keep the authored length sensible at all.

### A walk that limps, while every direction measures correct

**Symptom.** "The legs and arms are not moving correctly, they feel backwards even
though they are facing the correct way now." Every sign checks out - opposition, knee
lead, elbow trail, heel strike - and it still reads wrong.

**Cause.** The two halves of the cycle did not match. Measured on the hips: one half
bobbed **4.57 cm** and the other **2.95**, peaking **ten** frames apart where half a
cycle is twelve. A cycle is two steps and the second is the first with the legs
swapped, so a mismatch is a LIMP, and a limp is felt long before it is seen.

**What did it.** The pelvis yaw and obliquity, applied as rotations of `Pelvis` -
which carries both thighs. Zeroing them made the halves exact, which is what named
them; halving them was not enough.

**Why it could never work there.** The rig is not mirror-symmetric. `L_Thigh`'s local
X runs (-0.007, -0.999, -0.044) against `R_Thigh`'s (+0.007, -0.992, +0.125). One
shared rotation on their parent cannot move the two legs alike, and whatever it does
to the STANCE leg's length is a change the foot-planting feeds straight into the hips.

**Fix.** State the pelvis's rotations on the LEGS. Yaw becomes extra reach for the
swinging thigh and less for the stance one; obliquity becomes adduction of the
swinging leg. Each side is then independent, the stance leg's vertical extent stays a
pure function of its own three angles, and the halves match by construction rather
than by hoping the asset is symmetric. It is also faithful to what is being modelled -
the brief's own reason for wanting hip yaw is that "the hips and legs are a unit".

**The obliquity's sign was inverted too**, which the brief warns is the one people get
backwards. Armature +X is forward, the left bones sit at +Y, and a positive turn about
+X carries +Y onto +Z - so a positive drop RAISED the swing hip, giving a hip-hitch
strut instead of a drop.

**And a test, so it cannot come back quietly.** `verify_gait.py` now refuses a clip
whose halves bob by ratios under 0.80 or whose peaks drift more than two frames from
half a cycle apart. This is the check that would have caught it from the start, and
nothing else in the file could: every per-frame direction was right.

### A bob with three peaks, and a foot that will not stay planted

**Symptom.** Hips rising 10.6 cm with THREE high points per cycle where a walk has
two, and a planted foot that slides anyway.

**Two separate causes.**

1. **The sole was a flat minimum over three bones.** The ankle sits higher off the
   ground than the toe does, so the moment the lowest point switched from one to the
   other - which is exactly what heel-strike-to-toe-off does - the measured sole
   stepped by the difference, and planting stepped the hips with it. **Fix:** each
   point carries its own rest height, so all three agree on a flat foot at rest and
   each tracks the sole correctly as the ankle rolls.
2. **Which foot to plant was being discovered rather than known.** Planting whichever
   foot measured lowest sounds more robust and is worse: this model's rest pose is not
   level (the right sole rests 1.4 cm higher), so the lower foot changed hands at
   moments having nothing to do with the gait. **Fix:** an eight-pose cycle already
   says which foot is down - the right lands at the start and pushes off halfway - so
   pass it in.

**And do not author the bob at all.** A hip bob added on top of posed legs is a second
source of the same motion, and the two disagree: the curve says the body rises at
passing while the geometry says it rises wherever the stance leg is straightest. Pose
the legs, measure the stance foot, move the hips by the difference. The bob that comes
out is the one a real walk has, for the reason a real walk has it - a straighter leg
is a longer leg.

### "speed = cadence x stride", and a stride measured with the wrong identity

**Symptom.** The run churns. Raising the speed makes it frantic; lowering it makes the
game feel slow. There is no setting that is right.

**Cause.** How far a cycle carries the warden was measured as **twice** one foot's
swing. A planted foot is STILL on the ground while the body travels over it, so
relative to the hips it moves backward at exactly the body's speed: if it travels `S`
during a stance lasting fraction `f` of the cycle, the body advances by **`S / f`**,
not `2S`.

The difference is the flight phase. A walk always has a foot down, `f` is about 0.6,
and `S / f` lands near `2S`. A run is airborne for part of its cycle, `f` is about
0.35, and the body covers nearly THREE times one foot's swing - distance it gets for
free while neither foot is planted. So the run was understated by 45%, and the only
way to reach the game's speed was to churn.

**Fix.** `dev/art/stride_measure.py` fits a line to the planted foot's travel and
reports the slope, so `f` falls out instead of being assumed. Measured off clips that
have a stance to measure: 1.935 m walking, 2.282 m running.

**A consequence worth stating.** The identity is exact and it CAPS the speeds. At
1.935 m a cycle and 140 steps a minute - the top of a real walking cadence - a walk
cannot exceed 2.25 m/s without becoming a jog, whatever it says on the constant. Going
faster needs a longer stride or a third clip, not a bigger number.

### The torso leaned BACK while running

**Symptom.** "Human spines don't lean back when we run." And they do not.

**Cause.** An axis constant applied to the wrong kind of bone.

`REACHES_FORWARD` was measured on thighs and upper arms, which point **DOWN** from
their joints. The spine points **UP**. The identical rotation therefore carries a
thigh's foot forward and a spine's head backward, so `swing(rig, "Waist", lean,
REACHES_FORWARD)` leant the torso back at every speed.

Measured rather than reasoned, once the report came in: a positive ten degrees about
`REACHES_FORWARD` moves the head **0.054 units BACKWARD** from the hips and the left
foot **0.079 FORWARD**. `Waist`, `Spine01` and `Spine02` all point +Z; `L_Thigh`
points −Z.

**The lesson is about the NAME, not the sign.** These constants exist so that a call
site states an intention and cannot state a sign — but a name like "reaches forward"
is only true for bones of the orientation it was measured on. There are two now, and
each says which way its bones point.

**And a second error in the same place:** `SPRINT_LEAN` was larger than `RUN_LEAN`,
which is backwards. Maximum-velocity sprinting is **more upright** than jogging; a big
lean belongs to acceleration, 45 degrees at a sprinter's block exit and nearly nothing
at top speed. Real trunk flexion is 4 to 12 degrees with the most economical near 6,
and game guidance quoting "15 to 30 for a sprint" is a two-to-four-times push that
makes a character read as permanently accelerating. Now 9 for the jog and 8 for the
sprint, measuring +6.97 and +6.2 degrees from the model's own resting posture.

**How it survived, which is the part worth keeping.** The research brief said it
plainly — "Torso: near-upright → forward lean, scaling with speed", "8-12 for a jog" —
and there were renders of the run and the sprint on screen. The renders were described
as showing "strong forward lean". They did not. **A render read for what it was
expected to show is not a check**, and this is the second time on this asset that
clips were judged without really being looked at.

**Fix.** `LEANS_THE_TORSO_FORWARD`, and a refusal in `verify_gait.py`: a clip with a
flight phase must have its trunk flexed forward by at least 4 degrees, and a walk must
stay within 3 of the model's own posture. Both measured in DEGREES against the rest
pose, because this figure stands with its chest 6.1 degrees behind vertical to begin
with — an absolute threshold refused a run that had leant forward perfectly well,
purely because it started from behind. The same calibration mistake as the sole
measurement, in a different place.

### "The limbs are still backwards"

**Symptom.** Knees folding like a bird's, elbows bending the wrong way, and a walk
that reads as wrong without it being obvious which part is wrong. Reported three
times, fixed twice by adjusting numbers, and still wrong both times.

**Cause — one sentence of prose in a docstring.** `animate_ranger.py` said:

> The model faces +X. So a limb swinging forward turns about the armature's Y axis. A
> positive swing is forward, which makes a knee's flexion negative and an elbow's
> positive.

Every clause after the first was wrong, and each call site then hard-coded a sign
from it: `swing(Calf, -knee)`, `swing(Forearm, 18.0)`. It was **reasoned about rather
than measured**, and reasoning got it inverted — which is the worst outcome, because
inverted-twice still animates, still exports, and still passes every test that checks
a clip exists.

**Measured three ways, all agreeing.**

1. **Directly.** +10 degrees about +Y moves the end of every limb BACKWARD — twelve
   bones, both sides, 0.002 to 0.078 units. Forward is **−Y**.
2. **The hinge test.** Bending a joint must FOLD the limb — shorten root-to-tip.
   Twists and swings do not. Of six candidate axes only **+Y** both folds the knee
   and puts it in FRONT of the hip-to-ankle line (+0.082), and only **−Y** both
   folds the elbow and puts it BEHIND the shoulder-to-wrist line (−0.074).
3. **The model's own idle.** `Ranger_Rig_Idle.glb` shipped with an authored pose by
   someone who could see it. Its knees turn about +Y by 42 degrees; its elbows trail
   by 0.037. Ground truth, in the same file, agreeing with the hinge test.

**A fourth defect the measurement found on its own.** The arms were not opposing the
legs because they were barely moving: `WALK_ARM` was **6 degrees**, moving the hands
0.04 against the feet's 0.46 — 9% of the legs' travel. Six had been a workaround for
the glove-in-pocket fault, which was repaired at its cause later, and the workaround
was never taken back out. A walk without visible opposition reads as wrong, and no
amount of correcting the knees fixes that.

**Fix.** The axes are measured once and NAMED for what they do —
`REACHES_FORWARD`, `FOLDS_THE_KNEE`, `FOLDS_THE_ELBOW`, `LIFTS_THE_TOE` — so a call
site states an intention and cannot state a sign. Arm amplitude back to 20/30.

**And `dev/art/verify_gait.py`, which is the part that matters.** It poses the
EXPORTED file over its own clips and refuses it unless the arms oppose the legs, the
knees lead, the elbows trail, and the arms carry at least a quarter of the legs'
travel. Wired into `animate_ranger.sh`, so a wrong sign now fails the export instead
of reaching a player. Every one of these three attempts was caught by the person
playing the game; that is the thing this fixes.

**Lesson.** A sign is a fact about a rig, not something to derive from how the rig
was described. Measure it, name it for its effect, and check the result.

**Still open, with a number.** The lower foot rides **0.095 m** over a walk cycle
(0.121 m running) instead of staying planted, because the hip bob is ADDED on top of
the rise a straightening leg already produces. `verify_gait.py` reports it. It is not
enforced, because the value it should have has not been established and a threshold
set to wherever the code happens to sit is decoration.

### An authored clip comes out as the splits

**Symptom.** A run where the legs reach nearly 90 degrees apart, the knees stay
straight, no foot is ever planted, and the torso is pitched back.

**Three causes, and only one of them was tuning.**

1. **Keyframing a bone that was never posed.** The clip that came with the rig
   leaves the armature posed. A bone keyed WITHOUT being set first records whatever
   it was holding — so the torso came out pitched back and the arms hung across the
   body, and none of that was in the keyframes written. **Reset every bone to rest
   before authoring**, not just the ones being driven: a bone left posed and never
   keyed holds that pose for the whole clip.
2. **A bone in the keyed list and not in the posing code.** `Waist` was keyed and
   never set. Same fault as above, one line away from being noticed.
3. **The stride was simply too big.** 42 degrees each way is 84 between the legs,
   which with straight knees is the splits. A stylised run wants a modest stride and
   a lot of KNEE — 28 and 62 here.

**And the real failure was not looking.** The clips were written, exported, and
handed over without a single frame being rendered — while saying out loud that the
tuning was unverified. `dev/art/gait_look.py` renders five frames of a named clip
into a strip; it takes seconds and would have shown all three faults at once.

### A figure that seems to drift across a render strip

Measure before believing it. A walk strip looked like the character was sliding
rightward frame by frame. The hip's world position was constant to four decimals in
X and Y across the whole cycle — only Z moved, which is the bob. Swinging limbs move
a figure's visual centre, and the eye reads that as translation.

## Fixing a generated texture

*The script this section describes, `ranger_texture.py`, went with the character on 2026-08-24.
The lesson did not: it is about where pixel work belongs, and it applies to whatever comes next.*

### White lines under a character's eyes

**Symptom.** Bright lines under the eyes on a face at walking distance.

**Cause.** Not the rig, not the sampler, not UV bleed. It is PAINTED IN: the
generated base-colour map gives the eye a wide sclera, and the crescent of it below
the iris reads as a line. Rows through the eye measure 251–255 against skin at
75–120.

**Fix.** Bring the sclera down to about 168 as a step in the asset pipeline, so
re-running the build cannot undo it.

### Editing a texture inside Blender can report success and change nothing

`image.pixels` was edited and `pack()` called. It printed a cheerful count and
exported the ORIGINAL bytes — peak luminance through the eye read 254 before and 254
after. Blender keeps the packed file it already has.

**Fix.** Do pixel work in a tool where the result can be read back and MEASURED —
here, numpy and pillow writing a PNG — then give Blender one job it cannot get wrong:
use this file. The pipeline now prints the peak luminance of what it actually wrote.

### Four heuristics that could not find an eye, and what worked

Worth the space, because each one looked reasonable:

1. **White near black.** 18,937 pixels — the white shirt against the black vest.
2. **Plus skin nearby**, which a shirt has none of. Down to 1,647, but only the
   crescent hugging the pupil; the rest of the sclera is further from the iris.
3. **Widen the reach** to cover the whole sclera. Thirty-five clusters along the
   atlas edge.
4. **By connected component** — a sclera is a small region, wardrobe whites are
   large ones. Found exactly two, and they were only PART of each eye: the sclera is
   painted in TWO tones, a pure white beside the pupil and a warm cream over the
   rest, and the cream is a separate region below the threshold.

What was wrong throughout: an eye in a generator's atlas is not one shape in one
colour, so no single rule describes it. The location is written down instead, as a
fact about one file — the same treatment the model's height and facing already get —
with the source's byte count as a guard, so a regenerated asset refuses rather than
dimming somebody's cheek.

**The lesson.** When the third heuristic fails, stop writing heuristics. Measure the
thing, write the measurement down, and guard it.

## Modelling people in a script

### A figure reads as a jointed doll, however the numbers are tuned

**Cause.** It was built from separate closed primitives — a ball for a head,
cylinders for arms, lumps for a torso. A body made of separate shells has a SEAM
everywhere a real one has a continuous surface, and the eye finds every one. No
amount of tuning the proportions fixes it, and several passes were spent trying.

**Fix.** A low-poly **cage plus subdivision**, which is how stylised characters are
actually made. A limb or a torso is one skin lofted through rings of
`(height, half-width, half-depth)` and subdivided once; limbs run INTO the torso
rather than up to it, so there is no seam at a shoulder.

**And a head is not part of that loft.** Lofting chest→neck→jaw→crown in one hull
gives a cone: the jaw ring and the head ring end up near enough the same width, so
there is no head in it — the face comes out as a long wedge with the hair sitting on
top like a cap. A cartoon head is a rounded volume on a short neck. A subdivided
CUBE makes a better one than a sphere: flatter face, squarer crown, and no pole in
the middle of the face.

### A wig disappears, or cuts away to nothing

Two faults in one place. The boolean cutters that carved a face out of a full cap
were positioned with **fixed heights**, so making the head bigger left them in the
wrong place and one style lost its whole cap — a bald figure. Tied to the head
properly, the boolean then returned *nothing at all*: a wig is several overlapping
shells joined together, not one solid, and a difference against that is unreliable
by nature.

**Fix: do not cut.** A wig is built to sit BEHIND the face in the first place —
every piece placed back and up from the head's middle, the cap stopping short of the
front of the head. That has no failure mode; it is the same arithmetic that places an
ear.

### A model does not appear at all, and nothing in the log mentions it

**Symptom.** The world draws, the camera follows the warden, and there is no warden.
No error about the character anywhere.

**Cause.** **Bevy decodes PNG by default and NOT JPEG.** The model's textures were
JPEG, so the image load failed, so the glTF load failed, so the scene never
instanced. The failure is three levels below the thing that is missing, and reads
from the outside as a bad model.

**Fix.** `bevy = { features = ["jpeg"] }`. And a test that makes the MODELS decide
what the manifest must enable — it reads the `mimeType` of every image in every
`.glb` in the game and checks the matching feature is declared, so a WebP texture
dropped in later fails naming the feature to add.

### A rigged character slides instead of walking

**Cause, and it is not the rig.** `bevy_animation` is **not a default feature**.
Without it a skinned mesh still draws — in its rest pose, for ever — so a rigged
figure glides about and it looks exactly like broken weights or a broken skeleton.
A whole pass went into re-measuring joint axes and rewriting a walk cycle before
this turned up. The keyframes were genuinely wrong as well, which is what made the
wrong explanation so convincing.

**Fix.** `bevy = { features = ["bevy_animation"] }`, plus a test: any model in
`assets/models` with joints asserts the feature is enabled.

**The general lesson.** Two of this project's worst afternoons have now gone on
BUILD-TIME features whose absence is silent at runtime. When something that should
obviously work does not, and the log says nothing, check what the engine was
actually compiled with before doubting the asset.

### A whole mesh comes out one flat colour after a join

**Cause.** A shading ramp given in world heights — a foot and a crown — but read
from `point.co.z`, which is measured from the object's ORIGIN. After a join that
origin is wherever the first part happened to sit, so a wig joined from a cap at
1.58 m had every vertex reading as below the ramp. The bodies were being shaded off
a ramp anchored at the head.

**Fix.** `obj.matrix_world @ point.co`. If a ramp is stated in world heights, measure
in world heights.

### Subdivision shrinks a cage, and everything measured off the cage is wrong

**Symptom.** Eyes bulge off a face like goggles, the neck looks too long, and the
arms hang clear of the shoulders — all at once, on a figure whose numbers look right
in the script.

**Cause.** Subdivision pulls a cage IN toward its limit surface, and by a lot.
Measured in a live Blender rather than guessed: **a cube at level 2 keeps 0.840 of
its cage; an eight-sided loft keeps 0.821 to 0.837.** So a head written as 0.325 wide
is 0.273 in the world — and eyes placed at the *written* face front sat about
fifteen millimetres in front of the real one.

Three separate-looking faults, one cause. Anything positioned against a cage
dimension is positioned against a number that no longer exists after the modifier
runs.

**Fix.** Build the cage DIVIDED by the factor, so what comes out matches what is
written. One constant, and every other number in the file becomes true again.

**Also measured:** for an eight-sided loft, subdivision **level 1 and level 2 are
the same shape** (0.3432 against 0.3446) — an eight-gon is already near its limit
surface. Level 2 was costing four times the triangles for nothing; a body went from
6,336 triangles to 2,464.

### A stylised figure keeps reading as a mannequin

**Cause.** Realistic proportions worn by a stylised model. The figure was drifting
toward five-and-a-bit heads tall with eyes a fifth of the face — those are *adult
human* ratios, and no amount of smoothing makes them read as a character.

**Fix.** The genre's conventions are specific and worth writing down: about **four
and a half heads** tall, eyes roughly **a third of the face's width** and taller than
wide, set low, with a big iris and a dark pupil; no nose, no mouth; short thick
limbs and mitten hands.

**And an eye is three parts, not two.** Two overlapping spheres reads as goggles —
a ball stuck on a face. A white, an iris and a dark pupil, each flattened hard in
depth so it is a disc with a slight dome, sitting a few millimetres proud of the
face. Sunk into it they come out as pinholes.

### Working out a shape in batch mode wastes the afternoon

Assets belong in `dev/art/*.py` under `blender --background`: same script, same rock,
every time. **Finding** a shape does not — four rebuild-and-render cycles went into
one figure, each starting Blender from nothing and discarding the scene that would
have answered the next question in a second.

`dev/blender_live.py` talks to Blender's own MCP add-on over its socket
(`localhost:9876`, null-byte-delimited JSON, `{"type": "execute", "code": …,
"strict_json": true}`), so a shape can be nudged and looked at against one live
scene. The numbers that come out of that get written back into `dev/art/` — live for
finding, script for keeping.

Notes on the add-on: it **sandboxes destructive operators** (`read_factory_settings`
is refused because it resets user preferences — use `read_homefile`), `result` must
be a **dict**, and `bpy.context.object` is unavailable, so work at the data level
(`bmesh`, `mesh.from_pydata`, `evaluated_get(depsgraph).to_mesh()`) rather than
through operators.

### A posed figure tears open, usually at the torso

**Symptom.** Rest pose looks fine. Posed, a sheet of the torso stretches from the
chest down past the hip, and a shoulder drags the chest with it.

**Cause.** Weights computed by **inverse distance to the nearest bones**, over the
whole skeleton. A vertex on the front of the belly is genuinely nearer to a thigh
bone than to the spine, and nothing in the rule said otherwise, so the thigh took
it.

**Fix.** Distance is the wrong instrument when the parts are known. Every piece of
this body is built by name — a vertex in the left sleeve belongs to the left arm and
to nothing else, whatever it happens to be near. So each part carries a **chain**:
which bones may claim it at all, and at what height each takes over. The vertex's
height along that chain decides how the claim is shared between the two bones it
lies between; nothing else can touch it.

Weights are assigned **before the parts are welded** — afterwards there is no
telling which vertex came from which piece. Blender merges vertex groups by name on
join, so they survive it.

### The skeleton stands where the body used to

Bones are written in world metres. The figure is seated on the floor *after* it is
built, and a rig made before that seating is left behind by however far the body
moved. Build the rig last, and pass it the same shift.

### A rig leaves through a checkbox

Skinning is exported by an option, and a body that has lost it looks exactly like
one that never had it — until something tries to bend the thing, which is a long way
downstream. `inspect` reads the joint count out of the file, and the test also checks
that every mesh carries `JOINTS_0` and `WEIGHTS_0`: exporting the skeleton without
the weights is entirely possible, and then the figure stands rigid while its bones
move under it.

The joint count is asserted as a **lower bound** (a spine of five, plus three bones
in each of four limbs) rather than an exact number, because the exact number belongs
to `dev/art/people.py` and copying it into the test would be one more pair of numbers
that has to agree.

**And `Vector.length_squared` is a property, not a method** — calling it is a
`TypeError: 'float' object is not callable`, which reads like a much stranger problem
than it is.

### Limbs look translucent, or a shell looks lit from inside

**Symptom.** A limb reads as see-through against the body: darker than the torso,
with the torso's silhouette showing through it.

**Cause.** The hull is inside out, so backface culling hides the near wall and shows
the lit interior of the far one. `loft` winds its side quads on the assumption that
each ring is ABOVE the last — and an arm is naturally described from the shoulder
DOWN, so the arm and leg lists descended and their winding reversed.

**Fix.** `loft` sorts its rings by height, so the order cannot matter. Describing a
limb downward is the natural way to describe a limb.

**How to see it.** Not from a screenshot of the finished figure — the vertex colours
and the sun hide it. Render with ONE plain material and backface culling forced on:
inside-out parts come out obviously dark against correct ones. A signed-volume test
on one loft in isolation had passed, because the list written for the test happened
to ascend.

### The model faces the wrong way, so the character walks backwards

Everything is modelled toward -Y, because that is where Blender's front view looks
from. The Y-up conversion turns Blender -Y into **+Z**, and the game's forward is
**-Z**. So a figure modelled the natural way walks backwards, and nothing says so
until somebody watches it move.

Turned half a circle at the end of the build rather than at spawn, so the rule stays
in one place. Turning about the up axis also carries +X to -X, so the `.l`/`.r`
naming has to flip with it or every bone lies about which side it is on.

The **eyes are the instrument** for a test: they are the one part of a body that is
only ever on the front, so a body whose eyes are not at negative Z is back to front,
however plausible it looks standing still.

### A joint bends the wrong way

**Symptom.** Knees bend backwards like a bird's; elbows hyperextend, so the arms
read as a zombie's.

**Cause.** Guessed rotation signs. Which way a bone turns depends on its rest
orientation, and for a downward-pointing bone it is not obvious.

**Measured, not assumed.** Pose one leg and one arm at **+30 degrees** about the
bone's local X and render from the side: **positive swings a limb FORWARD.**
Therefore

* a **knee flexes NEGATIVE** — the shin swings back, heel toward the buttock
* an **elbow flexes POSITIVE** — for an arm hanging down, the forearm comes forward

Two signs, one render, and it settles a class of bug that is otherwise argued about.

### A hat slides across the head as the character moves

**Cause.** Hair and hats were children of the CHARACTER, so they followed the body
and not the head. The moment a walk started bobbing and turning the head, the hat
stayed where the body was.

**Fix.** A worn thing belongs to the head BONE. The skeleton arrives as entities
named after their bones, so find the one called `head` and re-parent to it.

The offset has to be **worked out, not written**: a wig is authored in the body's
coordinates, so as a child of a bone its transform must be whatever maps body space
into that bone's space — `bone_global.inverse() * body_global`, taken at the moment
of attachment.

**And attach before anything plays.** That transform is captured from the skeleton's
rest pose. Attach mid-stride and the pose of that instant is baked into the offset
for good, so the hat sits wrong forever afterwards.

### A gate that refuses a model silently leaves the game with the old one

The worst shape of failure here so far, because it presents as something else
entirely. The export gate refused both bodies for floating 4 cm off the floor. A
refusal aborts the export. So `assets/models/` kept the PREVIOUS models — and
animation clips were being authored correctly, exported correctly, and never
arriving. It read as a broken exporter for a long time.

Three separate measurement mistakes underneath it:

* **`object.bound_box` ignores modifiers.** It is the mesh as authored, before
  subdivision pulls a closed cap inward and before Blender's smooth-by-angle
  geometry-nodes modifier. The export applies modifiers, so the gate was judging a
  surface that does not ship. Measure through the depsgraph.
* **An NLA track plays by default.** Once a walk was on one, the evaluated mesh was
  posed mid-stride — a leg out front, measuring as a body 0.93 m deep with a foot
  below the floor. Silence the tracks for the measurement, or measure before the
  clips exist.
* **A skinned mesh's node transform is IGNORED by glTF.** Seating the figure by
  moving the armature object looked right in Blender, exported faithfully, and did
  nothing in the game — such a mesh is placed entirely by its joints and their
  inverse bind matrices. The offset has to go into the vertices and the bone rest
  positions.

And the rule itself was wrong for a character: the floor rule exists for things
placed by their base, and a warden is placed by their root bone. A rigged figure now
gets a looser slack, which still catches one genuinely floating.

### Authoring a clip leaves the rig posed

`keyframe_insert` sets the value as well as recording it, so after writing a walk the
armature stands in whatever its last keyframe said. Saved that way, the mesh
evaluates deformed. Clear the pose after authoring — and know that clearing it does
not re-evaluate anything, so a measurement taken straight afterwards still sees the
old pose unless the view layer is updated.

### Blender 5 moved the Action API

`action.fcurves` is gone: curves live under layers, strips and channelbags. Reaching
for the old attribute is an `AttributeError`, which reads as a broken script rather
than a moved API. And `Vector.length_squared` is a property — calling it gives
`TypeError: 'float' object is not callable`.

### A test that keeps needing its threshold moved is asserting the wrong thing

Worth its own entry. The check for "this model still carries its baked shading" was
moved **four times**: 0.02 of absolute spread (a dark fence failed it), then a ratio
of 1.08 (an iris failed it), then scoped to meshes over 20 cm (a face failed it).

Every counterexample was legitimate. The mistake was that the assertion was an ART
JUDGEMENT — how strong a gradient should be — while both faults it exists to catch
are **binary**: a dropped `COLOR_0` gives no colour, and a misanchored ramp gives
exactly one value. The rule is now that a gradient EXISTS (1.005), and how strong it
is belongs to whoever painted it.

**The tell:** if a threshold has to move every time a new legitimate case appears, it
is measuring taste rather than correctness.

### The skeleton is a bag of spheres, and the bones stop short of the hands and head

**What you see.** "no human has spherical bones". "Bones should reach the top of the
head and ends of feet and hands, they dont". "what is this long angled bone". All three
have now been reported, fixed, and reported again.

**What it actually was.** None of it is in the file, and none of it can be. glTF stores
joint POSITIONS and nothing else — no lengths, no display — so the importer invents all
of it, on **every** import:

* `armature_display` builds an `Icosphere` and assigns it as `custom_shape` to all 41
  bones. A custom shape overrides `display_type`, so setting OCTAHEDRAL does nothing.
  Hiding the Icosphere OBJECT also does nothing — bones reference the datablock.
* Leaf bones get an invented length. Measured: `Head` 2.6 cm on a 27.8 cm head, both
  `Hand`s 8 cm past the fingertips, each `ToeBase` 15.9 cm where the geometry it drives
  is 6.7.
* `Root` and `Hip` arrive 85.0 cm. That one is *not* invented — `Root` sits on the floor
  and its only child is at the pelvis, so its tail genuinely spans the body.

Because the repair cannot be exported, this is not a bug to fix once. Every tool that
opens a GLB has to redo it — and the reason it kept coming back is that each tool did
its own subset. `ranger_blend.py` was saving a .blend containing all 41 spheres and the
2.6 cm head bone.

**What changed.** One function, `prepare_rig.make_the_import_readable(rig, mesh)`, doing
all three, called by `ranger_blend.py` and `gait_watch.py`. Ordering is part of its
contract: it disposes of the Icosphere by deleting unskinned meshes, so a floor or a
reference prop must be added AFTER it, never before.

It is safe on an animated file, and the reason matters. It only ever changes bone
LENGTHS, which are stored apart from `matrix_local`. REDIRECTING a bone rotates
`matrix_local`, and that is the basis the importer already converted the clip's keys
into — so a redirect after import silently corrupts the pose. Hence two versions of the
Root/Hip fix: `sane_root_and_hip` redirects and is for the BUILD only;
`shorten_the_controls` changes length alone and is for anything opening an animated file.

**The test.** Both length steps re-read every direction and roll afterwards and refuse
if a skinning basis moved — they report `0.000000 deg`.
`make_the_import_readable` refuses if any bone still wears a widget, and
`gait_watch.sh` reopens the saved .blend and asserts `widgets=0` before handing it over.

### A stance boundary that lands on a keyframe reads as airborne

**What you see.** Peak thigh extension a few degrees short of what the clip used to
reach, hips looking slightly ahead of the feet, and nothing refusing.

**What it actually was.** `planted` was `own < share` — half-open. Stance is the CLOSED
interval `[0, share]`: `share` *is* toe-off, and the sole is still down at that instant.
For the walk `share` is 5/8 over a span of 24, so own phase 0.6250 is exactly frame 16 —
a real keyframe, and the most extended pose in the cycle. It read as airborne, so
`rest_the_shoe_on_the_floor` **pushed its sole 1.61 cm up off the ground** instead of
pulling it down. Peak thigh extension measured −17.71 deg where the geometry allowed
−19.41.

What makes it a bug rather than a choice is that the code already disagreed with itself:
`where_the_balls_go` computes `lift = 0` and the same `along` from either branch at
`own == share`, so the PATH intended ground contact while the planted test denied it.
The test was written out **three times inline**, which is how the divergence survived.

The same discretisation was costing every clip its authored duty factor — the run
delivered 0.250 per foot against an intended 0.375, a jog running on a sprint's duty.

**What changed.** One function, `ik_gait.the_foot_is_down(own, share)`, closed at the
boundary with a float tolerance, used at all six sites. All three clips now deliver
their authored duty exactly: walk 1.25, run 0.75, sprint 0.5 — each 2× its share.

**The test.** `verify_gait.py` reports `duty_factor` per clip. Note that it did NOT
catch this: `THIGH_REACHES_BACK` is 12.0, the anatomical floor, so a fall from 22 to 17
sailed through. **A guard set at the bottom of a legitimate range cannot see a
regression inside it.**

### Fixing a systematic error mistunes everything built on top of it

**What you see.** A measured, correct fix to one thing, and an unrelated-looking number
gets worse. Here: the foot-turn convention was unified, every foot angle then measured
right, and thigh extension quietly fell from −22.4 to −17.7 deg.

**What it actually was.** The old `ankle_for` applied *pitch + 7.45 deg* — it silently
added the bind's own dip to every foot angle. Every value in the pose tables had been
tuned AGAINST that error, so the numbers carried the bug as compensation. Removing it
made the maths right and the tuning wrong in the same instant. The corrected derivation
moves the ankle 1.13 cm forward and 1.73 cm up; that shortens the hip-to-ankle chord and
folds the knee, and a bent knee's offset from that line goes forward, pulling the thigh
angle forward again. Two effects, same direction, 4.7 deg.

**The principle.** When a systematic error is removed, everything tuned on top of it is
mistuned by exactly the amount removed. Re-measure what is DOWNSTREAM of the change, not
only the thing changed. The failure here was checking the foot angles, seeing them
correct, and stopping.

Also: the −22.4 that looked like the target was itself partly an artifact of the
over-rotation, which sat the ankle further back and lower. **A number measured under a
broken instrument is not a specification.**

**The test.** None that catches this class. The nearest practice is to re-run the whole
`verify_gait.py` score after any change to shared geometry code and diff it, rather than
measuring only the quantity you touched.

### A foot angle that cannot be fixed by changing the foot angle

**What you see.** The jog lands on its forefoot with the heel 6.5 cm up and never puts
it down — closest approach 2.32 cm, all cycle. The obvious lever is the ankle-pitch
table, `RUN_LEG[0] = -16.0`, whose own comment calls it a forefoot landing.

**What it actually is.** Not a decision. At touchdown the stance leg is at **100.000%
extension** — hip-to-ankle 78.35 cm against a 78.35 cm maximum reach — with the knee at
0.28 deg where that row of the table asks for 12. A flat foot needs the ankle 7.05 cm
above the sole, and at that hip height the leg reaches only 10.83 cm forward. The clip
demands 28.41 cm. There is **17.58 cm of forward reach that does not exist**, and the
plantarflexion is buying it. The ankle is the last joint with anything left to give.

So editing the foot angle toward a heel strike cannot be neutral: with the leg saturated
it must either lift the forefoot clear of the floor or demand a hip drop the cap forbids.
The upstream levers are the forward reach (`RUN_CONTACT × RUN_LANDS_AHEAD` = 35.97 cm of
ball ahead of the hip), knee flexion at contact, or `HIP_DROPS_AT_MOST`.

The same shape explains why the run's hips barely travel. Of 9.2 cm of chord shortening
at mid-stance only 4.08 cm becomes hip drop; 4.9 cm is spent lifting the ankle, because
the heel is up. The table annotates mid-stance as `[thigh 2.0, knee 38.0]`, and 38 deg is
precisely what a 4.08 cm drop needs WITH THE ANKLE AT REST HEIGHT — the figure assumed a
flat foot and the clip has a raised one. The stance knee is already 56–59 deg against a
human jog's 40–45, so raising the cap would put it past human range: the walk's crouch
trap re-entered from the other side.

**The lesson, now recorded twice.** Five rounds of foot-pitch tuning were once lost to a
ball joint in the wrong place. This is the same shape. When the same fault survives three
measured fixes, stop tuning and question the STRUCTURE.

**Status.** Open. The forefoot landing and the missing bob are ONE fault, not two — fix
the reach and the heel comes down and the hips travel for free.

**The test.** None. `verify_gait.py` passes this clip.

## The ranger was replaced, and this is what carried over

On **2026-08-24** the ranger's mesh, rig, clips and its whole asset pipeline were deleted, to be
rebuilt from new source files. Everything that used to be written here about that character -
its walk, its jog and sprint, its shoes, the four passes spent reshaping a shoe that was already
right - described measurements of a mesh that no longer exists, and a log that names things that
are gone is worse than no log. It is in git: `git show ed006b9:TROUBLESHOOTING.md`.

What survives is the part that was never about that character.

**Weld before asking a topology question.** glTF stores one set of attributes per vertex, so
every UV seam and every hard edge duplicates the vertices along it. On the ranger, 9190 stored
vertices were 3655 real ones and 1438 apparent shells were 18. `is_boundary` on a mesh like that
calls every edge a boundary and answers no question at all - it produced a confident, wrong claim
that a shoe could not be lowered without leaving the ankle in mid-air. Positions survive an
export; connectivity does not.

**Never weld the mesh itself.** Those split vertices ARE the hard-edge encoding, and the custom
split normals riding on them describe a topology that merging destroys. The character is then
lit as a shape it is not, and no geometry measurement sees it. Weld VIRTUALLY - round coordinates
into buckets, union across edges - and leave the mesh alone.

**Custom split normals carry all of the smooth shading.** On a fully split mesh there is no
connectivity to smooth across, so any operation that adds geometry has to rebuild them over the
region it touched. Subdivision interpolates them instead, and interpolating a normal field
authored for one topology across a finer one gives mush - a surface that looks melted and
measures perfect.

**Look at form in CLAY.** Render with every material replaced by plain grey. A textured render
cannot show shape, because the paint hides the thing it is painted on: a 64-vertex blob read as
a trainer in every textured render taken of it, and as a sock the moment the texture came off.

**A texture authored for one shape is a limit on how far that shape may change.** Painted detail
sits at fixed places in UV space and only lines up while the geometry keeps its proportions. Any
non-uniform reshape slides it off.

**A guard must compare against the SPEC, not against its own input, and it must know its
baseline.** Two failures of this in one afternoon: a check that refused an ankle junction at an
absolute 2.2 cm on a mesh that already carried 7.96 cm edges there, and a "has this already run"
test written as a ratio that was already satisfied before anything happened.

**Object-mode selection does not survive into edit mode.** 226 faces selected in object mode
arrive as 7139, so `bpy.ops.mesh.subdivide` cuts the whole mesh. Pick faces through bmesh once
edit mode is open, and count afterwards.

**Confirm which object is being looked at before changing any of them.** Delivered animation
files can carry their own copy of the character. For several rounds one side reported on one
mesh while the other measured a different one, and neither said so.

**Two tests before believing a negative result.** An attempt to rule normals out wrote zero
vectors instead of removing the layer; the render came back unchanged and that was read as
"not the normals". Check that a negative test actually did the thing it claims to have done.

**A fault reported twice means the model of the problem is wrong**, not that the fix was too
timid. By the second report, put every state the thing has been in on one labelled contact sheet
and ask which one is right.


## An arm that reads as attached to the torso

**What you see.** "The idle still has his right arm seem attached to the torso." The sleeve and
the jacket side merge into one lit plane, with no separating crease, and the right side is worse
than the left.

**What it actually was.** A POSE, not the mesh. Measured across the delivered idle, the angle
between the upper arm and the spine:

    L arm   min  9.6 deg   mean 15.7 deg   max 25.8 deg
    R arm   min  8.8 deg   mean 11.9 deg   max 18.9 deg

The right arm is held about four degrees tighter to the body than the left for the whole clip.
That asymmetry is the report. An arm resting against the ribs has no gap to shade, so nothing
done to the armpit geometry can put one there - and two attempts proved it. Sinking the recorded
webbing faces toward the armpit apex moved 92 vertices by up to 2.23 cm and the render was
indistinguishable: moving surface vertices toward a POINT slides them along the surface rather
than recessing them. Drawing each vertex toward its own bone's axis instead does recess them, but
at a sink large enough to see (10.5 cm) it punched visible spikes through the shoulder, because
the recorded centroids are a scattered set of faces and not a clean band.

**What changed.** `lift_the_arms` in `dev/art/build_character.py`: a pose-fixup layer that adds a
constant abduction at each shoulder until the clip's CLOSEST frame clears `ARMS_REST_AT`
(16 degrees). It keeps the animator's arm swing, costs no geometry, adapts to a second body for
the character creator, and unlike every mesh edit tried here it cannot open a hole. Both arms now
rest at 15-16 degrees and, more importantly, match each other.

Two details are load-bearing. The axis is DERIVED, not assumed: rotating a direction `u` about
`n` moves it by `n x u`, so abduction - which is `180 - angle(u, spine)` - opens fastest about
`u x spine`. The first version negated that cross product and drove the left arm from 10.0
degrees DOWN to 4.5, pressing it further into the body. And the offset composes on the REST side
(`offset * keyed`, not `keyed * offset`), because an abduction is a constant swing of the whole
posed arm about an axis fixed in the shoulder - unlike `roll_the_hands`, whose twist follows the
bone and therefore composes on the posed side.

**The test.** `lift_the_arms` re-measures after lifting and refuses the build if the arm ended up
closer to the body than it started: "the abduction axis is pointing the wrong way, so the arm was
pressed INTO the body". That is the exact failure that shipped once.


## Eight of sixteen "rest pose" renders were not the rest pose

**What you see.** Lifting the idle's arms by six degrees made the golden gate report THIRTEEN
changed shots out of sixteen - including the FEET, and including shots labelled `rest`, which no
clip edit can reach. Every kept-versus-now pair showed the same geometry, translated.

**What it actually was.** Two separate instrument faults, both of which made the gate answer a
question other than the one it was asked.

The camera was framed from the POSED mesh: height from its bounds, centre from the mean of every
vertex. That makes the framing a function of the pose, so moving the arms outward shifted the
centroid and slid the frame on every shot in the sheet. A gate whose camera moves with its
subject cannot tell "the armpit changed" from "the camera slid", which is the one thing it exists
to tell.

And the rest shots were never at rest. `rig.animation_data.action = None` followed by
`view_layer.update()` does not re-evaluate - Blender keeps handing back the last evaluated pose -
so those eight shots showed frame one of whichever clip the importer left bound. This is the
fourth time the stale depsgraph has produced a mislabelled measurement on this character (a rest
pose in the strain audit, a bind in the hand measure, every bone reading 0.00 cm in the twist
test, and now half the golden sheet). Nudging the frame off and back does not force it either.

It was caught by a cross-check that cost nothing: the two builds' meshes are byte-for-byte
identical - 7859 vertices, worst difference 0.0000 cm, read straight out of the two .glb files
with plain Python - yet the renderer reported their rest centres as 0.1250 and 0.1214. A number
that moves when the thing it measures does not is the instrument, every time.

**What changed.** In `dev/art/render_clay.py`, `at_rest` reads `body.data.vertices` - the STORED
geometry, which is the bind pose by definition and cannot be stale because nothing evaluates it -
and returns a floor, height and centre that are constants of the character rather than of the
frame. `stand_at_rest` zeroes every pose bone's `matrix_basis` instead of trusting a cleared
action, which is deterministic. The slot must be unbound BEFORE the action, or Blender raises
"Cannot set slot without an assigned Action".

Shots aimed at a BONE still follow that bone into the pose: a hand close-up has to find the hand
where the clip put it. It is the framing that is fixed, not the aim.

**The test.** Verified in both directions, the way the gate's own threshold was. Rendering the
same shot from two builds that differ only in clip curves now gives 0.0000. Blessing a no-lift
build and then turning the lift on changes exactly ONE shot - `idle_worst`, the only clip-posed
idle shot in the sheet - with rest, both hands, the feet, walk and run all at 0.00. Before the
fix the same change reported thirteen.


## A looping clip can close in rotation and still snap

**What you see.** Nothing, until the character twitches once per loop.

**What it actually was.** The clip audit reported `first to last pose 0.00 deg <- loops` for the
idle, and separately that the hip "travels 26.9 cm". Neither number says whether the hip comes
HOME: `travel` is the largest excursion from frame one, so a clip that sways 27 cm and returns
reads identically to one that walks 27 cm away and stays there. Rotation closing is only half of
a loop.

**What changed.** `the_clips` in `dev/art/audit_character.py` now also reports the net first-to-
last hip offset, as "hip ends N cm from where it began".

**The test.** It is the report. All three clips currently read 0.0 cm and "lands", so the idle's
26.9 cm is a weight shift that comes home rather than a drift - which is what wanted checking.


## The elbow twisted, three times

**What you see.** "There does seem to be a twist in the mesh of the elbow render." Then, two
fixes later, "Twisted arm at the elbow again." The forearm reads wrung, with a hard shattered
wedge of faces at the cuff.

**What it actually was.** Not what either earlier fix assumed. The first attempt read it as the
palm roll fighting the clip's own pronation and cut `PALMS_ROLL_IN` from 90 degrees to 30, which
helped and did not fix it. The second left the roll bones alone on the reasoning that the clips
already pronate correctly. Both were guesses about a mechanism. Two measurements settle it.

Every bone's own rotation about its length, worst frame per clip:

    clip    Forearm        Twist01   Twist02   Hand
    idle    -92 / +111      0.0       0.0      -+30
    run     -84 / +119      0.0       0.0      -+30
    walk    -60 /  +97      0.0       0.0      -+30

And the weight map:

    Forearm            0 verts        ForearmTwist01   616 verts
    Upperarm           0 verts        ForearmTwist02   316 verts

`Forearm` and `Upperarm` deform NOTHING. They are pure bend bones, and the roll bones beneath
them are what the mesh is actually attached to - the rig was built for roll distribution, which
is why the bend bones have no weights at all. The clips never drove it: both roll bones read
exactly 0.0 in every clip, so the forearm's 119 degrees is inherited WHOLE by both of them and
every vertex from elbow to wrist turns as one rigid block. The crease is where that block meets
the upper arm, which is the elbow, which is where it was reported three times.

That also explains why fighting the roll never worked. The total twist was never the problem; its
DISTRIBUTION was, and adding or removing roll changes the total without touching the gradient.

**What changed.** `spread_the_twist` in `dev/art/build_character.py` splits each forearm key into
swing and twist about the bone's own length, leaves the swing on the bend bone, and hands the
twist to the chain in graded shares - `ForearmTwist01`, `ForearmTwist02`, then the hand at the
full amount. Removing it from `Forearm` costs nothing because nothing is weighted there. The
gradient along the arm went from 0 -> -92 -> -92 -> -92 to 0 -> -21 -> -73 -> -92, and the
forearm's own twist from 119 degrees to under 3.

Three details are load-bearing:

* The shares are MEASURED, not the textbook one-third/two-thirds. Each roll bone carries the
  twist belonging to where its skin actually sits, as a weighted centroid along the forearm, so
  it comes out at 23% and 79% on the left and 31% and 77% on the right and adapts to a second
  body for the character creator.
* The twist has to be conjugated into the axes of whichever bone carries it, using the
  CUMULATIVE rest rotation down from the forearm - one level for the first roll bone and the
  hand, two for the second.
* **The hand takes the whole twist**, and that is correctness rather than polish: with the
  forearm no longer twisting, the wrist only lands where the animator put it if the hand carries
  the roll locally. As the user put it, "that also means hands need to move too not just arms."

**The test.** The build measures each hand's world orientation before and after the spread and
refuses if any wrist moved more than half a degree: "the wrist must land exactly where the
animator put it, so the shares or the rest conjugation are wrong". It currently reports 0.000
degrees on three clips and 0.403 on the run. The golden gate independently confirms containment -
the spread changed only the three clip-posed shots, with every rest shot, both hands, the feet,
the armpits and the silhouette at 0.00.

**One trap worth naming.** `rest_down_to` first walked the parent chain comparing bones with
`is`, and refused with "L_ForearmTwist01 is not below L_Forearm" for a bone whose parent is
exactly that. Blender hands back a fresh wrapper object on every attribute read, so identity
comparison between two reads of the same bone is always false. Compare names.


## Thirteen bad frames from one joint in the wrong place

**What you see.** Thirteen of a twenty-five frame run called out as wrong, the shoe folding
across its own middle, and: "The toe bones should go to the end of the mesh."

**What it actually was.** The toe joint was at the MID-ARCH. Measured along each shoe, heel to
tip:

    L   shoe 33.14 cm   ankle at 27.8%   toe joint at 45.1%   toe bone ended at 62.4%
    R   shoe 32.93 cm   ankle at 21.2%   toe joint at 38.1%   toe bone ended at 55.0%

A ball of the foot is 65-75% along. `ToeBase` owned everything from 28.7% forward and hinged at
45%, so any toe rotation swung more than half the shoe about a point in the middle of the arch.
That is not a toe bending, it is a shoe snapping, and it is what all thirteen frames showed.

Worth naming why this took so long to find: two rounds were spent tuning the ROTATION - how much
break, over what share of stance, with what easing - and the rotation was never the problem. A
hinge in the wrong place cannot be tuned into the right one. The measurement that found it took
one script and asks a question none of the tuning did: not "how far does the toe bend" but
"WHERE does it bend".

**What changed.** `hinge_the_toes_at_the_ball` in `dev/art/build_character.py` moves each toe
joint to 70% along its own shoe, runs the bone on to the tip, and redistributes the weights about
the new hinge. Only the share held between `Foot` and `ToeBase` moves - each vertex keeps its
total, so nothing the calf or the ankle holds is disturbed. Done in edit mode with the rig at
rest, because Blender deforms by the difference between a bone's pose and its rest, and at rest
there is none, so the mesh does not move.

Afterwards: the joint sits at 70.0% on both feet, `Foot` owns 0-68.6% and `ToeBase` owns
70.7%-100%, which is a clean split at the hinge.

**The test.** `dev/art/audit_character.py` measures the legs and refuses on drift, and the golden
gate carries three foot shots at the run's worst frames. The hinge position itself is asserted by
the build: `the_shoe_runs` refuses if a shoe has no vertices or no horizontal axis, and the
re-weighting preserves each vertex's total, which `weights add to 1 within 0.000000` confirms.


## The jog leant like a sprinter, and levelling it made him look at the sky

**What you see.** "The jog SHOULD be easy. Less forward lean, less arm swing."

**What it actually was.** Both, measured, and both large:

    trunk    +35.3 deg forward OF ITS OWN REST   (the bind stands 7.6 deg behind vertical)
    arms      119 deg of swing, where its own walk swings 33

The research for this was already in this file, from the previous character: real trunk flexion
in running is **4 to 12 degrees**, most economical near 6, and game guidance quoting "15 to 30
for a sprint" is a two-to-four-times push that makes a character read as permanently
accelerating. The previous character shipped its jog at +6.97 from rest. This one was at five
times that - a sprinter's block-exit lean, held for a whole cycle.

**What changed.** `lean_a_chain` brings the trunk to +7 from rest, and `MOVES_MORE` - which
already existed to LIVEN the idle - takes the jog's arms to 0.45x. One knob, used both ways.

**Three separate things went wrong on the way, and each was caught by a check rather than by
looking:**

* **The axis was negated.** The correction ran the wrong way and left the trunk at +48.8 from
  rest instead of +7. This is the third derived axis on this character to be negated by hand.
  The guard in `lean_a_chain` refused the build immediately.
* **The measurement could not see half the correction.** The trunk was measured from
  `Spine01.head` to `Spine02.head` - which IS the Spine01 bone - so rotating `Spine02` moved the
  number not at all, and a 28.3 degree correction delivered 12.6. A chain has to be measured to
  the TAIL of its last bone or the bones above the measurement are invisible to it.
* **The iteration diverged.** Correcting by the full shortfall overshot, because rotating
  `Spine01` tips `Spine02` with it and then `Spine02` adds its own: the chain's gain is above
  one. It swung from +35.3 to -1.9 chasing +7.0. Halving each correction converges for any gain
  up to four.

**And the fix had a consequence worth naming.** Leaning the trunk back by forty degrees carried
the head with it, and the warden jogged along looking at the sky - 28 degrees above his own
resting gaze, which no measurement of the TRUNK would ever have reported. The same
`lean_a_chain` levels the head afterwards. **A correction to a chain is a correction to
everything above it**, and the thing to check is not whether the number you aimed at moved but
what else did.

**The test.** `lean_a_chain` refuses a chain that will not settle within a degree of its target,
and reports what it achieved rather than what it asked for. The trunk now reads +6.2 from rest
and the head -0.2.


## Re-deriving something the repo had already solved

**What you see.** "The arms still need to pump instead of whatever they're doing. This was a
solved problem we had so its crazy we're going through it again."

**What it actually was.** It HAD been solved, on the character deleted on 2026-08-24, and the
answer was sitting in git history the whole time. From commit `5a7c815`:

> "A pure cosine cannot pump: it spends its time evenly. SPRINT_PUMPS = 0.55 shapes the swing
> with an odd-symmetric power that flattens the peaks and steepens the middle, so the arm DWELLS
> at the ends and snaps between them - 16 of 24 frames now sit within 15% of an extreme against
> the run's 12. Phase and extremes are untouched, so the cycle still closes."

And the deleted `animate_ranger.py` still holds the constants, one commit back. It even carries
the SAME REPORT, already answered: "when people jog their forearms are more in front of the body.
Here the forearms are more outward" - cause, the elbow folding about a fixed armature axis; fix,
shoulder internal rotation turning the hinge plane across the body, `RUN_TUCK_IN = 12`.

Instead of that, amplitude was cut to 0.45 on the reasoning that "less arm swing" means a smaller
swing. It made the fault worse, and the reason is worth keeping: **a smaller swing on an arm that
is HELD OUT leaves the held-out part dominating.** Amplitude was never the axis the problem was
on. The old work says so directly - "the fold was always the fix for 'extended too far'; the
swing was never the problem".

**What changed.** `PUMPS` in `dev/art/build_character.py`, applying that same odd-symmetric
shaping to the delivered curves: each key's deviation from the bone's own average is normalised
against its widest excursion and raised to a power under one. At 0 and 1 it is unchanged, so the
extremes and the phase stay exactly where the animator put them and the loop still closes.
Amplitude back to 0.67, which restores the old jog's 80 degrees of swing.

Two other faults were found in the same pass, both invisible to every measurement then in place:

* **A sideways lean of -12.8 degrees from rest**, held for the whole cycle - asked as "do you see
  the lean?". Every lean measurement to that point was of the FORWARD axis and could not see it.
  Two axes, two faults, and correcting one says nothing about the other.
* **The `Hip` and `Root` bones drawn 84.23 cm long on a 170 cm figure** - "this hip bone is
  bigger than his body". `Hip`'s first child `Pelvis` sits ON its head, so the closest-child rule
  measured zero, the too-short guard skipped the bone, and it kept the importer's invented
  length. A child at zero distance tells a bone nothing about its length.

**The lesson.** Search the repository's own history before deriving. The previous character was
deleted; the work that went into it was not. `docs/` and `TROUBLESHOOTING.md` are the obvious
places and were checked - git log was not, and that is where this answer lived.


## The finger rig is mis-segmented (OPEN)

**What you see.** "Your hand bones dont fit the mesh correctly", with renders showing the finger
bones as stubs bunched near the fingertips instead of spanning knuckle to tip.

**What it actually is.** The digit basins in `add_the_fingers` are wrong on this hand. Measured -
how long each digit's three bones are, where they start from the wrist, and how far the hand
actually reaches in that direction:

    L Middle   bones span 11.35 cm from 10.87 cm out; the hand reaches 13.84
    L Ring     bones span 11.35 cm from 11.03 cm out; the hand reaches 13.75
    R Middle   bones span  3.30 cm from 12.51 cm out; the hand reaches 15.82
    L Thumb    bones span  6.28 cm from 10.39 cm out; the hand reaches 16.13

L Middle's chain runs from 10.87 to 22.2 cm from the wrist on a hand that reaches 13.84 - eight
centimetres past its own fingertip. R Middle spans 3.30 cm where L Middle spans 11.35, for
fingers that are nearly the same length. A thumb starting 10.39 cm from the wrist is not a thumb.

`the_joints_sit_inside` in the audit flags the consequence - eight bones outside the flesh they
drive - and the vertex counts point at the cause: every flagged bone owns 7 to 12 vertices where
a healthy one owns 25 to 81. The bones are not so much misplaced as barely skinned, because the
basin they were built from is not the finger.

**Why it is open rather than fixed.** Nothing drives these bones. Every clip leaves all thirty
finger bones on identity, so they ride the hand rigidly and the mesh deforms today exactly as it
would with no finger rig at all. It costs nothing until fingers are animated, which is stage
06/07 - and the docstring in `add_the_fingers` records that naming and segmenting digits "cost
the last character four wrong hands in a row". This is a piece of work, not a tweak, and guessing
at `A_DIGIT_STARTS` is how that four became four.

**The test.** `the_joints_sit_inside` runs on every audit and lists exactly which bones are
adrift, so whatever fixes this can be checked rather than eyeballed.


## Correcting a rest-pose constant per frame, again

**What you see.** Six rounds of foot corrections, each fixing a measurement and breaking
something else, ending in "I'm really struggling to understand why getting some feet in the right
direction is proving so difficult" and "the answer is in the docs you seem like you're refusing
to implement".

**What it actually was.** It was in the docs, and it had already been learned once.
`docs/rigging.md`, on the character deleted in August:

> "The delivered rig arrived with a 17.5 deg crouch, the two sides 5.45 cm from mirrored, and the
> character 5.7 cm under the floor. All three are rest-pose constants, which is why per-frame
> corrections kept failing: **correcting a constant per pose is what twisted the feet**, three
> separate times. Fixed once in the bind, and the authoring has no correction step at all now."

This rig arrived the same way, measured against its own mirror plane:

    positions    worst 5.60 cm, mean 3.24 over 16 pairs
    directions   worst 16.3 deg - and the worst pair is L_Foot and L_ToeBase

The feet were the most asymmetric bones in the rig, in the BIND. Every left/right difference
chased per-frame - the right foot 30 degrees off travel where the left is straight, the right arm
resting 4 degrees tighter, the right shoe squashing where the left does not - sat downstream of
that one constant.

**What changed.** `the_bind_is_mirrored` averages every pair with its own reflection about the
rig's own plane, so neither side is imposed on the other, and puts the centre bones on the plane.
In edit mode at rest, so the mesh does not move. Afterwards the sixteen body pairs are mirrored
to 0.0 degrees, the legs are identical at 37.72 + 38.99 where they were 38.69 + 37.64 against
36.91 + 40.59, and the count of bones sitting outside their own flesh fell from eight to five
without anything being aimed at them.

The clips are deliberately NOT compensated for the change. They were authored symmetrically and
retargeted onto an asymmetric rig, so the asymmetry lives in the rest; compensating the keys
would preserve exactly the look this exists to remove.

**What it did NOT fix, which matters.** The right foot still points about 30 degrees off travel
through the idle and the walk. That survived a bind that is now mirrored to four decimal places,
so it is in the delivered CLIP's keys and not in the rest. It is a constant over each clip, so
the fix is a constant offset per clip in the shape of `lift_the_arms` - not a per-frame
correction, which is the thing this whole entry is about.

**The lesson, which is not a new one.** Before correcting a fault that differs left from right,
or that holds steady across a whole clip, measure the BIND. A constant belongs in the rest pose.


## Four town bugs, and the one shape they share

**2026-08-29.** All four were reported by a player looking at the screen while every
measurement I had said the thing was fine. That is the tell.

### ISSUE: "Still no roads" — three times, against a paving mesh that measured

Every time it was doubted, the paving was asked to measure itself and answered 1,929
vertices, correct normals, correct height above the ground. So I concluded, three
times, that it was fine.

It was fine. The question was wrong. The paving spawned behind

```rust
if let Some(surface) = &road_surface { ... }
```

whose material came from a resource a `Startup` system filled in. That branch has
exactly one failure mode and it is **silent**: the buildings go up and the streets
do not.

**SOLUTION:** make the material where it is used, so there is no ordering to get
wrong and no branch that can skip a street. And a test that stands an app in a real
settlement and counts what actually spawned — `a_settlement_lays_its_streets_in_the_world`.
**A mesh that measures correctly in a function and never reaches the world is
exactly as useful as no mesh.**

### ISSUE: a beam through every doorway

`framing` laid its studs one per module and both its rails straight across the wall.
It had no idea where the openings were, so a stud landed dead centre in every
doorway and the bottom rail ran across the threshold at shin height.

**SOLUTION:** framing reads the SAME bay list the wall was built from. One
description of where the holes are, used by both — two would drift the first time
anybody moved a bay. Verified by measuring: 0 vertices inside the doorway of any
building, where before there was a post through the middle of each.

### ISSUE: windows and planters "not lined up" — on exactly half of each building

`_out`, the helper that pushes a dressing clear of the wall it belongs to, pushed in
the **negative** direction always. That is outward for the south wall and the west
flank, and *inward* for the other two — so every frame, sill, mullion, shutter and
flowerbox on the north wall and east flank was built a hand's width inside the room.
From the street: a window with no frame, and a flowerbox that is not there.

**SOLUTION:** a wall knows which way it faces; `facing` is threaded from `shell`
through `wall_run` into every dresser. And the check that finds this has to be a
render **lit from the camera** — the first attempt lit from a fixed sun, left the
north wall in pure shadow, and told me nothing.

### ISSUE: the buildings in the game were not the buildings in the .blend files

The `.blend` files measured 9.9 m across; the `.glb` files the game loads were the
6.9 m ones from before. `dev/model_export.sh` refuses a model that would import
half-buried **and stops**, and an untracked scratch file sorting alphabetically
before `town_` refused every run. Every town model had been silently stale for days.

**SOLUTION:** the stray moved aside, and the viewer's own scratch file now writes
outside `dev/art` — that folder is the asset folder, and a 260 m viewing shelf is
not an asset. Worth knowing: **a refusal that aborts the batch protects one model
and silently staleness every model after it.**

### The shape

Four bugs; one shape. Each time, the thing I measured agreed with me and the thing
the player saw did not — because I was asking the arithmetic instead of asking the
artefact. The paving's vertex count, the framing's own extents, the dressing's
offset, the `.blend`'s dimensions: all correct, all beside the point.

**Ask the thing itself.** Render it, spawn it, walk into it, open the file the game
actually loads. See also *Validate the ruler first* — this is its twin: there, the
instrument was wrong; here, the instrument was right and pointed at the wrong
object.


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

## A brown road that photographs grey

**ISSUE.** `ROAD_EARTH` was set to a good brown twice - a mid `(0.56, 0.40, 0.24)`
and a light `(0.82, 0.63, 0.40)` - and a photographed village lane came back neutral
grey both times, indistinguishable from a city's paving. Blamed on the near-cel
banding, then on the material. Neither was at fault: a magenta probe rendered
magenta, so vertex colours reach the screen intact.

**SOLUTION.** Divide an observed road pixel by the colour that produced it. That put
this world's road light at about `(0.22, 0.30, 0.50)` - a road is a flat upward face,
so nearly all the light on it is SKY light, and blue arrives 2.2x stronger than red.
Any colour whose blue channel is more than about a 2.2th of its red comes out
neutral however brown the constant looks. Crush the blue rather than raise the red:
`(0.85, 0.39, 0.15)` lands on screen at roughly 2.5:1 red-to-blue and reads as dirt.

Worth keeping for any surface that faces the sky. The constant is not the colour.

## Buildings standing in roads, with their doors facing correctly

**ISSUE.** Reported three times. Doors demonstrably addressed a street - there is a
test for it - and buildings still stood in roads.

**SOLUTION.** The clearance test reserved `footprint().length() * 0.55`, a little
over half the footprint's DIAGONAL, as a stand-in for "how much building is in the
way". A cottage's diagonal half is 5.86 m, so it reserved 3.22 m against a corner
that reaches 5.86: cleared to stand 6.22 m from a centreline, its corner sat 0.36 m
from it - inside a 3 m carriageway. It never showed against the street a lot was cut
from, because that is the shallow side; it showed every time against a street
crossing behind or beside it.

Replaced with the box's exact support function (`reach_toward`): project the two
half-extents onto the direction of the street being asked about. Against its own
street that is the half depth, so a set-back building still passes; against a street
off its flank it is the half width, which is what is really in the way.

Two follow-ons, both of which failed the suite before they were handled:
- Testing properly and DROPPING the failures thinned a city from 34 buildings to 17.
  A corner plot slides along its own frontage instead, which keeps the facing that
  frontage gave it. That is what a surveyor does with a corner plot.
- The guild hall used to face the square unconditionally. Pushed a ring outward to
  find room, that turned its back on the high street. It now fronts whichever street
  it stands nearest.

`no_building_stands_in_a_road` guards it by walking each carriageway between its
kerbs and asking whether any point of road is inside a building - deliberately NOT
by rerunning the placement rule, which cannot fail against itself. Checked: it
catches a guild hall with 0.4 m of carriageway inside its walls under the old rule.

## A map of a world nobody lives in

**ISSUE.** The overview painted ground colour only. A settlement LEVELS its ground,
so the biggest city on the map showed as a slightly flatter patch of the same green
as the country round it - which is to say, as nothing. The failure is silent: the
map looks fine, it is just empty.

**SOLUTION.** `world::chart` paints the works over the ground - roads, bridges, and
a mark per settlement with a pale ring under it so it reads against snow, sand or
grass alike. `the_map_shows_what_people_built` asks the PIXELS at each thing's own
coordinates rather than rerunning the painter, which cannot fail against itself.

The painting is shared by the terrain tool's overview and the map a player pulls up
with M. Two maps drawn by two pieces of code are two maps that disagree the first
time one of them changes.

**And the labels.** Pale text is legible over green, sea and sand and INVISIBLE over
snow: photographed, "Marrowmede" read as "Marrowme" and "Colderry" as "Colde", the
rest of each name lost in the icefield behind it. Names now sit on a dark backing.
They also sit 11 px clear of their mark rather than 7 - at 7 the disc covered the
first letter, so "Bellwether" read as "ellwether" - and flip to the left of the mark
near the map's right-hand edge.

## Colours that render twice as bright as their number

**ISSUE.** A city street measured `(200, 195, 191)` on screen from a base colour of
`0.31`. Three separate darkenings of that constant each moved it far less than they
should have. Before that, two perfectly good browns photographed pale, and a "dirt"
road had to be pushed to an almost fluorescent `(0.85, 0.39, 0.15)` before it read as
brown at all.

**SOLUTION.** A vertex colour reaches the shader as LINEAR light. Every road constant
was written as though it were sRGB - the value you would type into a colour picker -
and linear 0.31 is sRGB 0.58. Every road in the world shipped about twice as bright
as its number said.

Found by MEASURING the rendered pixel and dividing by the constant that produced it,
after adjusting the constant three times stopped working. The same move that found
the blue-biased sky light: when a change does less than the arithmetic says it should,
the arithmetic is being done in the wrong units.

Blender never had this problem - `masonry.paint` has always run `to_linear` on its
palette. It was only ever possible on the Rust side, where a colour is a bare
`[f32; 4]` with nothing to say which space it is in.

**And the trap is closed, not just the four holes.** `town::srgb` takes a colour in
the space a person picks one in and converts it on the way out, so the constants now
READ in sRGB and cannot be wrong. `a_colour_written_in_srgb_arrives_linear` checks the
conversion against values anybody can verify by hand.

**Swept the rest.** `build::shape` already converts (`block.colour.to_linear()`);
`biome::surface_color` is documented as returning linear and does; `chart` writes
`[u8; 3]` into an `Rgba8UnormSrgb` texture, which is the correct path; the sky's stars
are near-white in either space. The only raw arrays left are the maker's bench floor,
which is a dark checker either way and is not in the shipped world.

## A doorway you could see and only half walk through

**ISSUE.** Nothing was reported. This was found by building the cottage's interior
plan and measuring the mesh it produced, and it is the worst fault the town has had:
the cottage's visible doorway ran from **+0.16 to +1.35 m** across its front wall,
and the gap `Plot::walls` leaves for the player to walk through runs from **-1.10 to
+1.10**. So a quarter of the door you can see was solid, and 1.25 m of blank plaster
beside it was not. Every building in the game had it.

Two more numbers were wrong on the way there. The doorway was **1.195 m clear**, not
the 1.9 m `DOOR_WIDE` says and the research doc repeated; and a comment on
`DOOR_CLEAR` said the built doorway was 1.4 m, which it never was.

**SOLUTION.** One cause, three symptoms. `_bays` splits a wall into equal bays and
puts the door in the middle one - and the middle bay of SIX is not the middle of the
wall, it is 0.75 m off it. The clear opening was then `min(DOOR_WIDE, bay - 0.3)`,
which on a 1.5 m bay is 1.2 m and never 1.9.

- A wall with a door in it now gets an **odd** number of bays, so the door bay is the
  middle of the wall. That is what a facade with a central entrance has anyway.
- A door bay takes the width a door needs and the rest of the wall gives it up
  between them, so `DOOR_WIDE` is what gets built.
- `bay_places` is the only thing that knows where a bay is. `wall_run`, `framing` and
  the figures all ask it. There were three copies of that arithmetic.
- `Building::walk_in` sizes the collision gap from the opening the model was built
  with, and a tower gets its lobby (3.04 m) rather than a cottage's door.

**Measured, both ways.** `dev/art/town.py` walks across each figure's front wall at
three heights, finds the gap, and refuses to build if it is not centred and the width
the constant claims. Those numbers go into `assets/models/town.txt`, and
`the_doorway_you_can_see_is_the_one_you_can_walk_through` checks every one of them
against the gap the game leaves. Blender proves the mesh matches the plan; Rust proves
the plan matches the game. Neither guard can pass by comparing a number to itself.

## A chimney two and a half metres from its own fire

**ISSUE.** Every cottage in the world had a flue coming down through the roof onto an
empty corner of the room. The townhouse was worse: fire at `-wide * 0.25`, stack at
`(wide * 0.5 - 0.6, -deep * 0.15)` - **the opposite corner of the house**, six metres
across and five back.

**SOLUTION.** `fireside` is one expression and both are told it. The same shape as the
bay-grid fault above and the colour-space fault before it: one fact, two derivations,
in two places that nothing ever put side by side. Neither line is wrong on its own,
which is why reading either of them finds nothing.

`the_chimney_comes_down_onto_its_own_fire` checks the built stack's footprint against
the built fireplace's, off the mesh.

## A window cut through a chimney breast, and a stud through a front door

**ISSUE.** Two more found by the same measuring pass. The bay rule alternates windows
along a wall and cannot know what is behind them, so it cut one straight through the
cottage's chimney breast - from the street a window with a wall of stone in it. And
`shell` recorded its openings keyed on the wall alone, so on a two-storey house the
loop wrote the ground floor's bays and then **overwrote them with the first floor's**,
which has no door in it; `framing` then framed the ground floor believing there was no
doorway and stood a timber post in the middle of the townhouse's front door, at the
exact height a warden walks.

`framing`'s own docstring describes fixing that fault. It was fixed for buildings with
one storey.

**SOLUTION.** `blind_behind` makes solid any bay a fireplace stands against, and
`shell` keys its openings per storey.

## A shutter hanging off the corner of the house

**ISSUE.** A shutter is hung outside its window frame, and on the last bay of a wall
that put it 26 cm past the corner, in mid air under the eave.

**SOLUTION.** `_dress_window` is told how much wall there is either side of its bay
and narrows the leaf to fit. Both leaves, to the same width: narrowing only the one
that would not fit gives a window with a wide shutter on one side and a thin one on
the other, which reads as a mistake rather than as a shutter against a corner - and on
the cottage that is both its front windows, so the whole village wears it.

## Two thirds of a cold start spent on a feature that is switched off

**ISSUE.** `RIVERS` has been false for a long time. `build_chunk` called
`build_river` anyway, which samples a 65×65 grid where every sample begins with
`drawn_height` - four terrain heights - and returns `None` for every chunk.

Measured over the 253-chunk view disc with `--measure stream`:

    build_chunk   1360.6 ms -> 461.7 ms
    of it ground   427.9 ms    458.2 ms
    the rest       932.7 ms ->   3.5 ms

**SOLUTION.** `if RIVERS { build_river(..) } else { None }`.

The switch was honoured everywhere it was visible - `terrain.rs` carries three
`if RIVERS` guards - and not on the one path with no visible symptom, because
nothing looks wrong about a river that is not there. Found by Codex's audit.

**And the ruler came first.** `--measure stream` exists because every performance
claim in this project's history that was not measured turned out to be about the
wrong thing. It times `build_chunk` rather than the functions underneath it: timing
`build_river` directly would have gone on reporting the same cost after the call to
it was removed. It is honest about its edges - it cannot see frame time, GPU passes
or mesh upload, and those still need Tracy on real hardware.

## A shot called `night_node`, taken at noon

**ISSUE.** The photo matrix has three viewpoints documented as "the lighting
evidence, at the hours it has to be judged at". All three were photographed at
`EVIDENCE_HOUR` - midday - because a run had an hour and a shot did not. Nobody had
opened the files.

**SOLUTION.** A `Shot` carries its own hour; the lighting ones ask for 22:00.

Worth more than the fix: an instrument that reports the wrong thing under a
convincing label is how a fault survives a review. Every check in this file exists
because something was measured; the corollary is that a measurement nobody looks at
is not evidence, it is decoration.

## A test that passed with the change taken out

**ISSUE.** `let_it_fall` hid all 800 raindrops every frame the sky was clear, which
marks 800 components changed a frame to say nothing happened. Fixed two ways - an
early return once the pool is down, and a comparison before every visibility write -
and a test written to prove it.

The test passed with the early return **deleted**. Counting writes cannot tell a
system that skipped its loop from one that ran it and found nothing to change.

**SOLUTION.** The test now also shows one drop from outside while the sky is clear:
a system still iterating puts it straight back down, a settled one never looks. That
doubles as writing down the contract the gate depends on - the pool belongs to
`let_it_fall`, and nothing else may set a drop visible.

Each half of the change was confirmed to go red on its own before being kept. A test
that passes the moment it is written has not been shown to work yet.

## Lit windows on the plaster, and two more hanging beside a chimney

**ISSUE.** `light_the_windows` placed its lit panes from the building's **lot
footprint**: two across the front at 24% of the width, one halfway down each flank,
on a 0.9 × 1.15 pane at 1.7 m. Not one of those numbers was true of any building in
the world — the lot is what a building keeps clear on the ground, and it is bigger
than the building.

`Building::storeys` made it worse. It said a cottage had two storeys; a cottage has
one, so half its windows were lit at 5.3 m on a wall that stops at 3.6 — out in the
air above the eaves. The shop and the guild hall were wrong too.

Invisible until somebody photographed a village after dark, which nobody had.

**SOLUTION.** Blender measures the glass it builds — a `glass` box's thin axis is the
wall it sits in and the way it faces — and writes every window into `town.txt` in the
game's frame, stood proud of its wall. The game reads them. It no longer has an
opinion about where a window is; it only decides which are lit. The floor count comes
from the windows themselves, and `storeys` delegates to `facade` instead of repeating
it.

**What checks it.** `the_lit_panes_are_where_the_glass_is` compares two independent
measurements: what the cottage plan derived from its bay grid, and what was measured
off the glass that got built. That is the only check available that is not a number
against itself — the game cannot verify a window position on its own, which is
exactly why it used to invent one.

**And what the second check taught.** `every_lit_pane_stands_on_a_wall_of_its_own_building`
first asserted that every window sits ON a wall at the footprint boundary. It failed
twice, both times on real architecture: the townhouse's **jetty**, whose upper storey
genuinely oversails the ground it stands on, and the guild hall's **tower**, set back
well inside the hall's footprint with its own windows fifteen metres up. A check has
to claim only what it can know; it now says only that no window hangs off the end of
the building.

## A fence measured by height, when height was the wrong property

**ISSUE.** `Building::fenced` answered with a gate width alone, so every fenced yard
was taken to be closed on four sides. The city's service bay is closed on three - it
is a loading bay - and the game fenced its open mouth anyway. The old-world gate
widths were wrong too: 3.06 and 2.2 against a measured 2.92 and 2.06, the difference
being a gatepost.

**SOLUTION.** `dev/art/yard.py` measures each side's largest hole and writes
`assets/models/yard.txt`; `fenced` reads it.

**The ruler was wrong three times first, and that is the lesson.**

1. Counting anything within 35 cm of the line caught the BOLLARDS 30 cm inside the
   bay's mouth, and reported a five-metre gateway on a bay with no front run - the
   exact fault the measurement exists to catch, manufactured by the measurement.
2. A height band then called the city green's 34 cm KERB a fence on all four sides: a
   walled box with no way into it.
3. Height cannot separate a fence from a kerb here at all. The garden's fence is one
   rail on 72 cm posts, topping out at 38 cm; the kerb tops out at 34.

**Pick the property that differs by an order, not by a margin.** Height differed by
12% and gave three wrong answers. Thickness differs by 3× - a rail is 9 cm through, a
kerb is 34 - and gave the right one immediately.

## A rule that was really a frame-rate rule, and the tests that could not see it

**Issue.** `may_step` ended with `rise <= STEP_UP` — a 26 cm allowance meant for
kerbs and doorsteps — and the rise it measured was one frame's movement. A jog
covers about 9 cm in a 60 Hz frame, so the clause bought a slope of nearly 3:1
against a `CLIMB_LIMIT` of 1.4, and about 11:1 at 240 Hz where a frame is 2.3 cm.
The canyon walls gated the world on a slow machine and not on a fast one.

Nothing caught it. Every test of the rule took a single sample of a comfortable
size — 1.5 m — which is a look-ahead, not a stride, and at that distance the rule
gives the right answer.

**Solution.** Ask over a distance the frame cannot change. `STEP_LANDS` is 0.6 m,
about a stride, sampled at four fixed places along it, and a step is a rise with
nothing along the way higher than the step itself. The rule and the sampling are
separate functions (`may_climb` takes the ground as a closure) because there is no
way to put a metre-high, twenty-centimetre-thick ridge into generated terrain, and a
rule that can only be asked about real ground can only be tested on the shapes real
ground happens to have.

**And the general lesson.** A test that takes one comfortable sample of a rule is
testing the sample, not the rule. Where a rule consumes a per-frame quantity, drive
it at several frame rates and require the same answer — `dev/evidence`'s playtest
driver does exactly this on the assembled game, and reproduces the fault as five
routes whose verdict flips between 30 Hz and 240 Hz.

## A road drawn five metres wider than it could be walked

**Issue.** `RoadSection` was written so a street's cross-section is decided once.
Two commits later `pave` still computed its own shoulder — the full 5.4 m country
verge — while `stands_on` had moved to the section's, which closes to 35 cm as the
paving arrives. Every city street was drawn with a five-metre brushed fringe that
the warden could not stand on. Reported twice as "there's still that gradient next
to them", and both times I looked at the terrain rather than the road.

The same fault a second time in the same file: `stands_on`'s cheap reject used
`wide * 0.5 + SHOULDER_WIDE`, which is the section's reach at wander 1.0, while the
section it then built was scaled by a wander of up to +17%.

**Solution.** Both bounds belong to the section. `pave` consumes `cut.shoulder`;
`RoadSection::most_it_reaches` owns the reject's bound. When a fact has two
derivations, the fix is never to correct the second one — it is to delete it.
