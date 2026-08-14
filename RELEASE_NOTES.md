## The world

There is a world to walk and nothing yet to do in it — no monsters, no ranch, no
guild exams. What this release is for is judging the ground.

**An 8 km continent**, traced from a hand-drawn map and streamed as you go. The
map decides where the land is; everything on it is generated from that, and the
whole of it ends in open ocean rather than at a wall.

**Coasts that shelve.** Land climbs a beach's width from the waterline; the sea
floor falls away over a shelf. Some stretches are sand, some are rock — a coast
is beach where the sea has somewhere to put sediment, and stone where it hasn't.

**Level country, and mountains where mountains belong.** Most of the map is plain
enough for forest, farmland and walking. Ranges sit inland, along ridge lines,
never rising straight out of the sea.

**Six cities and fourteen towns**, each with ground levelled for it, joined by
graded roads that climb steadily enough to be walked and carted. Nothing is built
on them yet — this is ground, prepared.

**A sea that moves**, with a tide that walks the waterline up the beach and back.
You can wade to about your waist; past that the water turns you back. Boats come
later.

### Testing on macOS

**Apple Silicon only** (M1 or later). The build is arm64; it will not run on an
Intel Mac.

Easiest route is the **Baz Studios launcher** — it installs the app and clears
the download quarantine for you.

Installing the `.tar.gz` by hand needs one extra step, because the app is
**ad-hoc signed** rather than notarised. macOS quarantines anything downloaded
and will say *"Ranger is damaged and can't be opened"* — it isn't damaged, it
just isn't from an identified developer:

```bash
tar -xzf ranger-game-macos-aarch64.app.tar.gz
xattr -dr com.apple.quarantine Ranger.app
open Ranger.app
```

The world lives inside the bundle at `Ranger.app/Contents/MacOS/assets`, beside
the binary. Keep it there — macOS launches an app with the working directory at
`/`, so that folder is the only place the game can find its map.

### Playing it

`WASD` to move, `Shift` to sprint, mouse to look, wheel to zoom. `F` for free-fly
to see the shape of the place. `F3` hides the overlay. `Esc` for the menu.

### Shaping it

The world is sculpted in **[Opificium](https://github.com/Baz-Studios-LLC/Opificium)**,
the studio's maker's bench, at its terrain bench — not in the game. This build
reads what that writes.

**Since v0.1.0:** the menu's Terrain Tool button is gone. Sculpting moved out to
the bench, and the button had been left behind pointing at a mode with nothing in
it. See `DESIGN.md` for how a change at the bench reaches the game.
