#!/usr/bin/env python3
# Usage: scripts/rotary_strategy_picture.py crates/help/pictures/rotary_machining_toolpath
"""End-on picture of 4-Axis Rotary Machining's three strategies: Radial, Off centre, Indexed."""
import math
import sys

from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1]
S = 2  # drawn twice the size, then halved, for smooth edges
F = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
font = ImageFont.truetype(F, 14 * S)
bold = ImageFont.truetype(FB, 16 * S)
W, H = 780, 330
PANEL = W // 3
WOOD, WOOD_EDGE = (222, 184, 135), (160, 120, 70)
STOCK = (240, 226, 205)
BIT, SHANK = (120, 130, 145), (170, 175, 185)
BLUE, AMBER, INK, GREY = (40, 100, 200), (215, 130, 0), (40, 40, 40), (150, 150, 150)

im = Image.new("RGB", (W * S, H * S), (250, 250, 248))
d = ImageDraw.Draw(im)


def p(x, y):
    return (x * S, y * S)


def part(cx, cy):
    """The material (dashed circle) and a bumpy part inside it, seen down the axis."""
    r = 78
    d.ellipse([*p(cx - r, cy - r), *p(cx + r, cy + r)], fill=STOCK, outline=GREY, width=S)
    pts = []
    for k in range(180):
        a = 2 * math.pi * k / 180
        rr = 58 + 9 * math.sin(5 * a) + 4 * math.sin(3 * a + 1)
        pts.append(p(cx + rr * math.sin(a), cy - rr * math.cos(a)))
    d.polygon(pts, fill=WOOD, outline=WOOD_EDGE, width=2 * S)
    d.ellipse([*p(cx - 4, cy - 4), *p(cx + 4, cy + 4)], fill=INK)


def bit(x, tip):
    """A ball nose bit, its tip at (x, tip)."""
    w, flutes, shank = 14, 46, 40
    d.rectangle([*p(x - w / 2, tip - flutes), *p(x + w / 2, tip - w / 2)], fill=BIT)
    d.pieslice([*p(x - w / 2, tip - w), *p(x + w / 2, tip)], 0, 180, fill=BIT)
    d.rectangle([*p(x - w / 2, tip - flutes - shank), *p(x + w / 2, tip - flutes)], fill=SHANK)


def arrow(a, b, colour, width=3):
    d.line([p(*a), p(*b)], fill=colour, width=width * S)
    ang = math.atan2(b[1] - a[1], b[0] - a[0])
    for s in (2.6, -2.6):
        end = (b[0] + 11 * math.cos(ang + s), b[1] + 11 * math.sin(ang + s))
        d.line([p(*b), p(*end)], fill=colour, width=width * S)


def turning(cx, cy, colour):
    """A curved arrow round the material: A turning."""
    r = 92
    box = [*p(cx - r, cy - r), *p(cx + r, cy + r)]
    d.arc(box, 200, 250, fill=colour, width=3 * S)
    a = math.radians(250)
    end = (cx + r * math.cos(a), cy + r * math.sin(a))
    tang = a + math.pi / 2
    for s in (2.6, -2.6):
        tip = (end[0] + 11 * math.cos(tang + s), end[1] + 11 * math.sin(tang + s))
        d.line([p(*end), p(*tip)], fill=colour, width=3 * S)
    d.text(p(cx - r - 26, cy - 20), "A", font=bold, fill=colour)


def title(x, text, sub):
    d.text(p(x + 14, 12), text, font=bold, fill=INK)
    d.multiline_text(p(x + 14, 272), sub, font=font, fill=INK, spacing=3 * S)


cy = 168
# Radial: straight over the axis while A turns
cx = PANEL // 2
part(cx, cy)
bit(cx, cy - 64)
d.line([p(cx, 40), p(cx, cy)], fill=GREY, width=S)
turning(cx, cy, BLUE)
title(0, "Radial", "Straight over the axis\nwhile A turns")

# Off centre: held to one side while A turns
cx = PANEL + PANEL // 2
part(cx, cy)
off = 30
bit(cx + off, cy - 52)
d.line([p(cx, 40), p(cx, cy)], fill=GREY, width=S)
d.line([p(cx + off, 44), p(cx + off, 70)], fill=GREY, width=S)
arrow((cx, 52), (cx + off, 52), AMBER, 2)
d.text(p(cx + off + 6, 42), "Offset", font=font, fill=AMBER)
turning(cx, cy, BLUE)
title(PANEL, "Off centre", "Held to one side: the side\nof the ball cuts, not its tip")

# Indexed: A holds still, the bit moves across
cx = 2 * PANEL + PANEL // 2
part(cx, cy)
for k in range(4):
    a = math.radians(90 * k)
    d.line([p(cx, cy), p(cx + 86 * math.sin(a), cy - 86 * math.cos(a))], fill=GREY, width=S)
bit(cx - 22, cy - 60)
arrow((cx - 50, 70), (cx + 50, 70), AMBER, 2)
arrow((cx + 50, 70), (cx - 50, 70), AMBER, 2)
d.text(p(cx + 56, 60), "across", font=font, fill=AMBER)
title(2 * PANEL, "Indexed", "A holds still at each side\nwhile the bit goes across")

for k in (1, 2):
    d.line([p(k * PANEL, 20), p(k * PANEL, H - 20)], fill=(205, 205, 205), width=2 * S)

im.resize((W, H), Image.LANCZOS).save(f"{OUT}/strategies.png", optimize=True)
