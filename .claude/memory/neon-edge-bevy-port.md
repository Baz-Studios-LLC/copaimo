---
name: neon-edge-bevy-port
description: "Rust + Bevy 0.16 native port of VIOLET EDGE at Desktop/neon-edge-bevy — hand-authored 1-15 wave arc (loops after 15; non-boss waves 120s), 3 bosses w/ HP bars (Warden shield / Glutton eat-overload-shrink / Slinger cannonball), mobs = yellow lobber + Limpet parasite (both die in 1 shot), orange explosive asteroids wired into waves 11-14, chain/mass/drone pickups (each tied to its boss), gold 1UP economy, top-5 high scores, menu/Controls(rebind kbm+gamepad; Settings merged in)/Briefing/achievements, controller support, post-boss NEXT-WAVE countdown, SFX; waves 1-15 CONTENT-COMPLETE (holding here for playtest); 98 headless tests green; released v0.2.6 to GitHub (Violet-Edge, CI builds win/mac-dmg/linux); I compile/test, user runs the window"
metadata:
  node_type: memory
  type: project
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

Porting [[neon-drift-project]] (NEON EDGE) from JS/Canvas to native **Rust +
Bevy 0.16** (started 2026-07-16, user's call). Lives at
`C:\Users\jsull\Desktop\neon-edge-bevy\` — a SEPARATE project/folder from the JS
game (which stays the reference + shippable fallback). Under git → GitHub Baz-Studios-LLC/Violet-Edge (see [[neon-edge-github-repo]]).

- **Toolchain:** Rust/cargo **1.97** IS installed, but at `C:\Users\jsull\.cargo\bin\`
  — NOT on the shell session's PATH. Use the full path:
  `& "C:\Users\jsull\.cargo\bin\cargo.exe" check --manifest-path "…\neon-edge-bevy\Cargo.toml" --color never`.
  First build compiles ~486 crates (~2min `check`, ~6min `build`); background it.
- **I can compile/verify but cannot RUN it** (windowed GPU app, no display). The
  USER runs `cargo run` to see visuals. **VERIFY GAME LOGIC WITH HEADLESS TESTS:**
  `#[cfg(test)] mod tests` in main.rs builds an `App` with `MinimalPlugins` (no
  window) + the systems, injects input/entities, `app.update()`, asserts on the
  World. `cargo test` runs them fast. This is how I proved fire/collision/physics/
  death without the window — my loop is: edit → `cargo test` → user `cargo run` for
  feel/visuals. (5 tests currently: fire, bullet→asteroid, asteroid separation,
  ship death, invuln survives, last-life→game-over.) GOTCHA: a headless test can
  pass while the FEATURE is broken in-game if it checks the wrong thing (the drone
  NaN-bug pattern) — assert on the OBSERVABLE. GOTCHA: `init_state` PANICS under
  `MinimalPlugins` ("StateTransition schedule missing") — in tests that only need
  `NextState<S>` for a system param, `insert_resource(NextState::<S>::default())`
  instead of `init_state` (real `main()` has DefaultPlugins so `init_state` is fine).
- **Bloom-on-gizmos CONFIRMED working** (user screenshot 2026-07-16) — the
  lightweight gizmo + HDR/`Bloom` path glows; NO need for emissive meshes.
- **Status: milestone 3 done — builds clean, 0 warnings, 6 tests green.** Has:
  left-click + Space fire; elastic asteroid physics (`resolve` + `iter_combinations_mut::<2>()`,
  ship↔asteroid); **ship DIES on contact** → debris burst + despawn → `respawn`
  (blinking `invuln`); particles (`Particle`+`burst()`/`dim()`); bullet trails;
  starfield; grid dimmed + brighter per-line shimmer; **thrust flame + exhaust
  sparks** (`Ship.flame` ramp); **Bevy STATE MACHINE** `GameState{Playing,Paused,
  GameOver}` (`init_state`, gameplay `.run_if(in_state(Playing))`, `OnEnter/OnExit`
  spawn/despawn overlays); **PAUSE** (Esc toggles, `spawn_pause_ui` overlay);
  **GAME OVER** (last life → `next.set(GameOver)`, overlay shows score, Enter =
  `gameover_restart` clears+resets+respawns → Playing); **lives HUD** (persistent
  UI "LIVES" `Text` top-right + gizmo ship icons in `render` per `Run.lives`).
  UI text = bevy_ui `Text`/`TextFont{font_size,..default()}` (default font, no
  asset)/`TextColor` + `Node` overlays via `overlay()`/`text()` helpers.
  Palette rule honored (purple = player, incl. flame). `Run.lives` starts 3.
- **Architecture:** Bevy ECS. Single `src/main.rs` for now (split into modules as
  it grows). Components `Ship`/`Velocity`/`Asteroid`/`Bullet`; resources
  `Score`/`Arena`; systems chained input→fire→integrate→bounds/recycle→collide→render.
  Pinned `bevy = "0.16"` (resolved 0.16.1) + `rand = "0.8"` — DO NOT bump bevy
  without updating the code (heavy API churn between versions).
- **Rendering:** Bevy **gizmos** (immediate-mode line drawing) for the wireframe
  vector look, on an HDR `Camera2d` + `Bloom::default()` + `Tonemapping::TonyMcMapface`
  for the neon glow; bright `Color::srgb(r,g,b)` with values >1.0 to trigger bloom.
  OPEN QUESTION: whether gizmos actually bloom on the user's GPU — if not, switch
  key shapes to emissive meshes. (Wrote the blind code by copying Bevy 0.16's own
  bloom_2d + 2d_gizmos examples via WebFetch — every API call compiled first try.)
- **Motion is per-second (delta-time),** framerate-independent; the JS per-frame
  @60fps tuning is only ballpark-ported — re-tune once it runs.
- **Milestone 4 done (2026-07-16):** exhaust fixed (emits straight out the back,
  no velocity skew, rate cut); **3-min WAVE timer** (`Wave{level,timer}`,
  `wave_timer` advances on survive + streams rocks to `population_target(level)`
  = `min(4+level,16)`); **HUD** score (top-left) + wave/timer (top-center, centered
  wrapper) via `Text` `t.0 = format!(...)` update systems; **WARP ultimate** (Shift):
  `Warp{charges,cooldown}` — **3 charges**, fire each independently; ONLY when all
  3 are spent does the long `WARP_COOLDOWN` (~22s) refill them together (burst-then-
  wait, not spammable). `warp_fire`→`WarpMissile`→`black_hole_update` pulls asteroids
  (`WARP_PULL_RADIUS`) + consumes (`WARP_CONSUME_R`); ship immune. VISUALS: inward-
  travelling DRAIN spiral (gizmo funnel arms + bright streams whose phase advances
  inward); the GRID bends toward the hole (`WarpField` resource + `warp_point`,
  `WARP_GRID_RADIUS`/`STRENGTH`) and RUBBER-SNAPS back when it closes
  (`ease_out_elastic`, amount overshoots negative = grid bulges out). Charge pips +
  refill bar bottom-center. Grid draws straight 2-pt lines unless warping (cheap).
  9 tests green.
