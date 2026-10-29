"""Shaded orthographic renders of binary STL models, for the app's pictures.

load(path) -> triangles (n, 3, 3), centred on the model's bounding box.
render(tri, size, az, el, color, fit) -> (rgba uint8 array, project): `color` is one "#rrggbb" or
an (n, 3) array of 0..1 colours, one per triangle; `fit` is the
(x0, y0, x1, y1) part of the picture the model is scaled into, and project(points)
maps model points to picture pixels, so labels and arrows can be drawn to match.
"""
import math
import numpy as np


def load(path):
    b = open(path, "rb").read()
    n = int.from_bytes(b[80:84], "little")
    d = np.frombuffer(b, dtype=np.dtype([("n", "<f4", 3), ("v", "<f4", (3, 3)), ("a", "<u2")]), count=n, offset=84)
    tri = d["v"].astype(float)
    flat = tri.reshape(-1, 3)
    return tri - (flat.min(0) + flat.max(0)) / 2


def rotation(az, el):
    a, e = math.radians(az), math.radians(el)
    rz = np.array([[math.cos(a), -math.sin(a), 0], [math.sin(a), math.cos(a), 0], [0, 0, 1]])
    rx = np.array([[1, 0, 0], [0, math.cos(e), -math.sin(e)], [0, math.sin(e), math.cos(e)]])
    return rx @ rz


