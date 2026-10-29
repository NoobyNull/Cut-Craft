#!/usr/bin/env python3
"""Draws the icons Cut Craft's tools were missing, in the style of its own: 128 px. Design
tools: blue strokes, orange for what the tool does, white nodes, pale fills. Toolpath tools:
wood in tan with a brown edge, the tool's path in green.

    scripts/draw_icons.py OUT_DIR   (then copy each into its folder under assets/icons)
"""
import math
import sys
from PIL import Image, ImageDraw

SIZE = 128
SCALE = 4  # drawn large, then scaled down for smooth edges
BLUE = (43, 99, 198, 255)
ORANGE = (242, 140, 40, 255)
WHITE = (255, 255, 255, 255)
PALE = (220, 231, 250, 255)
STROKE = 10  # at 128 px, as the existing icons
NODE = 11
GREEN = (22, 160, 90, 255)
WOOD = (241, 214, 168, 255)
WOOD_SIDE = (232, 199, 147, 255)
EDGE = (184, 135, 76, 255)


def canvas():
    im = Image.new("RGBA", (SIZE * SCALE, SIZE * SCALE), (0, 0, 0, 0))
    return im, ImageDraw.Draw(im)


def s(p):
    return (p[0] * SCALE, p[1] * SCALE)


def line(d, pts, colour, width=STROKE):
    """A round-ended stroke: a round brush stamped along the path (Pillow's own wide
    polylines leave notches at every joint)."""
    r = width * SCALE / 2
    for (x0, y0), (x1, y1) in zip(pts, pts[1:] or pts):
        steps = max(1, int(math.dist((x0, y0), (x1, y1)) * SCALE))
        for i in range(steps + 1):
            x, y = s((x0 + (x1 - x0) * i / steps, y0 + (y1 - y0) * i / steps))
            d.ellipse((x - r, y - r, x + r, y + r), fill=colour)


def node(d, p):
    x, y = s(p)
    h = NODE * SCALE / 2
    d.rectangle((x - h, y - h, x + h, y + h), fill=WHITE, outline=BLUE, width=3 * SCALE)


def arc(cx, cy, r, a0, a1, n=48):
    return [
        (cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
         cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / n)))
        for i in range(n + 1)
    ]


def finish(im, path):
    im.resize((SIZE, SIZE), Image.LANCZOS).save(path)


def join_vectors(path):
    """Two open curves, their end nodes joined by orange segments into one closed shape."""
    im, d = canvas()
    cy, r, apart = 64, 42, 10  # each half's centre sits `apart` from the middle
    gap = 28  # degrees left open at the top and bottom of each half
    left = arc(64 - apart, cy, r, 90 + gap, 270 - gap)
    right = arc(64 + apart, cy, r, -90 + gap, 90 - gap)
    for half in (left, right):
        line(d, half, BLUE)
    # the joins: right's bottom end to left's bottom end, left's top end to right's top end
    line(d, [right[-1], left[0]], ORANGE)
    line(d, [left[-1], right[0]], ORANGE)
    for p in (left[0], left[-1], right[0], right[-1]):
        node(d, p)
    finish(im, path)


def plate_production(path):
    """A sheet of nameplates, each with its own line of text."""
    im, d = canvas()
    cols, rows = 2, 3
    margin, gutter = 12, 8
    w = (SIZE - 2 * margin - gutter * (cols - 1)) / cols
    h = (SIZE - 2 * margin - gutter * (rows - 1)) / rows
    for row in range(rows):
        for col in range(cols):
            x0 = margin + col * (w + gutter)
            y0 = margin + row * (h + gutter)
            d.rounded_rectangle(
                (*s((x0, y0)), *s((x0 + w, y0 + h))),
                radius=7 * SCALE, fill=PALE, outline=BLUE, width=5 * SCALE)
            # the plate's text: one line, a different length on each
            length = w * (0.45 + 0.12 * ((row * cols + col) % 3))
            ty = y0 + h / 2
            line(d, [(x0 + (w - length) / 2, ty), (x0 + (w + length) / 2, ty)], ORANGE, 7)
    finish(im, path)


def vector_unwrapper(path):
    """A closed cross-section, unrolled into an open profile with legs at its ends."""
    im, d = canvas()
    # the cross-section: a lobed ring
    cx, cy = 30, 44
    ring = [(cx + (22 + 4 * math.cos(math.radians(5 * a))) * math.cos(math.radians(a)),
             cy + (22 + 4 * math.cos(math.radians(5 * a))) * math.sin(math.radians(a)))
            for a in range(0, 361, 4)]
    line(d, ring, BLUE, 8)
    # the arrow across
    line(d, [(62, 44), (92, 44)], ORANGE, 8)
    d.polygon([s((104, 44)), s((90, 34)), s((90, 54))], fill=ORANGE)
    # the unrolled profile: the same lobes along a line, legs down to the baseline
    x0, x1, base, top = 12, 116, 112, 86
    wave = [(x0 + (x1 - x0) * i / 60,
             top - 6 * math.cos(math.radians(5 * 360 * i / 60)))
            for i in range(61)]
    line(d, [(x0, base)] + wave + [(x1, base)], BLUE, 8)
    for p in ((x0, base), (x1, base)):
        node(d, p)
    finish(im, path)


