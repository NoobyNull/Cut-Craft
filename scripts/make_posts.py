#!/usr/bin/env python3
"""Write Cut Craft's GRBL and Mach3 post variants from their two hand-written templates.

    scripts/make_posts.py

The templates are crates/post/posts/grbl_arcs_mm.pp, mach3_arcs_mm.pp and fluidnc_atc_mm.pp:
edit those, then run this. For each it writes, in millimetres and inches:
  Arcs              the template (G2 / G3, centres from the arc's start)
  Straight          lines only, for controllers without arcs
  Wrap Y to A       rotary, a cylinder along X; feeds in units a minute
  Wrap Y to A G93   the same, feeds in inverse time
  Wrap X to A       rotary, a cylinder along Y
  Wrap X to A G93
and crates/post/src/built_in.rs, the list of Cut Craft's own posts.
"""

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent
POSTS = ROOT / "crates/post/posts"
# the FluidNC posts are hand-written and listed first: the defaults for flat and rotary jobs.
# Machines know posts by name, so theirs stay as they were; the FluidNC variants come after.
FIXED = ["fluidnc_atc_mm.pp", "fluidnc_atc_wrap_a_mm.pp"]
# stem: (controller, template file). Every family has every variant, so a machine's 4th axis
# finds its rotary posts by name (FluidNC's Wrap Y to A G93 (mm) is the default rotary post's
# twin under that name).
TEMPLATES = {
    "grbl": ("GRBL", "grbl_arcs_mm.pp"),
    "mach3": ("Mach3", "mach3_arcs_mm.pp"),
    "fluidnc_atc": ("FluidNC ATC", "fluidnc_atc_mm.pp"),
}
ARC_VARS = ("VAR ARC_CENTRE_I_INC_POSITION", "VAR ARC_CENTRE_J_INC_POSITION")


def named(text, name):
    return re.sub(r'POST_NAME = "([^"]*)"', lambda m: f'POST_NAME = "{name}"', text, count=1)


def described(text, lines):
    """`text` with its opening comment saying `lines`, and where it was made."""
    body = text.split("\n\n", 1)[1]
    head = "".join(f"+ {line}\n" for line in lines)
    return head + "+ Made by scripts/make_posts.py: edit its template and run it again.\n\n" + body


def inches(text):
    text = text.replace('UNITS = "MM"', 'UNITS = "INCHES"').replace("G21", "G20")
    # positions to a ten-thousandth of an inch, feeds to a hundredth (inverse time is a
    # rate a minute, not a length, so it stays as it is)
    text = re.sub(r"(\[(?!FI\|)[A-Z]+\|[AC]\|[A-Z]+\|)1\.3\]", r"\g<1>1.4]", text)
    text = text.replace("[F|C|F|1.1]", "[F|C|F|1.2]")
    return text.replace("] mm across", "] in across")


def straight(text):
    """No arc blocks, so every cut is written as lines."""
    text = re.sub(r"\n\+ arcs:.*?(?=\nbegin TOOLCHANGE)", "", text, flags=re.S)
    text = "".join(l for l in text.splitlines(True) if not l.startswith(ARC_VARS))
    # arc centre mode means nothing without arcs
    return text.replace('"G91.1"\n', "")


def add_to_block(text, block, line, first=False):
    start = text.index(f"begin {block}\n") + len(f"begin {block}\n")
    if first:
        return text[:start] + line + "\n" + text[start:]
    end = text.find("\n\n", start)
    end = len(text.rstrip("\n")) if end < 0 else end
    return text[:end] + "\n" + line + text[end:]


