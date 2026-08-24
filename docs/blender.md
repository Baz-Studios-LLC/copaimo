# Blender and glTF

Most of what follows is either a documented export rule or a trap this project has already paid
for. The measured ones have the cost attached, because that is what makes them memorable.

## glTF export rules

**STANDARD.** These cause **silent** failures — the file exports, nothing errors, and the model
is wrong in the engine:

| Rule | What happens otherwise |
|---|---|
| The mesh must **not** be a child of the armature; use an armature **modifier** | bone weights and indices are **silently dropped** |
| Every vertex needs at least one bone | Blender treats it as un-skinned; glTF exports it wrong |
| At most 4 influences per vertex | the 5th and beyond vanish at export |
| Turn **off** *Preserve Volume* on the armature modifier | that is dual-quaternion skinning; engines do not do it, so Blender shows a deformation the game will not produce |
| Set rotation interpolation to **linear** before export | curve interpolation does not survive faithfully |

**MEASURED (Copaimo).** The linear one is already handled — `make_it_linear` converts every
rotation curve and reports the count. The four-influence and weights-sum guards are in
`check_the_skin` and refuse the build.

**STANDARD.** glTF stores **joint positions, not bone lengths**. A leaf bone's length is invented
by the importer on the way back in.

**MEASURED (Copaimo).** This is why nothing pivoted at the toe: `ToeBase` came back 6.1 cm long
inside a 29.4 cm shoe. `reach_the_ends` re-derives each leaf bone's length from the geometry it
drives, every time the file is read — and re-deriving is correct here precisely because the
information is not in the file.

## Split vertices and custom normals

**STANDARD.** glTF stores one set of attributes per vertex, so every UV seam and every hard-edge
boundary **duplicates the vertices along it**. A mesh that is one connected surface in the DCC
arrives as hundreds of disconnected shells.

**MEASURED (Copaimo).** 7582 vertices in **1442 shells**, 7475 non-manifold edges, largest shell
37 vertices. Welding them by position gives 19 clean shells, moves the surface 0.0005 mm, and
keeps the face count.

**And welding is still the wrong thing to do.** Those split vertices *are* the hard-edge
encoding. The custom split normals riding on them end up describing a topology that no longer
exists, and the character is lit as if it were a different shape — shoes read as shards, seams and
melted forms. **No numeric guard caught it**: edge lengths and dimensions still matched the source
to 0.01 cm.

The technique that replaces it: **weld virtually.** Round coordinates into buckets, union across
edges, and work with the resulting groups — the connectivity is recovered without the mesh being
altered at all. Used in this project for:

- identifying garment pieces (`unfuse.cloth_pieces`)
- finding real boundary loops — 6975 of 10131 edges *look* like boundary on split topology; welded
  it is 140 of 6710
- separating fingers — 309 vertices are 95 real ones
- **tear detection** — if two copies of a point end up on different bones, they come apart when
  those bones move; weld in rest, pose the extreme, measure the spread within each group

Always check `mesh.data.has_custom_normals` after any mesh edit. Losing them is the fault that
lights the character as a different shape, and positions will not show it.

## `bpy` traps

**Operators act on the selection and the context, not on arguments.** `bpy.ops.mesh.*` in edit
mode acts on **every selected object**.

**MEASURED (Copaimo). Cost: the entire body**, 7264 vertices down to 318, because more than one
object was in edit mode.

The fixes, in order of preference:

1. Don't use the operator. `bmesh.ops.*` and direct data access take explicit arguments.
2. `bpy.types.Context.temp_override(...)` — the supported way since Blender 3.2, replacing the old
   context-override dictionary. Copy the current context as the basis and override what you mean
   to change.
3. If you must use an operator on a selection: deselect all **objects**, select and activate the
   one, then enter edit mode, and assert the selection took.

**Object-mode face selection is derived from vertex selection.** Setting `poly.select = True` in
object mode does not survive the switch into edit mode.

**MEASURED (Copaimo). Cost: subdividing the whole body** — 7534 to 34037 vertices — while the log
said "293 finger faces cut". Nothing in the per-part numbers was wrong; only the total gave it
away. Fix: select through `bmesh.from_edit_mesh`, `ensure_lookup_table()`, set `face.select`
explicitly, `select_flush`, `update_edit_mesh`, then **count what is selected and refuse if it is
not what you asked for**.

**Deleting or subdividing renumbers vertices.** Any index held across such an operation refers to
something else afterwards. If a later step needs identity, carry it by **position**, not index.

**MEASURED (Copaimo).** Digit names are carried through a subdivide by fingertip position, because
tips do not move (14.0 cm before, 14.0 cm after) while every index changes.

**Three different bone collections, and they are not interchangeable:**

| Collection | What it is | Note |
|---|---|---|
| `rig.data.bones` | rest / bind data | **cannot** be posed; read this for rest geometry |
| `rig.pose.bones` | the current pose | reading a "rest" value here gets whatever the last clip left |
| `rig.data.edit_bones` | editable topology | only exists in edit mode |

**MEASURED (Copaimo).** Reading a rest ankle angle from `pose.bones` picked up the previous clip's
leftover pose and was **46° out** on the left. Read rest geometry from `data.bones`.

**An assigned action re-drives its bones on every depsgraph update.** Set a pose, update, and the
action puts it back.

**MEASURED (Copaimo).** A sweep meant to show six hand rotations produced six identical images.

