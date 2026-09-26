# Copaimo

A monster-companion adventure game. You play a warden who raises monsters on a
ranch, travels between cities, and upgrades your Copaimo License by passing the
exam set by each city's Wardens Guild.

Touchstones: **Pokémon** and **Monster Rancher**. Monsters are companions you
raise, not enemies you fight off.

Rust + Bevy 0.16.

## Picking this up on a new machine

```bash
git clone https://github.com/Baz-Studios-LLC/copaimo.git
cd copaimo
cargo run
```

That is the whole of it for playing and for working on the game's code. Everything
the game loads is in the repository: the models, the heightmap, the hand-sculpted
terrain, the settlement files, the fonts.

**What you need installed**

| | | |
|---|---|---|
| Rust | 1.97.1 | pinned in `rust-toolchain.toml`, so rustup fetches it for you |
| Blender | 5.2 LTS | **only** to rebuild the art; `dev/art/blender.sh` finds it on the usual paths |

**What is not in the repository, and does not need to be**

The `.blend` files are intermediates — `dev/art/build.sh` rebuilds each one from its
`.py` and exports the `.glb` the game actually loads. The `.glb` are committed, so a
clone runs without Blender installed at all. You only need Blender to CHANGE a
building.

**What is in the repository and is not code**

`.claude/memory/` is Claude's memory for this project, checked in when the machine it
lived on was reset. It normally lives outside any repository; see the README in that
folder for where it goes and how to restore it.

## Running

```bash
cargo run
```

The main menu leads to **Explore World** — walk the map as the warden.

The world's shape is *sculpted elsewhere*; see below.

## Current state

Stage one is the world. It's generated from a source map image
(`assets/world/heightmap.png`), which decides where the continents are, and
streamed as chunks around the viewer. Land is currently **flat** while the
continent outlines are being checked — every hill and mountain is sculpted by
hand with the terrain tool on top of it.

Not started yet: cities, the ranch, monsters, battles, guild exams.

## Shaping the world

Terrain is sculpted at the **terrain bench in
[Opificium](https://github.com/Baz-Studios-LLC/Opificium)**, the studio's
maker's bench. This game only *reads* what the bench writes.

Open Opificium, go to **BENCH → THE TERRAIN**, press **OPEN A WORLD…** and pick
`assets/world/heightmap.png`. The folder it sits in is the world. The bench
remembers it, so it's one click next time.

A world is *not* an Opificium project — the terrain bench is a tool you bring
ground to, like the kiln. Nothing here needs an `opificium.json`.

Two programs, no shared code, only files:

| File | Direction | What it is |
| --- | --- | --- |
| `assets/world/heightmap.png` | game → bench | The map the world is drawn from |
| `assets/world/world.json` | game → bench | The recipe — every constant in `config.rs` that shapes the ground |
| `assets/world/edits.bin` | bench → game | Sculpted ground, as signed height offsets |

Offsets rather than absolute heights, so re-rolling the noise or redrawing the
map never moves hand-placed geography.

**Re-export the recipe whenever a world-shaping constant changes**, or the bench
and the game will disagree about the ground underneath and every sculpted hill
will sit at the wrong height:

```bash
cargo test export_world_for_opificium -- --ignored --nocapture
```

## Documentation

**[DESIGN.md](DESIGN.md)** is the reference: pillars, the core loop, how the
world is built, invariants that must not be broken, controls, and a change log.
Read it before changing world generation — several approaches have already been
tried and rejected for reasons recorded there.

**[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** is the other half of that: what the
game did when it was BROKEN, arranged by symptom. If something looks wrong on
screen, look there first — most of the hard bugs in this project have been one of
six recurring shapes, and the entry names the test that was meant to stop it.

**[docs/](docs/README.md)** is the reference library — how games are actually
made, from outside this project. Rigging standards and bone budgets, locomotion
techniques for the foot-sliding problem, Blender and glTF rules that fail
silently, what studios commit versus derive, and where an AI is weak on this kind
of work. Started because too much of what went wrong here was already solved and
written down by somebody else; each claim is marked as an industry **standard**, a
**measured** figure from this project, or **open**.

## Tests

```bash
cargo test -- --nocapture
```

Prints the generated world as an ASCII map, which is the quickest way to see
what a map swap or a tuning change actually did.

---

Baz Studios LLC
