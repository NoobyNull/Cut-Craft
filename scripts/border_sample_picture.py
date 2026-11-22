#!/usr/bin/env python3
"""Draws the sample border Generate Border's help shows: one repeat of a carved moulding as a
depth map (16-bit greys, white highest) and as a lit photo, the pair the Borders add-on
saves, written to crates/help/pictures/generate_border/ (sample-depth.png, sample.jpg) with
depth-and-photo.png, the two side by side for the help page.

    python3 -I scripts/border_sample_picture.py [OUT_DIR]

Needs Pillow and NumPy. Made by hand, not by Google, so the help has no generated picture
in it and runs offline.
"""

import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

W, H = 640, 360  # 16:9, as Generate Border's Medium shape


def depth():
    x = np.linspace(0.0, 1.0, W, endpoint=False)[None, :]
    y = np.linspace(0.0, 1.0, H)[:, None]
    # the moulding's profile, top to bottom: a bead, a cove, the carved field, a bead
    bead = lambda c, r: np.clip(1 - ((y - c) / r) ** 2, 0, None) ** 0.5
    profile = 0.55 * bead(0.08, 0.07) + 0.55 * bead(0.92, 0.07)
    profile = profile + 0.25 * np.clip(1 - np.abs(y - 0.5) / 0.36, 0, 1)
    # two scrolls to a repeat, each a spiral leaf that wraps round to meet the next
    field = np.zeros((H, W))
    for cx in (0.25, 0.75):
        dx = (x - cx + 0.5) % 1.0 - 0.5
        dx = dx * W / H
        dy = y - 0.5
        r = np.hypot(dx, dy)
        a = np.arctan2(dy, dx)
        spiral = np.cos(a * 1.0 - r * 38.0)
        leaf = np.clip(1 - r / 0.3, 0, 1)
        field = field + 0.45 * leaf * np.clip(spiral, 0, 1) ** 0.6
    # beading between the scrolls
    beads = np.clip(1 - np.hypot(((x * 16) % 1 - 0.5) * W / 16 / (H * 0.03), (y - 0.2) / 0.03), 0, 1)
    beads = beads + np.clip(1 - np.hypot(((x * 16) % 1 - 0.5) * W / 16 / (H * 0.03), (y - 0.8) / 0.03), 0, 1)
    d = profile + field * (np.abs(y - 0.5) < 0.3) + 0.25 * beads ** 0.5
    d = d - d.min()
    return d / d.max()


def photo(d):
    gy, gx = np.gradient(d * 40.0)
    n = np.dstack([-gx, -gy, np.ones_like(d)])
    n = n / np.linalg.norm(n, axis=2, keepdims=True)
    light = np.array([-0.5, -0.6, 0.62])
    light = light / np.linalg.norm(light)
    shade = np.clip((n * light).sum(axis=2), 0, 1)
    gold = np.array([212, 168, 84]) / 255
    dark = np.array([92, 64, 28]) / 255
    t = (0.35 * d + 0.65 * shade)[..., None]
    rgb = dark + (gold - dark) * t
    spec = np.clip(shade - 0.85, 0, None)[..., None] * 4
    return np.clip(rgb + spec, 0, 1)


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "crates/help/pictures/generate_border"
    os.makedirs(out, exist_ok=True)
    d = depth()
    Image.fromarray((d * 65535).round().astype(np.uint16)).save(os.path.join(out, "sample-depth.png"))
    rgb = (photo(d) * 255).round().astype(np.uint8)
    Image.fromarray(rgb).save(os.path.join(out, "sample.jpg"), quality=85)
    # the help page's pair: depth map and photo side by side, no words in the picture
    small = (W // 2, H // 2)
    grey = Image.fromarray((d * 255).round().astype(np.uint8)).convert("RGB").resize(small)
    lit = Image.fromarray(rgb).resize(small)
    pad = 12
    both = Image.new("RGB", (small[0] * 2 + pad * 3, small[1] + pad * 2), (255, 255, 255))
    both.paste(grey, (pad, pad))
    both.paste(lit, (small[0] + pad * 2, pad))
    both.save(os.path.join(out, "depth-and-photo.png"), optimize=True)


main()
