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

## The walk, issue by issue

Getting one 24-frame walk to read as walking took most of a working day and about thirty
distinct faults. This is the index: every issue as it was actually seen, and the thing
that fixed it. Several have fuller write-ups above; this exists so none of it has to be
rediscovered, and so **the same list can be checked against the jog before tuning it.**

The column that matters is the second one. Roughly a third of these were fixed by
tuning a number, and two thirds were structural — the wrong joint, the wrong basis, or
two pieces of code disagreeing. Tuning against a structural fault is what burned the
most time, every time.

### The rest pose — before a single frame was authored

| Issue | Solution |
| --- | --- |
| "no human has spherical bones" — every joint drawn as a ball | The importer's `armature_display` hangs an Icosphere on all 41 bones as `custom_shape`, which overrides `display_type`. Clear the shapes; hiding the Icosphere OBJECT does nothing because bones reference the datablock. `prepare_rig.drop_the_widgets` |
| "Bones should reach the top of the head and ends of feet and hands, they dont" | glTF stores joint positions and no lengths, so leaves get invented ones (`Head` 2.6 cm on a 27.8 cm head). Set each leaf to the geometry it drives, ONE length per L/R pair or the sides desynchronise. `prepare_rig.reach_the_ends` |
| "what is this long angled bone" | `Root` and `Hip` span 85 cm because Root sits on the floor and its child is at the pelvis. Shorten to 20/12 cm. `sane_root_and_hip` in the build; `shorten_the_controls` for animated files, where redirecting would corrupt the imported keys |
| "the skeleton isnt centered on the mesh… the blue is what I see and the red is whats there" | The spine sat 1.67 cm off the mesh midline, so every spine rotation swung the torso about the wrong axis. Move central bones onto the limb midline, then shift ALL bones onto the mesh's silhouette centre. `centre_the_skeleton` |
| The two sides 5.45 cm from mirrored | Average each pair across the midline, rolls last — an unmirrored roll hands its children a mirrored head and a wrong basis. `make_the_sides_mirrors`, now 0.0000 |
| A 17.5° crouch baked into the bind | A-pose it and bake as rest, with `KNEE_EASE` 2° of forward fold — a dead-straight chain is IK-singular. `stand_in_an_a_pose` + `bake_the_pose_as_rest` |
| Soles not on the floor | `put_it_on_the_floor` |
| Feet visibly toed out while the log said 0.0° | Holding each foot's bind orientation meant nothing ever read `TOE_OUT`; they baked 17.65° out apiece. Yaw the bind about world up to reach the target, and guard the BAKED heading per side |
| "the backpack straps attached to the arms", gloves fused to pockets | Cross-limb weight repair, shared by the build and the animator. `unfuse.unfuse_the_gloves_from_the_pockets` |
| Shoes reading as "just chunky", mesh torn into shards | Welding coincident vertices destroyed glTF's split-vertex hard-edge encoding, leaving custom split normals describing a topology that no longer existed. Every numeric check passed while it looked destroyed. Weld removed; `unfuse.cloth_pieces` welds virtually instead |

### The feet — the single biggest time sink

| Issue | Solution |
| --- | --- |
| "there are no separate toes so the walk goes heel → false toes, but there is no bend because toes dont exist" | Give `ToeBase` a real joint at the ball, and flex it |
| "We're still heel walking" — after five separate rounds of pitch tuning | **The pitch was never the problem.** `ToeBase` ran horizontally at ankle height, its head at 46% (L) and 33% (R) along the shoe, 8.4 cm up — so the foot see-sawed about its own arch. The ball is the 1st MTP joint at 70–79% of foot length. Move it there, a third up the shoe's own section, tail at 97%. `put_the_ball_where_the_shoe_bends` |
| "The back foot isnt using toes, both feet need to go heel → flat → toe" | Toes held flat while planted (`flat_bend`, capped by `ik_gait.TOES_BEND_UP_TO`), easing out over the first quarter of swing so the foot leaves pointed |
| "feet still angle oddly to the side" | Two foot conventions differing by 7.45° — the bind's own dip. One shared `ik_gait.how_the_foot_turns` |
| "the knees bend inward" | Pole-target search scored on the knee tracking over the toe |
| A planted foot sliding 13.6 mm per cycle | Bezier auto-handles overshoot between keys, and the exporter resamples the overshoot. Force LINEAR. `animate_ranger.make_it_linear` (0.92 mm) |
| Every foot reading as planted on every frame, silently eating every arc fix layered on top | A 0…1 phase float was passed where a 0…7 pose STEP was expected, and every float is below any stance count |
| Peak thigh extension short by 1.7°, nothing refusing | `own < share` is half-open, so the boundary — which IS a keyframe, and the most extended pose — read as airborne and its sole was pushed 1.61 cm off the floor. `ik_gait.the_foot_is_down`, and it also restored all three clips' authored duty factor |

