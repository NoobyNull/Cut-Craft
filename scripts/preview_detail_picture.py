#!/usr/bin/env python3
"""Draws the help picture for Preview detail (crates/help/pictures/concepts/preview_detail.png).

The same corner of a part, a 10 mm radius cut round with a 6 mm bit and an 8 mm hole, as the
preview works it out at two spacings: 0.5 mm between points (the old whole-sheet preview of
a 600 mm sheet) and 0.1 mm (the slider at 50%, the toolpaths' area alone). Each point is a
square of its height, lit from the top left, as the 3D view shades it.

    python3 scripts/preview_detail_picture.py
"""

import pathlib

import numpy as np
from PIL import Image, ImageDraw, ImageFont

OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/help/pictures/concepts/preview_detail.png"
VIEW = 30.0  # mm across each panel
PANEL = 420  # pixels across each panel
DEPTH = 3.0


def heights(spacing):
    """The cut seen from above, a height a point `spacing` mm apart."""
    n = int(round(VIEW / spacing))
    centres = (np.arange(n) + 0.5) * spacing
    x, y = np.meshgrid(centres, centres)
    z = np.zeros_like(x)
    # a part's corner, 10 mm radius at (16, 16), cut round on the outside by a 6 mm bit
    cx = cy = 16.0
    r = 10.0
    dx, dy = np.maximum(cx - x, 0), np.maximum(cy - y, 0)
    outside = np.where((x < cx) & (y < cy), np.hypot(dx, dy), np.maximum(dx, dy))
    groove = (outside > r) & (outside < r + 6.0)
    z[groove] = -DEPTH
    # an 8 mm hole in the part
    z[np.hypot(x - 22.0, y - 22.0) < 4.0] = -DEPTH
    return z


def shade(z):
    gy, gx = np.gradient(z)
    light = np.clip(0.9 + 1.6 * (-gx - gy), 0.35, 1.15)
    wood = np.array([222, 184, 135], dtype=float)
    floor = np.array([120, 84, 52], dtype=float)
    mix = (z < -0.1)[..., None]
    base = np.where(mix, floor, wood)
    rgb = base * light[..., None]
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8)).resize(
        (PANEL, PANEL), Image.NEAREST
    )


def main():
    font = None
    for name in ("DejaVuSans.ttf", "Arial.ttf"):
        try:
            font = ImageFont.truetype(name, 22)
            break
        except OSError:
            continue
    font = font or ImageFont.load_default()
    gap, top = 24, 46
    picture = Image.new("RGB", (2 * PANEL + 3 * gap, PANEL + top + gap), (250, 250, 250))
    draw = ImageDraw.Draw(picture)
    for i, (spacing, label) in enumerate(
        [(0.5, "Faster: 0.5 mm between points"), (0.1, "More detail: 0.1 mm")]
    ):
        left = gap + i * (PANEL + gap)
        draw.text((left, 12), label, fill=(30, 30, 30), font=font)
        img = shade(heights(spacing)).transpose(Image.FLIP_TOP_BOTTOM)
        picture.paste(img, (left, top))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    picture.save(OUT, optimize=True)
    print(OUT)


if __name__ == "__main__":
    main()
