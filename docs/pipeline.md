# Pipeline

## What to commit and what to derive

This is the single most valuable idea in this file, because getting it wrong cost this project a
sleeve cuff, part of a trouser leg, and part of a shoulder — each removed by its own build script,
repeatedly, over weeks.

**The rule: derive what is deterministic, commit what is a judgement call.**

A build step that computes a *transform* — re-derive it every time. It will give the same answer,
and re-deriving means the source stays pristine and the step stays honest.

A build step that makes a *decision* — commit its output. Every run is a fresh chance to decide
wrongly, and a classifier asked to re-decide "is this dark shape near the arm a sleeve cuff or a
hanging strap?" on every build has to be right **forever**.

**MEASURED (Copaimo).** `prepare_rig.py` did both kinds of work and ran on every build:

| Kind | Examples | Right call |
|---|---|---|
| Rig repair — deterministic transforms | mirror the two sides (5.45 cm out), remove a 17.5° crouch, re-derive leaf bone lengths glTF does not store, put the soles on the floor | **derive.** These are rest-pose constants, and correcting a constant *per pose* is what twisted the feet three times |
| Mesh work — judgement | cap holes, tell a strap from a cuff, add finger geometry | **commit.** Do it once, verify once, keep it |

The tell that this was wrong: every time the classifier cut the wrong thing, the response was to
**tune the threshold**. That is treating a design fault as a numbers fault. Three separate
thresholds got tuned before the shape of the problem became visible.

Now: `dev/art/ranger_apose.glb` is the committed source asset, the build only reads it, and
`dev/art/bootstrap_rig.sh` is the only thing that re-derives it from the original delivery — with
a confirmation flag, because running it discards hand work.

**STANDARD, and this is what studios do.** The DCC source file is committed and versioned; the
engine-ready artefact is built from it deterministically. Blender Studio's asset pipeline appends
models as collections into files where rigging adjustments are reproducibly automated — the
*adjustments* are scripted, the *asset* is a file.

## Validation

**STANDARD.** Studios build validation tools that enforce, automatically:

- naming conventions
- skeleton standards
- rig integrity
- performance budgets

Plus automated retargeting, batch export, and validation scripts as standard pipeline tooling.
This is a named role — the **technical animator** / animation technical artist, who sits between
art and engineering and whose job is largely tools, automated pipelines and asset quality.

**STANDARD.** Quality moves up a ladder: individual artist self-QA, then supervisor, then team
lead, then division head, against standardised documentation covering technical checks, animation
guidelines and engine validation rules.

**MEASURED (Copaimo), and the principle that matters most.** *A guard must compare against the
specification, not against its own input.* The A-pose bake step was checked by comparing its
result to the shape fed into it. When the input turned out to be wrong, the check **passed
happily** and wrote a mesh in one pose bound to a skeleton in another.

Every guard here is now absolute: soles at zero, arms at 45°, sides mirrored to within half a
millimetre, weights summing to one, four influences, rest pose unmoved to a micron.

Two corollaries learned the hard way:

- **A number that does not move when you turn a knob means the knob is not connected.** A head-bob
  term measured *exactly* no effect — head travel 6.29 cm before and after — because the bone was
  posed and never keyframed. The maths was fine; nothing reached the file.
- **A number stated in two places will disagree with itself.** Stance shares were typed in both
  the authoring script and the verifier; the day one changed, the verifier measured a window the
  clip did not have and reported a duty factor of 0.667 for a clip authored at 0.583. The shell
  script now greps the constant out of the Python.

## Golden images and visual regression

**STANDARD.** Render a known-good baseline, compare new output pixel-wise, fail past a threshold,
and emit a diff image. Standard in graphics runtimes and CAD libraries, and integrated into CI so
every change is compared automatically.

> **→ For Copaimo. OPEN, and probably the highest-value thing missing.** There is a lot of
> rendering-to-look-at in `dev/art/` — contact sheets, gait watch, hand isolation — and none of it
> is compared against a baseline. Several regressions in this project were *visible* and went
> unnoticed for hours because nobody re-rendered.
>
> A cheap version: render a fixed set of poses to PNG on every asset build, diff against committed
> baselines, and fail the build past a threshold. The renders already exist; only the comparison
> and the baselines are missing. This would have caught the shoulder, the trouser leg and the
> cuffs immediately, all three of which were numerically fine.

## Scope, for a small team

