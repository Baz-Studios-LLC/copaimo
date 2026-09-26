---
name: neon-edge-github-repo
description: GitHub repo, gh auth, and release recipe for VIOLET EDGE (the neon-edge-bevy port)
metadata:
  node_type: memory
  type: reference
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
  modified: 2026-08-04T15:36:40.722Z
---

Repo: **`Baz-Studios-LLC/Violet-Edge`** (`https://github.com/Baz-Studios-LLC/Violet-Edge.git`) — renamed
2026-07-17 from `Neon-Drift` (old URL redirects). Holds the Rust/Bevy port ([[neon-edge-bevy-port]]).
`Desktop/neon-edge-bevy` is the git repo (branch `main`, origin = Violet-Edge). History was force-wiped
to the Bevy commits (old JS history gone from the remote; still local in `Desktop/neon-asteroids`, the
JS reference, whose origin is the old Neon-Drift URL). `.gitignore` excludes `/target`, `*.save`, `*.hiscore`.
Launcher repo = **`Baz-Studios-LLC/baz-studios-launcher`** (HTML, same org) — NOT cloned locally yet ([[baz-studios-launcher]]).

**`gh` IS AUTHENTICATED** (2026-07-17) as **JPhantom-sh**, scopes `gist read:org repo workflow`, binary at
`C:\Users\jsull\AppData\Local\Programs\GitHubCLI\bin\gh.exe` (v2.96). So `gh release create` / `gh repo …`
work directly now — no need to route the user through the UI. (Plain `git push` also works via the Windows
Credential Manager token.) ⚠️ Never handle a raw token; if re-auth is ever needed the USER runs `gh auth login`.

