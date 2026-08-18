## Copaimo: The Wardens Guild

**The game has a name.** It was "Ranger", which was also what the player was
called — one word doing two jobs. It is **Copaimo: The Wardens Guild** now, and
the player is a **Warden**. The title screen carries the real logo rather than the
name typed out in whatever font happened to load, and the game has an icon: the
Wardens Guild crest, on the window, on the executable, and on the macOS bundle.

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This release is the name, and the tools that build the world.

### The maker's tools have menus you can click

Both tools were keyboard-only, which is fine for somebody who already knows them
and hostile to everybody else: a keybind is invisible until you have read a list
of them, and a list of keys at the top of the screen is not an interface.

Both have a proper panel now, built from the same pieces so they look and behave
alike. Everything is pressable, and every row **shows its key** — so the keyboard
is discoverable rather than documented, and a maker who knows the tool never has
to reach for the mouse.

The terrain tool's eleven brushes are grouped into four foldable branches — shape
the ground, lay over it, grow and mark, take it back — and its sliders are
draggable rather than being pictures of sliders.

The workbench had the worst of it: a wall of text over a grey grid, no menus and
no way to move. It has the same panel as the terrain tool, a shelf of parts you
click, colour swatches you click, and a floor that ends where the work is instead
of stretching to the horizon.

### None of it ships

The terrain brush, the workbench and the model kiln are not hidden behind a menu
in a player's build — they are not compiled in. A player's build should not carry
a way to break a save, and it certainly should not carry code that can spend
somebody's credits. The release workflow greps the binary and refuses to publish
one with the tools in it, because a dropped build flag is otherwise silent: a
release that ships a brush looks exactly like one that does not.

### Also in this one

**A biome brush.** Where the deserts and the snow country sit used to be numbers
in code. Paint them instead.

**A workbench that generates.** Ask for a house, a fence, a tower or a shelter and
what arrives is ordinary pieces you can take apart — generating is for skipping
the boring half of making a building, not for handing you one you cannot change.
Or send a picture away and get a 3D model back.

**Things stand where you put them.** `placed.json` says what stands where, and it
survives the ground being resculpted underneath it.

**The camera stays above the ground**, and the overview has a heading needle.

### Testing on macOS

**Apple Silicon only** (M1 or later). The build is arm64; it will not run on an
Intel Mac.

Grab the **`.dmg`** and drag Copaimo out of it, or install through the **Baz
Studios launcher**.

The app is **ad-hoc signed** rather than notarised, so macOS quarantines anything
downloaded and may say *"Copaimo is damaged and can't be opened"*. It isn't
damaged, it just isn't from an identified developer. Either **right-click the app
→ Open** and confirm, or:

```bash
xattr -dr com.apple.quarantine /Applications/Copaimo.app
```

The world lives inside the bundle at `Copaimo.app/Contents/MacOS/assets`, beside
the binary. Keep it there — macOS launches an app with the working directory at
`/`, so that folder is the only place the game can find its map.

### Playing it

`WASD` to move, `Shift` to sprint, mouse to look, wheel to zoom. `F` for free-fly
— `Q`/`E` down and up, `-`/`=` for speed. `F3` hides the overlay.

`F6` and `F7` push the hour back and forward so you can look at a dusk without
waiting for one; `F8` gives you back real time.
