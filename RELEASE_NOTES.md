## A world with weather, water and a time of day

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This one is about making the world worth crossing.

**It keeps your clock.** Nine in the morning where you are is nine in the morning
here: the sun, the sky, the light and every shadow follow your machine's own time.
Playing before school is a different world from playing after dinner, and neither
costs you a wait. Nights have a moon, nine hundred stars, and enough light to
walk by.

**Clouds** drift overhead and take the sun's colour, so they turn with the
evening. Rounded, low enough to read as weather, and few enough that a clear day
is still a clear day.

**Towns have nothing standing on them yet.** The hand-written cottage that stood
at every site while the building reader was being built has been taken out —
buildings are drawn at Opificium now, and a stand-in raised twenty times was the
world claiming something untrue about itself.

**Rivers**, and nobody placed them. Water falls on the continent, runs downhill
and gathers, and where enough has gathered there is a river — so they lie in
valleys, join as they descend, and reach the sea. Nearly 1,800 stretches of
channel across the map, with still water standing in them.

**Biomes.** The world knows what kind of place it is at any point — water, shore,
grassland, forest, desert, rock, snow, or ground somebody has settled. It is what
decides which trees grow where, where grass and flowers appear, and later which
monsters live where.

**Seven kinds of tree**, taken from real ones, each growing where it belongs: oak
and birch in open country, spruce and pine on the mountain, acacia and palm in
dry country, willow and palm on the shore. A birch is a pale whip, a spruce is a
dark cone with its limbs to the ground, a pine is a tuft on a bare pole.

**Grass, flowers and dry scrub**, thickest in open country, thinner under a
canopy, sparse on rock and in yards, absent from open water.

**Roads are yours now.** The world no longer lays its own between towns — a
graded run holds its grade across a ridge and cuts through, which is a machine's
answer to a question that wants a person's. Use PATH in the terrain tool.

**A tool you can work in.** Hold `Alt` to free the pointer and click the world
overview to fly there. `0` is REVERT, which puts ground back exactly as it was
generated — the tool that was missing when old paths could not be undone. Your
work saves itself after two minutes, and `Esc` asks before it throws anything
away.

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
