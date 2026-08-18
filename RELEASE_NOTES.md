## The tools you shape it with — and the first build that ships without them

Still a world to walk and nothing yet to do in it. This one is almost entirely
about the tools, and about making sure none of them reach a player.

**No maker's tools in this build.** The terrain brush, the workbench and the model
kiln are not hidden behind a menu — they are not compiled in. A player's build
should not carry a way to break a save, and it certainly should not carry code
that can spend somebody's credits. The release workflow checks the binary itself
and refuses to publish one with the tools in it, because a dropped build flag is
otherwise a silent failure: a release that ships a brush looks exactly like one
that does not.

Everything below is in a maker's build, built from the same source with the tools
switched on.

### Painting the map's biomes

Where the deserts and the snow country sit used to be decided by numbers in code,
and moving one meant reading a marker's position off a screenshot and guessing
which constant it implied. That went wrong five times in one evening — not through
carelessness, but because the person who can *see* where a desert belongs and the
person who can edit the number were not the same person.

There is a brush now. `B` in Shape the World, press again to cycle ordinary,
desert and snow country; the right button clears back to whatever the world would
have decided for itself. It undoes and saves with everything else, and the
overview redraws so you can see the region while you are drawing it.

### A workbench

**Workbench** on the main menu: houses and fences built piece by piece from a kit
of seven parts — posts, rails, walls, floors, beams, roof panels, ridge caps — on
a quarter-metre lattice.

The sizes are fixed and the turns are quarters, and both are the point. Free boxes
at free angles give you the freedom to make every wall a slightly different
thickness and stand it three degrees off, which is a freedom nobody wants and
every eye notices. A fence and a house come out of the same parts.

The mouse aims and **snaps**; keys still nudge. `G` asks for a house, a fence, a
tower or a shelter — and what arrives is ordinary pieces, so the next thing you do
is take a wall out for a wider door. Generating is for skipping the boring half of
making a building, not for handing you one you cannot change.

### Things stand where you put them

The world used to raise one building at the middle of every town site, cycling
through whatever was in the folder. `assets/world/placed.json` says *this thing,
here, turned this way, this big* — and it survives the ground being resculpted,
because what is stored is the height above the ground rather than a height.

It stands buildings from the bench and **models generated from a picture**, which
is the other new thing: `F5` sends the image on the bench wall away to be made
into a 3D model, and what comes back lands in `assets/models` ready to place.

### Editor

The free-fly camera can no longer go under the ground. Under the map is not a
place — the world is a single surface with no underside, so from below you see the
backs of hills and the sea from inside, and nothing about the view tells you that
is what happened.

The overview has a heading needle and says which way is north. A dot tells you
where you are and nothing about where you are facing, which on a map with no
landmarks is half the information missing.

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