### The motion

| Issue | Solution |
| --- | --- |
| "the torso is still a bit leaning back" (reported three times, while the number said forward) | Lean the spine BEFORE aiming the arms — aiming first meant the parent then carried them 7° back. And a loaded walker leans into the load: the backpack's mass sits behind him, so upright reads as reclining |
| "the left arm is jumping" | A local named `reach` (the arm angle) was rebound to a leg length, killing all forward swing. Name locals for their SUBJECT. Plus `close_the_loop` to make frame 1 identical to the last |
| Arms at 72% of their authored range, 14° arriving as +2.7° | Composing rotations about different axes COUPLES them; authored degrees stop meaning degrees. State the target direction and turn onto it by shortest arc |
| "both knees are in front of the hips so there's no way he'd be balanced" | Drove hip-ahead-of-both-feet to 0 frames of 25 |
| "The character is sliding backwards" | The feet barely left the ground; per-gait `swing_lift` and a `swing_shape` that skews where the arc peaks |
| "There's a bounce and jitter first" | Ride height was hand-set and the reach limit was treated as a TARGET rather than a ceiling, so the hip lurched 16 cm in one frame. Derive it: a cosine fitted under the ceiling's deepest point, floored at `HIP_DROPS_AT_MOST`. Worst step 1.02 cm |
| Double support missing entirely from the walk | `share` was capped at 0.5, which is the definition of a run. Cap removed |
| "the legs are a bit too bent, make him slightly more upright" | `ik_gait.STANCE_LEG_EXTENDS` 0.98 |
| "Movement is far too fast" | `set_speed` is a MULTIPLE of a clip's natural rate, not a rate — and the multiple depends on the clip's own frame count. Measure what a cycle covers off the planted foot |
| A clip left the rig posed, and the next clip inherited it | Rest the pose at the top of every frame |

### The instruments — which is where most of the real time went

| Issue | Solution |
| --- | --- |
| A bake produced bones in A-pose and mesh in crouch, and the guard passed | The guard compared the result against its own INPUT. Every guard now compares against the SPECIFICATION — soles at 0, arms at 45° |
| A guard reporting "the chest is now 73% spine" while the jacket tore into triangles | Numeric guards cannot see shading or silhouette. Render the pose that exercises the change and LOOK |
| Two fixes in a row not moving a number at all | The number wasn't connected to the knob — stale pycache, a clamped share, a wrong parameter type. Stop tuning and trace it |
| A flatness guard failing three rebuilds at a constant 0.86 cm | Its radius reached the OTHER shoe. A constant residual across code changes means the guard measures something else |
| A refusal contradicting itself — "one half bobs 0.0224 and the other 0.0224, a ratio of 0.02" | A shadowed local again (`rise`). Second one in a day |
| A threshold refusing the sprint while suiting the walk | An absolute per-frame hip limit can't work across gaits; compare against the bob's own SHAPE |
| A 22°→17° regression sailing through | `THIGH_REACHES_BACK` is 12.0, the anatomical floor. A guard at the bottom of a legitimate range cannot see a regression inside it |
| An A-pose lost mid-build | A live session someone is also clicking in is for finding numbers, never a build substrate. Builds run from source via a script that re-derives everything |
| `action.fcurves` raising `AttributeError` | Blender 5 moved it: slots → layers → strips → channelbags, via `anim_utils.action_ensure_channelbag_for_slot` |

## The jog and the sprint, issue by issue

The walk's list above is mostly poses; this one is mostly FRAMES OF REFERENCE. Half of
these were a rotation authored against the wrong thing, or a ruler measuring against
something that moved with what it measured. None showed up as a wrong number — they showed
up as a correct number describing the wrong quantity.

### Frames of reference: the recurring fault

