#!/usr/bin/env python3
"""The 3D tools' icons: simple solids built here in numpy, shaded by stlrender in the same
three-quarter view as the model pictures, with make_icons' strokes drawn over them.

usage: python3 -I scripts/make_3d_icons.py OUT_DIR   (after make_icons.py has written OUT_DIR;
these replace its flat drawings for the same names)
"""
import base64, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
import stlrender
from make_icons import (ACC, DARK, GREY, INK, RED, TP, WHITE, WOODD, arrow, circ, curved_arrow, darrow, dot, eye, folder,
                        line, path, picture, pl, pencil, plus, rect, solid)

BODY = "#7aa6e4"   # a 3D component, as in the model pictures
WOOD = "#f1d6a8"   # material
PALE = "#c7d3e3"   # a component as it was, or one that is hidden
AZ, EL = -35, 32
PX = 4             # rendered pixels per icon unit: 48 units become 192 pixels


def rgb(c):
    return np.array([int(c[i:i + 2], 16) for i in (1, 3, 5)], float) / 255


# ---- solids: arrays of triangles (n, 3, 3) ----

def grid(fn, x0, x1, y0, y1, n=56, base=0.0, keep_flat=True, walls=True):
    """The surface z = fn(x, y) over a rectangle, closed down to `base` round its edge. Without
    keep_flat, cells lying on the base are left out, so a shape stands alone with no plate."""
    xs, ys = np.linspace(x0, x1, n + 1), np.linspace(y0, y1, n + 1)
    X, Y = np.meshgrid(xs, ys)
    Z = np.maximum(fn(X, Y), base)
    P = np.stack([X, Y, Z], -1)
    a, b, c, d = P[:-1, :-1], P[:-1, 1:], P[1:, 1:], P[1:, :-1]
    t = np.concatenate([np.stack([a, b, c], -2).reshape(-1, 3, 3), np.stack([a, c, d], -2).reshape(-1, 3, 3)])
    if not keep_flat:
        t = t[(t[:, :, 2] > base + 1e-6).any(1)]
    out = [t]
    if walls:
        for edge in (P[0], P[-1], P[:, 0], P[:, -1]):
            top = edge
            bot = edge.copy()
            bot[:, 2] = base
            q0, q1, q2, q3 = top[:-1], top[1:], bot[1:], bot[:-1]
            out += [np.stack([q0, q1, q2], 1), np.stack([q0, q2, q3], 1)]
    t = np.concatenate(out)
    # walls of no height, round a shape that stands alone, would only plot a square of dots
    area = np.linalg.norm(np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0]), axis=1)
    return t[area > 1e-9]


def box(x0, y0, z0, x1, y1, z1):
    return grid(lambda X, Y: np.full_like(X, z1), x0, x1, y0, y1, n=1, base=z0)


def revolve(profile, cx=0.0, cy=0.0, seg=64):
    """A turned solid from (radius, z) points, bottom to top."""
    out = []
    a = np.linspace(0, 2 * math.pi, seg + 1)
    ring = lambda r, z: np.stack([cx + r * np.cos(a), cy + r * np.sin(a), np.full_like(a, z)], -1)
    rings = [ring(r, z) for r, z in profile]
    for p, q in zip(rings, rings[1:]):
        out += [np.stack([p[:-1], p[1:], q[1:]], 1), np.stack([p[:-1], q[1:], q[:-1]], 1)]
    top = profile[-1]
    centre = np.tile([cx, cy, top[1]], (seg, 1))
    out.append(np.stack([rings[-1][:-1], rings[-1][1:], centre], 1))
    return np.concatenate(out)


def tube(points, r, seg=16):
    """A round tube along a polyline of 3D points."""
    p = np.asarray(points, float)
    out = []
    a = np.linspace(0, 2 * math.pi, seg + 1)
    rings = []
    for i in range(len(p)):
        d = p[min(i + 1, len(p) - 1)] - p[max(i - 1, 0)]
        d /= np.linalg.norm(d)
        u = np.cross(d, [0, 0, 1.0])
        if np.linalg.norm(u) < 1e-6:
            u = np.cross(d, [0, 1.0, 0])
        u /= np.linalg.norm(u)
        v = np.cross(d, u)
        rings.append(p[i] + r * (np.outer(np.cos(a), u) + np.outer(np.sin(a), v)))
    for q, w in zip(rings, rings[1:]):
        out += [np.stack([q[:-1], q[1:], w[1:]], 1), np.stack([q[:-1], w[1:], w[:-1]], 1)]
    return np.concatenate(out)


