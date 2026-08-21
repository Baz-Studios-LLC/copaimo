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