| Issue | Solution |
| --- | --- |
| "compressed back foot" on frames 9–12, while the shoe measured rigid (0.7 mm of length lost) and unyawed (0.0°) | `RUN_LEG` authors sole pitch against the FLOOR. Right while planted — the floor is what the sole rests on — and wrong in swing, where the shank sweeps most of a right angle and a near-flat sole leaves the ANKLE JOINT to absorb the difference. Measured +65° of dorsiflexion against a human running range of about −25..+30: the toes hauled up into the shin. The swing entries went from −14/−12/−6/−2 to −30/−36/−34/−18, each set by subtracting the dorsiflexion that frame was carrying |
| "forearms are more outward" instead of across the front | The elbow folded about `FOLDS_THE_ELBOW`, a FIXED armature axis, so the hinge plane did not follow where the upper arm pointed — with the arm hanging out to the side, folding threw the hand laterally. An elbow is a hinge and cannot carry the hand inward; shoulder INTERNAL ROTATION does. The axis is derived per frame now as `upper × heads`. Hands went 24.9–27.0 cm out from the midline to 8.1–13.6. See `FOREARMS_TUCK_IN` |
| The hand visibly twisted the moment the elbow fold went from 62 to 88 | `PALM_IN` rotated the hand about a fixed world axis, which stops being pronation and becomes a twist once the elbow folds. About the FOREARM's own axis instead. Fixed in the gait path and MISSED in the idle path — the same bug in two places, hidden because the idle's elbow barely bends |
| "the elbows dont go back far enough", and more arm swing barely moved them | The elbow sits one upper-arm length from the shoulder, so its travel is capped at 26 cm — it was already at 25.8, 99% of the geometric limit. What was missing is that the TORSO never rotated at all. `WALK_TWIST`/`RUN_TWIST`/`SPRINT_TWIST` on Spine02 at 5/10/18°, applied BEFORE the arms are aimed so their angles are unchanged and only their origin moves |
| A 3 cm change in elbow travel that read as EXACTLY zero | The elbow was measured against the SHOULDER, and counter-rotation carries shoulder and elbow together, so that offset is blind to it. Against the PELVIS, which does not twist, the same change reads 8.9 → 13.4 cm. **A ruler fixed to the thing it measures cannot see what you added** |
| "something still seems off about the characters balance" | The gait leaned `Waist` and `Spine01` and countered nothing above them, so the head inherited the whole lean and led the body by 8.7 cm. `HEAD_HOLDS_BACK` takes 65% back out at the neck. The idle ALREADY did this — Spine01 +1.0 against NeckTwist01 −0.8 — so the gait was the outlier |
| The lean bending him at the ribs rather than tilting him | It was 40% waist / 60% chest. A runner does not curl forward; the whole body tilts from low down and the trunk stays a straight line. `LEAN_AT_THE_WAIST` 0.8 / `LEAN_AT_THE_CHEST` 0.2 |

### The flight phase, and why it needed a structural change

| Issue | Solution |
| --- | --- |
| The jog read as a fast walk: flight 12.5% where the reference wants 25%, feet 1.3 cm off the floor on the airborne frames | The DURATION was right and the HEIGHT was not. `fill_in_the_flight` and `RUN_BOUND` — a 3.7 cm ballistic arc — had been defined and never called since a `gait()` rewrite orphaned them |
| Wiring the arc in made it LIMP, hips failing to repeat by 20% | The cycle wraps and a list does not. `fill_in_the_flight` arcs between KNOWN indices, so the airborne stretch straddling the seam had no known index after it, fell through to "hold the nearest", and got no arc at all — one bound of the cycle got its full arc and the other none. Filled over two cycles, taking the FIRST copy (the second has its own unfilled tail) |
| The arc then made a 2.73 cm one-frame hip step, refused as a bounce | With only 2 airborne frames per stretch a parabola cannot be a parabola: it plateaus and then drops off a cliff. `fill_in_the_flight`'s own comment warns of exactly this |
| Only 4 airborne frames, and no way to get more | `share = stance / POSES` could only say 0.375 or 0.25 in whole eighths, and a jog's duty is 0.333. At 0.375 the closed stance interval covers 10 of 24 frames per foot, leaving 4 airborne. Stance is given as a SHARE now, not a count of poses: `RUN_SHARE = 1/3` gives 9 planted and 6 airborne, which is both the reference shape and the room the arc needed. `verify_gait` takes a share too, and still accepts the old eighths |
| Tracking the reach ceiling per frame — which looked like the principled fix — was far worse | Hips rose 5.7 cm ABOVE bind height, moved 5.59 cm in one frame, halves disagreed by 92%. The ceiling also jumps at LANDING, not only where nothing is down, and tracking it inherits the mesh's left-right asymmetry that a fitted curve averages away. A fitted cosine PLUS the arc, not the ceiling |

### Fixed by fixing what a number meant

| Issue | Solution |
| --- | --- |
| "the leg locks at 0.3°" at contact | It did not. That was a whole-cycle MINIMUM and it was picking up toe-off, where a straight leg is correct — that is the push. The knee at contact measured 14.3° against Heiderscheit's 17.8 ± 4.0, and always had. The metric reports the LANDING knee now |
| `verify_gait` refusing a heel strike on any clip with a flight phase | It asserted "a run lands on the forefoot", which is a style claim wearing a correctness check's clothes — and false at jog pace (Breine: zero forefoot strikers in 52 runners at 3.2 m/s). Worse, asserting the strike left flying clips with NO reversal check, since a backwards run lands toes-down at the front, which is what the branch demanded. It checks that the leading foot is more toes-up than the trailing one — true of walk, jog and sprint, false the moment the cycle reverses |
| The sprint's elbow refusing as "folds backwards", and TIGHTENING the elbow making it worse | The intuition is that an arm thrown too far BEHIND causes it. Measured per frame, it was the FORWARD arm: 15.05 cm behind the line at the back extreme and 1.69 cm in FRONT at the forward one, because `elbow_swing` ADDS at the front. The crossing sits near 106° of fold — 105.9 put the elbow 1.32 cm behind, 107.3 put it 0.94 in front. `SPRINT_ELBOW_HELD` 94 ± 4 |
| Frame 1 with nothing touching the floor, its ball 9.5 cm off the path the other stance frames sat on | `close_the_loop` copied frame `span+1` onto frame 1 because "the last frame is computed with everything in place" — but a later change had made `span+1` a verbatim copy of frame 1's pre-bake targets, so the copy laundered frame 1's own cold solve back onto itself. Replaced by `LEAD_IN` frames of run-up before frame 1, discarded after the bake: the phase formula wraps for negative frames, so frame 0 and frame `span` are the same pose and the seam closes by construction |

