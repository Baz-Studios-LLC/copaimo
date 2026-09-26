---
name: opificium-terrain-bench
description: "Opificium is the Baz Studios game-dev app (Bevy 0.19, OTHERS EDIT IT — use PRs); the terrain sculpting tool lives there as a 4th bench"
metadata: 
  node_type: memory
  type: project
  originSessionId: fce466b1-3e22-4a2d-80b0-184c08cd035b
  modified: 2026-08-15T01:32:36.442Z
---

**Opificium = "essentially our Baz Studios game dev app"** (user's words, 2026-08-14) — `Desktop/Opificium`, repo `Baz-Studios-LLC/Opificium`, Rust + **Bevy 0.19**, edition 2024. Default branch is **`master`**, not main.

⚠️ **OTHER PEOPLE EDIT THIS REPO.** User: "lets really ensure we're not messing anything up". So: `git fetch` and compare before starting, work on a BRANCH, open a **PR** — do not commit to `master`. (v0.5.2 landed from someone else mid-session and the branch had to be rebased onto it.) PR #1 = the terrain bench, **merged and released as v0.6.0 on 2026-08-14**.

**Releasing:** CI (`.github/workflows/release.yml`) fires on a `v*` tag, builds macOS+Windows, runs `cargo test --release`, and attaches assets named `-macos-aarch64.app.tar.gz` / `-windows-x86_64.zip`. `RELEASE_NOTES.md` becomes the release body AND the launcher's patch notes, so rewrite it before tagging.

**Architecture:** benches are plugins behind a `Bench` enum (Builder / Kiln / Rig / **Terrain**), switched from a menu strip in `menu.rs`. It holds no game content — it opens a **project** (a game's folder with `opificium.json`) and speaks to games ONLY through files, all documented in `FORMATS.md`. `project.rs` has a path helper per file kind. `look.rs` owns the whole look: palette ramps loaded from the game's `data/palette.json`, `PANEL_WIDE`, `text_at()`, Cinzel/EBGaramond fonts, dark panels + gold accents.

**Terrain bench** = `src/terrain/` (mod/ground/chunk/edit/opened/shelf/settle/tree/forest). **Nine** brushes on keys 1-9 (raise, lower, smooth, flatten, path, roughen, erode, ramp, plant), undo/redo grouped per STROKE, chunks re-meshed live under the brush, paints in the open game's own ramps (water/sand/grass/foliage/stone/snow).

⚠️ **Don't wander in here uninvited** — see [[stay-in-the-game-repo]].

**The generation is shared now, not twinned.** `terrain-core` (repo `Baz-Studios-LLC/terrain-core`, branch `master`, glam-only + noise, **names no engine**) holds the world generation, forest scatter, tree growing and the sculpting brush; ranger-game links it and Opificium links it on **PR #2, still open as of 2026-08-14**. Its glam requirement is deliberately a RANGE (`>=0.29, <0.33`) — Bevy 0.16 carries glam 0.29 and 0.19 carries 0.32, and pinning either makes the other program link a second glam and see `Vec2` as not-`Vec2`. Widen it when a program moves to a newer Bevy.

⚠️ **A WORLD IS NOT A PROJECT — I got this wrong once and the user rejected it.** I first wired the terrain bench to Opificium's project system (`opificium.json`, `project::world()`), so you had to point the whole app at the game to reach the tools. User: *"no this is not correct at all. Its not a project, the terrain tools just need to be in the app, I should then be able to load the ranger game separately to use the tools."* The right shape: the bench is a TOOL you bring ground to (like the kiln takes an image) — **OPEN A WORLD… on the terrain shelf** picks a `heightmap.png`, and the folder it sits in IS the world. Last one remembered in `project::support()/terrain.json` (the bench's own settings, never the world's folder). `project.rs` untouched.

**Input at the terrain bench departs from the other benches on purpose:** BOTH mouse buttons are tools (right = inverse brush), so the eye turns on **Shift-drag** and the drafting angles move to **Shift+1-6**. Rule of thumb there: Shift means you're talking to the camera. (The game's own terrain mode needs none of this — mouse-look takes no button.)

**Why the recipe travels as data:** a maker sculpts OFFSETS and the game adds them to ground it generates itself — disagree by a metre and every hill sits at the wrong height with nothing on screen to say why. So `world.json` is exported by the game, exactly as `palette.json` already is.

**How to apply:** when adding to Opificium, keep changes to shared files (`camera.rs`, `stage.rs`, `main.rs`, `menu.rs`, `project.rs`) additive or gated on the new `Bench` variant, so the other benches' code paths stay identical — that's what made the diff reviewable. Match the house comment voice: long, opinionated, explains WHY and what was rejected. Run the full `cargo test` (105+ tests) before pushing.
