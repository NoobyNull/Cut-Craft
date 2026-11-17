#!/usr/bin/env python3
"""The 3D Pencil Toolpath help picture: three views of one job from above, shaded, with what
is left over the model in red. Left: after 3D Finishing with a 6 mm ball (its corners left
round). Middle: the pencil toolpath along the valleys. Right: after the pencil pass.

usage:
  PENCIL_PICTURE=/tmp/p.json cargo test -p cmd-3d-pencil-toolpath help_picture -- --ignored
  python3 -I scripts/pencil_help_picture.py /tmp/p.json crates/help/pictures/pencil-before-after.png \
      crates/help/pictures/pencil-pass.gif
The GIF plays the same: the finished job, the pencil passes drawn in as they cut, then the
job after them.
"""
import json, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont

WOOD = np.array([233, 207, 158], float)
RED = np.array([214, 50, 40], float)
PATH = (31, 158, 102)
SCALE = 1  # pixels a cell


def shade(z, cell):
    gy, gx = np.gradient(z, cell)
    n = np.dstack([-gx, -gy, np.ones_like(z)])
    n /= np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.5, 0.6, 0.75])
    light /= np.linalg.norm(light)
    return np.clip(0.35 + 0.65 * (n @ light), 0, 1)


def panel(z, model, cell, left_red=True):
    lit = shade(z, cell)[..., None] * WOOD
    if left_red:
        left = np.clip((z - model) / 0.4, 0, 1)[..., None]
        lit = lit * (1 - 0.85 * left) + RED * 0.85 * left * shade(z, cell)[..., None] * 1.3
    img = Image.fromarray(np.clip(lit, 0, 255).astype(np.uint8)[::-1])
    return img.resize((img.width * SCALE, img.height * SCALE), Image.LANCZOS)


def main():
    data = json.load(open(sys.argv[1]))
    c, r, cell = data["columns"], data["rows"], data["cell"]
    grid = lambda k: np.array(data[k], float).reshape(r, c)
    model, finished, cleaned = grid("model"), grid("finished"), grid("cleaned")
    # only the job's rows and columns that hold the 60 x 40 mm job
    a = panel(finished, model, cell)
    b = panel(finished, model, cell, left_red=False)
    d = ImageDraw.Draw(b)
    h = b.height
    to_px = lambda x, y: (x / cell * SCALE, h - y / cell * SCALE)
    last = None
    for x, y, z, cut in data["cuts"]:
        p = to_px(x, y)
        if cut and last is not None:
            d.line([last, p], fill=PATH, width=1)
        last = p
    e = panel(cleaned, model, cell)
    pad, top = 16, 44
    out = Image.new("RGB", (a.width * 3 + pad * 4, a.height + top + pad), "white")
    try:
        font = ImageFont.truetype("DejaVuSans.ttf", 22)
    except OSError:
        font = ImageFont.load_default()
    draw = ImageDraw.Draw(out)
    titles = ["1. After 3D Finishing (6 mm ball)", "2. 3D Pencil toolpath (3 mm ball)", "3. After the pencil pass"]
    for i, (img, title) in enumerate(zip((a, b, e), titles)):
        x = pad + i * (a.width + pad)
        out.paste(img, (x, top))
        draw.text((x, 12), title, fill=(30, 30, 30), font=font)
    out.save(sys.argv[2], optimize=True)
    if len(sys.argv) > 3:
        gif(a, e, data, cell, sys.argv[3])


def gif(before, after, data, cell, path):
    """The pencil passes drawn in over the finished job a few at a time, then the result."""
    h = before.height
    to_px = lambda x, y: (x / cell * SCALE, h - y / cell * SCALE)
    segments, last = [], None
    for x, y, z, cut in data["cuts"]:
        p = to_px(x, y)
        if cut and last is not None:
            segments.append((last, p))
        last = p
    frames, steps = [before.copy()] * 4, 20
    canvas = before.copy()
    d = ImageDraw.Draw(canvas)
    for k in range(steps):
        for s in segments[k * len(segments) // steps:(k + 1) * len(segments) // steps]:
            d.line(s, fill=PATH, width=1)
        frames.append(canvas.copy())
    frames += [canvas.copy()] * 3 + [after] * 8
    small = [f.convert("P", palette=Image.ADAPTIVE, colors=96) for f in frames]
    small[0].save(path, save_all=True, append_images=small[1:], duration=180, loop=0, optimize=True)


if __name__ == "__main__":
    main()