def shift(t, dx=0.0, dy=0.0, dz=0.0):
    return t + np.array([dx, dy, dz])


def dome_fn(cx=0.0, cy=0.0, r=0.8, h=0.75):
    return lambda X, Y: h * np.sqrt(np.clip(1 - ((X - cx) ** 2 + (Y - cy) ** 2) / r ** 2, 0, 1))


def cone_fn(cx=0.0, cy=0.0, r=0.8, h=0.8):
    return lambda X, Y: h * np.clip(1 - np.hypot(X - cx, Y - cy) / r, 0, 1)


def plateau_fn(cx=0.0, cy=0.0, r=0.8, h=0.45, edge=0.04):
    return lambda X, Y: h * np.clip((r - np.hypot(X - cx, Y - cy)) / edge, 0, 1)


def bump_fn(cx=0.0, cy=0.0, s=0.33, h=0.85):
    # a bell that reaches the base at three spreads, so it stands without a plate
    return lambda X, Y: h * np.clip((np.exp(-((X - cx) ** 2 + (Y - cy) ** 2) / (2 * s * s)) - 0.011) / 0.989, 0, 1)


def shape(fn, r=0.95, n=64):
    """A component standing alone: the surface fn over its footprint, no plate."""
    return grid(fn, -r, r, -r, r, n=n, keep_flat=False)


def on_slab(fn, z0=0.0, depth=0.18, n=64, half=1.0):
    """A relief on a block of material: fn sits on the block's top."""
    return grid(lambda X, Y: z0 + depth + fn(X, Y), -half, half, -half, half, n=n, base=z0)


# ---- one icon: solids in colours, drawn, then strokes on top ----

