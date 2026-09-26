---
name: violet-edge-produced-music
description: "VIOLET EDGE music is ALL Antigravity-produced tracks (migration done 2026-07-30); procedural score deleted; corruption tiers dormant; per-track wiring checklist"
metadata:
  node_type: memory
  type: project
  originSessionId: a92be22f-4cd2-4f48-8804-9baf0ef0de22
  modified: 2026-07-30T20:42:54.803Z
---

VIOLET EDGE's score is migrating from fully-procedural synthesis ([[neon-edge-bevy-port]]) to
**externally-produced tracks the user generates in Antigravity** (Google's agentic IDE; music via
the Gemini/Lyria path). Per the user, **the ENTIRETY of Wingman was made on Antigravity** — which
explains its `track1.mp3` AIGC tag — so they consider commercial-use licensing settled by that
precedent; **don't re-raise it**.

**Workflow that works:** I render the current procedural tracks as reference mp3s
(`cargo test render_full_tracks -- --ignored` → wavs, then ffmpeg to mp3) + a spec sheet
(BPM/key/progression/arrangement/instrumentation + the game's loop requirements); the user feeds
those to Antigravity and hands back enhanced tracks; I wire them in. ffmpeg is available via
`pip install imageio-ffmpeg` (binary path printed by `imageio_ffmpeg.get_ffmpeg_exe()`).

**✅ MIGRATION COMPLETE (2026-07-30)** — all three tracks ship produced, `include_bytes!`-embedded
(bevy `mp3` feature): `assets/main.mp3` (`MAIN_MP3`, ~26s arcade drive), `assets/boss.mp3`
(`BOSS_MP3`, ~24s dark industrial), `assets/gameover.mp3` (`GAMEOVER_MP3`, ~15s ambient synthwave).
Only the boss BUILDUP riser is still synthesized. **The procedural score was DELETED** (user: "the
easiest thing to do is remove the dead code") — ~280 lines incl. the `corrupt()` tier DSP; it's in
git history if ever wanted.

**Wiring checklist for each new produced track:**
1. **Loop check** — `silencedetect=noise=-45dB` must find NO silence at either edge, or the loop
   audibly gaps (these play under `PlaybackMode::Loop`).
2. **Level-match** — measure `volumedetect` mean; `play_track` takes a per-cue `gain`. Produced
   tracks have arrived BOTH quieter and hotter: `GAMEOVER_GAIN` 1.2, `MAIN_GAIN` 0.61,
   `BOSS_GAIN` 0.73 (those two also peaked at full scale — trimming reclaims sfx headroom).
3. ⚠️ **Corruption tiers are DORMANT** ([[neon-edge-design-doc]]): the main ships as ONE track, and
   the tier index CLAMPS to `dir.mains.len()` — which is also what prevents the track restarting at
   act boundaries (pinned by `a_single_main_variant_never_restarts_the_track`). To revive the story
   beat, add per-act produced variants to `mains`; no other code changes. The old DSP is gone, so a
   DSP revival would need mp3→PCM decoding first.
4. Reference renders for future prompting = the shipped mp3s in `assets/` (the old
   `render_full_tracks` tool went with the procedural score).