**STANDARD.** Pipeline scope scales with project size — a solo developer may compress a character
into hours where a AAA studio spends 18 months on one hero. What matters for a small team is
**strong silhouettes, focused colour, readable personality and a realistic production scope**,
over visual complexity.

Stylised art is the production-friendly choice: fewer technical constraints, more design-driven,
and better suited to frequent updates and large content volumes. More triangles and more separate
materials mean more modelling, UV and texturing time *and* higher draw calls.

> **→ For Copaimo.** Consistent with the project's existing stance that the game is stylised and
> visuals are judged by how they read, not against real-world measurements. Worth keeping in view
> as monster count grows: the number of creatures is the thing that will multiply pipeline cost,
> so per-creature effort is the number to watch, not per-creature polish.

## Character budgets

**STANDARD (2026).**

| Role | Triangles |
|---|---|
| Hero, non-Nanite, 3–5 LODs | 30k–80k |
| Hero, closest LOD, action game | 15k–20k |
| Hero, casual | 5k–8k |
| Enemy / NPC | 1k–8k, low end when seen in packs |

Skinned meshes cost more than static ones because they run the vertex pipeline twice. Per-vertex
cost drops with influence count — eight to four, or two for crowd LODs. Strip unused bones to keep
the bone matrix array small, and share rigs across crowd characters to allow GPU instancing.

**MEASURED (Copaimo).** Body is 9189 vertices / 7113 triangles after the finger subdivision, plus
a 371-vertex backpack. Comfortably inside hero budget with no LODs yet. The asset file is 18.19 MB
including five clips.

**A correction, since this file claimed otherwise:** the export does NOT carry a leftover
`Icosphere` widget. Read straight out of the GLB's JSON it holds exactly two meshes, `Backpack`
and the body. The Icosphere appears whenever the file is IMPORTED, because Blender's glTF
importer builds one and assigns it as a custom shape to every bone — glTF has no bone lengths, so
there would be nothing to draw otherwise. Seeing it in a Blender session and calling it shipped
geometry is the same mistake as measuring split topology and calling it holes: the artefact
belongs to the tool doing the looking. `prepare_rig.drop_the_widgets` removes them.

## Version control for binary assets

**STANDARD.** Studios use Perforce for binary-heavy projects because Git handles large binaries
poorly; Git LFS is the common middle ground on smaller teams.

**MEASURED (Copaimo).** `.glb` files are committed to Git directly — 18 MB for the character,
rewritten on every asset build. That is a growing repository, and it is also **exactly what made
the source-asset switch safe**: `git log` on `ranger_apose.glb` shows what a re-derive would
discard, which is why `bootstrap_rig.sh` prints it before asking.

> **→ For Copaimo. OPEN.** Worth watching repo size rather than acting now. If it becomes a
> problem, LFS for `assets/models/*.glb` (the *derived* artefact) while keeping the source asset
> in Git proper preserves the useful history and moves the churn.

## Sources

- [Game Animation Pipeline: Tools and Workflow — MoCap Online](https://mocaponline.com/blogs/mocap-news/game-animation-pipeline-guide)
- [Technical Art Pipeline in Game Development — Mimic Gaming](https://www.mimicgaming.com/post/technical-art-pipeline-game-development)
- [Key responsibilities of a Technical Animator — Torchora](https://www.torchora.com/job-specification/technical-animator)
- [Asset Pipeline 2022 update — Blender Studio](https://studio.blender.org/blog/asset-pipeline-update-2022/)
- [Graphics module tests and golden images — rive-runtime](https://deepwiki.com/rive-app/rive-runtime/11.2-graphics-module-tests-and-golden-images)
- [Golden testing a CAD library — Joe Warren](https://doscienceto.it/blog/posts/2026-04-27-golden-testing-cad.html)
- [Visual regression testing — Applitools](https://applitools.com/blog/visual-regression-testing/)
- [Polygon budgets by platform 2026 — low-poly.com](https://low-poly.com/blog/polygon-budgets-by-platform-2026)
- [Low-poly character optimization — Simplygon](https://www.simplygon.com/posts/0179d2c5-a440-49d7-850a-0a9a94152d1b)
- [Character skinning for games — Game-Ace](https://game-ace.com/blog/game-character-skinning/)
- [Stylized 3D characters art direction playbook — Nasty Rodent](https://nastyrodent.com/stylized-3d-characters-art-direction-principles/)
- [Game art pipeline explained — Pixune](https://pixune.com/blog/game-art-pipeline/)
- [Character design for indie games — Tripo](https://www.tripo3d.ai/blog/character-design-for-indie-games)
