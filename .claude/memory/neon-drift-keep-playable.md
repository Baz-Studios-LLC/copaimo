---
name: neon-drift-keep-playable
description: "after ANY neon-edge code change: rebuild bundle + redeploy Artifact + commit locally, but DO NOT git push unless the user explicitly says so (as of 2026-07-14)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

For the [[neon-drift-project]] game, whenever I change the game code I must immediately rebuild the single-file bundle and redeploy the Artifact — the user tests it inline in the conversation, not via a localhost tab.

Steps after every code change:
1. `python build/bundle.py` (from `neon-asteroids/`) → regenerates `dist/neon-edge.html`
2. Verify the bundle has no leaked `import`/`export` and initializes without throwing
3. Redeploy with the Artifact tool using `dist/neon-edge.html` (same file path → same URL: https://claude.ai/code/artifact/cd76d93f-967c-4caa-96be-5f470be06946)

**Why:** user explicitly asked "always make sure the game is playable here when updated" (2026-07-09). The modular `js/` source is canonical but is NOT what the user plays — the Artifact bundle is.

**How to apply:** treat "rebuild + redeploy Artifact" as the mandatory last step of any neon-drift edit, same as you'd run tests. Don't end a turn that changed game code without it.

**⛔ DO NOT PUSH TO GIT unless the user explicitly says so (added 2026-07-14).** The user asked to hold all pushes ("Don't push anymore to Git unless I tell you"). This OVERRIDES the earlier auto-push rule. Current ritual after a code change: `python build/bundle.py` → verify → redeploy Artifact → `git commit` LOCALLY (keep reviewable history, don't lose work) → **STOP; do NOT `git push`.** When the user says to push, push all accumulated commits to `main` (repo still `Baz-Studios-LLC/Neon-Drift` pending the Neon-Edge rename; push auth via Git Credential Manager) — a push to `main` triggers the release CI (`.github/workflows/release.yml`, Windows + macOS Tauri prerelease). bundle.py emits BOTH `dist/neon-edge.html` (Artifact fragment) and `dist/index.html` (standalone = Tauri frontend); commit both. **Why:** the user wants to gate what reaches GitHub/CI while iterating. See [[neon-drift-project]] desktop section.

**Bundler gotcha (learned the hard way):** the bundle flattens ALL modules (incl. `main.js`) into one IIFE scope, so two modules with the same top-level name = fatal `SyntaxError: X already declared` → the whole Artifact silently shows only a frozen menu. This shipped broken for turns because my verify harness concatenated modules *without* main.js. Two safeguards now: (1) `build/bundle.py` scans for duplicate top-level identifiers and fails the build; (2) ALWAYS smoke-test the real bundle after building — fetch `dist/neon-edge.html`, extract the `<script>`, run it via `new Function(src)()` in a preview_eval try/catch, and confirm it doesn't throw. Verifying a hand-concatenated graph that omits main.js is NOT sufficient.
