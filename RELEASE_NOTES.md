## Woods, a ranch, and the terrain tool back in the game

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. What this release adds is things standing *on* the ground, and the
means to shape it without leaving.

**Forests.** Trees are grown rather than modelled: a trunk that tapers and leans,
limbs that fork off it at their own angles, leaf clusters at the ends. Twenty
varieties from 6 m to 17 m, spires through to broad spreading trees, each wearing
its own green. Where they stand is decided by the ground itself — moisture, slope,
height under the treeline, and clear of beaches, roads and the levelled ground
under towns.

**A flatter country with one mountain.** Most of the map is plains and hills you
cross rather than terrain that stops you. Against that stands a single massif,
340 m and about a kilometre across, in whatever part of the map is furthest from
the sea. It is *found*, not placed — redraw the map and it moves to the new
heartland.

**The ranch.** Your farm's ground is levelled on the north-west coast, and you
now start there rather than in the middle of nowhere.

**Shape the World**, on the main menu. The terrain tool is back in the game: nine
brushes — raise, lower, smooth, flatten, path, roughen, erode, ramp and plant —
with live re-meshing under the brush, undo and redo, and a whole-world view. It
is the same tool as Opificium's terrain bench, driving the same code, so ground
shaped in either place is shaped identically.

**Buildings can come in from the bench.** Houses, signs and bridges drawn at
Opificium's builder can be read and stood on the ground. One goes up at each town
site for now — laying out a street is a job for later.

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
to see the shape of the place. `F3` hides the overlay. `Esc` for the menu.

### Shaping it

**Shape the World** from the main menu. `1`–`9` pick a tool, drag to apply, right
drag inverts, the wheel sizes the brush and `[` `]` set its strength. `Ctrl+Z`
and `Ctrl+Y` take strokes back and put them again; `Ctrl+S` saves the ground and
the woods together. Ramp is *clicked* rather than dragged — one end, then the
other.

Sculpting is read at startup, so relaunch to walk what you shaped. An installed
build has its own copy of the world, so shaping one does not change the other.

The same tool is also a bench in
**[Opificium](https://github.com/Baz-Studios-LLC/Opificium)**, for shaping a
world without opening the game. See `DESIGN.md`.
