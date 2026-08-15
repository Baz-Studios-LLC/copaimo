## Trees worth looking at, roads that are roads, and a tool you can work in

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This one is about how the world reads and how it is shaped.

**The woods were regrown, twice.** Branches used to be aimed at the sky whatever
direction their parent was heading, so a canopy came out as a fan of parallel
canes on a trunk the width of a broom handle, and every tree in the world wore
the same green. A limb now leans from its *parent*, low limbs go out near
horizontal and sag under their own weight, and the crown is broad at its foot and
narrow at the top. Trunks are drawn from the tree's height — 0.20 m to 1.33 m
across the pool — heights run 5 m to 18 m, and each variety carries its own
shade of green. Nothing hangs in the grass any more either.

**PATH lays a dirt road instead of digging a cutting.** It used to level to one
height, which on any slope is a trench with shoulders. A road follows the land:
it grades the bumps out and leaves the hill a hill. And it is *worn* — there is a
surface layer now, so ground can be bare earth where somebody decided it is,
whatever the climate says should be growing there.

**Planting works.** It was going through the ground rebuild, so a wide stroke
found most of its chunks busy and dropped them, and trees appeared slowly or not
at all.

**The brush radius works.** The wheel was read as notches, so on a trackpad or a
high-resolution mouse a single flick slammed it from its smallest to its largest
and back.

**Getting about.** Hold **Alt** to free the pointer and click the world overview
to fly there — an 8 km map crossed by pointing the nose and holding W was the
worst thing about shaping one. Free-fly carries a standing speed on `-` and `=`.

**Your work keeps itself.** Every layer writes itself after two minutes of
sitting unsaved, and Esc with work outstanding says so before it will leave.

**The brush shows its falloff**, so the difference between FLATTEN and PATH is
visible before you press anything.

Under all of it, chunk building does a quarter of the terrain sampling it used
to, and a handful of real bugs went with it — undo could reach into the wrong
layer, and planting never marked itself unsaved, so woods could be lost on quit
under a panel saying everything was written.

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
to see the shape of the place — `Q`/`E` down and up, `-`/`=` for speed. `F3`
hides the overlay. `Esc` for the menu.

### Shaping it

**Shape the World** from the main menu. `1`–`9` pick a tool, drag to apply, right
drag inverts, the wheel sizes the brush and `[` `]` set its strength. Hold `Alt`
to free the pointer and click the overview to fly somewhere. `Ctrl+Z` and
`Ctrl+Y` take strokes back and put them again — across ground, woods and roads
alike, in the order you did them. `Ctrl+S` saves all three. Ramp is *clicked*
rather than dragged: one end, then the other.

Sculpting is read at startup, so relaunch to walk what you shaped. An installed
build has its own copy of the world, so shaping one does not change the other.

The same tool is also a bench in
**[Opificium](https://github.com/Baz-Studios-LLC/Opificium)**, for shaping a
world without opening the game. See `DESIGN.md`.
