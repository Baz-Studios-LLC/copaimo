# Ranger

A monster-companion adventure game. You play a ranger who raises monsters on a
ranch, travels between cities, and upgrades your Ranger License by passing the
exam set by each city's Ranger Guild.

Touchstones: **Pokémon** and **Monster Rancher**. Monsters are companions you
raise, not enemies you fight off.

Rust + Bevy 0.16.

## Running

```bash
cargo run
```

The main menu leads to **Explore World** — walk the map as the ranger.

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

## Tests

```bash
cargo test -- --nocapture
```

Prints the generated world as an ASCII map, which is the quickest way to see
what a map swap or a tuning change actually did.

---

Baz Studios LLC