def render(tri, size, az, el, color, fit=(0.04, 0.04, 0.96, 0.96), ss=4, line=0.4, small_px=2.0, jump_share=0.03, bounds=None, line_px=1.0, fit_points=None):
    if isinstance(color, str):
        color = np.array([int(color[i:i + 2], 16) for i in (1, 3, 5)], float) / 255
    cols = np.broadcast_to(np.asarray(color, float), (len(tri), 3))
    R = rotation(az, el)
    W = size * ss
    t = tri @ R.T
    # `bounds` (lo, hi) keeps one projection when parts of a model are drawn separately
    flat = tri.reshape(-1, 3)
    blo, bhi = bounds if bounds is not None else (flat.min(0), flat.max(0))
    corners = np.array([[x, y, z] for x in (blo[0], bhi[0]) for y in (blo[1], bhi[1]) for z in (blo[2], bhi[2])]) @ R.T
    if fit_points is not None:  # size to these points as seen, not to their box
        corners = np.asarray(fit_points, float) @ R.T
    lo = np.array([corners[:, 0].min(), -corners[:, 2].max()])
    hi = np.array([corners[:, 0].max(), -corners[:, 2].min()])
    box = np.array(fit) * W
    scale = min((box[2] - box[0]) / (hi - lo)[0], (box[3] - box[1]) / (hi - lo)[1])
    off = (box[:2] + box[2:]) / 2 - (lo + hi) / 2 * scale

    def to_screen(p):  # rotated points -> supersampled pixels
        return p[..., 0] * scale + off[0], -p[..., 2] * scale + off[1]

    def project(points):
        x, y = to_screen(np.asarray(points, float) @ R.T)
        return np.stack([x, y], -1) / ss

    X, Y = to_screen(t)
    Z = t[..., 1]
    nrm = np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0])
    area = np.abs(nrm[:, 1]) * scale * scale / 2  # projected area in pixels
    nrm /= np.linalg.norm(nrm, axis=1, keepdims=True) + 1e-12
    light = np.array([-0.45, -0.75, 0.5]); light /= np.linalg.norm(light)
    shade = 0.35 + 0.65 * np.abs(nrm @ light)
    zbuf = np.full((W, W), np.inf)
    img = np.zeros((W, W, 3))
    small = area < small_px
    # tiny triangles: plot their corners and centre, nearest last so it wins
    px = np.concatenate([X[small], X[small].mean(1, keepdims=True)], 1).ravel()
    py = np.concatenate([Y[small], Y[small].mean(1, keepdims=True)], 1).ravel()
    pz = np.concatenate([Z[small], Z[small].mean(1, keepdims=True)], 1).ravel()
    ps = np.repeat(shade[small], 4)
    pc = np.repeat(cols[small], 4, axis=0)
    ix, iy = px.astype(int), py.astype(int)
    ok = (ix >= 0) & (ix < W) & (iy >= 0) & (iy < W)
    ix, iy, pz, ps, pc = ix[ok], iy[ok], pz[ok], ps[ok], pc[ok]
    order = np.argsort(-pz)
    ix, iy, pz, ps, pc = ix[order], iy[order], pz[order], ps[order], pc[order]
    zbuf[iy, ix] = pz
    img[iy, ix] = pc * ps[:, None]
    for i in np.nonzero(~small)[0]:
        x, y, z = X[i], Y[i], Z[i]
        x0, x1 = max(int(x.min()), 0), min(int(x.max()) + 1, W - 1)
        y0, y1 = max(int(y.min()), 0), min(int(y.max()) + 1, W - 1)
        if x1 < x0 or y1 < y0:
            continue
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
        d0 = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
        if abs(d0) < 1e-12:
            continue
        a = ((y[1] - y[2]) * (gx - x[2]) + (x[2] - x[1]) * (gy - y[2])) / d0
        b = ((y[2] - y[0]) * (gx - x[2]) + (x[0] - x[2]) * (gy - y[2])) / d0
        c = 1 - a - b
        inside = (a >= 0) & (b >= 0) & (c >= 0)
        zz = a * z[0] + b * z[1] + c * z[2]
        sub = zbuf[y0:y1 + 1, x0:x1 + 1]
        m = inside & (zz < sub)
        sub[m] = zz[m]
        img[y0:y1 + 1, x0:x1 + 1][m] = cols[i] * shade[i]
    alpha = np.isfinite(zbuf)
    # fill pinholes the plotted points leave: a hole with model on most sides takes their mean
    for _ in range(2):
        nb = sum(np.roll(np.roll(alpha, dy, 0), dx, 1).astype(int) for dy, dx in ((0, 1), (1, 0), (0, -1), (-1, 0)))
        hole = ~alpha & (nb >= 3)
        if not hole.any():
            break
        acc = sum(np.roll(np.roll(np.where(alpha[..., None], img, 0), dy, 0), dx, 1) for dy, dx in ((0, 1), (1, 0), (0, -1), (-1, 0)))
        zacc = sum(np.roll(np.roll(np.where(alpha, zbuf, 0), dy, 0), dx, 1) for dy, dx in ((0, 1), (1, 0), (0, -1), (-1, 0)))
        img[hole] = acc[hole] / nb[hole][:, None]
        zbuf[hole] = zacc[hole] / nb[hole]
        alpha = alpha | hole
    zf = np.where(alpha, zbuf, zbuf[alpha].max() + 1e3)
    edge = np.zeros_like(alpha)
    jump = (hi - lo).max() * scale * jump_share
    for dy, dx in ((0, 1), (1, 0), (0, -1), (-1, 0)):
        edge |= np.abs(zf - np.roll(np.roll(zf, dy, 0), dx, 1)) > jump
    edge &= alpha
    for _ in range(max(1, round(ss * line_px / 2))):
        edge = edge | ((np.roll(edge, 1, 0) | np.roll(edge, 1, 1)) & alpha)
    img[edge] = img[edge] * line
    rgba = np.concatenate([img, alpha[..., None].astype(float)], -1).reshape(size, ss, size, ss, 4).mean((1, 3))
    rgb = np.where(rgba[..., 3:] > 0, rgba[..., :3] / np.maximum(rgba[..., 3:], 1e-6), 0)
    return (np.concatenate([rgb, rgba[..., 3:]], -1) * 255).clip(0, 255).astype(np.uint8), project


def save_png(rgba, out):
    import subprocess
    h, w = rgba.shape[:2]
    subprocess.run(["magick", "-size", f"{w}x{h}", "-depth", "8", "rgba:-", out], input=rgba.tobytes(), check=True)