**Releases are automated** via `.github/workflows/release.yml` (added 2026-07-18): a `v*` tag push (or
manual `gh workflow run release.yml -f tag=vX.Y.Z`) builds Windows/macOS/Linux natively on GH runners
and attaches `violet-edge-<tag>-windows.zip`, `-macos-arm64.dmg` (a real `VIOLET EDGE.app` in a
drag-to-Applications dmg), `-linux.zip`. This is the single source for release artifacts. So the recipe
is just: bump `Cargo.toml` version → commit → `git tag -a vX.Y.Z && git push origin vX.Y.Z`. (Optionally
also `gh release create` a locally-built Windows exe for instant delivery; CI adds the rest.) The exe is
self-contained (font `include_bytes!`, procedural audio, no runtime asset loads); release build has NO
dev keys (F1/F2/F3 are `#[cfg(debug_assertions)]`).
⚠️ **User requirement (2026-07-18): EVERY release must ship an up-to-date macOS `.dmg`.** Always verify
it's attached after a release (`gh api repos/Baz-Studios-LLC/Violet-Edge/releases/tags/vX.Y.Z --jq
'.assets[].name'`); re-run the workflow if the mac job failed.
⚠️ macOS-runner gotcha: `hdiutil` fails **"No space left on device"** on the .dmg. `rm -rf target` alone
was NOT enough (recurred on v0.2.7). **Fix (2026-07-21):** the mac step now ALSO `strip`s the release
binary (Bevy release is ~100MB → ~30MB, so the .app + dmg fit the scratch budget) and frees
`~/.cargo/registry` + `~/.cargo/git`. If the mac job ever fails again, re-run just that release:
`gh workflow run release.yml -R Baz-Studios-LLC/Violet-Edge -f tag=vX.Y.Z` (workflow_dispatch re-uses
main's workflow def + attaches to the existing release).
⚠️ **Watching a release run: guard the run id.** The obvious `RUN=$(gh run list --limit 1 --jq ...)`
then `until gh run view "$RUN" ... = completed` loop has NO failure exit: one transient network blip
makes `gh run list` return empty, and the loop then polls `gh run view ""` forever (this actually
happened 2026-08-03 — a watcher ran for ~21h until it was found and killed). Either pass the run id
explicitly (the tool result of the tag push gives it), or bail when `$RUN` is empty / after N
consecutive failures.
**LATEST = v0.6.1** (2026-08-04) "The front door: a neon tube, a lockup, and a sky that moves" — the
menu presentation pass (two-layer neon-tube border, the `VIOLET [ship] EDGE` inline lockup, uniform
300px buttons, progress counts moved off the menu into each section, an EXIT button, drifting parallax
starfield) + the two fixes that missed the v0.6.0 tag (special-rock cap must not count an act's own
carrier rock; cap keys off the wave target not the live count). PATCH bump. All 4 assets verified,
`.dmg` valid UDIF v4 + bundle reports 0.6.1, CI green on all three runners.
Previously **v0.6.0** (2026-08-03) "The Facet, the Husk, the Gorge Round, and a field you can read" —
2 new NG+ rock types, the Gorge Round powerup, Glutton+ INHALE/REGURGITATE, the mob rework (telegraph
cut, rounds 150px/s, mobs now die to rocks/wells/mines/each other), field density 18→12. MINOR bump
(user approved: "No it's fine"). All 4 assets verified; `.dmg` = valid UDIF v4, 24.3MB payload, and
the `.app` carries a real arm64 Mach-O (57MB) + Info.plist 0.6.0.
⚠️ **v0.6.0 was tagged, then RE-CUT before publish:** a bug check found the new special-rock cap
counted Act III's carrier (red) as a garnish, which would have squeezed beacons+clusters out of waves
21-29. Cancelled the run, deleted the tag, re-tagged from the fixed commit. Confirms the rollback
recipe below works for a not-yet-published release (no `gh release delete` needed — CI creates the
release only at the END, so before that a tag delete is enough).
Earlier: **v0.5.0** "A real soundtrack" (2026-07-30 — the whole score is now Antigravity-PRODUCED mp3s (main/boss/
gameover), procedural music score deleted, corruption tiers dormant; MINOR bump because it's a
milestone, not a modest addition — see [[violet-edge-produced-music]]). **Full release audit
2026-07-30: every release v0.2.6→v0.5.0 carries all 4 assets** (linux zip, macos .app.tar.gz,
versioned .dmg, windows zip); v0.2.0-v0.2.5 predate the current naming (legacy, launcher unaffected
— it matches the newest release by suffix). All 20 git tags have matching releases; the only two
non-success CI runs in history are v0.4.2 (deliberately CANCELLED aim-assist rollback) and v0.2.7
(failed then re-run — its assets are all present). v0.4.9 (2026-07-30 —
the first PRODUCED music track: the Antigravity game-over theme). v0.4.8 (2026-07-30 —
NEW GAME+ v1 (menu-gated on `stats.phantom`, Warden+, full-roster act I), the SPLIT ECONOMY (large
sheds 1-2 mediums / medium sheds 2 smalls or dies — user design to thin small-rock crowding), the
big balance pass (boss HP ramp ~2x = 50/60/85/72/90 + Phantom 95/phase, Warhead → 1.3s siege
cadence, waves 120→100s w/ rescaled gold economy, small rocks 30→26), vortex sfx voice; all 4
assets verified). v0.4.7 (2026-07-30 —
the JUICE pass (hit-stop/shake/kill pops/BossDown boom), corruption v3 tape-sag music tiers (+ the
render_tier_previews audition tool), bell-dirge game-over track, Baz Studios boot splash + sting
(bevy mp3 feature), warhead detonates-on-impact w/ real AoE, ribbon 72pts; all 4 assets verified).
v0.4.6 (2026-07-29): flight-feel pass (turn 5.2, analog trigger thrust, deadzone fix), drift tune
(FRICTION 0.10/THRUST 1200), sealed Pilot Log + decrypt toasts + log sfx, Pacifist achievement
(restraint-not-survival), mine bounty w/ zero blast-rock score. v0.4.5 (2026-07-29 —
23 achievements + restart ladder, Game Over best-wave/nearest-grind progress lines, Detonator flow fix
(2.5s prime, no wave-20 orange), beacon aura 200→270, armed warhead round visuals, longer ship ribbon,
Glutton fangs, Pilot Log/menu fit; all four CI assets verified incl. the .dmg, mac leg passed first try).
v0.4.4 (2026-07-28): living-boss spectacle revamp + warning banner, act
ownership (no rock outlives its act), Cluster + Beacon rock types, random finale field, Limpet
REMOVED, ship-shaped Nova shell; patch bump per the slow-versioning policy. v0.4.3 earlier the same
day: Nova Shield, 12 achievements, PILOT LOG lore, labeled top HUD, any-screen scaling. **v0.4.2 — bigger small targets, aim assist REMOVED:**
small asteroids 22→30, mines `MINE_R` 13→18, mobs `ENEMY_R` 14→19 (body/hit/draw radii only — a mine's
kill is still `MINE_BLAST_R`, mobs still threaten only with bullets, so no danger change).
⚠️ **Aim-assist saga:** v0.4.1 shipped a slight snap-assist (`assisted_aim`); a stronger version (wider cone
+ target leading) was cut as a first v0.4.2 then the user **REJECTED aim assist entirely** — that release was
CANCELLED + deleted pre-publish and v0.4.2 was RE-CUT as bigger hitboxes ([[neon-edge-hittability]]). The whole
`assisted_aim` helper/consts/test are gone. **Rollback recipe for an in-flight release:** `gh run cancel <id>`
+ `gh release delete vX -R Baz-Studios-LLC/Violet-Edge --yes --cleanup-tag` + `git tag -d vX` (GitHub re-flags
the prior release as Latest, so the launcher falls back to it). Cargo.toml/lock now 0.4.2. **v0.3.0:** The Haunt finale + the full wave-30 six-boss
run. **v0.4.0 (2026-07-23) "The Haunt reforged":** P2 POSSESSION mechanic, the cinematic death scene,
charge-only P3, aimed full-arena ray, the Haunt's own sfx (`SoundFx::Haunt`), rock-dissolve, + a game-wide
photosensitivity pass ([[neon-edge-photosensitivity]]). **Release cadence: BATCH changes; hold pushes until
the user says go** ([[neon-edge-release-cadence]]). `Cargo.toml` reads 0.6.1 (bump it + Cargo.lock per release).

✅ Stale `latest` tag gone. **Launcher assets (native, version-less):** CI emits
`violet-edge-windows-x86_64.zip` + `violet-edge-macos-aarch64.app.tar.gz` (+ human versioned `.dmg`).
The [[baz-studios-launcher]] matches them by SUFFIX and auto-picks-up the newest published release — so
future game releases need NO launcher change (verified v0.2.7 resolves via the launcher's own API query).
