#!/usr/bin/env python3
# Usage: scripts/rotary_sides_picture.py crates/help/pictures/rotary_machining_toolpath
"""End-on picture of Sides, then details: the top cleared at A=0, the other side at A=180,
then A locked at a tilt for what the sides could not reach, the collet nut kept clear."""
import math
import sys

from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1]
S = 2  # drawn twice the size, then halved, for smooth edges
F = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
font = ImageFont.truetype(F, 14 * S)
bold = ImageFont.truetype(FB, 16 * S)
W, H = 780, 340
PANEL = W // 3
WOOD, WOOD_EDGE = (222, 184, 135), (160, 120, 70)
STOCK, GONE = (240, 226, 205), (250, 250, 248)
BIT, SHANK, NUT = (120, 130, 145), (170, 175, 185), (95, 100, 110)
BLUE, AMBER, RED, INK, GREY = (40, 100, 200), (215, 130, 0), (200, 50, 40), (40, 40, 40), (150, 150, 150)

im = Image.new("RGB", (W * S, H * S), GONE)
d = ImageDraw.Draw(im)


def p(x, y):
    return (x * S, y * S)


def turned(pts, cx, cy, a):
    """Points in the part's frame turned by A (degrees, clockwise) about the axis at cx, cy."""
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a))
    return [p(cx + x * c - y * s, cy + x * s + y * c) for x, y in pts]


# a plane seen nose on: a body with a wing each side, a tail fin up; y down is +
BODY = [(18 * math.cos(t / 20 * math.pi), 18 * math.sin(t / 20 * math.pi)) for t in range(40)]
WINGS = [(-92, -4), (92, -4), (92, 6), (-92, 6)]
FIN = [(-5, -12), (5, -12), (3, -62), (-3, -62)]
BOX = [(-96, -70), (96, -70), (96, 70), (-96, 70)]


def plane(cx, cy, a, left):
    """The material left (a box less what is cleared) and the plane in it, turned by a."""
    if left:
        d.polygon(turned(BOX, cx, cy, 0), fill=GONE, outline=GREY, width=S)
    for shape in left:
        d.polygon(turned(shape, cx, cy, a), fill=STOCK)
    for shape in (WINGS, FIN, BODY):
        d.polygon(turned(shape, cx, cy, a), fill=WOOD, outline=WOOD_EDGE, width=2 * S)
    d.ellipse([*p(cx - 3, cy - 3), *p(cx + 3, cy + 3)], fill=INK)


def holder(x, tip):
    """A ball nose bit, its tip at (x, tip), and the wider collet nut above it."""
    w, flutes, stick = 10, 30, 52
    d.rectangle([*p(x - w / 2, tip - flutes), *p(x + w / 2, tip - w / 2)], fill=BIT)
    d.pieslice([*p(x - w / 2, tip - w), *p(x + w / 2, tip)], 0, 180, fill=BIT)
    d.rectangle([*p(x - w / 2, tip - stick), *p(x + w / 2, tip - flutes)], fill=SHANK)
    d.polygon([p(x - 17, tip - stick), p(x + 17, tip - stick), p(x + 13, tip - stick - 16),
               p(x - 13, tip - stick - 16)], fill=NUT)
    return tip - stick


def title(x, text, sub, colour=INK):
    d.text(p(x + 14, 10), text, font=bold, fill=colour)
    d.multiline_text(p(x + 14, 282), sub, font=font, fill=INK, spacing=3 * S)


cy = 170
# 1. A=0: the top cleared, a level at a time; the underside still whole
cx = PANEL // 2
plane(cx, cy, 0, [[(-96, 6), (96, 6), (96, 70), (-96, 70)]])
for k, y in enumerate((-56, -38, -20)):
    d.line([p(cx - 92, cy + y), p(cx + 92, cy + y)], fill=BLUE, width=S)
holder(cx + 50, cy - 6)
title(0, "1. Top, A=0", "The bulk cleared a level\nat a time, then finished")

# 2. A=180: the other side, the same way
cx = PANEL + PANEL // 2
plane(cx, cy, 180, [])
holder(cx - 50, cy - 7)
title(PANEL, "2. Other side, A=180", "Turned over by A,\ncleared and finished")

# 3. A locked at a tilt: the root of the fin, which neither side could reach square on
cx = 2 * PANEL + PANEL // 2
plane(cx, cy, 35, [])
holder(cx - 12, cy - 21)
d.arc([*p(cx - 110, cy - 110), *p(cx + 110, cy + 110)], 290, 325, fill=BLUE, width=3 * S)
d.text(p(cx - 10, cy - 136), "A locked at 35°", font=font, fill=BLUE)
title(2 * PANEL, "3. Locked tilts", "A holds still at each angle\npicked, for the details left")

for k in (1, 2):
    d.line([p(k * PANEL, 20), p(k * PANEL, H - 20)], fill=(205, 205, 205), width=2 * S)

im.resize((W, H), Image.LANCZOS).save(f"{OUT}/sides-then-details.png", optimize=True)