def arrow(d, a, b, colour, width=6, head=11):
    """A double-headed dimension arrow from `a` to `b`."""
    line(d, [a, b], colour, width)
    for tip, tail in ((a, b), (b, a)):
        ang = math.atan2(tip[1] - tail[1], tip[0] - tail[0])
        side = [(tip[0] - head * math.cos(ang + k), tip[1] - head * math.sin(ang + k))
                for k in (0.5, -0.5)]
        d.polygon([s(tip), s(side[0]), s(side[1])], fill=colour)


def material_setup(path):
    """The material from the side, the model in it, its thickness and Z zero on top."""
    im, d = canvas()
    x0, x1, top, bottom = 10, 100, 40, 104
    d.rectangle((*s((x0, top)), *s((x1, bottom))), fill=WOOD, outline=EDGE, width=5 * SCALE)
    # the model's top, a dome sitting on the bottom of the material
    d.pieslice((*s((24, 64)), *s((86, 144))), 180, 360, fill=BLUE)
    line(d, [(x0 - 4, top), (x1 + 4, top)], ORANGE, 6)  # Z zero, the surface
    arrow(d, (116, top + 2), (116, bottom - 2), BLUE)
    finish(im, path)


def materials(path):
    """A material library: three woods, each with its grain."""
    im, d = canvas()
    woods = [(241, 214, 168, 255), (203, 150, 92, 255), (150, 92, 58, 255)]
    for i, wood in enumerate(woods):
        y0 = 10 + i * 38
        d.rounded_rectangle((*s((14, y0)), *s((114, y0 + 32))), radius=6 * SCALE,
                            fill=wood, outline=EDGE, width=4 * SCALE)
        grain = tuple(int(c * 0.82) for c in wood[:3]) + (255,)
        for k in range(3):
            y = y0 + 9 + k * 7
            wave = [(20 + j * 4, y + 2 * math.sin(j * 0.6 + i + k)) for j in range(23)]
            line(d, wave, grain, 3)
    finish(im, path)


def inlay_toolpath(path):
    """A V-carved pocket in the block and the plug that fills it, point down above it."""
    im, d = canvas()
    # the block, its V pocket cut in from the top
    d.polygon([s(p) for p in [(8, 76), (44, 76), (64, 112), (84, 76), (120, 76), (120, 120),
                               (8, 120)]], fill=WOOD, outline=EDGE, width=4 * SCALE)
    line(d, [(44, 76), (64, 112), (84, 76)], GREEN, 7)
    # the plug, a darker wood, its V point over the pocket
    plug = (150, 92, 58, 255)
    d.polygon([s(p) for p in [(36, 14), (92, 14), (92, 30), (64, 62), (36, 30)]],
              fill=plug, outline=EDGE, width=4 * SCALE)
    line(d, [(36, 30), (64, 62), (92, 30)], GREEN, 7)
    finish(im, path)


def drag_knife_toolpath(path):
    """A cut on the sheet, the swivel blade trailing its pivot round the corner."""
    im, d = canvas()
    d.rounded_rectangle((*s((6, 22)), *s((122, 122))), radius=8 * SCALE, fill=WOOD,
                        outline=EDGE, width=4 * SCALE)
    cut = [(20, 104), (20, 60)] + arc(46, 60, 26, 180, 270, 16) + [(76, 34)]
    line(d, cut, GREEN, 7)
    # the knife: a shank down to the pivot, the blade swept back behind it
    pivot = (92, 30)
    line(d, [(112, 4), pivot], BLUE, 12)
    d.polygon([s(pivot), s((70, 18)), s((72, 40))], fill=(205, 212, 224, 255),
              outline=BLUE, width=4 * SCALE)
    finish(im, path)


if __name__ == "__main__":
    out = sys.argv[1]
    join_vectors(f"{out}/join_vectors.png")
    plate_production(f"{out}/plate_production.png")
    vector_unwrapper(f"{out}/vector_unwrapper.png")
    material_setup(f"{out}/material_setup.png")
    materials(f"{out}/materials.png")
    inlay_toolpath(f"{out}/inlay_toolpath.png")
    drag_knife_toolpath(f"{out}/drag_knife_toolpath.png")