def rotary(text, wrapped, inverse_time):
    """`wrapped` (X or Y) wrapped onto A, as Cut Craft's FluidNC rotary post does."""
    text = straight(text)
    settings = f'ROTARY_WRAP_{wrapped} = "A"\n'
    if inverse_time:
        settings += "INVERSE_TIME_MODE = YES\n"
    text = text.replace('SUBSTITUTE = "({)}"\n', 'SUBSTITUTE = "({)}"\n' + settings)
    text = text.replace(
        "VAR TOOL_NUMBER = [T|A|T|1.0]\n",
        "VAR TOOL_NUMBER = [T|A|T|1.0]\nVAR WRAP_DIAMETER = [WRAP_DIA|A||1.3]\n",
    )
    text = re.sub(
        r'"\(Material: X .*\)"',
        '"(Rotary: [WRAP_DIA] mm across, [XLENGTH] x [YLENGTH] unrolled, Z zero on the [Z_ORIGIN])"',
        text,
    )
    if inverse_time:
        text = text.replace("VAR FEED_RATE = [F|C|F|1.1]", "VAR INVERSE_TIME = [FI|A|F|1.3]")
        # inverse time needs its F on every feed move, not only the first
        text = text.replace(" [F]\"", ' [FI]"')
        text = text.replace('begin FEED_MOVE\n"G1 [X] [Y] [Z]"', 'begin FEED_MOVE\n"G1 [X] [Y] [Z] [FI]"')
        # inverse time while cutting; units a minute for the tool change and the end
        text = add_to_block(text, "HEADER", '"G93"')
        text = text.replace('"G0 [SAFEZ]"\n"M5"', '"G0 [SAFEZ]"\n"G94"\n"M5"')
        text = add_to_block(text, "TOOLCHANGE", '"G93"')
        text = add_to_block(text, "FOOTER", '"G94"', first=True)
    return text


def variants(stem, controller, template):
    along = {"Y": "X", "X": "Y"}
    for units, unit_name in (("mm", "millimetres"), ("inch", "inches")):
        base = template
        intro = [line.format(units=unit_name) for line in INTRO[stem]]
        made = [
            ("arcs", "Arcs", base, "Arcs: G2 / G3 with I J from the arc's start."),
            ("straight", "Straight", straight(base), "Straight: lines only, for controllers without arcs."),
        ]
        for wrapped in ("Y", "X"):
            for g93 in (False, True):
                feeds = "feeds in inverse time (G93)" if g93 else "feeds in units a minute"
                made.append((
                    f"wrap_{wrapped.lower()}_a" + ("_g93" if g93 else ""),
                    f"Wrap {wrapped} to A" + (" G93" if g93 else ""),
                    rotary(base, wrapped, g93),
                    f"Rotary: {wrapped} wrapped onto A, for a cylinder along "
                    f"{along[wrapped]}; {feeds}. Needs an A axis.",
                ))
        for key, label, text, what in made:
            # inches last, so what the variants add is converted too
            text = text if units == "mm" else inches(text)
            ext = re.search(r'FILE_EXTENSION = "([^"]*)"', text).group(1)
            name = f"Cut Craft {controller} {label} ({units}) (*.{ext})"
            yield f"{stem}_{key}_{units}.pp", described(named(text, name), intro + [what])


INTRO = {
    "grbl": [
        "Cut Craft's post for hobby routers on grbl (grbl 1.1, grblHAL), {units}, tools changed",
        "by hand: grbl refuses M6, so a tool change stops the spindle and pauses (M0) for the",
        "new tool, to be resumed from the sender once it is in and zeroed.",
    ],
    "mach3": [
        "Cut Craft's post for Mach3 (and Mach4), {units}, with Mach3's tool change (T M6).",
    ],
    "fluidnc_atc": [
        "Cut Craft's post for FluidNC with an automatic tool changer (M6), {units}.",
    ],
}


def main():
    files = list(FIXED)
    for stem, (controller, source) in TEMPLATES.items():
        template = (POSTS / source).read_text()
        for file, text in variants(stem, controller, template):
            # the template is its own Arcs (mm)
            if file == f"{stem}_arcs_mm.pp":
                if source not in files:
                    files.append(source)
                continue
            (POSTS / file).write_text(text)
            files.append(file)
    rows = "".join(f'    ("{f}", include_str!("../posts/{f}")),\n' for f in files)
    (ROOT / "crates/post/src/built_in.rs").write_text(
        "// Made by scripts/make_posts.py: run it again after adding a post.\n\n"
        "/// Cut Craft's own posts, as (file name, text): provided, not edited (customise a copy).\n"
        f"pub const BUILT_IN: [(&str, &str); {len(files)}] = [\n{rows}];\n"
    )
    print(f"{len(files)} posts")


main()
