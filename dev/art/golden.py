"""Renders a fixed sheet of the character and compares it against the kept copies.

    dev/art/golden.sh                 render and compare
    dev/art/golden.sh --bless         accept what is there now as the new truth

Stage 10 of `docs/character-pipeline.md`, pulled forward on purpose. It was ordered last because
it protects finished work - but the stages before it were being finished with nothing protecting
them, and "re-render it and squint" is not a gate. Every claim that something still looks right
was an assertion until this existed.

# What is in the sheet, and why each one

Not a gallery. Each shot is the cheapest view of a thing that has actually gone wrong on this
character, so a regression in it shows up as a number rather than as a report weeks later:

  rest front / side / quarter     proportions, and the whole silhouette
  hands, curled and splayed       the fingers, and the web between them
  feet                            where the shoes were fought over for a day
  armpits, arms out               the membrane, and any future gusset
  the worst-tearing frame of
  every clip                      deformation where it is measurably hardest

# Clay, and a threshold that was measured rather than assumed

Rendered without texture, because form is what regresses invisibly.

The threshold started at 1.5 on the assumption that EEVEE sampling jitters between runs. It does
not: re-rendering an unchanged asset gives 0.00 on all sixteen shots. And the assumption was not
merely unnecessary, it was dangerous - checked by DOUBLING the finger web sink, a real mesh edit
moved the sheet 0.55 and the gate called it "same". A gate that admits a doubled edit is not a
gate. At 0.05 the noise floor is still zero and the same edit fails four shots loudly.

Verified in both directions before being relied on: clean passes at 0.00, a deliberate change
fails. A gate nobody has watched fail is a gate nobody knows works.
"""
import os
import shutil
import subprocess
import sys

ART = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(ART))
KEPT = os.path.join(ART, "golden")

# (name, extra arguments to render_clay). One render per row.
SHEET = (
    ("rest_front", ["--only", "front"]),
    ("rest_side", ["--only", "side"]),
    ("rest_quarter", ["--only", "quarter"]),
    ("rest_torso", ["--only", "torso"]),
    ("hand_L_rest", ["--only", "hand_L"]),
    ("hand_R_rest", ["--only", "hand_R"]),
    ("hand_L_curled", ["--curl", "45", "--only", "hand_L"]),
    ("hand_R_curled", ["--curl", "45", "--only", "hand_R"]),
    ("feet", ["--only", "feet"]),
    ("armpit_L_out", ["--lift", "70", "--only", "armpit_L_front"]),
    ("armpit_R_out", ["--lift", "70", "--only", "armpit_R"]),
    ("silhouette", ["--silhouette", "--only", "front"]),
    # The worst-tearing frame of each clip, from `audit_character`'s own measurements.
    ("idle_worst", ["--clip", "idle", "--frame", "428", "--only", "quarter"]),
    ("walk_worst", ["--clip", "walk", "--frame", "49", "--only", "quarter"]),
    ("run_worst", ["--clip", "run", "--frame", "10", "--only", "quarter"]),
    ("run_worst_front", ["--clip", "run", "--frame", "10", "--only", "front"]),
)

# Mean absolute difference per channel, 0-255, above which a shot counts as changed.
#
# MEASURED, not guessed. It was 1.5 on the assumption that EEVEE sampling would jitter - and
# re-rendering an unchanged asset gives 0.00 on all sixteen shots, so there is no jitter to
# allow for. Worse, 1.5 let a real change through: DOUBLING the finger web sink moved the sheet
# only 0.55 and the gate called it "same". A threshold that admits a doubled mesh edit is not a
# gate. At 0.05 the noise floor is still zero and that same edit fails loudly.
MOVED_BY = 0.05


def blender():
    for name in ("BLENDER", "blender"):
        found = os.environ.get(name)
        if found and os.path.isfile(found):
            return found
    for guess in (
        r"C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
        r"C:\Program Files\Blender Foundation\Blender 4.2\blender.exe",
        "/usr/bin/blender",
    ):
        if os.path.isfile(guess):
            return guess
    raise SystemExit("REFUSED: Blender not found")


def main():
    import numpy
    from PIL import Image

    bless = "--bless" in sys.argv
    model = os.path.join(ROOT, "assets", "models", "person_ranger.glb")
    if not os.path.isfile(model):
        raise SystemExit(f"REFUSED: {model} is missing - build the character first")
    fresh = os.path.join(ART, "golden_now")
    os.makedirs(fresh, exist_ok=True)
    os.makedirs(KEPT, exist_ok=True)

    exe = blender()
    print(f"rendering {len(SHEET)} shots")
    for name, extra in SHEET:
        into = os.path.join(fresh, name)
        # Emptied first: a leftover from a previous run is a second image in the directory, and
        # the "exactly one" check below would then refuse a render that was perfectly fine.
        shutil.rmtree(into, ignore_errors=True)
        os.makedirs(into, exist_ok=True)
        run = subprocess.run(
            [exe, "--background", "--python-exit-code", "1",
             "--python", os.path.join(ART, "render_clay.py"), "--",
             "--model", model, "--out", into] + extra,
            capture_output=True, text=True)
        if run.returncode != 0:
            raise SystemExit(f"REFUSED: rendering {name} failed\n{run.stdout[-800:]}")

    changed, missing, drift = [], [], []
    for name, _ in SHEET:
        into = os.path.join(fresh, name)
        made = [f for f in sorted(os.listdir(into)) if f.endswith(".png")]
        if len(made) != 1:
            raise SystemExit(
                f"REFUSED: {name} rendered {len(made)} images ({', '.join(made)}) and this "
                f"expects exactly one. Taking the first was how `rest_front` in the kept sheet "
                f"became an armpit close-up.")
        now = os.path.join(into, made[0])
        keep = os.path.join(KEPT, f"{name}.png")
        if bless:
            Image.open(now).save(keep)
            continue
        if not os.path.isfile(keep):
            missing.append(name)
            continue
        a = numpy.asarray(Image.open(keep).convert("RGB")).astype(numpy.float32)
        b = numpy.asarray(Image.open(now).convert("RGB")).astype(numpy.float32)
        if a.shape != b.shape:
            changed.append((name, float("inf")))
            continue
        moved = float(numpy.abs(a - b).mean())
        drift.append((name, moved))
        if moved > MOVED_BY:
            changed.append((name, moved))

    if bless:
        print(f"blessed {len(SHEET)} shots into {KEPT}")
        return

    for name, moved in sorted(drift, key=lambda row: -row[1]):
        mark = "CHANGED" if moved > MOVED_BY else "same"
        print(f"  {name:<20s} {moved:6.2f}  {mark}")
    if missing:
        print(f"\n{len(missing)} shot(s) have no kept copy: {', '.join(missing)}")
        print("run with --bless to accept the current render as the truth")
    if changed:
        print(f"\n*** {len(changed)} shot(s) changed past {MOVED_BY}:")
        for name, moved in changed:
            print(f"      {name} by {moved:.2f}")
        print("Look at dev/art/golden_now/<name>/ against dev/art/golden/<name>.png.")
        print("If the change is wanted, --bless it. If not, it is a regression.")
        raise SystemExit(1)
    if not missing:
        print(f"\nall {len(drift)} shots match, worst drift "
              f"{max(m for _, m in drift):.2f} against a {MOVED_BY} threshold")


if __name__ == "__main__":
    main()