class Icon:
    def __init__(self, fit=(0.06, 0.06, 0.94, 0.94), bounds=None, az=AZ, el=EL):
        # without bounds the picture is sized to its own solids
        self.bounds, self.fit, self.az, self.el = bounds, fit, az, el
        self.layers = []   # (solids, opacity)
        self.under = []    # SVG elements in the 48 box, behind the solids
        self.marks = []    # and in front of them
        self._project = None
        self.extra = []    # points of strokes that reach past the solids, so they stay in the picture

    def include(self, pts):
        """Size the picture to take in these model points too; call before any p()."""
        self.extra.append(np.asarray(pts, float).reshape(-1, 3))
        return self

    def add(self, solids, opacity=1.0):
        """solids: [(triangles, colour)] drawn together, hiding each other properly."""
        self.layers.append((solids, opacity))
        self._project = None
        return self

    def project(self, pts):
        """Model points to the 48 box."""
        if self.bounds is None:
            flat = np.concatenate([t.reshape(-1, 3) for solids, _ in self.layers for t, _ in solids])
            self.bounds = (flat.min(0), flat.max(0))
            self.fit_points = np.concatenate([flat[:: max(1, len(flat) // 20000)], *self.extra])
        elif not hasattr(self, "fit_points"):
            lo, hi = self.bounds
            self.fit_points = np.array([[x, y, z] for x in (lo[0], hi[0]) for y in (lo[1], hi[1]) for z in (lo[2], hi[2])])
        if self._project is None:
            self._project = stlrender.render(box(-1, -1, 0, 1, 1, 0.01), 8, self.az, self.el, "#000000",
                                             fit=self.fit, bounds=self.bounds, ss=1, fit_points=self.fit_points)[1]
            scale = 48 / 8
            inner = self._project
            self._project = lambda p: inner(p) * scale
        return self._project(pts)

    def p(self, x, y, z):
        q = self.project([[x, y, z]])[0]
        return float(q[0]), float(q[1])

    def svg(self, path_out):
        self.project([[0, 0, 0]])  # settles the bounds
        parts = list(self.under)
        for solids, opacity in self.layers:
            tri = np.concatenate([t for t, _ in solids])
            cols = np.concatenate([np.tile(rgb(c), (len(t), 1)) for t, c in solids])
            img, _ = stlrender.render(tri, 48 * PX, self.az, self.el, cols, fit=self.fit, bounds=self.bounds,
                                      jump_share=0.04, line_px=0.9, fit_points=self.fit_points)
            tmp = path_out + ".tmp.png"
            stlrender.save_png(img, tmp)
            data = base64.b64encode(open(tmp, "rb").read()).decode()
            os.remove(tmp)
            parts.append(f'<image href="data:image/png;base64,{data}" x="0" y="0" width="48" height="48"'
                         + (f' opacity="{opacity}"' if opacity < 1 else "") + "/>")
        body = "".join(parts) + "".join(self.marks)
        with open(path_out, "w") as fh:
            fh.write(f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 48 48" width="48" height="48">{body}</svg>')


I = {}


def icon(name):
    def wrap(fn):
        I[name] = fn
        return fn
    return wrap


# ---- the icons ----

@icon("modeling/create_shape_round")
def _():
    return Icon().add([(shape(dome_fn()), BODY)])


@icon("modeling/create_shape_angled")
def _():
    return Icon().add([(shape(cone_fn(), n=96), BODY)])


@icon("modeling/create_shape_flat")
def _():
    return Icon().add([(shape(plateau_fn(), n=96), BODY)])


@icon("modeling/create_shape_smooth")
def _():
    return Icon().add([(shape(bump_fn()), BODY)])


@icon("modeling/scale_model_z_height")
def _():
    ic = Icon(fit=(0.0, 0.04, 0.8, 0.98)).add([(shape(dome_fn(h=0.95)), BODY)])
    _, top = ic.p(0, 0, 0.95)
    _, bot = ic.p(0, 0, 0.0)
    ic.marks.append(darrow(43, top, 43, bot, ACC, 2.5, 4.5))
    return ic


@icon("modeling/turn_and_spin")
def _():
    z = np.linspace(0, 1, 48)
    r = 0.24 + 0.16 * np.sin(z * 5.2 - 0.9) + 0.1 * z
    r[:3] = [0.36, 0.38, 0.36]  # a foot
    ic = Icon(fit=(0.04, 0.04, 0.96, 0.96)).add([(revolve(list(zip(r, z))), BODY)])
    ic.include(ring3(0, 0, 0.62, 0.45))
    ring = [ic.p(0.62 * math.cos(a), 0.62 * math.sin(a), 0.45) for a in np.linspace(math.radians(200), math.radians(500), 40)]
    ic.marks.append(pl(ring[:-1], ACC, 2.2))
    ic.marks.append(arrow(*ring[-6], *ring[-1], ACC, 2.2, 5))
    return ic


@icon("view/3d_view")
def _():
    return Icon(fit=(0.14, 0.1, 0.86, 0.9)).add([(box(-0.6, -0.6, 0, 0.6, 0.6, 1.0), BODY)])


@icon("modeling/texture")
def _():
    wave = lambda X, Y: 0.06 * (np.sin(X * 9) * np.cos(Y * 9) + 1)
    return Icon().add([(on_slab(wave, n=80, depth=0.22), WOOD)])


def surface_line(fn, pts, lift=0.02):
    """Points on a surface z = fn(x, y), a little above it so a stroke sits on top."""
    pts = np.asarray(pts, float)
    z = fn(pts[:, 0:1], pts[:, 1:2])[:, 0] + lift
    return np.stack([pts[:, 0], pts[:, 1], z], -1)


def ring3(cx, cy, r, z, n=48):
    a = np.linspace(0, 2 * math.pi, n + 1)
    return np.stack([cx + r * np.cos(a), cy + r * np.sin(a), np.full_like(a, z)], -1)


def stroke3(ic, pts, c=ACC, w=2.2, dash=None):
    return pl([ic.p(*q) for q in pts], c, w, dash=dash)


def pair(fn_a, fn_b):
    return lambda X, Y: np.maximum(fn_a(X, Y), fn_b(X, Y))


TWO = pair(dome_fn(-0.3, 0.15, 0.55, 0.6), cone_fn(0.4, -0.2, 0.5, 0.75))


@icon("modeling/create_shape")
def _():
    ic = Icon(fit=(0.02, 0.16, 0.78, 0.98)).add([(shape(bump_fn()), BODY)])
    ic.marks.append(pencil(31, 20, 0.66))
    return ic


@icon("modeling/add_draft_to_model")
def _():
    ic = Icon().add([(shape(plateau_fn(r=0.85, h=0.5, edge=0.32), n=96), BODY)])
    a, b = ic.p(0.85, 0, 0), ic.p(0.53, 0, 0.5)
    ic.marks.append(line(*a, *b, ACC, 2.2))
    return ic


@icon("modeling/add_zero_plane")
def _():
    ic = Icon(fit=(0.04, 0.08, 0.86, 0.9))
    ic.add([(box(-1, -1, -0.03, 1, 1, 0), "#9fb8e0")], 0.75)
    ic.add([(shape(dome_fn(r=0.55, h=0.5)), BODY)])
    ic.marks.append(plus(40, 39, 6, ACC, 3.5))
    return ic


@icon("modeling/bake_components")
def _():
    ic = Icon(fit=(0.06, 0.24, 0.94, 0.98)).add([(on_slab(TWO, depth=0.25), WOOD)])
    ic.marks.append(arrow(24, 1, 24, 13, ACC, 2.8, 6))
    return ic


def levels():
    ic = Icon(fit=(0.08, 0.04, 0.92, 0.96))
    solids = []
    for k, (z, c) in enumerate(((0.0, "#9fb8e0"), (0.42, BODY), (0.84, "#9fb8e0"))):
        solids.append((shift(box(-0.8, -0.8, 0, 0.8, 0.8, 0.1), dz=z), c))
    solids.append((shift(shape(dome_fn(r=0.45, h=0.22)), dz=0.94), BODY))
    return ic.add(solids)


I["modeling/component_levels"] = levels
I["view/component_levels"] = levels


@icon("modeling/create_component_from_bitmap")
def _():
    ic = Icon(fit=(0.34, 0.4, 1.0, 1.0)).add([(shape(dome_fn()), BODY)])
    ic.under.append(picture(2, 2, 24, 19))
    ic.marks.append(arrow(18, 22, 25, 30, ACC, 2.5, 5))
    return ic


@icon("modeling/create_component_from_visible_model")
def _():
    ic = Icon(fit=(0.02, 0.22, 0.86, 1.0)).add([(shape(TWO), BODY)])
    ic.marks.append(eye(37, 9, 0.48))
    return ic


@icon("modeling/create_cross_section_vector")
def _():
    fn = dome_fn(r=0.75, h=0.65)
    ic = Icon().add([(on_slab(fn, depth=0.2), WOOD)])
    xs = np.linspace(-1, 1, 60)
    ic.marks.append(stroke3(ic, surface_line(lambda X, Y: 0.2 + fn(X, Y), np.stack([xs, np.zeros_like(xs)], -1)), ACC, 2.0))
    return ic


@icon("modeling/create_texture_area")
def _():
    wave = lambda X, Y: np.where((np.abs(X) < 0.6) & (np.abs(Y) < 0.6), 0.06 * (np.sin(X * 14) * np.cos(Y * 14) + 1), 0)
    ic = Icon().add([(on_slab(wave, n=96, depth=0.2), WOOD)])
    sq = [(-0.65, -0.65), (0.65, -0.65), (0.65, 0.65), (-0.65, 0.65), (-0.65, -0.65)]
    ic.marks.append(stroke3(ic, [(x, y, 0.21) for x, y in sq], INK, 1.8))
    return ic


@icon("modeling/create_vector_boundary_around_selected_components")
def _():
    ic = Icon().add([(shape(TWO), BODY)])
    a = np.linspace(0, 2 * math.pi, 64)
    rr = 0.98 + 0.08 * np.cos(2 * a + 0.6)
    edge = np.stack([rr * np.cos(a), rr * np.sin(a), np.zeros_like(a)], -1)
    ic.include(edge)
    ic.marks.append(stroke3(ic, edge, ACC, 2.4))
    return ic


@icon("modeling/emboss_component")
def _():
    ic = Icon(fit=(0.06, 0.06, 0.94, 0.94), bounds=((-0.95, -0.95, 0), (0.95, 0.95, 0.85)))
    ic.add([(shape(bump_fn(h=0.85)), PALE)], 0.45)
    ic.add([(shape(bump_fn(h=0.28)), BODY)])
    _, top = ic.p(0, 0, 0.8)
    _, low = ic.p(0, 0, 0.36)
    ic.marks.append(arrow(40, top, 40, low, ACC, 2.6, 5))
    return ic


@icon("modeling/export_as_stl")
def _():
    ic = Icon(fit=(0.0, 0.3, 0.72, 1.0)).add([(shape(dome_fn(), n=7), BODY)])
    ic.marks.append(arrow(30, 18, 44, 4, ACC, 3, 7))
    return ic


@icon("file/export_as_3d_clipart")
def _():
    ic = Icon(fit=(0.0, 0.3, 0.72, 1.0)).add([(on_slab(dome_fn(r=0.7, h=0.6), depth=0.15), WOOD)])
    ic.marks.append(arrow(30, 18, 44, 4, ACC, 3, 7))
    return ic


@icon("modeling/import_component_3d_model")
def _():
    ic = Icon(fit=(0.24, 0.3, 1.0, 1.0)).add([(shape(dome_fn()), BODY)])
    ic.marks.append(arrow(4, 3, 18, 17, ACC, 3, 7))
    return ic


@icon("file/open_clipart_folder")
def _():
    ic = Icon(fit=(0.3, 0.34, 0.86, 0.84)).add([(shape(dome_fn()), BODY)])
    ic.under.append(folder(3, 8, 42, 34))
    return ic


@icon("layers/component_visible")
def _():
    ic = Icon(fit=(0.0, 0.02, 0.8, 0.8)).add([(shape(dome_fn()), BODY)])
    ic.marks.append(eye(37, 39, 0.45))
    return ic


@icon("layers/component_hidden")
def _():
    ic = Icon(fit=(0.0, 0.02, 0.8, 0.8)).add([(shape(dome_fn()), PALE)], 0.55)
    ic.marks.append(eye(37, 39, 0.45, GREY, closed=True))
    return ic


@icon("modeling/extrude_and_weave")
def _():
    xs = np.linspace(-1, 1, 60)
    a = np.stack([xs, -0.25 + 0 * xs, 0.25 + 0.18 * np.sin(xs * math.pi * 1.5)], -1)
    b = np.stack([xs, 0.25 + 0 * xs, 0.25 - 0.18 * np.sin(xs * math.pi * 1.5)], -1)
    return Icon().add([(tube(a, 0.15), BODY), (tube(b, 0.15), "#9fb8e0")])


@icon("modeling/offset_model")
def _():
    ic = Icon(bounds=((-0.95, -0.95, 0), (0.95, 0.95, 0.85)))
    ic.add([(shape(dome_fn(r=0.65, h=0.55)), BODY)])
    ic.add([(shape(dome_fn(r=0.92, h=0.82)), "#9fb8e0")], 0.4)
    ring = ring3(0, 0, 0.92, 0.0)
    ic.marks.append(stroke3(ic, ring, ACC, 1.8, "3 2.5"))
    return ic


@icon("modeling/replace_below")
def _():
    ic = Icon(fit=(0.06, 0.04, 0.94, 0.96))
    ic.add([(box(-0.8, -0.8, 0, 0.8, 0.8, 0.35), "#9fb8e0"), (shift(shape(dome_fn(r=0.8, h=0.6)), dz=0.35), BODY)])
    sq = [(-0.8, -0.8), (0.8, -0.8), (0.8, 0.8)]
    ic.marks.append(stroke3(ic, [(x, y, 0.35) for x, y in sq], ACC, 2.2, "3 2.5"))
    return ic


def reset(inside):
    bumps = lambda X, Y: 0.07 * (np.sin(X * 11) * np.sin(Y * 11) + 1)
    hole = lambda X, Y: np.hypot(X, Y) < 0.55
    fn = (lambda X, Y: np.where(hole(X, Y), 0, bumps(X, Y))) if inside else (lambda X, Y: np.where(hole(X, Y), bumps(X, Y), 0))
    ic = Icon().add([(on_slab(fn, n=110, depth=0.2), WOOD)])
    ic.marks.append(stroke3(ic, ring3(0, 0, 0.58, 0.21), ACC, 2.2))
    return ic


I["modeling/reset_inside_selected_vectors"] = lambda: reset(True)
I["modeling/reset_outside_selected_vectors"] = lambda: reset(False)


@icon("modeling/sculpting")
def _():
    fn = pair(bump_fn(-0.2, 0.1, 0.3, 0.5), bump_fn(0.35, -0.25, 0.22, 0.3))
    ic = Icon(fit=(0.0, 0.2, 0.84, 1.0)).add([(on_slab(fn, depth=0.18), WOOD)])
    ic.marks.append(pencil(25, 23, 0.72))
    return ic


@icon("modeling/slice_model")
def _():
    fn = dome_fn(r=0.85, h=0.95)
    solids = []
    for k, (a, b) in enumerate(((0, 0.32), (0.32, 0.64), (0.64, 0.95))):
        t = grid(lambda X, Y, b=b: np.minimum(fn(X, Y), b), -0.9, 0.9, -0.9, 0.9, n=64, base=a, keep_flat=False)
        solids.append((shift(t, dz=k * 0.16), BODY if k != 1 else "#9fb8e0"))
    return Icon().add(solids)


@icon("modeling/split_selected_component")
def _():
    fn = dome_fn(r=0.8, h=0.75)
    left = grid(fn, -0.85, 0, -0.85, 0.85, n=48, keep_flat=False)
    right = grid(fn, 0, 0.85, -0.85, 0.85, n=48, keep_flat=False)
    ic = Icon().add([(shift(left, dx=-0.18), BODY), (shift(right, dx=0.18), BODY)])
    cut = [(0, -0.95, 0), (0, 0.95, 0.85)]
    ic.include(cut)
    ic.marks.append(stroke3(ic, cut, ACC, 2.2, "3 2.5"))
    return ic


@icon("modeling/two_rail_sweep")
def _():
    w = lambda X: 0.45 + 0.3 * np.cos(X * math.pi * 0.6)
    fn = lambda X, Y: 0.6 * np.sqrt(np.clip(1 - (Y / w(X)) ** 2, 0, 1)) * (0.75 + 0.25 * np.cos(X * 2.2))
    ic = Icon().add([(grid(fn, -1, 1, -0.8, 0.8, n=80, keep_flat=False), BODY)])
    xs = np.linspace(-1, 1, 40)
    for sgn in (1, -1):
        ic.marks.append(stroke3(ic, np.stack([xs, sgn * w(xs), 0 * xs], -1), ACC, 2.4))
    return ic


@icon("toolpaths/3d_finishing_toolpath")
def _():
    fn = dome_fn(r=0.75, h=0.6)
    top = lambda X, Y: 0.18 + fn(X, Y)
    ic = Icon().add([(on_slab(fn, depth=0.18), WOOD)])
    xs = np.linspace(-0.95, 0.95, 50)
    for y in np.linspace(-0.75, 0.75, 6):
        ic.marks.append(stroke3(ic, surface_line(top, np.stack([xs, np.full_like(xs, y)], -1)), TP, 1.1))
    return ic


@icon("toolpaths/3d_roughing_toolpath")
def _():
    fn = lambda X, Y: np.floor(dome_fn(r=0.8, h=0.7)(X, Y) / 0.175) * 0.175
    ic = Icon().add([(on_slab(fn, n=120, depth=0.18), WOOD)])
    for k in range(1, 4):
        z = 0.18 + k * 0.175
        r = 0.8 * math.sqrt(1 - (k * 0.175 / 0.7) ** 2)
        ic.marks.append(stroke3(ic, ring3(0, 0, r, z + 0.01), TP, 1.6))
    return ic


def main():
    out = sys.argv[1]
    only = set(sys.argv[2:])
    for name, fn in sorted(I.items()):
        if only and name not in only:
            continue
        dest = os.path.join(out, name + ".svg")
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        fn().svg(dest)
    print(f"{len(only) or len(I)} 3D icons written to {out}")


if __name__ == "__main__":
    main()