### The backpack

| Issue | Solution |
| --- | --- |
| The pack moving oddly through the cycle | Skinned across `Spine01` (49%), `Spine02` (20%), `Waist` and `Head`. A RIGID object spread over four bones that rotate differently must shear: measured, 3.25 cm on a 73 cm diagonal, 4.4%. Split into its own object, rigid on one bone; distortion is now 0.000% |
| Whether it could be a separate object at all | Checked by rendering both halves BEFORE cutting: the pack comes away as a recognisable bag and the jacket back is INTACT, because the pack is additive geometry over the garment rather than a panel cut into it. There was no hole to patch. It is not its own connected shell though — the mesh is 1442 shells over 7584 vertices — so it needs a selection rule, not a topological split. `split_out_the_backpack` |
| The separate went BACKWARDS: pack 7578 vertices, body 0 | `polygon.select = False` in object mode leaves the VERTEX selection untouched, and `separate(SELECTED)` reads that — a freshly imported mesh arrives fully selected and separates whole. The deselect must go through the edit-mode operator |
| A conservation guard refusing a correct split: 7255 + 370 against 7578 | `separate` DUPLICATES the seam into both objects — 47 vertices, exactly the pack's boundary. Correct behaviour, wrong guard. It refuses on LOSS, and on growth beyond what the selection's boundary could account for |
| Several tools would now hand a 370-vertex bag to code measuring the ranger | They took the FIRST skinned mesh, which was fine while there was one. `prepare_rig.the_body()` — largest wins, no name test, because glTF suffixes duplicate names on round trip. `animate_ranger` already did this; `gait_watch` and `ranger_blend` did not; `verify_gait` turned out to work purely off bones |

### The sprint limp, and three wrong guesses

**Status: open.** Worth its own entry because the diagnosis is solid and the fix is not applied.

Two refusals survive: the thighs disagree 7.99° half a cycle apart, and the hips fail to
repeat by 42%. Three hypotheses were tried, and the first two were wrong:

* **Foot landmarks carrying the mesh asymmetry.** Shared them between the sides
  (`make_the_landmarks_mirrors`) — 8.25 → 7.97°. Kept, because the motion should not
  inherit mesh asymmetry either way, but it was not the cause.
* **Pole-angle quantisation.** The pole is searched once per side on a 36-step grid, so each
  leg carried a standing error of up to 5° in whichever direction its own grid point fell.
  Refined to 1° — 7.97 → 7.99. Kept, and equally not the cause.
* **Solver history.** `LEAD_IN` from 3 to 12 frames changed it by nothing at all.

Traced rather than guessed, the answer was unambiguous. The authored motion matches to
**0.00° on 20 of 24 frames**; ankle FORWARD and SIDEWAYS placement are identical to the
digit; only ankle HEIGHT differs, by up to 5.14 cm. On airborne frames that height is set
by `rest_the_shoe_on_the_floor`, which reads each shoe's own deformed sole — and the two
shoes sit about 4 cm differently on their bones.

**The fix, when someone wants it:** share the sole clearance between the sides for AIRBORNE
feet only, keeping each foot's own geometry while planted. An airborne foot only needs to
not penetrate the floor, so precision there buys nothing; a planted one needs its real
sole. That removes the asymmetry from exactly the frames it appears on. It touches the
floor solve, which all three clips depend on, which is why it is written down rather than
done.

**The lesson, which is this whole section's lesson:** three code changes were made before
anything was traced. Two were harmless, one was wasted effort. Trace first — the shape of
the divergence names the cause, and here it named it in a single measurement.

### ISSUE: the head bob measured exactly no effect, before and after every change

**What you see.** The run reads stiff — reported as "from the side the run resembles the
old 'scooby doo character' run". The named cause is a lack of overlapping action: when the
torso twist, head bob and limbs all ride one phase, a character reads as a rigid toy rather
than as alive. So a head bob was added, with a follow-through form so the head trails the
chest instead of riding along with it. Head travel measured **6.29 cm**. The amplitude was
raised. Still 6.29 cm. Changed to a different formulation. Still **exactly** 6.29 cm.

