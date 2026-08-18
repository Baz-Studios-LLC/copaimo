## A world with weather, ground cover, and places you can name

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This one is about what the ground under your feet is made of.

**Cloud shadows, and they are the actual clouds.** The usual way to do this is
scrolling noise, which looks fine until you stand in a patch of shade, look up,
and find clear sky. These come from the cloud list itself, one soft disc per
cloud at the point the sun's own line through it strikes the ground — so look up
from a shadow and the cloud casting it is overhead. They slide as the clouds
drift and as the sun climbs, and they fade out near dawn and dusk, when a cloud
two hundred metres up would throw its shadow a kilometre sideways.

**Tall grass you could lose something in.** Grass used to be single blades dotted
evenly over every field in the world, which is the same thin stubble everywhere.
The same amount is now *gathered* into meadows with bare ground between them, and
a meadow's middle comes up past the knee — a different kind of ground, visible
from across a field, and somewhere a wild monster could be without being seen.
It parts as you walk through it and springs back behind you.

Deserts don't get meadows. Dry scrub is sparse by nature, and gathering it into
lush patches would be inventing oases.

**Litter.** Boulders, scree, bushes, stumps, fallen logs, dead standing snags,
cactus and dry brush, keyed to where they belong. A wood gets its floor of
wreckage, bare rock sheds stone, dry country grows cactus and dead sticks, the
shore gets driftwood. A landscape of ground and trees reads as a golf course.

**Places you can name.** What kind of ground a point carried used to be decided
point by point from a moisture field, so desert appeared wherever the noise
happened to dip — patches inside grassland, no two hundred metres of the map the
same as the next. You could not say "the northern desert" about a place whose
character changed every time you walked a field's width.

The map is divided into countries now: the green world, the northern desert, and
the snow country in the east. There is no humidity model behind it — a region
simply *is* a country, which is the whole point. Boundaries are bands rather than
lines, so sand gives way to scrub gives way to grass across a walk.

**Snow country reaches its own shoreline**, with conifers standing on the snow
rather than a ring of green around every white island.

**Trees no longer transparent.** Every tube in every tree had been wound
inside-out, so a trunk was a crescent of its own dark interior and limbs behind
it showed straight through. Every triangle a tree emits is now wound to agree
with its own corners — walls, caps and leaves alike.

**Rivers are switched off.** The machinery is written, tested and shared with the
bench; what killed them was width. A channel's cut spreads over three times its
own width because banks do, so water at any useful depth spread about that far as
well, and across the network that came to a fifth of the land under water. Not
rivers through a landscape — a landscape with a lake on it. It wants solving
rather than tuning.

**F3 shows where you are on the map** — the coordinates the regions are written
in, plus which country has claimed the ground you are standing on.

### Testing on macOS

**Apple Silicon only** (M1 or later). The build is arm64; it will not run on an
Intel Mac.

Grab the **`.dmg`** and drag Ranger out of it, or install through the **Baz
Studios launcher** (which uses the `.tar.gz` and clears the quarantine for you).

The app is **ad-hoc signed** rather than notarised, so macOS quarantines anything
downloaded and may say *"Ranger is damaged and can't be opened"*. It isn't
damaged, it just isn't from an identified developer. Either **right-click the app
→ Open** and confirm, or:

```bash
xattr -dr com.apple.quarantine /Applications/Ranger.app
```

The world lives inside the bundle at `Ranger.app/Contents/MacOS/assets`, beside
the binary. Keep it there — macOS launches an app with the working directory at
`/`, so that folder is the only place the game can find its map.

### Playing it

`WASD` to move, `Shift` to sprint, mouse to look, wheel to zoom. `F` for free-fly
— `Q`/`E` down and up, `-`/`=` for speed. `F3` hides the overlay.

`F6` and `F7` push the hour back and forward so you can look at a dusk without
waiting for one; `F8` gives you back real time.

### Shaping it

**Shape the World** from the main menu. `1`–`9` pick a tool and `0` reverts,
drag to apply, right drag inverts, the wheel sizes the brush and `[` `]` set its
strength. Hold `Alt` to free the pointer and click the overview to fly somewhere.
`Ctrl+Z` and `Ctrl+Y` take strokes back and put them again — across ground, woods
and roads alike, in the order you did them. `Ctrl+S` saves all three. Ramp is
*clicked* rather than dragged: one end, then the other.

Sculpting is read at startup, so relaunch to walk what you shaped. An installed
build has its own copy of the world, so shaping one does not change the other.

The same tool is also a bench in
**[Opificium](https://github.com/Baz-Studios-LLC/Opificium)**, for shaping a
world without opening the game. See `DESIGN.md`.
