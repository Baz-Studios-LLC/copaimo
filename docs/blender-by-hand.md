# Fixing the character by hand, in Blender

For the small things. A toe pointing wrong on one frame is thirty seconds of work in the viewport
and was repeatedly half an hour of measurement for me, so this is worth having.

Written against `assets/models/person_ranger.glb` and the rig it carries — 71 bones, the names
below are the real ones.

## The one rule, first

**The `.blend` the viewer opens is disposable.** It lives at

    %TEMP%\copaimo_view\character.blend

and `dev/art/build_character.sh` writes a fresh one from `dev/art/source/character/*.glb` every time it
runs. Edit it freely to *find out* what a fix should be — nothing you do there can break the build —
but the edit itself dies on the next build. [Making a fix stick](#making-a-fix-stick) is the last
section and it is the important one.

## Opening it

```bash
bash dev/art/see_the_character.sh --clip jog --in-place
```

`--clip` picks which of `idle`, `walk`, `jog` it opens on. `--in-place` runs him on the spot; leave
it off and the scene carries him forward at his own stride, which is how you see whether a planted
foot **slides**. On the spot is how you see how a foot **lands**, because the contact stays under
the camera instead of walking out of it. Two different questions — use the matching one.

It closes any Blender already open first, on purpose. Two windows is how a fault gets reported
against a stale one, which happened here.

## Looking

- **Space** plays. **Left/Right arrow** steps one frame — that is the one you want most.
- The frame counter is bottom left of the timeline. The jog is frames 1–25, and **frame 25 is a
  copy of frame 1** (it is the loop seam), so the real cycle is 1–24.
- **Numpad 1 / 3 / 7** are front, side, top. **Numpad 5** toggles orthographic, which is what you
  want for judging angles — perspective lies about them.
- The Action Editor is open along the bottom; its dropdown lists every clip, so you can switch
  between idle, walk and jog without reloading.
- Twist bones are hidden (there are 18 of them and they are noise). **Alt-H** in the viewport brings
  them back if you need them.

## Selecting a bone and seeing what it is doing

Click the character, then **Ctrl-Tab** (or the mode dropdown, top left) for **Pose Mode**. Bones
turn selectable.

The ones that matter for the feet, in the order they hang off each other:

| bone | what it is |
| --- | --- |
| `L_Thigh` / `R_Thigh` | hip to knee |
| `L_Calf` / `R_Calf` | knee to ankle — the shin |
| `L_Foot` / `R_Foot` | ankle to the ball of the foot |
| `L_ToeBase` / `R_ToeBase` | the ball to the toe tip |

Press **N** for the side panel, **Item** tab. With a bone selected it shows its rotation. Those
numbers are *local* — relative to the bone's rest pose — so `0,0,0` means "exactly as the bind
built it", which is usually what you want a foot to be near.

One thing that will confuse you, because it confused me for three rounds today: **the toe bone is
drawn wrong.** glTF stores joints as points and has no concept of a bone's length, so Blender's
importer invents one on the way in — it comes out about twice as long as the real bone and pointing
out through the sole. The *joint* is in the right place and the mesh deforms correctly. Judge the
shoe, not the bone.

## Fixing a pose

1. Select the bone.
2. **R** then move the mouse rotates it. **R** twice rotates in the view plane, which is usually
   what you want.
3. Better: **R X**, **R Y**, **R Z** constrain it to one axis, and typing a number after that is
   exact. `R X 15 Enter` is fifteen degrees about X.
4. **Alt-R** resets the bone to its rest rotation. Good for "what did this look like before
   someone got at it".

For a foot specifically, the three axes are worth naming because mixing them up cost me most of
today:

- **Pitch** — toe up and down. This is the one an animator actually poses.
- **Roll** — twisting about the foot's own length, so the sole faces off to one side. Should be
  near zero always.
- **Yaw** — which way the foot points on the floor. Should be along the way he travels, always.

If a foot looks wrong and you cannot say which of those three it is, look from **behind** (Numpad
1, then orbit round) — yaw and roll are obvious from there and nearly invisible from the side.

## Keying it

A pose you do not key is gone the moment you step frames.

- **I** with the mouse over the viewport, then **Rotation**, keys the selected bones on the current
  frame.
- Or turn on **auto-key** (the record button left of the play controls) and every change keys
  itself. Convenient and occasionally destructive.
- **Alt-I** removes the key on this frame.

If you change frame 1 or frame 25, **change both** — they are the same pose and the clip pops at
the seam otherwise.

## Checking your work rather than trusting it

The soles need to be on the floor and flat while planted. By eye, side view, orthographic, zoomed
in on the shoe. By number:

```bash
bash dev/art/audit_character.sh
```

That prints, among a lot else, `THE FACING` (all three witnesses should read 0.00 deg off the
axis), `THE FOOTFALLS` (the planted foot should travel within a percent or so of what the clip
claims), and how close the legs come to each other.

## Making a fix stick

Three places a fix can live. Pick by what kind of fix it is.

**1. A tuning number.** Most of what I changed today is a constant at the top of
`dev/art/author_gait.py` with a comment saying what measurement earned it. If the toe bends too far
at push-off, `THE_TOE_LIFTS_AT_TOE_OFF` is a number you can change and rebuild. Same for
`THE_HEEL_LIFTS_AT_TOE_OFF`, `THE_FOOT_POINTS_AT_MOST`, `THE_LEGS_STAND_APART_BY`. This is the
cheapest kind of fix and it survives everything.

**2. A change to the delivered animation.** The clips come from `dev/art/source/character/idle.glb`,
`walk.glb` and `run.glb`. Edit one of those and rebuild, and the change flows through. This is the
right home for "the arm swing is wrong on frame 12" — a performance note.

**3. A change to the rig or the mesh.** `dev/art/build_character.py` does this, and it does it in
code rather than by hand so that it happens again next build. If a bone is in the wrong place, that
is a function in there, not a manual edit.

Then rebuild and look:

```bash
bash dev/art/build_character.sh
bash dev/art/see_the_character.sh --clip jog --in-place
```

## What is easy to break

- **Editing the viewer `.blend` and expecting it to last.** It will not. See the top.
- **Keying frame 1 without frame 25.** Pops every loop.
- **Judging an angle in perspective.** Numpad 5 first.
- **Trusting the drawn toe bone.** See above; it is an import artefact.
- **Fixing pitch when the fault is yaw.** Look from behind before deciding.