**What it actually was.** Nothing to do with the maths. `key()` in `animate_ranger.py`
inserts a `rotation_quaternion` keyframe for every pose bone but only inserts `location`
for a named few:

```python
if posed.name in ("Hip", "Root"):
    posed.keyframe_insert("location", frame=frame)
```

`head.location` was being set on the pose and then thrown away — never keyed, so never
exported. The 6.29 cm was the head being carried by the hip and the spine, which is why it
did not move when the head's own term did.

**What changed.** `"Head"` added to that set. Head travel went 6.29 → 13.50 cm, and the
head's peak moved to frame 10 where the hip's is at 11 — the overlap the term was for.

**The principle.** *A number that does not move when you change its input means the knob is
not connected.* Identical output across three different formulations is not weak effect, it
is no effect, and the next place to look is the plumbing rather than the model. This is the
second instance of exactly this fault in this file — see the pelvis sway measuring 0.00 cm
because `Pelvis` is a connected bone and Blender ignores `location` on those. Both times
the code read correctly and nothing reached the file.

**The test.** None yet, and it should have one: a guard that every bone the authoring code
writes `location` to is also in `key()`'s list would have caught both instances. Worth
adding, because the failure mode is silent by construction.

### ISSUE: the jog felt "like running through water", and the bound existed three times

**What you see.** In-game movement reads sluggish, repeatedly, across several tuning passes.
Every authored speed gain got given back somewhere else.

**What it actually was.** Two things, and the second is why the first kept happening.

`JOG_SPEED` was pinned just under the run's handover ceiling, and that ceiling came from
`CHURNS_ABOVE`, which was the **human** running band of 150–200 steps a minute. So the
speed was not a tuning value at all — it was whatever a human cadence band permitted. It
could not be raised without the guard refusing it, and the guard was enforcing realism in a
fantasy game about collecting and raising monsters.

Worse, that band existed in **three** places with three different values:
`CHURNS_ABOVE` said 140/200/260; the churn test carried its own `(90,140)/(150,200)/
(220,260)`; and `each_clip_is_authored_near_the_time_its_stride_takes` bounded `stretch` to
`(0.4..2.5)` — which, since `stretch` is exactly `lasts * cadence / 120`, is a magic 300
steps a minute in disguise. Raising one refused on another.

**What changed.** One table, `CHURNS_BETWEEN`, which the ceilings and both guards derive
from. Its tops were then raised as a deliberate stylistic choice — run to 235, sprint to
290 — and the speeds moved with them: jog 2.39 → **2.81 m/s**, sprint 4.10 → **4.58 m/s**.

Note what was NOT needed: stride warping. Research pointed at it, and it is the wrong fix
here — `playback_rate` is already unclamped and driven by measured `covers`, so it absorbs
the whole mismatch and the correct stride scale today is exactly 1.0. Adding `actual /
authored` on top would have multiplied, 2.40 × 2.40 at the sprint.

**The principle.** *A bound that exists twice is a bound that drifts*, and a bound that
exists three times will refuse a legitimate change from a copy you forgot about. Derive
guards from one another so they agree by construction rather than by a coincidence of
factors — the same argument already written on `hands_over_above`.

**The test.** `the_gaits_churn_like_a_person_at_the_speeds_they_are_driven` and
`each_clip_is_authored_near_the_time_its_stride_takes`, both now reading `CHURNS_BETWEEN`.
The first says the band it came from; the second reports the multiple *and* the range its
cadence band allows.

**The lever still on the table.** Cadence and stride multiply into speed, and stride is the
higher-quality half — it buys speed without churning the legs faster. That is an authoring
change in `animate_ranger.py`, not a constant, and it has not been done.

### ISSUE: the warden jitters while running, "the frames are horribly messed up"

**What you see.** In game the run stutters badly, as though the animation frames were
corrupt. The clips themselves are fine — every loop seam measures **0.000 deg** and
**0.000 cm** last-frame-to-first on all three, so nothing was wrong with the authoring.

**What it actually was.** `Striding::speed` was measured like this:

```rust
let went = transform.translation.distance(before);
pace.speed = went / time.delta_secs();
```

Two faults in one line, and then a third that turned them into a visible stutter.

1. **It was a 3D distance.** The warden is planted on the terrain every frame, so `went`
   included the vertical travel of climbing or descending. On any slope the measured speed
   read *higher* than the ground speed.
2. **It was unsmoothed.** Frame-time wobble, a step clamped by `bounds`, and a step refused
   by `may_step` and retried per-axis all land in it as spikes.
3. **The gait SELECTION read it.** `find(|gait| pace.speed <= gait.upto)` — so a noisy
   number was choosing the clip. `JOG_SPEED` had just been set to 2.81 against a run
   handover of 2.83, a 0.7% margin, so on a slope the speed crossed the ceiling and
   uncrossed it on consecutive frames. Each crossing calls `moves.play(..)` with a `BLEND`,
   restarting a transition every frame. The playback rate was fed the same noisy number, so
   the clip's tempo flickered too.

