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

The main menu leads to two modes:

* **Explore World** — walk the map as the ranger
* **Terrain Tool** — sculpt the world's shape

## Current state

Stage one is the world. It's generated from a source map image
(`assets/world/heightmap.png`), which decides where the continents are, and
streamed as chunks around the viewer. Land is currently **flat** while the
continent outlines are being checked — every hill and mountain is sculpted by
hand with the terrain tool on top of it.

Not started yet: cities, the ranch, monsters, battles, guild exams.

## The terrain tool

A separate mode with six brushes (raise, lower, smooth, flatten, path,
roughen), undo/redo, live chunk re-meshing and a world overview.

Edits are stored as **signed height offsets** on top of the generated terrain,
so re-rolling the noise or swapping the map image never moves hand-placed
geography. `Ctrl+S` saves them to `assets/world/edits.bin`.

The tool is `src/editor/` plus `src/world/edit.rs` and is deliberately free of
any dependency on the rest of the game, with a view to reusing it across
projects. What it needs from a host project is listed at the top of
`src/editor/mod.rs`.

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