- **Rebuild plan (user, 2026-07-16): redo the JS wave flow UP TO WAVE 5 + the
  first boss, IN STEPS.** Wave flow so far: timer + population scaling + continuous
  throttled `top_up_asteroids` (streams replacements ~1/1.6s up to `population_target`),
  gated on `Wave.calm` (f32, never set >0 yet — the boss step sets it for the post-boss
  lull to pause spawns + the wave timer). `render` runs in **PostUpdate** (after all
  Update incl. `ship_bounds`) to fix border ghosting; `ship_bounds` also zeros the
  into-wall velocity. STEPS: (1) ✅ **mines** (W2+) DONE — `Mine{armed,fuse}`,
  `top_up_mines` (capped `mine_target` = fraction of asteroids, calm-gated),
  `mine_update` (drift/recycle + arm-on-proximity blink + fuse + detonate→`kill_ship`),
  shootable in `collisions` (+`MINE_SCORE`); crimson diamond + kill-ring when armed.
  Factored a shared `kill_ship(commands, &mut run, &mut next, e, pos, rng)` used by
  ship_death + mine_update. **Mines shatter rocks via THREE detonation triggers, all
  routed through a factored `blast_asteroids(commands,rng,&mut score,&asteroids,&mut
  broken,center)`** (which calls `break_asteroid(...,chunk_mult)`; break_asteroid also
  replaced the inline bullet-split at mult 1.0). ⚠️ **`break_asteroid` must spawn the
  two child chunks ALREADY CLEAR of each other** (offset ±`asteroid_radius(child)+3`
  along a random axis, opposite velocities + a little jitter) — the port first spawned
  both AT the parent's point, and since `resolve` only skips `d2==0.0` exactly, the
  instant `integrate` nudged them apart the collision resolver force-separated them at
  `MAX_SEP` 6px/frame and swamped their 60–150px/s velocities → chunks oozed apart in
  place instead of flying off (user report). JS `splitAsteroid` does the offset for the
  same reason. Detonation triggers: (a) SHOT — bullet↔mine in `collisions`;
  (b) SHIP contact / armed+fuse-in-blast — kills ship too; (c) **DRIFTS INTO A ROCK —
  any entered mine touching an asteroid detonates, NO life lost** (mirrors JS
  `updateMines`→`blastAsteroids`). ⚠️ (c) was MISSING in the first pass → user reported
  "mines don't destroy asteroids" (a mine sat on a rock doing nothing, screenshot) —
  the shot/ship paths alone weren't enough; ALWAYS check the JS `updateMines` for the
  full trigger set. **HIDDEN bit (undocumented, per [[favor-emergent-skill-discovery]]):**
  mine-broken chunks fling at `MINE_CHUNK_MULT`=1.9× normal split speed (JS
  `MINE.chunkSpeed`) — the fast scatter is the subtle part; the detonation itself is now
  visible/automatic. `mine_update` + `collisions` both take `asteroids: Query<..,
  Without<Mine>>` (disjoint from `&mut mines`/`With<Mine>` via the archetype split) so
  both can pass `&asteroids` to `blast_asteroids`; each owns a `broken`/`dead_a` HashSet.
  **DEV INVINCIBILITY TOGGLE (F1):** `#[derive(Resource,Default)] Dev{invincible}`
  (always inserted) + shared `immune(ship,dev)= ship.invuln>0 || dev.invincible` used
  by BOTH ship_death & mine_update (replaced the raw `invuln>0` guards); `render` draws
  a pulsing shield ring (dimmed ship_color, purple = player kit) around the ship while
  on. The **toggle system + its registration are `#[cfg(debug_assertions)]`** — god-mode
  physically can't compile into a `--release` build (self-enforcing; no "remember to
  strip" like [[crashout-release-target]]). main() refactored to statement form
  (`let mut app=…; #[cfg(debug_assertions)] app.add_systems(Update,dev_toggle); app.run()`)
  so the dev system is added conditionally.
  (2) ✅ **ENEMY SHIPS done (W3+, 2026-07-16)** — `Enemy{fire,life,strafe,entered,
  fleeing}` + `EnemyBullet{life}` (neon-YELLOW `enemy_color`, never purple). Systems:
  `top_up_enemies` (capped `enemy_target`=fraction of rocks, calm-gated, `EnemyClock`),
  `enemy_update` (glide in → hover+strafe at `ENEMY_PREF_DIST` around ship, **steer
  away from mines+rocks**, lob slow shots; **only sets Velocity — `integrate` moves it**),
  `enemy_bullets` (expire + kill ship via `kill_ship`+`immune`). **Die in ONE shot**
  (bullet→enemy in `collisions`, +`ENEMY_SCORE`). **Warp-affected:** `black_hole_update`
  gained an enemies `&mut Velocity` query (disjoint via `Without<Asteroid>`) to drag+
  consume them; `enemy_update` yields control (`continue`) when a hole is within
  `WARP_PULL_RADIUS` so the pull isn't capped away. **Never overstay:** after
  `ENEMY_LIFETIME`~11s → `fleeing`, accelerates out, despawns off-screen. User plan:
  W3 = enemies, **W4 = same as W3**, W5 = boss NEXT. Also removed the mine kill-ring
  from `render` (user). ⚠️ **Bevy system-param limit is 16** — `render` (now 16) and
  `gameover_restart` both hit it; fix = group queries into ONE nested-tuple param
  (`foes: (Query<…>, Query<…>)`, `hazards: (Query,Query,Query)`) and index `.0/.1`.
  23 tests green (5 new enemy tests: target/one-shot/bullet-kills-ship/warp-consumes/flee).
  (3) ✅ **OCTOPUS BOSS done (W5/10/15…, 2026-07-16)** — magenta `Boss{hp,rot,pulse,
  entered,charge,fire,capture}` + `Shielded{slot,grab}` (on captured rocks) + `Thrown(f32)`
  (brief no-regrab). `is_boss_wave(l)=l%5==0`; `BossState{fought}` res spawns exactly one
  boss/wave. Flow: `boss_director` (on boss-wave entry: spawn boss gliding from top, despawn
  mines + set enemies `fleeing` → only rocks remain; `top_up_mines`/`enemies` gated off on
  boss waves; `wave_timer` PAUSES on boss waves = ends on KILL). `boss_update` (glide→bob,
  `charge` invuln power-up, core contact kills ship, hp<=0 → burst + release shield + `wave
  .calm=BOSS_CALM` + level++ + `BOSS_SCORE`). `boss_shield` (reel-in/orbit held rocks at
  `BOSS_ORBIT_R`, grab nearest free rock into an arm, **throw a held SIZE-1 rock at the
  ship**; also ages out `Thrown`). Shield rocks stay `Asteroid`s (render + break normally in
  `collisions` = shield blocks/opens gaps) but are `Without<Shielded>`-excluded from
  `asteroid_collisions`/`_bounds` (boss owns their position). Bullet→boss core in `collisions`
  (skipped while charging). Ported from JS `boss.js`/`stepBoss` (r38,hp28,6 arms,orbit132,
  throwSpeed,10s calm,magenta #ff5ad0). **Render split:** boss draws in a NEW `render_boss`
  PostUpdate system (cameo telegraph + core star + bezier tentacle arms) — done because
  `render` was at the 16-param cap. **Background CAMEO:** during the last `BOSS_CAMEO_SECS`
  (10s) of the wave BEFORE a boss wave, a faint magenta silhouette drifts across (pure
  render, no entity). Also this pass: **field starts EMPTY** (deleted `spawn_field`; rocks
  drift in from edges via `top_up_asteroids`) so none spawns on the player; **DEV F2 wave-
  skip** (`dev_wave_skip`, debug-only: kills boss on a boss wave else `timer=0`). 29 tests
  green (6 boss tests). NOTE: boss balance (hp/throw cadence/charge) untested by feel — iterate.
- **TUNING PASS (2026-07-16, all from user playtest):** WARP — missile flies farther
  (`WARP_MISSILE_LIFE` 0.7→1.3) before the hole opens; **mines are now pulled + consumed**
  by the hole too. Factored `warp_pull(pos,hole,pull_r,dt)->Vec2` (shared by rocks/enemies/
  mines in `black_hole_update`, which gained a mines query `(With<Mine>,Without<Asteroid>,
  Without<Enemy>)`). ⚠️ **Pull radius `WARP_PULL_RADIUS` = pure FEEL dial, ping-ponged**: 440(orig,"a bit
  small")→755(dynamic quadrant,TOO FAR)→520→360(JS `VORTEX.pullRadius`,TOO SMALL — our missile
  throws FARTHER than JS's so the JS radius under-reaches)→ **500 (final)**. Just a tunable
  const — iterate by feel, don't over-derive. `black_hole_update` no longer needs `Res<Arena>`;
  `enemy_update` caught-check uses `WARP_PULL_RADIUS`. (`WARP_PULL` force stays 2000, stronger
  than JS, per the earlier "more aggressive" request.) ASTEROIDS — bigger, clearer tiers
  `asteroid_radius` **3=88 (LARGE) / 2=46 (MID) / 1=22 (SMALL)** (were 65/40/20);
  `spawn_edge_asteroid` now spawns a MIX (70% large size-3, 30% mid size-2). BOSS — grab
  slowed (`BOSS_CAPTURE_EVERY` 0.63→1.1); capture now **prioritizes biggest rock** (size
  desc, nearest tie-break; `free` query gained `&Asteroid`); **free rocks BOUNCE off held
  shield rocks** via new `shield_deflect` system (static bounce: push out + reflect, held
  immovable) — shield rocks are back to interacting, just as static obstacles. 33 tests green
  (+warp-consumes-mine/pulls-mine, boss-grabs-biggest, free-bounces-off-shield). ⚠️ headless
  dt=0 on the FIRST `app.update()` → pull/motion tests must loop a few frames (or assert on
  dt-independent effects like consume/despawn).
- **BOSS HP BAR + SHIELD-SHRINK REWORK (2026-07-16, user):** (a) **HP bar** drawn in
  `render_boss` — dim full-width track + bright magenta fill (`boss.hp/BOSS_HP`), stacked
  `line_2d`s, top-center at `h.y-42`. (b) **Held shield rocks now shrink IN PLACE when shot**
  (was: broke into free chunks). `collisions` free-rock query is now `Without<Shielded>`;
  a separate `shield_rocks: Query<..&mut Asteroid.., With<Shielded>>` handles hits — size>1 →
  `size-=1` + regenerate verts (factored `asteroid_verts(size,rng)`, also used by
  spawn_asteroid), stays on the arm; size==1 → despawn (frees the arm) +score. Checked
  BEFORE the boss core (shield intercepts). `blast_asteroids` + `mine_update` asteroid queries
  also gained `Without<Shielded>` to match `blast_asteroids`' param type (all free-rock queries
  must share the exact filter tuple or the call won't type-check). (c) `boss_shield` reordered
  to **throw the size-1 BEFORE grabbing** a replacement (frees its `used[slot]` first); capture
  recomputes held from `used`. Net loop: grab big rock → player whittles it 3→2→1 in place →
  boss hurls the size-1 → grabs another big one. Aligns with [[neon-edge-difficulty]]
  (manageable chaos: shrink-in-place makes less debris than the old free-chunk break).
- **STUCK-ROCKS + GLOW + VIOLET (2026-07-16, user):** (a) **rocks getting stuck** — two fixes:
  `MIN_DRIFT`=30 floor in `asteroid_bounds` (fully-elastic `RESTITUTION=1` hits could zero a
  rock's velocity → it sat dead; now free rocks never fully stop — Shielded rocks excluded so
  the boss still pins them); and `shield_deflect` now ejects free rocks **radially OUTWARD from
  the boss centre** (added a `bosses` `&Transform` query) so they can't get trapped inside the
  spinning shield ring (fallback to away-from-rock when no boss). (b) **Vortex glow boosted** —
  warp render `glow` 1.8→2.6 + a white-hot center circle (KEPT — user likes the glow). Briefly
  tried 4 stacked concentric halo rings but the user said "remove the extra rings" → back to a
  SINGLE soft halo. (c) **Ship → neon violet** `ship_color` (2.6,0.9,5.0)→
  (3.2,0.7,6.5): peak channel kept HIGH so bloom keeps it bright, not dark (still purple-family,
  [[neon-edge-purple-is-player]] holds). User feedback: "waves 1-5 coming together nicely."
- **BOSS TRANSITION + MUSIC + RENAME (2026-07-16, user):** (a) **Mines/mobs no longer wiped
  at boss spawn** — `boss_director` now ONLY sets enemies `fleeing` (they leave); existing
  mines are left alone to behave normally + `mine_update` (now takes `Res<Wave>`) **despawns
  off-edge mines instead of recycling during a boss wave** so they drift off for good; no new
  mines/enemies spawn (top_up gated). (b) **Music eased 138→132 BPM.** (c) ✅ **RENAMED to
  "VIOLET EDGE"** — web/Steam/trademark check found NO existing "Violet Edge" game or mark
  (lots of "Violet …" games but not that exact name), so the user committed. Changed: window
  title `"NEON EDGE"→"VIOLET EDGE"`, Cargo `name = "neon-edge"→"violet-edge"` (binary is now
  violet-edge.exe), main.rs doc header. ⚠️ NOT changed (kept to avoid churn): the FOLDER is
  still `Desktop/neon-edge-bevy`, and these memory files keep their `neon-edge-*` slugs — the
  GAME is Violet Edge, the paths/slugs are legacy. 37 tests green.
- **BOSS DEATH ANIM + FIRST POWERUP: CHAIN SHOT (2026-07-16, user):** (a) **Slow boss death**
  — `Boss.dying: f32`; hp<=0 sets `dying=BOSS_DEATH_SECS`(2.2s) + scatters the shield, then
  `boss_update` crackles/spins/shrinks (render scales the core star by `dying/DEATH_SECS`,
  hides the HP bar) and only at `dying<=0` does it despawn + `calm=BOSS_CALM`(10s) + level++ +
  `BOSS_SCORE`. No move/contact/damage while dying (`collisions` + `chain_update` skip
  `dying>0`). (b) **Chain shot** — a wide lightning BEAM secondary. `Chain` res
  {unlocked,charges,recharge,cooldown}; 3 charges regenerating one per `CHAIN_RECHARGE`(5.5s).
  `chain_fire` on **RIGHT-CLICK** (SETTLED 2026-07-16: warp stays on SHIFT; NOT space — space is
  the primary gun, dual-binding would fire bullets + chain together). Spawns `ChainShot{life,perp}`
  (Velocity-driven); `chain_update`
  = segment `centre±perp·CHAIN_HALF`, mows EVERYTHING it touches each frame (rocks/enemies/mines)
  via `seg_dist2` helper — doesn't stop on first hit. ⚠️ **Does NOT damage the boss core** (user,
  balance: bosses are beaten via their asteroid mechanic, not brute force — see
  [[neon-drift-asteroids-core]]). Electric-violet `chain_color` (player kit). (c) **Pickup** —
  `Pickup{rot,pulse}` orb; spawned **ONLY after the FIRST boss** (`if wave.level==BOSS_WAVE_INTERVAL`
  at death, before the level-up) — skip it and it's gone FOREVER (never re-offered by later bosses).
  `pickup_update` drifts/bounces, fly into it (ship contact) → unlock + fill charges, or LEAVE it →
  despawns when `wave.calm<=0` (window closes = hardcore run, chain stays locked). Rendered in a NEW `render_extras` PostUpdate system
  (jagged bolt + end dots; hex orb). ⚠️ **Playing systems now 3 chained groups** (was 2) —
  hit the 20-per-tuple limit again; `((g1),(g2),(g3)).chain()`. `gameover_restart` resets
  `Chain::default()` (bundled with BossState in a tuple param) + despawns ChainShot/Pickup
  (added to the hazards tuple). 41 tests green.
  NEXT: playtest boss death + chain feel; dense/orange rocks, other bosses, more power-ups, menus.
- **PICKUP ROADMAP (user, 2026-07-16):** one pickup per boss window, each unlocked ONLY at its
  wave (skip = gone forever), each countering that boss:
  · **W5 octopus → CHAIN SHOT** (line-clear beam) ✅ DONE.
  · **W10 (devourer boss) → MASS SHOT** — a bigger, higher-damage REPLACEMENT for the standard
    shot (permanent primary-fire upgrade, no ammo). JS `MASS` config: r6 fat bullet, damage 2,
    amber `#ff9f1c`.
  · **W15 (raider boss) → DRONE** — a wingman that follows + auto-fires at a SLIGHTLY SLOWER pace
    than the player. JS `DRONE` config + `drone.js`. Purple (player kit — see [[neon-edge-purple-is-player]]).
  Build each when its wave/boss is built (they need their boss to trigger + be testable). Generalize
  `Pickup` with a `kind` enum then. All reinforce [[neon-drift-asteroids-core]] (rock-clearing tools).
- **WAVE BANNER + MUSIC done (2026-07-16):** (a) big center-screen **"WAVE n" flash**
  — `WaveBanner{timer}` res + `WaveBannerText` (full-screen centered Node child, alpha
  0) + `wave_banner_update` (quick fade-in → hold → fade-out over `WAVE_BANNER_FADE`,
  via `color.0.with_alpha`); triggered by setting `banner.timer=WAVE_BANNER_SECS` in
  `wave_timer` (level-up), at startup (initial res value → "WAVE 1"), and in
  `gameover_restart`. (b) **Procedural techno soundtrack** — FIRST module split:
  `src/audio.rs` (`mod audio;`), pure-std synth `techno_loop_wav()->Vec<u8>` renders a
  seamless 8-bar **138 BPM** A-minor loop (four-on-the-floor kick, offbeat sub bass,
  closed/open hats, backbeat clap, and a relentless **16th-note detuned-saw arp across
  the whole loop**; roots Am-F-C-G) to an in-memory 16-bit mono **WAV**, voice tails
  wrap + a 4ms release fade in `add_voice` for a clean, click-free loop. (User asked for
  more energy / less downtime → bumped 126→138 BPM + arp every 16th full-loop, was every
  8th 2nd-half-only.) Played via `AudioSource{bytes:
  Vec<u8>.into()}` + `AudioPlayer` + `PlaybackSettings{mode:PlaybackMode::Loop,
  volume:Volume::Linear(0.55)}` in a `start_music` Startup system; **M** mutes
  (`music_toggle` → `AudioSink::pause/play`, needs `use bevy::audio::AudioSinkPlayback`).
  ⚠️ **WAV needs the `wav` cargo feature** (`bevy={version="0.16",features=["wav"]}`) —
  Bevy's default audio feature is only `vorbis`/ogg; without it rodio panics
  UnrecognizedFormat. Synth is testable headless (`audio::tests::loop_is_wav_and_nonsilent`).
  ⚠️ RECURRING: adding a res param to a system (WaveBanner→wave_timer) breaks every
  headless test that runs that system — must `insert_resource` it there too. 17 tests green.
  Misc this pass: warp cooldown 22→35s; bullets got a layered glowing head; 13 tests green.
  Playing systems split into two `.chain()` groups (Bevy's 20-system tuple limit).
- **SFX + DENSE GREEN ROCKS + WAVE-6 MOB STOP (2026-07-16, user):** (a) **One-shot SFX** —
  `audio.rs` gained `fire_sfx_wav` (descending saw pew), `break_sfx_wav` (crunch+thump),
  `mine_sfx_wav` (low boom sweep), all via a shared `render_sfx(dur,voice)` (pure std, no
  assets, same WAV path as the loop). Wiring: `#[derive(Event)] SoundFx{Fire,Break,Mine}` +
  `app.add_event::<SoundFx>()`; emitters `fire`/`collisions`/`mine_update`/`chain_update` take
  `EventWriter<SoundFx>` and `.write(..)`; a single `play_sfx` reader **dedups per-kind per
  frame** (a mine/chain hitting many rocks = ONE break sound, not a wall of noise) and plays
  each via `one_shot(commands,clip,vol)` (`PlaybackMode::Despawn`). `SfxBank` res holds the 3
  handles (built in `start_sfx`). ⚠️ **EventWriter param PANICS at init if `Events<T>` isn't
  registered** → EVERY headless test now needs `app.add_event::<SoundFx>()` after
  `add_plugins(MinimalPlugins)` (added to all). (b) **Yellow mobs STOP after wave 5** —
  `ENEMY_LAST_WAVE=5`; `enemy_target` returns 0 outside `ENEMY_FIRST_WAVE..=ENEMY_LAST_WAVE`.
  (c) **Dense (green) asteroids from wave 6** replace the mobs as the wave-6+ threat.
  `Asteroid` gained `dense: bool` + `hp: i32` (=`size` when dense, else 1). A dense rock is a
  MULTI-HIT tank: bullet→rock in `collisions` does `if a.hp>1 { a.hp-=1 + green chip burst }
  else { break_asteroid }` — so a size-3 dense rock takes 3 bullets (chip, chip, break). Worth
  **2× score**; children **inherit density** (a dense rock splits into dense chunks). **Chain
  beam + mine blast SHEAR dense rocks outright** (ignore hp — they pass `a.dense` to
  `break_asteroid` and never chip) so those AoE tools stay meaningful vs a green field.
  `dense_color()=srgb(0.5,5.0,1.4)` neon green. Spawn: DRY `roll_dense(level,rng)` =
  `level>=DENSE_FIRST_WAVE(6) && rng.gen_bool(DENSE_FRACTION(0.5))`, called by both
  `wave_timer` + `top_up_asteroids` before `spawn_edge_asteroid(..,dense)`. **Render:** dense
  rocks draw green with a **concentric inner ring that shrinks with remaining hp** (integrity
  read at a glance) — factored a `ring(scale)` closure so the outline+inner share one builder.
  ⚠️ signature ripple: `spawn_asteroid`/`spawn_edge_asteroid`/`break_asteroid`/`blast_asteroids`
  all gained a `dense` arg, and `collisions`/`mine_update` asteroid queries became `&mut
  Asteroid` (for the chip) — all free-rock queries must keep the SAME `(Without<Mine>,
  Without<Shielded>)` filter tuple or `blast_asteroids` won't type-check. ⚠️ EDIT GOTCHA: a
  `replace_all` on `spin: 0.0 }` (to add the new fields to test `Asteroid` literals) also hit
  `BlackHole { .., spin: 0.0 }` literals (test AND the runtime warp-hole spawn) — had to revert
  those by hand; when bulk-editing struct literals, anchor on a field unique to that struct.
  43 tests green (+`dense_rock_chips_before_it_breaks`: chip→still-there@hp1→2nd-hit→2 dense
  chunks + double score). clippy clean. NOT playtested by feel yet — user runs `cargo run`.
- **CHAIN HUD + WARP BLACK-HOLE REWORK + ENEMY ANTI-STACK (2026-07-16, user playtest):**
  (a) **Chain-shot HUD** — bottom-LEFT cluster (warp stays bottom-center): a little
  lightning-bolt glyph + `CHAIN_MAX_CHARGES` electric-violet (`chain_color`) pips (lit/dim) +
  a refill bar (`1.0 - chain.recharge/CHAIN_RECHARGE`), drawn ONLY when `chain.unlocked` (so
  it appears exactly when the W5 pickup is taken — teaches the unlock). ⚠️ `render` was AT the
  16-param cap, so folded `Chain` into the old `warp_res` slot: param is now `abilities:
  (Res<Warp>, Res<Chain>)`, re-bound at top via `let (warp_res, chain) = (&abilities.0,
  &abilities.1);` (net params unchanged). (b) **Warp = real black hole** — `WARP_CONSUME_R`
  52→**120** (event horizon) and consumption is now **EDGE-based**: `dist < WARP_CONSUME_R +
  entity_radius` for rocks (`asteroid_radius(size)`), enemies (`ENEMY_R`), mines (`MINE_R`).
  FIXES the user's clump bug — with a 52px mouth < a large rock's 88px radius, pulled-in rocks
  couldn't be swallowed and jostled/clumped around the center via `asteroid_collisions`; now a
  rock is devoured the instant its edge touches the horizon (well before the crowded center).
  **EXEMPTIONS (user): player** (never in the queries), **bosses** (carry neither Asteroid nor
  Enemy → auto-exempt), **boss-HELD rocks** (added `Without<Shielded>` to the asteroids query —
  can't warp a shield away, aligns with [[neon-drift-asteroids-core]]); FREE rocks near a boss
  still get eaten. Edge-of-arena slingshot (tangential escapes) is fine/intended (user). Visual
  bumped to match: `r_out` 132→150, `r_in` 10→24 (fatter throat), bigger hot core, + ONE bright
  **event-horizon ring at `WARP_CONSUME_R`** so the kill zone is legible (kept to a single ring
  — user previously rejected stacked halos). (c) **Enemies no longer stack** — `enemy_update`
  pre-collects an `others: Vec<(Entity,Vec2)>` snapshot (can't re-borrow the `&mut enemies`
  query mid-iter) and adds a mutual-separation steering loop (push apart within `ENEMY_SEP_R =
  ENEMY_R*4`, same shape as the mine/rock avoid) so they spread into a loose formation. 44 tests
  green (+`warp_spares_boss_held_rocks`: held rock on the hole survives, a free rock at the same
  spot is devoured). clippy clean. NOT feel-tested yet — user runs `cargo run`.
  ↳ **VISUAL/AUDIO FOLLOW-UP (2026-07-16, user screenshot):** the warp's OUTER rings read as
  concentric circles (disliked). Fixes: removed the faint outer halo; loosened `wind` 3.2→2.4
  (arms wrap less); draw funnel arms **segment-by-segment, brightness fading in from the rim →
  bright at the throat** (no hard outer boundary = drain read, not rings); flipped comet-head
  brightness to brighten inward (`0.35+0.65*hp`). Kept the bright event-horizon ring + hot core.
  Also **`break_sfx_wav` reworked heavier** — was a thin hissy crunch; now a CRACK (short
  filtered-noise snap) + THUD (low sine sweeping 150→52 Hz) + coarse rumble body, soft-clipped
  for punch. clippy clean, 44 tests still green.
  ↳ **`mine_sfx_wav` reworked too (user: explosions sound soft)** — was a clean gentle sine
  boom (no attack). Now a punchy detonation: CRACK snap + deep BOOM (sine 220→35 Hz) + broadband
  NOISE blast, all `*1.8` OVERDRIVEN into tanh saturation so it lands heavy. Also bumped its
  playback volume 0.6→0.8 in `play_sfx` (explosion should dominate briefly; music is 0.55, break
  0.5, fire 0.3). NB the "soft" wasn't volume — it was the clean un-saturated sine; saturation is
  the fix. Per-kind one-shot volumes live in `play_sfx`.
  ↳ **ROUND 2 (2026-07-16, 2nd screenshot):** the "outer circle" was STILL there — the funnel
  arms + comet-head dots at `r_out=150` formed a big faint ring OUTSIDE the bright event-horizon
  ring (additive bloom makes even faded overlapping arcs sum into a visible circle). Real fix:
  pulled the whole spiral **inside** the ring — `r_out` 150→**112** (< `WARP_CONSUME_R`=120), so
  the bright horizon ring is now the vortex's clean outer edge with NOTHING purple beyond it.
  Also: arm outer fade steeper (`0.34*f*(0.05+0.95*p1)` ≈ 0 at rim), comet brightness `f*hp*hp`
  + head radius `1.5+2*hp` (dark/tiny at the rim → no dots on any circle), ring brightness
  0.6→0.75. Lesson: with additive bloom, "faint + overlapping at one radius" = a visible ring;
  don't let many arms share an outer radius. And **both sfx made DEEPER (user)**: break thud
  150→52 Hz ⇒ **120→42 Hz**; mine boom 220→35 Hz ⇒ **160→28 Hz** (subbier), boom weighted up
  1.3→1.5. clippy clean, 44 green.
  ↳ **ROUND 3 (2026-07-16):** break still "sounded like two blocks of wood" — the culprit was
  the clean SINE (any clear pitch reads as a woodblock "tok"). Rebuilt `break_sfx_wav` with NO
  tonal sweep: body is now layered **sample-and-hold noise** (`noise(i/26)`, `/90`, `/200` — a
  held-value staircase concentrates energy LOW = crunch/rumble, not hiss) over a deep fixed 50 Hz
  sub-thump (quick decay = weight only). KEY LESSON for "rocky vs woody": rock shatter = broadband
  low NOISE, not a pitched tone — drop the sine. Bigger S&H divisor = lower/rumblier. 44 green.
  ↳ **ROUND 4 (2026-07-16): break too LOUD + no SHIP-DEATH sound (user).** Break rock was good
  but drowned the music → lowered its `play_sfx` volume 0.5→**0.3** (it saturates, so its RMS is
  high for its linear vol; break plays constantly so it must sit under the 0.55 music). Added a
  4th SFX **`SoundFx::Death`** — a `death_sfx_wav` (descending "doom" 420→60 Hz via integral-phase
  sweep like the kick, + explosion burst + 45 Hz sub; longer/mournful, distinct from the mine's
  punchy boom), played at 0.7. Wired it the DRY way: emit inside the shared **`kill_ship`** helper
  (added a `sfx: &mut EventWriter<SoundFx>` param) so ALL death paths cover it — ship_death,
  mine_update (already had sfx), enemy_bullets, boss core-contact all now pass `&mut sfx` (added
  the param to the 3 that lacked it; all under the 16-param cap). SfxBank/start_sfx/play_sfx got
  the 4th clip. New test `ship_death_emits_a_sound` (drains `Events<SoundFx>`, asserts a Death).
  Current SFX volumes: fire 0.3, break 0.3, mine 0.8, death 0.7, music 0.55. 45 green, clippy clean.
  ↳ **ROUND 5 (2026-07-16): break "still too high — was perfect in the JS version".** Stopped
  guessing and PORTED the JS `playBreak` from `neon-asteroids/js/audio.js` exactly: white noise
  through a LOWPASS whose cutoff SWEEPS DOWN, and **size-aware** — `f0 = 520 + (3-size)*430` (size3
  ~520 Hz deep boom … size1 ~1380 Hz crack), sweeping to `max(120, f0*0.3)`, `dur = 0.12+size*0.05`.
  That size-awareness (big rock = deep) is what made the JS one "perfect"; my single sample-and-hold
  clip was the same pitch for every rock = never deep enough. Rust has no realtime biquad, and the
  cutoff moves per-sample (needs filter STATE), so `break_sfx_wav(size: u8)` can't use the stateless
  `render_sfx` — it runs **two cascaded one-pole lowpasses** by hand over `noise(i)` then normalizes
  to peak 0.9 (a low cutoff passes little energy → levels vary by size). Plumbing: `SoundFx::Break(u8)`
  carries the size; `SfxBank.break_rock` is now `[Handle;3]` (built from `[1u8,2,3].map(..)`); `play_sfx`
  plays ONE break/frame = the DEEPEST (max size) that broke; emit sites pass `a.size`/`ast.size`.
  LESSON: when the user says "it was perfect in JS," go read `neon-asteroids/js/audio.js` and port it,
  don't re-synthesize from scratch. 45 green, clippy clean.
  ↳ **ROUND 6 (2026-07-16): big 7-part batch.** (1) SFX: added enemy-fire (`enemy_shot_wav`, low buzzy
  460→120 saw), enemy-death (`enemy_die_wav`, small zap-pop), and WARP (`warp_wav` — ported JS
  `playVortex`: saw 640→52 + sine 1020→80 plunge + swept-bandpass noise whoosh). SoundFx enum +
  EnemyShot/EnemyDie/Warp; play_sfx vols eshot 0.28, edie 0.45, warp 0.6. Emit: enemy fire in
  enemy_update, enemy death via new DRY **`kill_enemy`** helper (score+burst+EnemyDie+despawn) used by
  BOTH bullet(collisions) & chain kills; warp on launch in warp_fire. (2) SPAWN "sometimes all small
  rocks": ROOT CAUSE = population_target is a COUNT, so breaking bigs→smalls hits the cap and suppresses
  big spawns. Fix = **BIG_FLOOR=3** (top_up_asteroids now queries `&Asteroid`, forces a size-3 when
  size-3 count < floor, even at cap) + bumped POP_BASE 4→5, POP_CAP 16→18. `spawn_edge_asteroid` got a
  `force_big: bool` param. Also fixes the boss having big rocks to grab (its whole ask). (3) WARP
  off-screen fix: `warp_missile_update` now takes `arena`, detonates at the edge & clamps the hole fully
  on-screen (margin=WARP_CONSUME_R) — firing at the edge opens a usable hole instead of sailing into the
  void. (4) Warp glow bump: `glow` 2.6→3.3, arm alpha 0.34→0.46 (tunable; watch for wash-out). (5) CHAIN
  PICKUP: added `life` field (PICKUP_LIFE=20s) → outlives the 10s boss calm (was: despawn when calm
  ends); now also collectible by SHOOTING it (pickup_update gained a `Bullet` query; dropped the `Wave`
  param). Tests: renamed calm test → `ungrabbed_pickup_expires_after_its_life`, added
  `shooting_the_pickup_grants_the_chain_shot`. 46 green, clippy clean.
  ↳ **ROUND 7 (2026-07-16): multi-track music (user asked "how many tracks can we make & cycle?").**
  Answer given: RAM/startup are non-issues on desktop (~1.3 MB/track, few-ms synth each) — the limit
  is authoring distinctness. User picked **5 rotating tracks + 1 boss track**. Refactored the music
  synth: `techno_loop_wav` → parameterized `render_track(&TrackSpec)` driven by a spec {bpm, roots[4]
  progression, arp[8], arp_mul, Groove enum (Four/Driving/HalfTime/Breaks), lead:bool}; `groove_hits`
  picks drum pattern. 5 `NORMAL_TRACKS` consts (distinct key/tempo/groove) + `BOSS_TRACK` (150 BPM,
  Driving, lead on). Public API: `MUSIC_TRACK_COUNT`, `normal_track_wav(i)`, `boss_track_wav()`
  (removed `techno_loop_wav`/`root_for_bar`/`BPM` const). main.rs: `MusicDirector` resource (holds all
  handles) + `music_director` system + `Track{Normal(i),Boss}` enum, replacing the single-loop
  start_music/music_toggle. Rotates one track per NEW normal wave, spikes to boss on `is_boss_wave`
  (level%5==0) WITHOUT burning a rotation slot, **N** = skip track, **M** = mute (both live via
  swap-by-marker despawn + respawn with `PlaybackSettings.paused`). Tests: `tracks_are_wav_and_nonsilent`
  (all 6), `music_rotates_per_wave_and_spikes_on_boss` (used `Handle::default()` placeholders under
  MinimalPlugins — spawning AudioPlayer w/o AudioPlugin is inert/fine). 47 green. clippy: new
  `manual_is_multiple_of` lint → `s % 2 == 0`→`s.is_multiple_of(2)`.
  ↳ **ROUND 8 (2026-07-16): reworked the ROUND-7 music per user feedback (supersedes it).** Two
  changes: (a) tracks were "all the same w/ slight speed diffs" — because they shared ONE set of
  voices. Fixed by adding selectable TIMBRE: `Lead` enum (Saw/Square/Sine/Pluck) + `Bass` enum
  (Sub/Reese) + `square`/`triangle` oscillators; `lead_voice`/`bass_voice` dispatch. TrackSpec gained
  `lead_kind`, `bass_kind`, `arp_step` (arp plays every N 16ths — 1=driving, 2=sparse/atmospheric),
  renamed `lead`→`top_lead`. The 5 tracks now differ in waveform+density+groove+key+tempo (e.g. t1 =
  sine lead/sparse/half-time atmospheric; t2 = square+reese chiptune; t3 = pluck breakbeat). Removed
  standalone `arp` fn (folded into `Lead::Saw`). (b) per-wave rotation doesn't scale (user targets
  **50 levels**) → switched to a **PLAYLIST with pauses**, wave-independent. `MusicDirector` now has a
  `MusicPhase{Playing,Pausing(f32),Boss}` state machine: play a one-shot track (`PlaybackMode::Despawn`
  so it self-removes → we detect the gap via `music.is_empty()`), then `MUSIC_PAUSE_SECS=5` silence,
  then next (idx cycles the pool). Boss waves still interrupt with the LOOPING boss track, resume after.
  N=skip, M=mute — mute is now by **volume** (`set_volume`, which needs `&mut AudioSink` in 0.16, NOT
  `&`) not pause, so one-shots still finish & the playlist keeps timing. Test renamed →
  `music_playlist_starts_skips_and_spikes_on_boss`. 47 green.
  ↳ **ROUND 9 (2026-07-16): tracks STILL too samey + abrupt start/end (user).** Key insight: swapping
  the lead WAVEFORM wasn't enough — every track was the same ARRANGEMENT (busy arp over drums+bass).
  Differentiate by ROLE instead: added a `pad_voice` (sustained detuned-saw triad, held across each
  2-bar phrase) + `pad: bool` on TrackSpec. Now the 5 split into arp-driven (0 saw/four, 2 square/
  driving/+lead, 3 pluck/breaks) vs. PAD-based atmospheric (1 sine-melody+pad+halftime, 4 reese-bass+
  pad+sparse-saw, dark). Track 2 got `top_lead: true` ("needs more"). **Abrupt start/end fixed**:
  `render_track` gained a `fade: bool` — normal tracks fade in 0.5s / out 1.4s (one-shot, so no hard
  cut); boss track passes `false` (it LOOPS, must stay seamless). **BUG fixed same round**: sparse arp
  (`arp_step` 4) indexed the pattern by `s % 8` → only ever hit notes 0 & 4; changed to
  `(s / arp_step) % 8` so sparse melodies walk the whole 8-note pattern. 47 green, clippy clean
  (pad's constant `s % 32 == 0` → `s.is_multiple_of(32)`; the variable `s % arp_step == 0` is NOT
  flagged). Pad/fade gains are ear-tune candidates.
  ↳ **ROUND 10 (2026-07-16): longer tracks + shorter pause + boss still too samey (user wants ~1-min
  songs).** `BARS` 8→**32** (~50-65s/track by tempo). To keep a minute from feeling like one loop
  repeated, added an ARRANGEMENT: `Section{Intro,Break,Full}` + `section_of(bar)` — one-shot tracks get
  a 2-bar drum-build intro (bars 0-1 arp+bass only, kick from bar 2), a mid breakdown (bars 16-19 drop
  kick+clap), else full. Gated on the renamed `render_track(spec, oneshot)` param (was `fade`; oneshot
  now controls BOTH the fade AND the arrangement); boss passes `false` (loops → constant intensity,
  seamless, no arrangement). Pause `MUSIC_PAUSE_SECS` 5→**1.5**. **Boss reworked to be its own beast**
  (was just a faster normal track): new `Groove::Pound` (kick every 8th, relentless) + a TRITONE
  progression (A→E♭ leap→F→E) + tritone/♭6-laced arp `[0,6,7,6,12,8,6,3]` + `pad:true` dark drone +
  top_lead. Mem/startup: ~5.3 MB/track now (60s), 6 tracks ≈ 32 MB; rendering all at startup is still
  sub-second but is the first lazy-render candidate if launch feels slow. 47 green, clippy clean.
  ↳ **ROUND 11 (2026-07-16): PIVOT — user gave up on 5 distinct tracks ("a lot still similar,
  fadeout still abrupt"), asked to "take the first track and extend it alone into a full-length
  track."** So: DELETED the whole generic multi-track system (TrackSpec, Groove+groove_hits,
  Lead/Bass enums+lead_voice/bass_voice, Section/section_of, NORMAL_TRACKS, render_track,
  MUSIC_TRACK_COUNT, normal_track_wav, BARS const, `triangle`). Replaced with TWO bespoke
  hand-arranged functions sharing the voice helpers + a `master()` (normalize/tanh/wav) helper:
  • **`main_track_wav`** — ~2 min (64 bars @128), A-minor, real arrangement: intro(0-7, drums build
    in) → DROP A(8-23, +hook melody) → breakdown(24-31, drums drop, pad carries) → build(32-39, noise
    riser swell + snare roll) → DROP B(40-55, hook up an octave) → outro(56-63, drums fade). Has a
    real 4-bar HOOK melody (`melody_voice`, absolute A-minor, `REST=i32::MIN`), a `crash` on drops.
  • **`boss_track_wav`** — bespoke 16-bar loop: Pound kick every 8th, tritone prog A→E♭→F→E, square
    arp + octave-up lead, reese bass, dark pad drone.
  Both **LOOP** (`PlaybackMode::Loop`) → NO fade, NO pause → the abrupt-fade complaint is gone (no
  track-end to cut). Voice fns now plain (no enums): `arp_saw`, `arp_square`, `melody_voice`, `reese`,
  `crash`, kept `bass`/`pad_voice`/`kick`/`clap`/`hat`/`saw`/`square`/`noise`. Director GREATLY
  simplified: `MusicDirector{main,boss,on_boss:Option<bool>,muted}` — swap main↔boss on `is_boss_wave`
  change, M mutes (volume). Dropped N/skip (one track). Test → `music_swaps_between_main_and_boss`.
  Mem ~10.6 MB main + ~2 MB boss. 47 green. **If user wants it even fuller: add more sections/bars,
  a counter-melody, or vary DROP B's progression — all inside `main_track_wav`.**
  ↳ **ROUND 12 (2026-07-16): music polish + boss pacing.** (a) Main track "reminded user of a
  CARNIVAL" → darkened: progression Am F C G (bright majors) → **Am F Dm E** (i-VI-iv-V dramatic
  minor), arp kept within one octave (dropped the bright octave-leap that read as a merry-go-round;
  added ♭6), hook made sparser/lower (base 440→**220**, moodier). (b) **Boss buildup**: new
  `boss_buildup_wav()` — ~10 s riser (swelling low-A drone + rising-cutoff noise sweep + accelerating
  "heartbeat" kicks) played in the last `BOSS_CAMEO_SECS`(=10) before a boss so it doesn't slam in
  cold. (c) Director rebuilt around a **`MusicCue{Main,Buildup,Boss,Silence}`** state machine (was
  main/boss only): desired = Silence if `calm>0`, else Boss if boss wave, else Buildup if
  `is_boss_wave(level+1) && timer<=BOSS_CAMEO_SECS`, else Main. `play_track` regained a `looping`
  arg (buildup is one-shot Despawn). (d) **Post-boss 10 s calm is now SILENT** (Silence cue) — no
  longer slams the track back on. (e) **`clear_calm_field`** system despawns leftover asteroids/mines
  (incl. the boss's scattered shield) while `calm>0` so the breather is clean (spawns were already
  gated). Test → `music_cues_follow_the_boss_cycle`. 47 green.
  ⏭️ **DEFERRED (next: the wave 6-10 gameplay arc, per user):** wave content schedule — 6 green
  introduced (as before), 7 ONLY green + mines, 8 green+mines+MOBS (mobs return — currently
  ENEMY_LAST_WAVE=5 stops them), 9 same as 8; and a **2nd boss at wave 10**: a small RED asteroid
  that SEEKS other asteroids and grows by eating them; player limits its "food" (clear rocks) while
  chipping its HP (much higher than boss 1's 28). Big new system — confirm design before building.
  ↳ **ROUND 13 (2026-07-16): boss-1 grab bug fix + 2nd-boss design LOCKED.** Bug: `ship_death`
  collided with ALL asteroids incl. `Shielded` ones, so a rock the boss reels in from behind the
  player killed the player mid-transit. First fix (`Without<Shielded>`) was TOO BROAD — user
  clarified the ORBITING shield SHOULD still hurt; only the reel-in transit shouldn't. Final fix:
  `ship_death` query includes `Option<&Shielded>` and skips a rock only while `grab < BOSS_GRAB_TIME`
  (=0.37s, still reeling in); settled/orbiting shield rocks (grab≥time) stay lethal, and free/thrown
  rocks always hit (a thrown rock drops Shielded + adds Thrown at boss_shield ~L1839). Tests:
  `rock_reeling_in_does_not_kill_the_ship` + `settled_shield_rock_still_kills_the_ship`. 49 green.
  **2nd boss (wave 10) CONFIRMED:** grow = bigger
  hitbox (sizes out the player) AND tankier (more HP); start HP=**70**; build next with the wave 6-9
  schedule.
  ↳ **ROUND 14 (2026-07-16): boss 1 now ROAMS (user: shouldn't just hover).** Replaced the in-place
  bob (removed `BOSS_BOB` const) in `boss_update`'s alive branch with a slow wandering Lissajous it
  EASES toward (no snap): `cx = sin(pulse*0.16)*(h.x-margin)*0.72` (wide horizontal sweep),
  `cy = rest_y - h.y*0.15*(...)` (stays up top, gentle dip), `p += (target-p)*(1-exp(-dt*2.6))`.
  So it sweeps the upper arena + its shield sweeps with it. 49 green. **After wave 10 = LOOP 1-10**
  (user wants to perfect 1-10 before building 11+): when I build the wave arc, map content by a cycled
  wave, don't add new 11+ content. is_boss_wave stays level%5 (bosses at 5,10,15,20 map to content
  5/10/5/10 → boss1/boss2 alternate correctly on a 10-cycle).
  ↳ **ROUND 15 (2026-07-16): grab tuning + music back to CLUB TECHNO.** (a) Grab too fast →
  `BOSS_GRAB_TIME` 0.37→**1.0**, reel ease `1-exp(-dt*10)`→`-dt*3` (slow, telegraphed; longer exempt
  window too). (b) Boss now only grabs from the **top half** (skip free rocks with `y<=0` in
  boss_shield capture) — no long cross-screen yanks; matches its roam zone. Updated the 2 capture
  tests to place rocks at y>0. (c) **MUSIC**: user said it "got away from the techno club vibe."
  Root cause = missing the two club signatures. Fix in `main_track_wav`: split into TWO buses
  (`drums` unpumped, `music`), added a **SIDECHAIN PUMP** (music ducks to 0.35 on each beat and
  swells back during four-on-floor sections — the club "breathing") + **RAVE STABS** (new
  `stab_voice`, supersaw chord hits on bp 0/6/10 in drops). REMOVED the sung hook melody
  (`melody_voice`→replaced by `stab_voice`; deleted `REST` const) — it was the song-y/carnival
  element; club is groove+stab, not melody. Kept dark Am-F-Dm-E + hypnotic 1-octave arp + offbeat
  bass + intro/drop/break/build/drop/outro arrangement. 49 green. **Can add a subtle lead back if
  the user misses a melody.**
- ⚠️ **README.md is STALE** — still says "vertical slice / not been compiled / Fire: Space"
  and lists mines/enemies/bosses/pickups/audio/dense rocks as "not yet ported" (all DONE),
  and pre-dates the VIOLET EDGE rename. Needs a full refresh (flagged as a separate task, not
  done inline to keep changes scoped).
- **PLAN/priority (user, 2026-07-16):** finish the Rust version FIRST, THEN update
  git + the repo + wire the [[baz-studios-launcher]]. Don't invest in launcher/
  release plumbing before the port is real. *(git DONE — see [[neon-edge-github-repo]]; README refreshed.)*
- **ROUND 16 (2026-07-17): WAVE 6-10 ARC + BOSS 2 (devourer) + 1-10 LOOP.** `content_wave(level) =
  ((level-1) mod 10)+1` — all wave-content gating routes through it so 11+ REPLAYS 1-10 (perfect 1-10
  first). Schedule (in `roll_dense`/`enemy_target`): 6 green mixed in (0.5), 7-9 ALL green (1.0), mobs
  in two windows content 3-4 & 8-9 (none 6-7 or boss waves); removed now-dead consts ENEMY_FIRST/LAST/
  PER_WAVE + DENSE_FIRST_WAVE/FRACTION. Bosses alternate by `is_devourer_wave` (content 10 = boss 2,
  content 5 = boss 1); `boss_director` spawns the right one. **`Devourer`** (boss 2, red, hot-red
  `devourer_color` no-blue): `devourer_update` SEEKS nearest free rock, EATS on contact → `grow`
  (radius base 22→max 110) + heals (`hp` up to 140 cap), hunts the ship when field is clear, contact-
  kills. Start HP 70. Bullets chip it (added a branch in `collisions`); chain/warp do NOT damage it —
  they clear its FOOD (starve mechanic). Shared `defeat_boss()` helper (reward+advance+calm) used by
  both bosses; boss-1 chain-pickup check now `content_wave==5` so it re-offers each loop. Rendered in
  `render_boss` (jagged red maw, HP-scaled core). 53 green, clippy clean.
  ⚠️ Minor polish left: the pre-boss CAMEO ghost (render_boss) always draws the magenta SHAMAN shape
  even before the devourer wave — cosmetic only. Committed LOCALLY, NOT pushed (user testing first).
- **ROUND 17 (2026-07-17): MASS SHOT (the wave-10 pickup).** Primary-weapon upgrade dropped by the
  devourer (boss 2), mirroring the chain drop after boss 1. `PickupKind{Chain,Mass}` on `Pickup`;
  drops route by boss; `pickup_update` unlocks the right one (Mass sets `MassShot{unlocked,active}` and
  auto-activates). **`Q` toggles standard↔mass** (in `fire`, only once unlocked). Mass = bigger
  (`MASS_BULLET_R` 7 vs 3), slower (`MASS_COOLDOWN` 0.5 vs 0.18), harder (`MASS_POWER` 3 vs 1). Added
  `mass: bool` to `Bullet` + `bullet_radius`/`bullet_power` helpers; `collisions` now queries `&Bullet`
  and uses per-bullet `br`/`power` for ALL hits (dense-rock chip `a.hp -= power`, boss core, devourer).
  Chain/warp still DON'T damage bosses (starve mechanic) — only bullets do, and mass 3× the punch.
  `mass_color` = hot violet (player kit); mass bullets render fatter+hotter; mass orb tinted. Reset in
  `gameover_restart` (added MassShot to the progress tuple + a Devourer despawn query to hazards).
  55 green, clippy clean. Pickup ROADMAP now: chain=W5 ✅, mass=W10 ✅, drone=W15 (future).
- **ROUND 18 (2026-07-17): MAIN MENU + run lifecycle + pause-quit.** Added `GameState::Menu` (now the
  DEFAULT — boots to the menu, not straight into play). `setup` no longer spawns the ship at Startup;
  a run spawns it on Start. New shared `reset_run()` (resets Run/Score/Wave/Warp/BossState/Chain/
  MassShot + spawns the ship) used by BOTH `menu_start` (Enter/Space in Menu) and `gameover_restart`.
  New `type GameplayEntity = Or<(With<Ship>,…13 markers…)>` — one filter for "everything in a run";
  `clear_field` (OnEnter Menu) wipes it (starfield+camera excluded → backdrop persists), which
  de-duplicated gameover_restart's old 13-query clear. **Pause-quit:** `pause_toggle` now handles
  Paused→`Q`→Menu (Esc still resumes). Game-Over: Enter restarts, `Esc`→Menu. Menu UI = "VIOLET EDGE"
  title + "Enter — Play" (overlay dims the grid/stars/HUD behind it, like Pause). Registered
  OnEnter/OnExit(Menu). Tests: `menu_start_resets_and_spawns_a_ship`, `clear_field_wipes_...`. 57 green.
  **NEXT: achievement system** (user asked for it same msg; deferred as its own pass — needs defs +
  unlock triggers + a toast + a menu list + persistence).
- **ROUND 19 (2026-07-17): ACHIEVEMENT SYSTEM.** 7 achievements (`Ach` enum + `ACHIEVEMENTS` array +
  `ach_meta`/`ach_met`): First Blood (kill an enemy), **Warden Off** (boss 1 = "the Warden"),
  **Glutton for Punishment** (boss 2 = "the Glutton"), True Blue (100 blue), Green Thumb (100 dense
  green), **Edgelord** (beat the arc = defeat boss 2), Purist (beat boss 2 w/ no powerup grabbed).
  Architecture: LIFETIME `Stats` resource (blue/green/enemies counts + warden/glutton/no_powerups
  bools) — the single source; `Achievements{unlocked:[bool;7]}` derived from it. `achievements` system
  polls Stats each frame, unlocks (flag + TOAST + chime + save) the first time each `ach_met`. Tracking
  is DIRECT increments at the real player-destruction sites (collisions + chain_update get `Stats`;
  count blue/green/enemies — NOT via SoundFx events, since the devourer's eat also emits Break and
  would pollute the count). Bosses set warden/glutton in boss_update/devourer_update; `RunFlags
  {powerup_used}` (per-run, set in pickup_update, reset in reset_run) drives Purist. **Toast**:
  `ToastRoot` persistent top-center column (Startup) + `Toast{life}` boxes (spawned as children,
  `toast_update` expires them after 3.5s). **Sound**: `achievement_sfx_wav` (bright rolled major
  arpeggio) → SfxBank.achievement, played directly via `one_shot` in the achievements system (NOT a
  SoundFx variant — a system can't be both EventReader+EventWriter<SoundFx>). **Persistence**:
  `violet-edge.save` (6 space-sep numbers), `load_progress` (Startup) + `save_progress` (on unlock),
  both `#[cfg(not(test))]` so tests never touch disk. Menu lists all 7 (earned = bright name+desc,
  locked = grey "??? — <goal>"). 58 green, clippy clean. **Boss names now canon: boss1=THE WARDEN,
  boss2=THE GLUTTON** (only in achievement text so far; no in-game name labels yet).
- **ROUND 20 (2026-07-17): main-menu polish.** (a) Title read pink → new `title_color()` =
  srgb(2.2,0.35,5.5) (B-dominant, low green so it doesn't bloom pink/white; ship_color stays bright
  for gameplay). (b) Achievements moved OFF the main menu into their OWN screen: new
  `GameState::Achievements`, `spawn/despawn_achievements_ui` (OnEnter/OnExit), `achievements_back`
  (Esc/Enter→Menu). Main menu now = title + "Enter — Play" + "A — Achievements (n/7)"; `menu_start`
  handles `A`→Achievements. (c) Grid + HUD hidden on the menu screens: added `run_active(state)`
  helper (= Playing|Paused|GameOver); render zeroes the grid color + skips the lives-icon gizmos when
  !run_active (State bundled into render's `abilities` tuple — it was already at the 16-param cap);
  HUD text entities tagged `Hud` + a `hud_visibility` system toggles Visibility. Starfield stays as a
  faint menu backdrop. 58 green, clippy clean.
- **ROUND 21 (2026-07-17): menu slickness + the color-clamp bug.** ⚠️ KEY LESSON: **UI `TextColor`
  CLAMPS each channel to 1.0** — so the HDR-style title `(2.2,.35,5.5)` collapsed to `(1,.35,1)` = hot
  pink. UI colors MUST be ≤1. Fixed `title_color`→`(0.62,0.18,1.0)` violet; unlocked-achievement color
  →`ach_earned_color (0.82,0.45,1.0)` (was mass_color, which clamps to white). (World/gizmo colors
  still use >1 for bloom — the clamp is UI-only.) Also: forgot to gate the **warp charge pips** +
  **chain pips** (bottom HUD, gizmos) by `show_run` — they showed on the menu ("warp shot still
  visible"); now gated (gap/py hoisted out of the gate since both pip blocks share them). Achievements
  screen now shows the NAME greyed when locked (not "???"). **Clickable buttons**: `MenuButton
  (MenuAction{Play,Achievements,Back})` + `Button` bordered pills via `menu_button()` helper (param
  type `&mut ChildSpawnerCommands` in 0.16); `button_shimmer` pulses the hovered border/text (mix by
  a time sine), `button_click` fires `MenuClick` on the `Changed<Interaction>` press edge; menu_start/
  achievements_back consume MenuClick alongside the keys (read events ONCE into a Vec — two
  EventReader.read() calls would drain the first). Tests that run menu_start need `add_event::
  <MenuClick>()`. 58 green, clippy clean.
- **ROUND 22 (2026-07-17): menu slickness + the TOFU bug.** ⚠️ LESSON: the default Bevy font has NO
  glyph for the em dash "—" (or other non-ASCII) → it rendered as a TOFU SQUARE (□) everywhere I used
  "key — action". **Stick to ASCII in UI text.** Fixes: achievements list → a real **two-column
  table** (name cell `Node{width:300}` | description, no separator glyph); pause/gameover hints
  reformatted to **"Action (Key)"** (no dash). Also caught more UI color CLAMP bugs (same as R21): the
  "PAUSED" `(2.4,1,4.6)` and "GAME OVER" `(5,1.2,1.2)` both clamped to WHITE — fixed to title_color
  (violet) and `(1,0.3,0.3)` (red). Slickness: `overlay()` gained an `alpha` param (menu 0.25 /
  achievements 0.5 = starfield shows through; pause/gameover stay 0.72); `MenuTitle` marker +
  `menu_pulse` system breathes the title between two violets; added a tagline "A NEON ASTEROIDS LOVE
  LETTER". 58 green.
- **ROUND 23 (2026-07-17): neon flicker + frame.** User picked (via Other): drop the tagline, title
  "flickers on like a neon light", add the neon frame. Done: removed tagline; `MenuTitle{age}` +
  `menu_title_fx` — NEON_WARMUP=1.5s of erratic blinks (two detuned sines, threshold drops as it
  settles → flickers ON) then a steady breathe; `dim(base,b)` scales the ≤1 UI color so low b = sign
  "off". `spawn_frame()` = a violet bordered+rounded full-screen-inset Node (24px) tagged MenuFrame,
  spawned BEFORE the overlay so it's behind the content (never eats button clicks); its border pulses
  with the title in menu_title_fx. 58 green, clippy clean. **FONT: user wants a more "neon" typeface —
  NOT done: the built-in Bevy font is the only one; a real neon font = embedding a `.ttf` (would be
  the project's FIRST asset, via include_bytes! + Assets<Font>). Offered to wire it if they drop a
  font file in. Flicker+glow gives the neon feel meanwhile.**
- **ROUND 24 (2026-07-17): Orbitron font wired — FIRST ASSET.** User picked "suggest an open font" →
  recommended **Orbitron** (SIL OFL, Google Fonts); they dropped the static/ folder. Wired
  `Orbitron-Bold.ttf` in: `load_font` (Startup) `include_bytes!("../assets/fonts/static/Orbitron-Bold
  .ttf")` → `Font::try_from_bytes` → `MenuFont(Handle<Font>)` resource; new `text_f(font,size,col,s)`
  helper (mirrors `text()` but sets `TextFont.font`). ALL overlay screens (menu/achievements/pause/
  gameover incl. title + buttons + table) now use Orbitron; the tiny in-game HUD keeps the default
  mono. Still ONE self-contained exe (font embedded, not runtime-loaded). Trimmed the 5 unused
  weights (kept Bold) + added `assets/fonts/README.txt` attribution (OFL). Font must stay committed
  (compile-time include). 58 green, clippy clean.
  ↳ **FIX (same day): the Startup `load_font` PANICKED at runtime** — the initial `OnEnter(Menu)` →
  spawn_menu_ui runs BEFORE a Startup system's deferred `insert_resource` flush lands, so
  `Res<MenuFont>` didn't exist. ⚠️ LESSON: a resource needed by the INITIAL state's OnEnter must be
  inserted at BUILD time, not via a Startup-system command. Fixed: `install_menu_font(&mut app)` in
  `main` after DefaultPlugins (provides `Assets<Font>` in build) + before `app.run()` — grabs
  `Assets<Font>`, adds embedded Orbitron, `app.insert_resource(MenuFont(..))` synchronously. Links clean.
- **Round (2026-07-17): field-density + gold 1UP rock.** (1) Small-rock clutter fix — `asteroid_bounds`
  now CULLS off-screen small debris (85% size-1 / 35% size-2; large always recycle) so breaking rocks
  can't silt the field up; BIG_FLOOR 3→4, edge-spawn large bias 0.7→0.8; Warden prefers big/mid rocks
  (grabs a small only if nothing bigger is on-screen). (2) `Fresh(FRAGMENT_GRACE=1.8s)` marker on break
  fragments — while it runs they recycle instead of culling, so a rock shattered at the edge doesn't
  lose its pieces before you can shoot them. (3) **Gold 1UP rock** (life economy, see DESIGN.md):
  `Gold` marker (inherited through EVERY break: bullet/chain/mine, via a `gold: bool` param on
  `break_asteroid`) + `GoldRush{active,forfeited}` resource. Spawns rarely on non-boss waves
  (`GOLD_CHANCE`). `gold_rush_update` grants +1 life (cap `LIFE_CAP=5`) when the whole lineage is gone
  AND not forfeited; a gold piece culled off-screen latches forfeited (asteroid_bounds). Devourer query
  gained `Without<Gold>` (no false 1UP). Distinct `life_sfx_wav` 1UP jingle + "EXTRA LIFE" toast. Menu
  flicker also refined (soft 3rd-strike glow-up, start-menu-only). **64 tests green, clippy clean.**
  ⚠️ patterns reused: shared `&Query` type must match across all callers (blast/collisions/mine); UI
  colours ≤1 (gold toast uses a plain gold, gizmos use HDR `gold_color()`); reset_run's `progress`
  tuple grew a 5th slot (GoldRush) to dodge the 16-param limit.
- **Round (2026-07-17, cont.): scoring/high-scores, orange asteroid, boss polish, warp reach, handling.**
  (a) **Scoring** finalized (classic-Asteroids 20/50/100 etc.; `WARP_ROCK_SCORE`=25) + **top-5 high
  scores** persisted to `violet-edge.hiscore` (`HighScores{top,just_placed}`, `record_high_score` on
  OnEnter(GameOver) BEFORE spawn_gameover_ui, "NEW BEST!"/"TOP 5!" board, menu "BEST" line). Score is
  rank-only (no score-extends). (b) **Menu** grew CONTROLS + BRIEFING screens (new GameState variants +
  shared `submenu_back`), pause = RESUME/QUIT **buttons** (pause_toggle reads MenuClick), and the
  start-fire bleed is fixed via `FireArmed` (disarm on OnEnter(Playing), re-arm on release). Title
  flicker plays once (`TitleIntroPlayed`). (c) **Orange explosive asteroid** built: `Explosive`+
  `Detonating{fuse}` markers, `detonate` system — a lit orange (bullet/chain/mine light it) blasts
  `ORANGE_BLAST_R`(150) and **obliterates** everything in it (gold spared), chains other oranges, kills
  the ship. **Not yet in wave content**; dev **F3** drops one mid-field. (d) **Gold economy** settled:
  spawns via a countdown (`gold_spawn`, `GOLD_MIN_GAP`..`MAX` ~4-6min, re-armed at spawn), can appear on
  any wave; `LIFE_CAP`=`START_LIVES` (restore-only); pieces get a long grace (`GOLD_GRACE`=6s, recycle)
  then cull→forfeit (not immediate). (e) **Glutton (Devourer)**: HP bar (shared `boss_hp_bar`, Warden
  too), grows to `DEVOURER_MAX_R`=200 then **OVERLOADS** (screen-wide burst `DEVOURER_BURST_R`=420 wipes
  field+kills player, then shrinks to base); gunfire shrinks it (`DEVOURER_SHRINK_PER_HIT`). (f) **Warp**
  now flies until the wall it's HEADING toward (velocity-aware edge detonation) w/ range to cross the
  arena (speed 550/life 3.0) — fixes short reach from a corner. (g) **Handling**: TURN_RATE 3.6→4.6,
  FRICTION 0.55→0.38 (less drift, top speed intact). **80 tests green, clippy clean, all committed
  locally (unpushed).** NEXT: the **Darter** mob (waves 12-13), then Slinger boss (15), then wire 11-15;
  full plan in `neon-edge-bevy/DESIGN.md`.
- **Round (2026-07-18): controller support + rebindable input (released v0.2.5).** Built an input layer:
  `Action` enum + `Bind`(Key/Mouse/Pad) + `Bindings{kbm,pad}` resource (rebindable) → `gather_input`
  (PreUpdate) resolves keyboard+mouse+**gamepad** into `ActionState` (analog `turn`/`thrust`; left stick
  turns, d-pad/RT/A/LT/RB/X/Start defaults); refactored ship_control/fire/warp_fire/chain_fire/
  pause_toggle/music_director off raw `ButtonInput` onto ActionState. Also: **later handling tune** — TURN
  4.6→? , FRICTION 0.38→0.15 + THRUST 620→1000 (much less drift for precise flying); bullet range now
  scales with the arena (`BULLET_RANGE_FRAC`, fixes short reach on big screens); .exe file icon via
  `build.rs`+winresource; logo masthead+window icon (`assets/logo.png`). `InputMethod`(Auto/KbM/Controller)
  + `GameState::Settings` screen: input-method selector + rebind every action for BOTH devices (RebindSlot
  cells, `Rebinding` capture — click cell → press input; Esc cancels/never binds; reset-to-defaults).
  ⚠️ **PENDING (told the user): (1) bindings DON'T persist across launches yet** — needs serde + bevy
  "serialize" feature to serialize `Bindings` (deferred pending a deps OK); **(2) controller MENU
  navigation** (menus still need mouse/keyboard; gameplay + rebind-capture work on pad). 84 tests, clippy
  clean. Release recipe now: bump+tag+push → CI does all 3 platforms (see [[neon-edge-github-repo]]).