**What changed.** The two jobs got separated. `Striding` now carries `wants` — the ASKED
speed, exactly one of three constants — alongside `speed`, the measured one. Selection reads
`wants`, so it cannot chatter by construction. The playback rate still reads `speed`, which
is what a measured speed is actually good for, but that is now **horizontal** (`.xz()`) and
settled at `SPEED_SETTLES = 16.0` per second, with a `BLOCKED_STILL_RUNS` floor so a warden
shoved against a wall keeps running in place instead of freezing on one pose.

**The principle.** *Choose from intent, scale by measurement.* This is also how the games
this one is measured against do it — Genshin Impact drives locomotion from a discrete
movement state and only uses velocity to scale the clip once the state has picked it. And
separately: a threshold and the value it is compared against should never be tuned to
within a percent of each other, because anything noisy in between becomes a per-frame flip.

**The test.** `every_handover_separates_the_speeds_it_sits_between` — it demands each
handover sit clear of both speeds it divides by a fifth of the gap, so the 2.81-against-2.83
arrangement is now a build failure rather than a stutter to be discovered in game.

### ISSUE: "still VERY slow moving" — the speed was never a knob

**What you see.** Movement stays sluggish across pass after pass of tuning. Raising a speed
gets refused by a test, or gets given back somewhere else.

**What it actually was.** The speeds were *derived*, not chosen. `GAITS` set each tier's
ceiling with `hands_over_above(covers, CHURNS_ABOVE.n)` — the speed at which cadence would
leave a believable band — and then `JOG_SPEED` was set just under that ceiling. So the
driven speed was not a design decision at any point. It was whatever a **human** cadence
band permitted, and every attempt to raise it hit a guard enforcing realism in a fantasy
game about collecting and raising monsters.

**What changed.** The dependency was inverted.

* `player::WALK_SPEED` / `JOG_SPEED` / `SPRINT_SPEED` are now the primary knobs, chosen by
  feel against Genshin Impact as the reference for movement and fluidity.
* `halfway()` replaced `hands_over_above()`: a handover is simply the midpoint between the
  two speeds it separates, which also gives the widest possible margin either side.
* `CHURNS_BETWEEN` stopped being a speed gate and became an **absurdity** bound, widened to
  60-180 / 140-330 / 200-400 so it has no opinion about a chosen speed. Its remaining job is
  catching a broken `covers`: cadence is `speed / covers`, so a mis-measured stride shows up
  as an impossible cadence. 60 or 400 steps a minute is a bug; 300 is a choice.

Jog 2.39 → **3.40 m/s**, sprint 4.10 → **5.40 m/s**.

**What this costs, stated plainly.** Cadence and stride multiply into speed, and only
cadence moved. The jog now churns at **287 steps a minute** and the sprint at **346**,
playing their clips at 2.50x and 3.00x the authored rate. Fast legs are not the same thing
as fluid movement, so if it reads busy the fix is not another cadence bump — it is
**stride**, which buys speed without churning: `covers` is `foot travel during stance /
stance share`, so a bigger fore-aft leg swing in `RUN_LEG` raises it directly. Going from
1.419 m to about 2.04 m a cycle would carry 4.08 m/s at a calm 240 steps a minute. That is
an authoring pass in `animate_ranger.py`, not a constant, and it has not been done.

**The principle.** *A value you cannot raise without a guard refusing it is not a knob, it
is an output.* When tuning keeps failing, check which direction the dependency runs before
tuning again.

**The test.** `the_gaits_churn_like_a_person_at_the_speeds_they_are_driven` and
`each_clip_is_authored_near_the_time_its_stride_takes`, both reading `CHURNS_BETWEEN` — now
as sanity bounds rather than gates.

### ISSUE: the "Scooby Doo run" was a measurement error, not a pose

**What you see.** The run reads as churning without going anywhere. Reported across many
sessions as "running through water", "the scooby doo character run", and finally the one
that cracked it: *"the character model moves quick but movement itself is slow"*.

**What it actually was.** `motion::RUN_COVERS` said the cycle carried 1.801 m. Measured off
the shipped clip, the planted contact patch travels back **10.39 cm every frame** — dead
steady, spread 0.41 cm across the whole stance — which over a 24-frame cycle is **2.495 m**.
The number was **28% too small**, and the walk was 9% small and the sprint 36%.

`playback_rate` is `speed * lasts / covers`, so understating `covers` makes the clip play
that much too fast. The legs turn at a rate implying far more speed than the body has, and
leg cadence is what the eye reads as speed — so the legs say one thing and the ground says
another. At 3.70 m/s the run was turning at **290 steps a minute**, a sprint cadence
carrying a jog speed. With the true figure it is **178**, which is a jog.

