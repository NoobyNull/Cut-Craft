#!/usr/bin/env python3
"""Draws the help's concept animations: ideas a screenshot cannot show, such as what stepover
is. Each is a looping GIF in the icons' style (blue ink, orange for what matters, green
toolpaths, wood tones), written to crates/help/pictures/concepts/.

    python3 -I scripts/make_help_animations.py [OUT_DIR]

Needs Pillow. Words stay out of the pictures except the measurement letters the help page's
words explain (the same letters the tool forms use), so search and screen readers get the
words from the page.
"""

import math
import os
import sys

from PIL import Image, ImageDraw, ImageFont

INK = (77, 126, 196)
ACC = (239, 143, 42)
TP = (47, 154, 103)
WOOD = (233, 207, 158)
CUT = (196, 160, 110)  # the cleared floor: deeper, so darker
WOODD = (176, 138, 85)
BIT = (154, 163, 178)
WHITE = (255, 255, 255)

SS = 3  # drawn this many times bigger, then made smaller: smooth edges
W, H = 448, 252


def font(px):
    for path in (
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "C:/Windows/Fonts/arialbd.ttf",
    ):
        if os.path.exists(path):
            return ImageFont.truetype(path, px * SS)
    return ImageFont.load_default()


def s(v):
    return v * SS


def arrow_head(d, tip, angle, size, colour):
    a = math.radians(angle)
    left = (tip[0] - size * math.cos(a - 0.45), tip[1] - size * math.sin(a - 0.45))
    right = (tip[0] - size * math.cos(a + 0.45), tip[1] - size * math.sin(a + 0.45))
    d.polygon([tip, left, right], fill=colour)


def dimension(d, x, y0, y1, letter, colour=ACC):
    """A vertical measurement from y0 to y1 at x, its letter beside it."""
    w = s(2.5)
    d.line([(s(x), s(y0)), (s(x), s(y1))], fill=colour, width=int(w))
    arrow_head(d, (s(x), s(y0)), -90, s(8), colour)
    arrow_head(d, (s(x), s(y1)), 90, s(8), colour)
    for y in (y0, y1):
        d.line([(s(x - 9), s(y)), (s(x + 9), s(y))], fill=colour, width=int(w))
    d.text((s(x + 12), s((y0 + y1) / 2)), letter, fill=colour, font=font(18), anchor="lm")


def stepover_frames():
    """A bit clearing a pocket in rows: each row moves over by the stepover (S), less than
    the bit's width (Dia), so the rows overlap and leave no ridges."""
    left, top, right, bottom = 40, 36, 330, 216  # the pocket
    r = 22  # the bit's radius
    step = r * 2 * 0.45  # stepover: 45 % of the bit
    rows = []
    y = top + r
    while y <= bottom - r + 0.01:
        rows.append(y)
        y += step
    if rows[-1] < bottom - r:
        rows.append(bottom - r)
    # the path: along each row, then down to the next, back and forth
    path = []
    for i, y in enumerate(rows):
        xs = (left + r, right - r) if i % 2 == 0 else (right - r, left + r)
        if path:
            path.append((xs[0], y))
        path.append((xs[0], y))
        path.append((xs[1], y))
    # points along the path, evenly spaced
    points = [path[0]]
    for a, b in zip(path, path[1:]):
        n = max(1, int(math.dist(a, b) / 9))
        points += [(a[0] + (b[0] - a[0]) * t / n, a[1] + (b[1] - a[1]) * t / n) for t in range(1, n + 1)]

    frames = []
    for k in range(0, len(points), 2):
        frames.append(stepover_frame(points[: k + 1], (left, top, right, bottom), r, rows, step, False))
    last = stepover_frame(points, (left, top, right, bottom), r, rows, step, True)
    return frames, last


def stepover_frame(done, pocket, r, rows, step, measured):
    left, top, right, bottom = pocket
    img = Image.new("RGB", (s(W), s(H)), WHITE)
    d = ImageDraw.Draw(img)
    # the material and the pocket's outline
    d.rounded_rectangle([s(8), s(8), s(360), s(H - 8)], radius=s(6), fill=WOOD, outline=WOODD, width=s(2))
    # what the bit has cleared: its circle swept along the path so far
    for x, y in done:
        d.ellipse([s(x - r), s(y - r), s(x + r), s(y + r)], fill=CUT)
    d.rectangle([s(left), s(top), s(right), s(bottom)], outline=INK, width=s(2))
    # the path so far, in toolpath green
    if len(done) > 1:
        d.line([(s(x), s(y)) for x, y in done], fill=TP, width=s(2), joint="curve")
    # the bit, from above
    x, y = done[-1]
    d.ellipse([s(x - r), s(y - r), s(x + r), s(y + r)], fill=BIT, outline=INK, width=s(2))
    d.ellipse([s(x - 3), s(y - 3), s(x + 3), s(y + 3)], fill=INK)
    if measured:
        # S between the first two rows, Dia across the bit, off to the right
        dimension(d, 384, rows[0], rows[1], "S")
        d.line([(s(330), s(rows[0])), (s(378), s(rows[0]))], fill=ACC, width=s(1))
        d.line([(s(330), s(rows[1])), (s(378), s(rows[1]))], fill=ACC, width=s(1))
        # the bit's width, shown on the bit parked at the first row's end
        bx, by = right - r, rows[0]
        d.ellipse([s(bx - r), s(by - r), s(bx + r), s(by + r)], outline=ACC, width=s(3))
        d.line([(s(bx - r), s(by)), (s(bx + r), s(by))], fill=ACC, width=s(2))
        d.text((s(bx), s(by - r - 4)), "Dia", fill=ACC, font=font(15), anchor="mb")
    return img.resize((W, H), Image.LANCZOS)


def save_gif(path, frames, hold, ms=60, hold_ms=3200):
    frames = [f.convert("P", palette=Image.ADAPTIVE, colors=64) for f in frames + [hold]]
    durations = [ms] * (len(frames) - 1) + [hold_ms]
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=durations, loop=0, optimize=True)
    print(path)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, "..", "crates", "help", "pictures", "concepts")
    os.makedirs(out, exist_ok=True)
    frames, last = stepover_frames()
    save_gif(os.path.join(out, "stepover.gif"), frames, last)
    last.save(os.path.join(out, "stepover.png"))
    print(os.path.join(out, "stepover.png"))


if __name__ == "__main__":
    main()
