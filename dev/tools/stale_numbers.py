"""Finds comments that quote a number the code no longer holds.

    python dev/tools/stale_numbers.py            # the suspicious ones
    python dev/tools/stale_numbers.py --all      # every documented constant and its numbers
    python dev/tools/stale_numbers.py --refs     # comments naming another constant with a value

# Why this exists as a tool rather than a careful read

The same fault has been found by accident eight times in two days: the clip-length block, two
places in motion.rs, models.rs, gait_watch.py, both speed constants, and a note in docs/ about a
widget mesh that is not shipped. Every one was a NUMBER RESTATED IN PROSE beside the code that
owns it, and every fix was the same shape - stop restating it.

Finding them by reading does not scale and, more to the point, has not worked: the reading has
happened repeatedly and missed them. So the check is mechanical and re-runnable, and lives in the
repository rather than in a session.

# What it flags, and why that rule

A constant is suspicious when its own comment quotes a number CLOSE TO but not EQUAL TO its
value - within a factor of `NEARBY` either way. That is the signature of a superseded value: a
tuning number moves from 2.74 to 3.70 and the prose still says 2.74. Numbers far from the value
are almost always something else entirely - a frame rate, a percentage, a year - and flagging
those buries the real ones.

It is a HEURISTIC and says so. Comparable figures from other games, historical values written up
as history, and derived quantities all trip it. The output is a worklist to read, not a verdict:
the fix for a false positive is to leave it alone.

# What it cannot see

Prose that is wrong without quoting a number - "the jog is at its ceiling", "45% of his height",
"nothing plays at 1.00x" - none of which this finds. Those need reading. What this buys is that
the mechanical half stops eating the time that should go on the other half.
"""
import argparse
import os
import re
import sys

HERE = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# How close a quoted number has to be to the value before it looks like a stale copy of it
# rather than an unrelated figure. 2.5x either way: wide enough to catch 2.74 against 5.90,
# narrow enough that "24 fps" beside a 0.98 does not show up.
NEARBY = 2.5

# Numbers that are almost never a stale value: counts, shares and round figures that appear in
# prose for their own sake.
NOT_A_VALUE = {0.0, 1.0, 2.0, 3.0, 100.0}

A_NUMBER = re.compile(r"(?<![\w.])(\d+(?:\.\d+)?)(?![\w.])")


def rust_constants(text):
    """Doc-commented constants in a Rust file, as (name, value, comment, line)."""
    lines = text.splitlines()
    found = []
    for number, line in enumerate(lines):
        hit = re.match(r"\s*(?:pub )?const (\w+): f(?:32|64) = ([0-9.\-]+);", line)
        if not hit:
            continue
        comment, back = [], number - 1
        while back >= 0 and re.match(r"\s*(?://[/!]|//)", lines[back]):
            comment.append(lines[back])
            back -= 1
        found.append((hit.group(1), float(hit.group(2)), "\n".join(reversed(comment)),
                      number + 1))
    return found


def python_constants(text):
    """Comment-blocked module constants in a Python file, same shape."""
    lines = text.splitlines()
    found = []
    for number, line in enumerate(lines):
        hit = re.match(r"([A-Z][A-Z0-9_]*) = ([0-9.\-]+)\s*(?:#.*)?$", line)
        if not hit:
            continue
        comment, back = [line] if "#" in line else [], number - 1
        while back >= 0 and lines[back].lstrip().startswith("#"):
            comment.append(lines[back])
            back -= 1
        found.append((hit.group(1), float(hit.group(2)), "\n".join(reversed(comment)),
                      number + 1))
    return found


def every_source():
    for root, _, files in os.walk(os.path.join(HERE, "src")):
        if "target" in root:
            continue
        for name in sorted(files):
            if name.endswith(".rs"):
                yield os.path.join(root, name), rust_constants
    for name in sorted(os.listdir(os.path.join(HERE, "dev", "art"))):
        if name.endswith(".py"):
            yield os.path.join(HERE, "dev", "art", name), python_constants


def quoted(comment):
    return [float(one) for one in A_NUMBER.findall(comment)]


def main():
    ask = argparse.ArgumentParser()
    ask.add_argument("--all", action="store_true", help="every constant, not just suspicious")
    ask.add_argument("--refs", action="store_true",
                     help="comments naming another constant alongside a number")
    said = ask.parse_args()

    # Every constant anywhere, for the cross-reference pass.
    everywhere = {}
    documented = []
    for path, read in every_source():
        text = open(path, encoding="utf-8", errors="replace").read()
        for name, value, comment, line in read(text):
            everywhere.setdefault(name, []).append((os.path.relpath(path, HERE), value))
            documented.append((os.path.relpath(path, HERE), name, value, comment, line))

    if said.refs:
        print("comments that name another constant and a number near it:\n")
        shown = 0
        for path, name, _value, comment, line in documented:
            for other, wheres in everywhere.items():
                if other == name or other not in comment:
                    continue
                theirs = {value for _, value in wheres}
                near = [q for q in quoted(comment)
                        if q not in NOT_A_VALUE
                        and not any(abs(q - t) < 1e-6 for t in theirs)
                        and any(t and 1 / NEARBY <= q / t <= NEARBY for t in theirs)]
                if near:
                    shown += 1
                    print(f"{path}:{line}  {name}")
                    print(f"    names {other} (= {sorted(theirs)}) alongside {near}")
        print(f"\n{shown} to look at")
        return

    print("constants whose own comment quotes a number close to but not equal to their value")
    print("(a heuristic - comparable figures and written-up history trip it too)\n")
    shown = 0
    for path, name, value, comment, line in documented:
        numbers = [q for q in quoted(comment) if q not in NOT_A_VALUE]
        stale = [
            q for q in numbers
            if abs(q - value) > 1e-6 and value and 1 / NEARBY <= q / value <= NEARBY
        ]
        if said.all:
            print(f"{path}:{line}  {name} = {value}")
            print(f"    quoted: {sorted(set(numbers))}")
            continue
        if not stale:
            continue
        shown += 1
        print(f"{path}:{line}  {name} = {value}")
        print(f"    comment says: {sorted(set(stale))}")
    if not said.all:
        print(f"\n{shown} to look at")


if __name__ == "__main__":
    sys.exit(main())