**`bpy.app.driver_namespace` is cleared on file load; `SpaceView3D` draw handlers are not.** A
draw handler added on load stacks another copy every reload — captions smeared. Use `sys.modules`
to hold state across a reload, and remove handlers before adding.

**`calc_normals_split` was removed in Blender 4.1+.** Split normals are available directly.

## Building headless

**STANDARD.** Run Blender with `-b` / `--background` and a `--python` script. This is how studios
batch: write the pipeline step once, run it 200 times. A five-person indie team is reported taking
daily asset integration from 2 hours to 15 minutes by putting Blender behind CI, with artists
committing .blend files and developers receiving validated exports.

**MEASURED (Copaimo), and worth knowing.** The glTF **importer fails during GUI startup** — it
dies in `armature_display` on a context that startup does not provide, so a window built by a
`--python` script at launch comes up **empty with the reason buried in the console**. The same
import is reliable in `--background`.

So the pattern is: **build the scene headless, save a `.blend`, verify it is not empty, then open
it.** `gait_watch.sh` does this and checks rigs, skinned meshes, action, frame range and viewport
count before handing the file over.

Two more things that pattern buys:

- A **reload watcher** registered inside the `.blend` (needs `--enable-autoexec`, a per-session
  flag) reverts the file when its timestamp changes, so a rebuild reaches an open window without
  anyone closing anything. Store the viewing angle in a sidecar and restore it after, because the
  view lives in the `.blend` and a plain revert snaps it back.
- **Stamp what the scene was built from.** The `.blend`'s own timestamp only changes when
  something rewrites the *scene*; the failure worth catching is the model moving on while the
  scene does not. Measured once at two hours and four rounds of changes apart — and a stale
  viewer makes every report coming back unreliable, which is worse than a bug.

## Auto-riggers

**STANDARD.**

| | Rigify | Auto-Rig Pro |
|---|---|---|
| Cost | free, ships with Blender | paid |
| Game export | works, but exporting clean skeletons to Unreal/Unity is where it hurts | dedicated Unity / Unreal / Godot presets, baked engine-ready skeletons |
| Extras | — | weight binding, retargeting, shape keys, bone picker, batch export |
| Verdict in the field | a legitimately good skill; take it if time is free | what most professionals reach for, especially at volume |

> **→ For Copaimo.** Neither is in use — the rig is repaired and extended by script, which is the
> right call for a single character that must be reproducible, and would be the wrong call for
> twenty. If monsters end up needing individual rigs, revisit; a per-creature auto-rig plus
> retargeting is the standard answer to that, not twenty bespoke scripts.

## Mesh repair on generated meshes

**STANDARD.** A manifold mesh has every edge shared by exactly two faces. Generated output
typically arrives at 40k–80k triangles with **no edge flow, overlapping faces, and open holes
where the generator lost confidence** — which matches this project's asset exactly.

Repair splits into surface-based (detect non-manifold elements, zipper, fill boundary loops) and
volumetric (reconstruct and remesh). "Make manifold" tools fill small holes, remove interior
faces, and fix edges shared by more than two faces.

**MEASURED (Copaimo), and the important caveat.** Blender's `fill_holes` cannot close a loop on
split topology, because on split topology the loop is not a loop. Anything that repairs by
boundary detection needs to weld virtually first or it is working on a mesh that does not exist.

## Sources

- [glTF 2.0 import/export — Blender Manual](https://docs.blender.org/manual/en/2.90/addons/import_export/scene_gltf2.html)
- [Bone weights silently dropped when mesh is a child of armature — glTF-Blender-IO #1970](https://github.com/KhronosGroup/glTF-Blender-IO/issues/1970)
- [Verts not assigned to a bone export wrong — glTF-Blender-IO #1151](https://github.com/KhronosGroup/glTF-Blender-IO/issues/1151)
- [Stricter skinning requirements — glTF #1665](https://github.com/KhronosGroup/glTF/issues/1665)
- [Authoring and exporting animations with Blender — ezEngine](https://ezengine.net/pages/docs/animation/skeletal-animation/blender-export.html)
- [Using the Blender glTF Exporter — Hubs Foundation](https://docs.hubsfoundation.org/creators-using-the-blender-gltf-exporter)
- [Game Engine Export — Auto-Rig Pro docs](https://www.lucky3d.fr/auto-rig-pro/doc/ge_export_doc.html)
- [Operators (bpy.ops) — Blender Python API](https://docs.blender.org/api/current/bpy.ops.html)
- [Rigify vs Auto-Rig Pro — CGDive](https://cgdive.com/rigify-vs-auto-rig-pro-auto-rigging-comparison/)
- [Best Blender rigging addons 2026 — StraySpark](https://www.strayspark.studio/blog/best-blender-rigging-addons-2026)
- [GitHub Actions & Blender: automate game assets — DrCodes](https://drcodes.com/posts/github-actions-blender-automate-game-assets-in-30-minutes)
- [Blender scripting for animation pipelines — CGWire](https://blog.cg-wire.com/blender-scripting-animation/)
- [Asset Pipeline update — Blender Studio](https://studio.blender.org/blog/asset-pipeline-update-2022/)
- [Non-manifold detection and repair — Tripo](https://www.tripo3d.ai/blog/explore/smart-mesh-non-manifold-detection-and-repair)
- [Mesh healing library — MeshLib](https://meshlib.io/feature/mesh-healing/)
- [AI 3D model Blender cleanup guide](https://blog.neural4d.com/user-guide/ai-3d-model-blender-cleanup-complete-mesh-fix-guide-2026/)
