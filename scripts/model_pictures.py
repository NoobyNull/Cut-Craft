#!/usr/bin/env python3
"""The pictures drawn from STL models: the Initial Orientation hand, and the import dialog's
Model Size and Position pictures round a statue. The models are not kept in the repository.

usage: python3 -I scripts/model_pictures.py HAND.stl STATUE.stl ICON_DIR [SIZE]
writes ICON_DIR/import/{pointing_hand,model_size_xyz,position_relative_to_modeling_plane}.png"""
import base64, math, os, subprocess, sys
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stlrender

INK, RED, GREEN, ACC, GREY = "#2b63c6", "#e04646", "#16a05a", "#f28c28", "#8a93a3"
WOOD, WOODD, BODY = "#f1d6a8", "#b8874c", "#7aa6e4"
AZ, EL = -35, 28

hand_model, model, icons = sys.argv[1], sys.argv[2], sys.argv[3]
size = int(sys.argv[4]) if len(sys.argv) > 4 else 256
out = os.path.join(icons, "import")
os.makedirs(out, exist_ok=True)

# the hand points up, so which face is up reads at a glance; seen from the front and a little above
hand, _ = stlrender.render(stlrender.load(hand_model), size, 35, 22, BODY, jump_share=0.04)
stlrender.save_png(hand, os.path.join(out, "pointing_hand.png"))

tri = stlrender.load(model)
flat = tri.reshape(-1, 3)
lo, hi = flat.min(0), flat.max(0)
u = size / 256  # stroke widths and offsets scale with the picture


def png_uri(rgba):
    path = os.path.join(out, "_tmp.png")
    stlrender.save_png(rgba, path)
    return "data:image/png;base64," + base64.b64encode(open(path, "rb").read()).decode()


def darrow(p, q, c, w=3.2, head=9):
    p, q = np.array(p, float), np.array(q, float)
    d = (q - p) / np.linalg.norm(q - p)
    n = np.array([-d[1], d[0]])
    h = head * u
    def tip(at, back):
        a, b = at - back * d * h + n * h * 0.55, at - back * d * h - n * h * 0.55
        return f'<path d="M{at[0]:.1f},{at[1]:.1f} L{a[0]:.1f},{a[1]:.1f} L{b[0]:.1f},{b[1]:.1f} Z" fill="{c}"/>'
    s, e = p + d * h * 0.9, q - d * h * 0.9
    return (f'<line x1="{s[0]:.1f}" y1="{s[1]:.1f}" x2="{e[0]:.1f}" y2="{e[1]:.1f}" stroke="{c}" stroke-width="{w * u:.1f}" stroke-linecap="round"/>'
            + tip(q, 1) + tip(p, -1))


def away(p, q, centre, by):
    """The segment pq moved `by` pixels away from `centre`, square to itself."""
    p, q = np.array(p), np.array(q)
    d = (q - p) / np.linalg.norm(q - p)
    n = np.array([-d[1], d[0]])
    if np.dot((p + q) / 2 - centre, n) < 0:
        n = -n
    return p + n * by * u, q + n * by * u


def corner(ix, iy, iz):
    return [(lo, hi)[ix][0], (lo, hi)[iy][1], (lo, hi)[iz][2]]


def svg(body, name):
    doc = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{size}" height="{size}">{body}</svg>'
    src = os.path.join(out, name + ".svg")
    open(src, "w").write(doc)
    subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), src, "-o", os.path.join(out, name + ".png")], check=True)
    os.remove(src)


# Model Size: the model in its dashed box, with its X, Y and Z extents
rgba, project = stlrender.render(tri, size, AZ, EL, BODY, fit=(0.2, 0.05, 0.78, 0.8), jump_share=0.08)
P = {(i, j, k): project([corner(i, j, k)])[0] for i in (0, 1) for j in (0, 1) for k in (0, 1)}
centre = np.mean(list(P.values()), 0)
edges = [((0, j, k), (1, j, k)) for j in (0, 1) for k in (0, 1)] + [((i, 0, k), (i, 1, k)) for i in (0, 1) for k in (0, 1)] + [((i, j, 0), (i, j, 1)) for i in (0, 1) for j in (0, 1)]
box = "".join(f'<line x1="{P[a][0]:.1f}" y1="{P[a][1]:.1f}" x2="{P[b][0]:.1f}" y2="{P[b][1]:.1f}" stroke="{GREY}" stroke-width="{1.6 * u:.1f}" stroke-dasharray="{5 * u:.1f} {4 * u:.1f}"/>' for a, b in edges)
# the bottom corner nearest the viewer, and the edges that leave it
front = max(((i, j, 0) for i in (0, 1) for j in (0, 1)), key=lambda c: P[c][1])
fx, fy = (1 - front[0], front[1], 0), (front[0], 1 - front[1], 0)
side = max(((i, j, 0) for i in (0, 1) for j in (0, 1)), key=lambda c: P[c][0])
arrows = (darrow(*away(P[front], P[fx], centre, 16), RED) + darrow(*away(P[front], P[fy], centre, 16), INK)
          + darrow(*away(P[side], P[(side[0], side[1], 1)], centre, 16), GREEN))
svg(box + f'<image href="{png_uri(rgba)}" x="0" y="0" width="{size}" height="{size}"/>' + arrows, "model_size_xyz")

# Position relative to the modeling plane: the plane cuts through the model; above it the model stands
# in front of the plane, below it shows through. The arrow marks how far the base sits below the plane.
CUT = 0.35  # the plane's height, as a share of the model's
fit = (0.12, 0.04, 0.76, 0.84)
h = lo[2] + (hi[2] - lo[2]) * CUT
whole, project = stlrender.render(tri, size, AZ, EL, BODY, fit=fit, jump_share=0.08, bounds=(lo, hi))
upper, _ = stlrender.render(tri[tri[:, :, 2].mean(1) > h], size, AZ, EL, BODY, fit=fit, jump_share=0.08, bounds=(lo, hi))
grow = np.array([2.2, 2.2, 1.0])
mid = (lo + hi) / 2
plane = [project([(np.array([*corner(i, j, 0)[:2], h]) - mid) * grow + mid])[0] for i, j in ((0, 0), (1, 0), (1, 1), (0, 1))]
pd = "M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in plane) + " Z"
slab = f'<path d="{pd}" fill="{WOOD}" fill-opacity="0.72" stroke="{WOODD}" stroke-width="{2.5 * u:.1f}" stroke-linejoin="round"/>'
right = max(plane, key=lambda p: p[0])
base = project([[mid[0], mid[1], lo[2]]])[0]
level = project([[mid[0], mid[1], h]])[0]
x = right[0] + 6 * u
arrow = darrow((x, level[1]), (x, base[1]), ACC)
image = lambda rgba: f'<image href="{png_uri(rgba)}" x="0" y="0" width="{size}" height="{size}"/>'
svg(image(whole) + slab + image(upper) + arrow, "position_relative_to_modeling_plane")
os.remove(os.path.join(out, "_tmp.png"))
