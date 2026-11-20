#!/usr/bin/env python3
# Usage: scripts/rotary_lathe_picture.py crates/help/pictures/rotary_lathe_toolpath
"""Side-on picture of Rotary Lathe's three operations: Face, Turn and Part off."""
import math
import sys

from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1]
S = 2  # drawn twice the size, then halved, for smooth edges
F = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
font = ImageFont.truetype(F, 14 * S)
bold = ImageFont.truetype(FB, 16 * S)
W, H = 780, 300
PANEL = W // 3
WOOD, WOOD_EDGE, GONE_EDGE = (222, 184, 135), (160, 120, 70), (190, 190, 190)
BIT, SHANK, NUT = (120, 130, 145), (170, 175, 185), (95, 100, 110)
BLUE, AMBER, INK, GREY = (40, 100, 200), (215, 130, 0), (40, 40, 40), (150, 150, 150)

im = Image.new("RGB", (W * S, H * S), (250, 250, 248))
d = ImageDraw.Draw(im)
R = 40  # the bar's radius
L = 200  # its length shown


def p(x, y):
    return (x * S, y * S)


def bar(x0, cy, radius):
    """The bar seen from the side, its radius at each step along given by radius(t), t 0..1:
    what is left in wood, what was there before as a grey outline."""
    d.rectangle([*p(x0, cy - R), *p(x0 + L, cy + R)], outline=GONE_EDGE, width=S)
    top, bottom = [], []
    for k in range(L + 1):
        r = radius(k / L)
        top.append(p(x0 + k, cy - r))
        bottom.append(p(x0 + k, cy + r))
    d.polygon(top + bottom[::-1], fill=WOOD, outline=WOOD_EDGE)
    d.line([p(x0 - 6, cy), p(x0 + L + 6, cy)], fill=GREY, width=S)
    # the chuck at the left
    d.rectangle([*p(x0 - 14, cy - R - 10), *p(x0, cy + R + 10)], fill=NUT)


def bit(x, tip):
    """A square end mill straight over the axis, its tip at (x, tip)."""
    w = 12
    d.rectangle([*p(x - w / 2, tip - 34), *p(x + w / 2, tip)], fill=BIT)
    d.rectangle([*p(x - w / 2, tip - 60), *p(x + w / 2, tip - 34)], fill=SHANK)
    d.polygon([p(x - 16, tip - 60), p(x + 16, tip - 60), p(x + 12, tip - 74),
               p(x - 12, tip - 74)], fill=NUT)


def spin(x, cy):
    """A turning on the axis, drawn as an ellipse arrow round the bar's end."""
    box = [*p(x - 8, cy - R - 8), *p(x + 8, cy + R + 8)]
    d.arc(box, 270, 80, fill=BLUE, width=3 * S)
    d.text(p(x + 10, cy + R - 4), "A", font=bold, fill=BLUE)


def title(x, text, sub):
    d.text(p(x + 14, 10), text, font=bold, fill=INK)
    d.multiline_text(p(x + 14, 240), sub, font=font, fill=INK, spacing=3 * S)


cy = 140
STUB = 6

# Face: the far end taken down to the holding stub
x0 = 26
bar(x0, cy, lambda t: R if t < 0.72 else STUB)
bit(x0 + L * 0.86, cy - STUB)
spin(x0 + L + 4, cy)
title(0, "Face", "An end taken down\nto the holding stub")

# Turn: the bar turned round to a bead's outline
x0 = PANEL + 26
bar(x0, cy, lambda t: STUB if t > 0.9 else 12 + 24 * math.sin(math.pi * min(t / 0.9, 1)) ** 0.7)
bit(x0 + L * 0.5, cy - 36)
spin(x0 + L + 4, cy)
title(PANEL, "Turn", "Down to the model's\noutline, round")

# Part off: one groove down to the stub
x0 = 2 * PANEL + 26
bar(x0, cy, lambda t: STUB if 0.62 < t < 0.68 else R)
bit(x0 + L * 0.65, cy - STUB)
spin(x0 + L + 4, cy)
title(2 * PANEL, "Part off", "One groove, so the part\ncan be snapped free")

for k in (1, 2):
    d.line([p(k * PANEL, 20), p(k * PANEL, H - 20)], fill=(205, 205, 205), width=2 * S)

im.resize((W, H), Image.LANCZOS).save(f"{OUT}/operations.png", optimize=True)
