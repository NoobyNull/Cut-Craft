#!/usr/bin/env python3
# Usage: scripts/rotary_room_pictures.py crates/help/pictures
"""End-on pictures for rotary jobs: the three shapes of material and how far each swings
(job_setup/rotary_shapes.png), and the room a board needs to turn on the rotary
(concepts/rotary_room.png)."""
import math
import sys

from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1]
S = 2  # drawn twice the size, then halved, for smooth edges
F = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
font = ImageFont.truetype(F, 14 * S)
bold = ImageFont.truetype(FB, 16 * S)
WOOD, WOOD_EDGE = (222, 184, 135), (160, 120, 70)
BOARD = (196, 160, 110)
BIT, SHANK = (120, 130, 145), (170, 175, 185)
BLUE, RED, INK, GREY = (40, 100, 200), (200, 40, 40), (40, 40, 40), (150, 150, 150)
BG = (250, 250, 248)


def canvas(w, h):
    im = Image.new("RGB", (w * S, h * S), BG)
    return im, ImageDraw.Draw(im)


def p(x, y):
    return (x * S, y * S)


def dashed_circle(d, cx, cy, r, colour):
    for k in range(0, 72, 2):
        a0, a1 = 2 * math.pi * k / 72, 2 * math.pi * (k + 1) / 72
        d.line([p(cx + r * math.cos(a0), cy + r * math.sin(a0)),
                p(cx + r * math.cos(a1), cy + r * math.sin(a1))], fill=colour, width=2 * S)


def arrow(d, a, b, colour, width=2, both=False):
    d.line([p(*a), p(*b)], fill=colour, width=width * S)
    for (x, y), (fx, fy) in [(b, a)] + ([(a, b)] if both else []):
        ang = math.atan2(y - fy, x - fx)
        for s in (2.6, -2.6):
            end = (x + 9 * math.cos(ang + s), y + 9 * math.sin(ang + s))
            d.line([p(x, y), p(*end)], fill=colour, width=width * S)


def centred(d, x, y, text, f=None, colour=INK):
    f = f or font
    w = d.textlength(text, font=f) / S
    d.text(p(x - w / 2, y), text, font=f, fill=colour)


def axis_dot(d, cx, cy):
    d.ellipse([*p(cx - 4, cy - 4), *p(cx + 4, cy + 4)], fill=INK)


def save(im, path):
    im.resize((im.width // S, im.height // S), Image.LANCZOS).save(path, optimize=True)


def shapes():
    """Round, square and board, seen down the axis, each with the circle it swings through."""
    w, h = 780, 300
    im, d = canvas(w, h)
    panel = w // 3
    cy = 140
    for n, (name, note) in enumerate([
        ("Round", "swings out to its radius"),
        ("Square", "corners swing out further"),
        ("Board", "edges swing out furthest"),
    ]):
        cx = panel * n + panel // 2
        if n == 0:
            r = 60
            d.ellipse([*p(cx - r, cy - r), *p(cx + r, cy + r)], fill=WOOD, outline=WOOD_EDGE, width=2 * S)
            swing = r
        elif n == 1:
            a = 46
            d.rectangle([*p(cx - a, cy - a), *p(cx + a, cy + a)], fill=WOOD, outline=WOOD_EDGE, width=2 * S)
            swing = a * math.sqrt(2)
        else:
            hw, ht = 104, 16
            d.rectangle([*p(cx - hw, cy - ht), *p(cx + hw, cy + ht)], fill=BOARD, outline=WOOD_EDGE, width=2 * S)
            swing = math.hypot(hw, ht)
            # thickness and width marks
            arrow(d, (cx - hw - 0, cy + ht + 14), (cx + hw, cy + ht + 14), INK, 1, both=True)
            centred(d, cx, cy + ht + 18, "Board Width (W)")
        dashed_circle(d, cx, cy, min(swing, panel / 2 - 8), RED)
        axis_dot(d, cx, cy)
        # D straight up through the axis
        top = {0: 60, 1: 46, 2: 16}[n]
        arrow(d, (cx + 4, cy + top), (cx + 4, cy - top), BLUE, 2, both=True)
        d.text(p(cx + 10, cy - 12), "D", font=bold, fill=BLUE)
        centred(d, cx, 14, name, bold)
        centred(d, cx, h - 30, note, colour=RED)
    save(im, f"{OUT}/job_setup/rotary_shapes.png")


def room():
    """A board on the rotary: the axis height above the spoilboard, the spoilboard taken out,
    and the room up to the bit lifted all the way."""
    w, h = 640, 420
    im, d = canvas(w, h)
    cx, cy = 300, 230
    bed_y, spoil_y = 370, 340
    # bed and spoilboard (dashed: taken out)
    d.rectangle([*p(40, bed_y), *p(600, bed_y + 18)], fill=(120, 120, 120))
    centred(d, 520, bed_y + 0, "bed", colour=(255, 255, 255))
    for x in range(40, 600, 16):
        d.line([p(x, spoil_y), p(x + 8, spoil_y)], fill=WOOD_EDGE, width=2 * S)
    d.text(p(430, spoil_y - 22), "spoilboard top (taken out)", font=font, fill=WOOD_EDGE)
    # chuck body under the axis
    d.rectangle([*p(cx - 40, cy + 20), *p(cx + 40, bed_y)], fill=(205, 208, 214), outline=GREY, width=S)
    # board, its swing
    hw, ht = 86, 13
    swing = math.hypot(hw, ht)
    d.rectangle([*p(cx - hw, cy - ht), *p(cx + hw, cy + ht)], fill=BOARD, outline=WOOD_EDGE, width=2 * S)
    dashed_circle(d, cx, cy, swing, RED)
    axis_dot(d, cx, cy)
    # bit lifted all the way up
    tip = cy - swing - 30
    bw = 14
    d.rectangle([*p(cx - bw / 2, tip - 50), *p(cx + bw / 2, tip - bw / 2)], fill=BIT)
    d.pieslice([*p(cx - bw / 2, tip - bw), *p(cx + bw / 2, tip)], 0, 180, fill=BIT)
    d.rectangle([*p(cx - bw / 2, tip - 90), *p(cx + bw / 2, tip - 50)], fill=SHANK)
    d.text(p(cx + 14, tip - 70), "Z all the way up", font=font, fill=INK)
    # measures, to the left
    x = 120
    arrow(d, (x, cy), (x, tip), BLUE, 2, both=True)
    d.text(p(x - 104, (cy + tip) / 2 - 8), "Room above", font=bold, fill=BLUE)
    arrow(d, (x, cy), (x, spoil_y), BLUE, 2, both=True)
    d.text(p(x - 104, (cy + spoil_y) / 2 - 8), "Axis height", font=bold, fill=BLUE)
    arrow(d, (x + 30, spoil_y), (x + 30, bed_y), BLUE, 2, both=True)
    d.text(p(x - 104, spoil_y + 6), "Spoilboard", font=bold, fill=BLUE)
    d.line([p(x - 6, cy), p(cx, cy)], fill=GREY, width=S)
    d.line([p(x - 6, tip), p(cx - 10, tip)], fill=GREY, width=S)
    # the swing
    arrow(d, (cx, cy), (cx + swing * math.cos(-0.6), cy + swing * math.sin(-0.6)), RED, 2)
    d.text(p(cx + 60, cy - 70), "swings out", font=font, fill=RED)
    centred(d, w / 2, 12, "A board must swing clear below and above", bold)
    save(im, f"{OUT}/concepts/rotary_room.png")


shapes()
room()