The bad ruler was `verify_gait`'s `covers_implied_m` = `contact_length / stance_share`,
wrong twice over: `contact_length` is the AUTHORED sweep and the reach solve clips it, so
the ask is not the outcome; and the achieved figure is taken between two landmark extremes,
so it misses travel the foot does while rolling past them. It read 0.60 m where the foot
genuinely swept 0.83.

**What changed.** `dev/art/measure_covers.py`, which measures the outcome from the
invariant that actually defines `covers`: through stance the contact patch travels backward
at a constant rate equal to the body's forward speed, so `covers` is that rate times the
span. The per-frame spread is how you know it is trustworthy. Walk 0.881 → 0.970, run
1.801 → **2.495**, sprint 2.111 → **3.308**.

**How much was wasted on the wrong thing.** Everything. Speeds were raised four times, the
cadence bands were rewritten twice, the stride was pushed until the leg saturated, and a
leg-lengthening was proposed and nearly built — all to fix a symptom of one wrong constant.
Two rulers had to be fixed before the fault was even visible, and the first replacement was
ALSO wrong: it asked whether the contact patch was stationary, which is right for a clip
carrying root motion and nonsense for an in-place clip, and it duly reported 53 cm of
failure on the signed-off walk.

**The principle.** *Before tuning a value, measure whether the value is even being read
correctly.* And the corollary this file keeps re-learning: a derived quantity is only as
good as its landmarks. `covers_implied_m` looked authoritative because it had a formula.

**The test.** None yet, and it wants one: a Rust test cannot read a GLB's skinned
deformation, so the honest guard is a probe run alongside `animate_ranger.sh` that refuses
when the declared `COVERS` and the measured rate disagree by more than a few percent. Worth
adding — this fault was invisible for weeks and cost more than any other in this file.

### ISSUE: "the head bob is extreme", and three knobs that did nothing

**What you see.** The head pumps up and down far too much through the run. Measured, head
travel was **14.74 cm** against a hip rise of 11.60 — the head was *amplifying* the pelvis,
when a running body stabilises the head above all else.

**What it actually was.** Three separate faults stacked, each one masking the next.

1. `key()` wrote a `location` channel only for `Hip` and `Root`, so `head.location` was set
   on the pose and discarded. Every value tried measured the same 6.29 cm.
2. Adding `"Head"` to that list appeared to fix it — travel jumped to 13.50 cm. It did not:
   `TRUNK_PITCHES` was raised from 2.0 to 4.0 in the same edit, and that is what moved it.
   The head channel was still doing nothing. **Two changes in one build, and the wrong one
   got the credit.**
3. The damping term was `axes @ Vector((0, 0, z))`, and `axes` is the basis built for
   `Root`. A pose bone's `location` is in its OWN rest space, so on the Head that pushed in
   an arbitrary, near-horizontal direction. Which is why more than doubling
   `HEAD_RIDES_LESS` — 0.4 to 0.85 — moved travel 12.08 cm to 11.76.

**What changed.** The lift is built from world up and taken into the head's own basis with
`head.bone.matrix_local.to_3x3().inverted()`, and it is applied after the root block, since
`rides` does not exist before it — authoring a damping term without the thing it damps is
how it came out amplifying. Run head travel **14.74 → 4.93 cm**, head/hip **1.04 → 0.43**.

**The principle.** *Change one thing per build.* Fault 2 cost the most, and it cost it by
being a real fix whose effect was invisible because a second change was louder. And: a
knob doing a twentieth of what the geometry says is not a knob that needs turning further —
see the identical lesson on `RUN_SINKS` and on pelvis sway.

**The test.** None. A guard that every bone the authoring writes `location` to is also in
`key()`'s list would have caught fault 1, and asserting head travel stays under the hip's
would have caught faults 2 and 3.

### ISSUE: the Blender viewer showed a clip two hours out of date

**What you see.** Nothing. That is the whole problem. Asked directly — "are you updating the
blender pages?" — and the answer was no.

**What it actually was.** `gait_watch.sh` builds a scene and opens it, and the scene carries
a registered watcher that reverts itself when the file's timestamp changes, so a rebuild is
supposed to reach an already-open window without anyone closing anything. That half worked.
The missing half is that `animate_ranger.sh` rewrites the **GLB** and nothing rewrote the
**scene**, so an open window went on showing whatever clip it was built from.

Measured when it was caught: the viewer scenes were written at 10:53 and the GLB at 13:02 —
two hours and four rounds of changes apart, including the entire arm-swing and lean pass.

**Why it is worse than a bug.** A stale scene is never broken, only old, so there is nothing
to notice. It makes the reports coming back **unreliable** — feedback on animation that is
no longer what the build contains — and neither side can tell which round is being judged.
Every other entry in this file was found by measuring the wrong thing; this one is measuring
the right thing on the wrong version.

