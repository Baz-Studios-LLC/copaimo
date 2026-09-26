---
name: baz-studios-launcher
description: "INTERNAL-only Tauri launcher (Baz-Studios-LLC/baz-studios-launcher) that lists all company games in one window; pulls each game live from its repo's published GitHub Release"
metadata:
  node_type: memory
  type: project
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
  modified: 2026-08-14T19:53:32.583Z
---

**Baz Studios launcher** — repo `Baz-Studios-LLC/baz-studios-launcher`, a **Tauri
v2** app (Rust `src-tauri/` + HTML `src/index.html`). **INTERNAL ONLY, never
public-facing** (user, 2026-07-16): a single window to install/launch/test all
current company projects. Launcher's own version is **0.1.11** (released 2026-07-28).

- **Game catalog is baked into `src-tauri/src/main.rs`** (`GAMES` array): each is
  `{slug, name, tagline, repo, accent, delivery}`. **Delivery is an enum (updated
  since 2026-07-16):** `Web { asset, port }` OR `Native { mac, windows }`.
  There is also a `kind` field: `Kind::Game` or `Kind::Tool` (Opificium is the Tool).
  Current games: **wriftheart** (Native, mac `WriftHeart-macos-aarch64.app.tar.gz`,
  win `WriftHeart-windows-x86_64.zip`), wingman (Web), **violet-edge** (Native — see below),
  crashout (Web), please-dont-shake, fly-on-the-wall, divus-factus, **opificium** (Tool),
  and **ranger** (Native, `Baz-Studios-LLC/copaimo`, accent `#5faa5c`, added 2026-08-14). (The old `neondrift`/"Neon Edge" Web row is GONE — replaced by `violet-edge`;
  its `neondrift.png` asset was deleted in v0.1.11.)
- **Version pulled LIVE per game.** Scans `api.github.com/repos/<repo>/releases`
  (per_page=20), skips drafts + **prereleases**, takes the first PUBLISHED release
  carrying the game's asset; version = that release's tag (minus `v`).
  - **Match is by SUFFIX (`ends_with`), not exact name** (`bundle_url`, since launcher v0.1.7) — so a
    game can version/rename its asset without breaking the launcher. Catalog stores stable tails.
  - Web: asset `<slug>-game.zip` (zip of the web `dist/`), served on a fixed localhost port.
  - Native: per-OS asset — **Windows** a `.zip` with the `.exe` (unzipped, `.exe` spawned); **macOS**
    a `.app.tar.gz` (untarred via system `tar` to preserve the bundle, then `open`ed). Native catalog
    entries store the studio-convention **suffixes** `-windows-x86_64.zip` / `-macos-aarch64.app.tar.gz`.
    No linux native (mac/windows only).
- **VIOLET EDGE IS WIRED IN ✅** the `violet-edge` **Native** catalog row (slug `violet-edge`,
  `repo: "Baz-Studios-LLC/Violet-Edge"`, suffixes `-macos-aarch64.app.tar.gz` / `-windows-x86_64.zip`,
  accent `#8a5cff`) pulls the game's latest release live (now **v0.4.2**). **LOGO + TEXT shipped in
  v0.1.11 (2026-07-28):** dropped `src/assets/violet-edge.png` + `-icon.png` (1254² square, from the
  game's `assets/logo.png`) so the arrow mark shows in the hero + sidebar tile (was the "VE" monogram);
  fixed the detail card's studio label **"New City Entertainment" → "Baz Studios"** (it was a hardcoded
  placeholder in `index.html`, NOT from the backend); and synced the browser-preview `mockInvoke` catalog
  to the real backend. Verified in the browser preview. Still not click-tested through the packaged GUI
  (Install→Play) — a manual/user check.
- ⚠️ **THE CATALOG IS BAKED INTO THE BINARY.** Pushing a `GAMES` edit to the repo does NOT change an
  installed launcher — the app self-updates ONLY to a higher launcher version. So **adding/changing a
  game requires a NEW LAUNCHER RELEASE**, whereas a game's own VERSION updates are fetched live (no
  launcher release needed). This is why the first catalog push "didn't update" for the user.
- **Launcher release recipe:** bump version in **all three** — `package.json`, `src-tauri/Cargo.toml`,
  `src-tauri/tauri.conf.json` (+ refresh `src-tauri/Cargo.lock`) — commit+push `main`, then
  `gh workflow run release.yml -R Baz-Studios-LLC/baz-studios-launcher -f tag=vX.Y.Z` (it's
  **workflow_dispatch**, NOT tag-push). `tauri-action` builds+signs both OSes (signing keys are CI
  secrets `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD`); a `manifest` job copies the release's `latest.json`
  → `updater.json` on `main`. Installed launchers poll `raw.githubusercontent.com/.../main/updater.json`
  (pubkey baked in tauri.conf.json) and self-update to the newer signed build on next start.
  ⚠️ The `manifest` job commits to `main`, so if you pushed the version bump you'll be BEHIND after —
  `git pull --rebase` before the next push. ⚠️ `raw.githubusercontent.com` CDN-caches ~5 min, so right
  after a release the endpoint may still show the old version; verify the REAL file via
  `gh api repos/.../contents/updater.json --jq .content | base64 -d`, not the raw URL.
- **Access:** `gh` IS authed (see [[neon-edge-github-repo]]). Launcher cloned at
  **`C:\Users\jsull\Desktop\baz-studios-launcher`** (now v0.1.11). ⚠️ `src-tauri/Cargo.lock` is
  **gitignored** here (cargo regenerates it at build) — so bump only `package.json` + `src-tauri/Cargo.toml`
  + `tauri.conf.json`. Two PRE-EXISTING warnings in
  `src-tauri/src/main.rs` (unused `Modifiers` import; unused `mut` ~L588) are unrelated to our edit.
