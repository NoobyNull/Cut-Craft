"""The Embellish Style list's pictures, one a style, drawn from the style's own vectors.

usage: EMBELLISH_ICONS=DIR cargo nextest run -p cmd-embellish style_pictures --run-ignored only
       python3 -I scripts/make_embellish_icons.py DIR assets/icons/drawing_forms

Each DIR/<style>.json is a list of outlines (lists of [x, y] in mm) on a 100 mm rounded
square; the picture is that square in the icon set's grey, the pieces filled in its ink.
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

INK = (0x4D, 0x7E, 0xC4, 255)
GREY = (0x9A, 0xA3, 0xB2, 255)
SIZE = 256
SCALE = 4  # drawn this many times bigger, then shrunk, for smooth edges


def area(ring):
    return 0.5 * sum(
        ring[i][0] * ring[(i + 1) % len(ring)][1] - ring[(i + 1) % len(ring)][0] * ring[i][1]
        for i in range(len(ring))
    )


def inside(ring, p):
    x, y = p
    hit = False
    for i in range(len(ring)):
        (ax, ay), (bx, by) = ring[i], ring[i - 1]
        if (ay > y) != (by > y) and x < ax + (y - ay) * (bx - ax) / (by - ay):
            hit = not hit
    return hit


def draw(rings, out):
    big = SIZE * SCALE
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = 18 * SCALE
    k = (big - 2 * pad) / 100.0

    def px(p):
        return (pad + p[0] * k, big - pad - p[1] * k)

    # the shape itself: the biggest outline
    rings = sorted(rings, key=lambda r: -abs(area(r)))
    frame, pieces = rings[0], rings[1:]
    d.polygon([px(p) for p in frame], outline=GREY, width=7 * SCALE)
    # biggest first, each filled or cut out by how many pieces it is inside: a ring inside
    # one other is that piece's hole, a piece inside a hole is solid again
    for i, r in enumerate(pieces):
        if len(r) < 3:
            continue
        depth = sum(
            1 for j, o in enumerate(pieces) if j != i and abs(area(o)) > abs(area(r)) and inside(o, r[0])
        )
        d.polygon([px(p) for p in r], fill=(0, 0, 0, 0) if depth % 2 else INK)
    img.resize((SIZE, SIZE), Image.LANCZOS).save(out)


def main():
    src, dst = Path(sys.argv[1]), Path(sys.argv[2])
    for f in sorted(src.glob("*.json")):
        draw(json.loads(f.read_text()), dst / f"embellish_{f.stem}.png")
        print(f.stem)


if __name__ == "__main__":
    main()