**What changed.** `animate_ranger.sh` now rewrites every `gait_watch_*.blend` that already
exists, as its last step, so the watchers fire and open windows reload themselves. Only
scenes that exist are touched: building one for a clip nobody has open would add a window's
worth of work to every run, and creating files nobody asked for is its own surprise.

Getting `win` for that meant the script had to stop carrying its own copy of `find_blender`
and source `blender.sh` — which is what `blender.sh`'s header already asked for, naming this
script first among four. The alternative was a fifth copy of a path helper in order to fix a
duplication problem, which is the wrong way round.

**The principle.** *If a person is going to judge the output, the thing they look at is part
of the build.* A pipeline that produces the artefact but not the view of it has an
un-versioned step in the middle of the feedback loop.

**The test.** The scene says so itself. Two layers, because the refresh alone is not a
guarantee — asked for one directly: *"make sure the blender always has the changes otherwise
I'll end up spotting the same issues that have been fixed"*.

1. `animate_ranger.sh` rewrites every existing `gait_watch_*.blend` as its last step, so the
   watchers fire and open windows reload themselves. This covers the ordinary case.
2. Each scene is **stamped** with the path and timestamp of the GLB it was built from
   (`built_from`, `built_at`, `built_clip` on the scene), and the in-blend watcher compares
   that against the GLB on disk every tick and **captions the viewport**: small and green
   when current, large and red when not — *"STALE — run rebuilt 14 min after this scene"*.

Layer 2 exists because layer 1 cannot cover everything. The watcher only ever watched the
**.blend's** own timestamp, so it notices a rewritten scene and is blind to the case that
actually bites: the GLB moving on while the scene does not. That happens whenever a build
refuses *after* `animate_ranger.py` has written the model — `set -euo pipefail` stops the
script before the refresh — or whenever the clips are rebuilt by any path that skips it.
Nothing changes, the watcher has nothing to notice, and the window keeps showing superseded
work.

Verified rather than assumed: the stamp survives the save, resolves its source, reads
"current" immediately after a build, and a GLB fifteen minutes newer reads STALE.

What neither layer can cover is Blender started without `--enable-autoexec`, since then no
registered script runs at all — no reload and no caption. `gait_watch.sh` always passes it.

### ISSUE: measuring a split mesh as though it were welded

**What you see.** Pale patches round the lower back and hip, and in game the impression that
"the legs are not connected to the torso".

**What it actually was.** Three small holes in the body, left by `split_out_the_backpack` —
`bpy.ops.mesh.separate` **moves** the selected faces out, so the body loses that surface and
you see its interior through the gap.

**Why it took four measurements to find.** Because every early measurement was taken on the
wrong topology. glTF encodes hard edges by SPLITTING vertices, so on the mesh as it arrives:

| measured on | boundary edges | "loops" | largest loop |
|---|---|---|---|
| the split mesh, as imported | 6975 of 10131 | 1362 | 29 verts |
| welded by position first | **140 of 6710** | **10** | 41 verts |

None of the first row is real. 7062 split vertices are 2302 actual ones, and until they are
unioned by position the word "boundary" means "hard edge" and every seam counts. The first
report — 1022 open edges at the waist — was that artefact, and 608 of those 1022 belonged to
the HANDS, which hang at hip height in an A-pose and are not the waist at all.

Welded, the ten real loops name themselves, and most are **meant** to be open: a 41-vertex
open chain on `Spine01`/`Spine02` is the jacket's zip, 38 closed vertices at the clavicles is
the collar, 26 open on `Head` is the hairline. Filling any of those would be a far worse bug
than the one being fixed. What was left was three closed punctures of 4 to 6 vertices.

**And then the fix went in the wrong place, twice.** The holes were measured on the EXPORTED
asset and the repair was wired into the top of `prepare_rig`, where it correctly found nothing
— the RAW export has no waist holes at all, only a collar, a neck and a hairline. Then after
the strap removal, still nothing. They are made by the very last step before export.

**What changed.** `split_out_the_backpack` now calls `bpy.ops.mesh.duplicate()` before
`separate`, so the pack is a COPY and the body keeps its surface. Six holes became three, and
the body renders clean at the back and hip.

**The principle.** *Measure the representation you are actually going to change.* A split
mesh, a welded mesh, the raw export and the exported asset are four different objects, and
this bug had a different answer on each of them.

**What is still open, and why it is not simply filled.** Three punctures remain, and
`fill_holes` cannot close them: it selects all 14 edges and adds 0 faces, because a loop that
is closed once welded is not a closed loop of real edges in the split mesh. Filling them needs
either a welded working copy or `edge_face_add` per loop.

**The test.** `close_the_holes_round_the_waist` reports what it finds every build, and
protects the openings that should stay open by size and closedness alone — the jacket's zip
is an open chain and the collar is 38 vertices, so neither can be caught by a cap of eight.

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
