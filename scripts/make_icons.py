#!/usr/bin/env python3
"""Cut Craft's icons, drawn from scratch: every icon is built here from a few shared shapes in
one style, so the whole set can be redrawn or restyled in one place.
usage: python3 -I scripts/make_icons.py OUT_DIR   (OUT_DIR gets the same tree as assets/icons)
Stdlib only. Icons are 48 x 48 SVGs; the dialog pictures are the same, drawn larger by the app."""
import math, os, sys

# ---- style -------------------------------------------------------------------------------
INK = "#2b63c6"      # vectors and outlines
LIGHT = "#dbe8fb"    # fills behind vectors
ACC = "#f28c28"      # what the tool does: the new part, the arrow, the highlight
TP = "#16a05a"       # toolpaths
TPL = "#c9efd9"
RED = "#e04646"
GREY = "#8a93a3"
DARK = "#3b4352"
WOOD = "#f1d6a8"     # material
WOODD = "#b8874c"
MODEL = "#9ec0ee"    # 3D components
MODELD = "#5b8fd8"
WHITE = "#ffffff"
SW = 3               # the stroke width every icon shares


def attrs(**a):
    out = []
    for k, v in a.items():
        if v is None:
            continue
        out.append(f'{k.rstrip("_").replace("_", "-")}="{v}"')
    return " ".join(out)


def stroke(c, w, dash, fill="none", cap="round"):
    return dict(stroke=c, stroke_width=w, fill=fill, stroke_linecap=cap,
                stroke_linejoin="round", stroke_dasharray=dash)


def line(x1, y1, x2, y2, c=INK, w=SW, dash=None):
    return f"<line {attrs(x1=x1, y1=y1, x2=x2, y2=y2, **stroke(c, w, dash))}/>"


def pl(pts, c=INK, w=SW, fill="none", closed=False, dash=None):
    p = " ".join(f"{x:.2f},{y:.2f}" for x, y in pts)
    tag = "polygon" if closed else "polyline"
    return f"<{tag} {attrs(points=p, **stroke(c, w, dash, fill))}/>"


def rect(x, y, w, h, rx=0, c=INK, sw=SW, fill="none", dash=None):
    return f"<rect {attrs(x=x, y=y, width=w, height=h, rx=rx or None, **stroke(c, sw, dash, fill))}/>"


def circ(cx, cy, r, c=INK, sw=SW, fill="none", dash=None):
    return f"<circle {attrs(cx=cx, cy=cy, r=r, **stroke(c, sw, dash, fill))}/>"


def ell(cx, cy, rx, ry, c=INK, sw=SW, fill="none", dash=None):
    return f"<ellipse {attrs(cx=cx, cy=cy, rx=rx, ry=ry, **stroke(c, sw, dash, fill))}/>"


def path(d, c=INK, sw=SW, fill="none", dash=None):
    return f"<path {attrs(d=d, **stroke(c, sw, dash, fill))}/>"


def solid(d, fill):
    return f'<path d="{d}" fill="{fill}"/>'


def dot(x, y, r=3, c=ACC):
    return f'<circle cx="{x}" cy="{y}" r="{r}" fill="{c}"/>'


def node(x, y, s=4.5, c=INK):
    return f'<rect x="{x - s / 2:.2f}" y="{y - s / 2:.2f}" width="{s}" height="{s}" fill="{WHITE}" stroke="{c}" stroke-width="2"/>'


def scissors(c=DARK):
    """Open scissors as a solid silhouette in a 48 box, tilted: two long slim blades opening unevenly
    up and to the right, their handles crossing through the pivot to oval rings below and to the left."""
    px, py = 24, 22
    out = []
    # (blade tip, ring centre): each blade's handle runs on through the pivot to its ring
    for tip, ring in (((35, 2), (19, 38)), ((45, 13), (8, 29))):
        dx, dy = tip[0] - px, tip[1] - py
        n = math.hypot(dx, dy)
        ux, uy, nx, ny = dx / n, dy / n, -dy / n, dx / n
        # a slim blade: full width at the pivot, a gently curved back, a straight cutting edge
        w = 3.0
        bx, by = px + nx * w - ux * 3, py + ny * w - uy * 3
        mx, my = px + dx * 0.5 + nx * w * 0.9, py + dy * 0.5 + ny * w * 0.9
        ex, ey = px - nx * w * 0.5 - ux * 3, py - ny * w * 0.5 - uy * 3
        out.append(solid(f"M{bx:.2f},{by:.2f} Q{mx:.2f},{my:.2f} {tip[0]},{tip[1]} L{ex:.2f},{ey:.2f} Z", c))
        rx, ry = ring
        hx, hy = rx - px, ry - py
        hn = math.hypot(hx, hy)
        k = 5.2 / hn
        out.append(line(px, py, rx - hx * k, ry - hy * k, c, 3.4))
        angle = math.degrees(math.atan2(hy, hx))
        out.append(f'<ellipse cx="{rx}" cy="{ry}" rx="6.4" ry="4.8" fill="none" stroke="{c}" stroke-width="3.2" transform="rotate({angle:.1f} {rx} {ry})"/>')
    out.append(dot(px, py, 1.4, WHITE))
    return out

def group(els, t):
    return f'<g transform="{t}">' + "".join(els) + "</g>"


def arrow(x1, y1, x2, y2, c=ACC, w=SW, head=7):
    a = math.atan2(y2 - y1, x2 - x1)
    bx, by = x2 - math.cos(a) * head * 0.8, y2 - math.sin(a) * head * 0.8
    l = (x2 - math.cos(a) * head + math.cos(a + math.pi / 2) * head * 0.6,
         y2 - math.sin(a) * head + math.sin(a + math.pi / 2) * head * 0.6)
    r = (x2 - math.cos(a) * head - math.cos(a + math.pi / 2) * head * 0.6,
         y2 - math.sin(a) * head - math.sin(a + math.pi / 2) * head * 0.6)
    return line(x1, y1, bx, by, c, w) + f'<polygon points="{x2},{y2} {l[0]:.2f},{l[1]:.2f} {r[0]:.2f},{r[1]:.2f}" fill="{c}"/>'


def darrow(x1, y1, x2, y2, c=ACC, w=2.5, head=6):
    """A dimension: arrowheads at both ends."""
    mx, my = (x1 + x2) / 2, (y1 + y2) / 2
    return arrow(mx, my, x1, y1, c, w, head) + arrow(mx, my, x2, y2, c, w, head)


def curved_arrow(cx, cy, r, a0, a1, c=ACC, w=SW, head=7):
    """An arc from angle a0 to a1 (degrees, clockwise on screen) ending in a head."""
    p0 = (cx + r * math.cos(math.radians(a0)), cy + r * math.sin(math.radians(a0)))
    p1 = (cx + r * math.cos(math.radians(a1)), cy + r * math.sin(math.radians(a1)))
    large = 1 if abs(a1 - a0) > 180 else 0
    sweep = 1 if a1 > a0 else 0
    tang = math.radians(a1 + (90 if sweep else -90))
    tip = (p1[0] + math.cos(tang) * 2, p1[1] + math.sin(tang) * 2)
    return (path(f"M{p0[0]:.2f},{p0[1]:.2f} A{r},{r} 0 {large} {sweep} {p1[0]:.2f},{p1[1]:.2f}", c, w)
            + arrow(p1[0] - math.cos(tang) * 0.1, p1[1] - math.sin(tang) * 0.1, tip[0], tip[1], c, w, head))


# ---- a tiny stroke font for the few letters icons need ---------------------------------------
GLYPHS = {
    "T": [[(0, 0), (1, 0)], [(0.5, 0), (0.5, 1)]],
    "A": [[(0, 1), (0.5, 0), (1, 1)], [(0.22, 0.62), (0.78, 0.62)]],
    "D": [[(0, 0), (0, 1), (0.55, 1), (0.9, 0.75), (1, 0.5), (0.9, 0.25), (0.55, 0), (0, 0)]],
    "2": [[(0.05, 0.25), (0.25, 0.02), (0.75, 0.02), (0.95, 0.25), (0.9, 0.45), (0.05, 1), (1, 1)]],
    "3": [[(0.05, 0.08), (0.8, 0.02), (0.95, 0.25), (0.75, 0.48), (0.35, 0.48)],
          [(0.75, 0.48), (1, 0.72), (0.85, 0.98), (0.05, 0.95)]],
    "X": [[(0, 0), (1, 1)], [(1, 0), (0, 1)]],
    "Y": [[(0, 0), (0.5, 0.5), (1, 0)], [(0.5, 0.5), (0.5, 1)]],
    "Z": [[(0, 0), (1, 0), (0, 1), (1, 1)]],
    "S": [[(1, 0.12), (0.7, 0), (0.25, 0), (0, 0.22), (0.15, 0.45), (0.85, 0.55), (1, 0.78), (0.75, 1), (0.25, 1), (0, 0.88)]],
    "L": [[(0, 0), (0, 1), (1, 1)]],
    "1": [[(0.2, 0.2), (0.6, 0), (0.6, 1)], [(0.2, 1), (1, 1)]],
}


def text(s, x, y, h, c=INK, w=SW, gap=0.35):
    """`s` in the stroke font, `h` tall with its top left at (x, y)."""
    out, cx = [], x
    for ch in s:
        wd = h * 0.62
        for stroke_ in GLYPHS[ch]:
            out.append(pl([(cx + px * wd, y + py * h) for px, py in stroke_], c, w))
        cx += wd + h * gap
    return "".join(out)


# ---- shared pictures ---------------------------------------------------------------------
def material(x=6, y=10, w=36, h=28):
    return rect(x, y, w, h, 2, WOODD, 2, WOOD)


def slab(x=6, y=22, w=36, h=10, d=8):
    """The material seen from the front and above."""
    top = [(x, y), (x + d, y - d), (x + w + d, y - d), (x + w, y)]
    side = [(x + w, y), (x + w + d, y - d), (x + w + d, y - d + h), (x + w, y + h)]
    return (pl(top, WOODD, 2, WOOD, True) + pl(side, WOODD, 2, "#e2bd83", True)
            + rect(x, y, w, h, 0, WOODD, 2, "#e8c793"))


def dome(cx=24, base=36, w=32, h=20, fill=MODEL, c=INK):
    """A 3D component: a dome standing on its base."""
    r = w / 2
    return (path(f"M{cx - r},{base} C{cx - r},{base - h * 1.3} {cx + r},{base - h * 1.3} {cx + r},{base} Z", c, SW, fill)
            + path(f"M{cx - r * 0.55},{base - h * 0.45} C{cx - r * 0.45},{base - h * 0.85} {cx - r * 0.1},{base - h * 0.95} {cx + r * 0.1},{base - h * 0.95}", WHITE, 2.5))


def plane(y=38, x0=4, x1=44):
    return line(x0, y, x1, y, GREY, 2.5)


def page(x=10, y=5, w=26, h=36, fold=8, fill=WHITE, c=DARK):
    return path(f"M{x},{y} H{x + w - fold} L{x + w},{y + fold} V{y + h} H{x} Z M{x + w - fold},{y} V{y + fold} H{x + w}", c, 2.5, fill)


def folder(x=4, y=10, w=40, h=30, c=WOODD, fill="#f7c873"):
    return (path(f"M{x},{y + 4} V{y + h} H{x + w} V{y + 8} H{x + w / 2.5} L{x + w / 2.5 - 4},{y + 2} H{x} Z", c, 2.5, fill)
            + path(f"M{x},{y + 12} H{x + w}", c, 2))


def floppy(x=7, y=7, s=34, c=INK, fill=LIGHT):
    return (path(f"M{x},{y} H{x + s - 7} L{x + s},{y + 7} V{y + s} H{x} Z", c, SW, fill)
            + rect(x + 7, y, s - 16, 10, 0, c, 2.5, WHITE)
            + rect(x + 6, y + s - 14, s - 12, 14, 1, c, 2.5, WHITE))


def pencil(x, y, s=1.0, c=ACC):
    """A pencil writing at (x, y), held up and to the right."""
    pts = [(0, 0), (4, -9), (19, -24), (24, -19), (9, -4)]
    pts = [(x + px * s, y + py * s) for px, py in pts]
    return (pl(pts, DARK, 2, c, True)
            + pl([(x, y), (x + 4 * s, y - 9 * s), (x + 9 * s, y - 4 * s)], DARK, 2, WOOD, True)
            + line(x + 4 * s, y - 9 * s, x + 9 * s, y - 4 * s, DARK, 2))


def eye(cx=24, cy=24, s=1.0, c=INK, closed=False):
    w, h = 18 * s, 10 * s
    if closed:
        return (path(f"M{cx - w},{cy} Q{cx},{cy + h * 1.6} {cx + w},{cy}", c, SW)
                + "".join(line(cx + dx * s, cy + h * 0.75, cx + dx * s * 1.15, cy + h * 1.25, c, 2.5) for dx in (-11, 0, 11)))
    return (path(f"M{cx - w},{cy} Q{cx},{cy - h * 1.6} {cx + w},{cy} Q{cx},{cy + h * 1.6} {cx - w},{cy} Z", c, SW, WHITE)
            + circ(cx, cy, 6 * s, c, SW, LIGHT) + dot(cx, cy, 2.6 * s, c))


def lock(cx=24, cy=27, open_=False, c=INK):
    shackle = (f"M{cx - 8},{cy - 4} V{cy - 11} A8,8 0 0 1 {cx + 8},{cy - 11} V{cy - 8}" if open_
               else f"M{cx - 8},{cy - 4} V{cy - 11} A8,8 0 0 1 {cx + 8},{cy - 11} V{cy - 4}")
    return path(shackle, c, SW) + rect(cx - 12, cy - 4, 24, 18, 3, c, SW, LIGHT) + dot(cx, cy + 5, 2.5, c)


def magnifier(cx=20, cy=20, r=12, c=DARK):
    return circ(cx, cy, r, c, SW, WHITE) + line(cx + r * 0.72, cy + r * 0.72, cx + r * 1.65, cy + r * 1.65, c, 5)


def picture(x=6, y=9, w=36, h=30, c=INK):
    return (rect(x, y, w, h, 2, c, 2.5, "#e9f2ff")
            + solid(f"M{x + 2},{y + h - 2} L{x + w * 0.35},{y + h * 0.45} L{x + w * 0.55},{y + h * 0.72} L{x + w * 0.7},{y + h * 0.55} L{x + w - 2},{y + h - 2} Z", "#7fb07a")
            + dot(x + w * 0.75, y + h * 0.3, 3.5, "#f2b632"))


def clipboard(x=9, y=6, w=30, h=36):
    return (rect(x, y + 3, w, h - 3, 3, WOODD, 2.5, WOOD) + rect(x + 4, y + 8, w - 8, h - 12, 1, DARK, 1.5, WHITE)
            + rect(x + w / 2 - 7, y, 14, 7, 2, DARK, 2, GREY))


def bit(kind, x=24, top=6, bottom=42, r=8, c=DARK):
    """A cutter in side view: its shank, then the cutting end."""
    shank = rect(x - r * 0.55, top, r * 1.1, (bottom - top) * 0.45, 1, c, 2, "#c9ced8")
    y0 = top + (bottom - top) * 0.45
    if kind == "end_mill":
        cut = path(f"M{x - r},{y0} H{x + r} V{bottom} H{x - r} Z", c, 2.5, "#aab3c2")
    elif kind == "ball_nose":
        cut = path(f"M{x - r},{y0} H{x + r} V{bottom - r} A{r},{r} 0 0 1 {x - r},{bottom - r} Z", c, 2.5, "#aab3c2")
    elif kind == "radiused":
        k = r * 0.45
        cut = path(f"M{x - r},{y0} H{x + r} V{bottom - k} Q{x + r},{bottom} {x + r - k},{bottom} H{x - r + k} Q{x - r},{bottom} {x - r},{bottom - k} Z", c, 2.5, "#aab3c2")
    else:  # v_bit
        cut = path(f"M{x - r},{y0} H{x + r} V{y0 + 3} L{x},{bottom} L{x - r},{y0 + 3} Z", c, 2.5, "#aab3c2")
    flutes = line(x - r * 0.5, y0 + 4, x + r * 0.4, y0 + (bottom - y0) * 0.55, c, 1.5)
    return shank + cut + flutes


def zigzag(x, y, w, h, n=5, c=TP, sw=2.5):
    pts = []
    for i in range(n):
        yy = y + h * i / (n - 1)
        pts += [(x, yy), (x + w, yy)] if i % 2 == 0 else [(x + w, yy), (x, yy)]
    return pl(pts, c, sw)


def cross(cx, cy, s=7, c=RED, w=4):
    return line(cx - s, cy - s, cx + s, cy + s, c, w) + line(cx + s, cy - s, cx - s, cy + s, c, w)


def tick(x, y, s=1.0, c=TP):
    return pl([(x - 7 * s, y), (x - 2 * s, y + 5 * s), (x + 8 * s, y - 6 * s)], c, 4)


def plus(cx, cy, s=7, c=ACC, w=4):
    return line(cx - s, cy, cx + s, cy, c, w) + line(cx, cy - s, cx, cy + s, c, w)


def badge(cx, cy, r, fill, els=""):
    return circ(cx, cy, r, WHITE, 2, fill) + els


def star_pts(cx, cy, ro, ri, n=5, rot=-90):
    pts = []
    for i in range(2 * n):
        r = ro if i % 2 == 0 else ri
        a = math.radians(rot + i * 180 / n)
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def poly_pts(cx, cy, r, n=6, rot=-90):
    return [(cx + r * math.cos(math.radians(rot + i * 360 / n)), cy + r * math.sin(math.radians(rot + i * 360 / n))) for i in range(n)]


def bars(horizontal, ref, lengths, offsets, c=INK, fill=LIGHT):
    """Align icons: bars lined up against a reference."""
    out = []
    for (a, b), off in zip(lengths, offsets):
        if horizontal:
            out.append(rect(a, off, b - a, 9, 1.5, c, 2.5, fill))
        else:
            out.append(rect(off, a, 9, b - a, 1.5, c, 2.5, fill))
    return out


def greyed(svg):
    for colour in (INK, ACC, TP, MODELD, RED):
        svg = svg.replace(colour, GREY)
    for colour in (LIGHT, TPL, MODEL):
        svg = svg.replace(colour, "#e6e8ec")
    return svg


# ---- the icons ---------------------------------------------------------------------------
I = {}


def icon(name):
    def put(fn):
        I[name] = fn
        return fn
    return put


# common
I["common/add.svg"] = lambda: [badge(24, 24, 19, TP), plus(24, 24, 10, WHITE, 5)]
I["common/add_tool.svg"] = lambda: [bit("end_mill", 18, 5, 43, 7), badge(35, 33, 10, TP, plus(35, 33, 5, WHITE, 3.5))]
I["common/copy_duplicate.svg"] = lambda: [rect(6, 6, 24, 24, 3, INK, SW, LIGHT), rect(18, 18, 24, 24, 3, ACC, SW, "#ffe9d2")]
I["common/delete.svg"] = lambda: [path("M12,14 L14,42 H34 L36,14 Z", RED, SW, "#fde2e2"), line(8, 12, 40, 12, RED), path("M19,12 V7 H29 V12", RED, 2.5),
                                  line(20, 20, 21, 36, RED, 2.5), line(28, 20, 27, 36, RED, 2.5)]
I["common/edit.svg"] = lambda: [line(8, 41, 30, 41, GREY, 2.5), pencil(8, 40, 1.35)]
I["common/error_close.svg"] = lambda: [badge(24, 24, 19, RED), cross(24, 24, 8, WHITE, 5)]
I["common/materials_appearance.svg"] = lambda: [path("M24,5 C38,5 44,15 42,24 C40,32 32,29 30,34 C28,40 34,43 24,43 C12,43 5,34 5,24 C5,13 13,5 24,5 Z", WOODD, 2.5, WOOD),
                                                dot(15, 18, 4, RED), dot(25, 13, 4, "#f2b632"), dot(34, 19, 4, TP), dot(14, 30, 4, INK)]
I["common/open.svg"] = lambda: [folder(4, 9, 40, 31), path("M8,40 L14,24 H46 L40,40 Z", WOODD, 2.5, "#fbd98e")]
I["common/reset_undo.svg"] = lambda: [path("M12,16 A15,15 0 1 1 9,29", ACC, SW), pl([(5, 9), (12, 17), (20, 11)], ACC, SW)]
I["common/save.svg"] = lambda: [floppy()]
I["common/tool_database.svg"] = lambda: [rect(4, 30, 40, 13, 2, WOODD, 2.5, WOOD), bit("end_mill", 12, 5, 32, 4.5), bit("ball_nose", 24, 5, 32, 4.5), bit("v_bit", 36, 5, 32, 4.5)]


# align: bars against a reference line or the material
def align_icon(side, material_=False):
    els = [material(4, 4, 40, 40)] if material_ else []
    g = ACC
    if side == "left":
        els += [line(8, 5, 8, 43, g)] + bars(True, 8, [(8, 38), (8, 26)], [12, 27])
    elif side == "right":
        els += [line(40, 5, 40, 43, g)] + bars(True, 40, [(10, 40), (22, 40)], [12, 27])
    elif side == "top":
        els += [line(5, 8, 43, 8, g)] + bars(False, 8, [(8, 38), (8, 26)], [12, 27])
    elif side == "bottom":
        els += [line(5, 40, 43, 40, g)] + bars(False, 40, [(10, 40), (22, 40)], [12, 27])
    elif side == "h":
        els += bars(True, 24, [(9, 39), (15, 33)], [11, 28]) + [line(24, 5, 24, 43, g, 2.5, "4 3")]
    elif side == "v":
        els += bars(False, 24, [(9, 39), (15, 33)], [11, 28]) + [line(5, 24, 43, 24, g, 2.5, "4 3")]
    elif side == "c":
        els += [rect(13, 13, 22, 22, 2, INK, 2.5, LIGHT), rect(18, 18, 12, 12, 1.5, INK, 2.5, WHITE),
                line(24, 4, 24, 44, g, 2.5, "4 3"), line(4, 24, 44, 24, g, 2.5, "4 3")]
    return els


for side, name in [("left", "left"), ("right", "right"), ("top", "top"), ("bottom", "bottom"),
                   ("h", "center_horizontally"), ("v", "center_vertically"), ("c", "center")]:
    I[f"drawing/align/{name}.svg"] = (lambda s=side: align_icon(s))
for side, name in [("h", "center_horizontally_in_material"), ("v", "center_vertically_in_material"), ("c", "center_in_material")]:
    I[f"drawing/align/{name}.svg"] = (lambda s=side: align_icon(s, True))


def spacing(direction):
    boxes = []
    if direction in ("lr", "rl"):
        for i, x in enumerate((5, 19, 33)):
            boxes.append(rect(x, 22, 10, 10, 1.5, INK, 2.5, LIGHT if i else "#ffe9d2"))
        a = arrow(8, 13, 40, 13) if direction == "lr" else arrow(40, 13, 8, 13)
    else:
        for i, y in enumerate((5, 19, 33)):
            boxes.append(rect(22, y, 10, 10, 1.5, INK, 2.5, LIGHT if i else "#ffe9d2"))
        a = arrow(13, 8, 13, 40) if direction == "tb" else arrow(13, 40, 13, 8)
    return boxes + [a]


I["drawing/align/left_to_right.svg"] = lambda: spacing("lr")
I["drawing/align/right_to_left.svg"] = lambda: spacing("rl")
I["drawing/align/top_to_bottom.svg"] = lambda: spacing("tb")
I["drawing/align/bottom_to_top.svg"] = lambda: spacing("bt")

# create vectors
cv = "drawing/create_vectors/"
I[cv + "draw_circle.svg"] = lambda: [circ(24, 24, 17, INK, SW, LIGHT), node(24, 24, 4.5, ACC), node(41, 24)]
I[cv + "draw_ellipse.svg"] = lambda: [ell(24, 24, 19, 12, INK, SW, LIGHT), node(24, 24, 4.5, ACC)]
I[cv + "draw_rectangle.svg"] = lambda: [rect(6, 11, 36, 26, 0, INK, SW, LIGHT), node(6, 11), node(42, 37, 4.5, ACC)]
I[cv + "draw_polygon.svg"] = lambda: [pl(poly_pts(24, 24, 18), INK, SW, LIGHT, True)] + [node(x, y) for x, y in poly_pts(24, 24, 18)]
I[cv + "draw_star.svg"] = lambda: [pl(star_pts(24, 25, 19, 8), INK, SW, LIGHT, True)]
I[cv + "draw_arc.svg"] = lambda: [path("M7,38 A19,19 0 0 1 41,30", INK, SW), node(7, 38), node(41, 30), node(22, 15, 4.5, ACC)]
I[cv + "draw_line_polyline.svg"] = lambda: [pl([(6, 38), (16, 12), (30, 30), (42, 8)], INK, SW)] + [node(x, y) for x, y in [(6, 38), (16, 12), (30, 30), (42, 8)]]
I[cv + "draw_curve.svg"] = lambda: [path("M6,38 C10,6 36,42 42,10", INK, SW), line(6, 38, 10, 6, GREY, 1.5, "3 2"), line(42, 10, 36, 42, GREY, 1.5, "3 2"),
                                    node(6, 38), node(42, 10), dot(10, 6, 2.5, ACC), dot(36, 42, 2.5, ACC)]
I[cv + "draw_freehand.svg"] = lambda: [path("M5,30 C9,22 13,36 18,28 S25,18 28,26 S35,34 38,24", INK, SW), pencil(36, 26, 0.85)]
I[cv + "draw_text.svg"] = lambda: [text("T", 11, 7, 32, INK, 5), line(6, 43, 42, 43, ACC, 2.5)]
I[cv + "edit_text.svg"] = lambda: [text("T", 5, 6, 28, INK, 4.5), pencil(23, 42, 0.95)]
I[cv + "convert_text_to_curves.svg"] = lambda: [pl([(8, 42), (24, 6), (40, 42), (33, 42), (29, 33), (19, 33), (15, 42)], INK, 2.5, LIGHT, True),
                                                pl([(21, 27), (24, 19), (27, 27)], INK, 2.5, WHITE, True)] + [node(x, y, 4, ACC) for x, y in [(8, 42), (24, 6), (40, 42), (15, 42), (33, 42)]]
I[cv + "auto_layout_text.svg"] = lambda: [rect(5, 5, 38, 38, 2, ACC, 2.5, None, "4 3"), text("TA", 9, 10, 12, INK, 3), text("AT", 9, 26, 12, INK, 3)]
I[cv + "text_on_a_curve.svg"] = lambda: [path("M5,40 Q24,8 43,40", GREY, 2, dash="3 3")] + [
    group([text("T", -4, -12, 10, INK, 3)], f"translate({x},{y}) rotate({r})") for x, y, r in [(11, 30, -45), (24, 22, 0), (37, 30, 45)]]
I[cv + "array_copy.svg"] = lambda: [rect(5 + 14 * c, 8 + 14 * r, 10, 10, 1.5, INK if (r, c) == (0, 0) else ACC, 2.5, LIGHT if (r, c) == (0, 0) else "#ffe9d2") for r in range(3) for c in range(3) if r < 2 or True][:9]
I[cv + "circular_copy.svg"] = lambda: [circ(24, 24, 15, GREY, 1.5, dash="3 3")] + [circ(24 + 15 * math.cos(math.radians(a)), 24 + 15 * math.sin(math.radians(a)), 4.5, INK if a == -90 else ACC, 2.5, LIGHT if a == -90 else "#ffe9d2") for a in range(-90, 270, 60)]
# Two open ends run on at an angle until they cross: the crossing point is where they get closed.
I[cv + "close_extend_vector.svg"] = lambda: [
    line(6, 42, 16, 26), line(42, 42, 32, 26),
    line(16, 26, 24.1, 13, ACC, 2.5, "3 3"), line(32, 26, 23.9, 13, ACC, 2.5, "3 3"),
    line(24.1, 13, 27.9, 7, ACC, 1.5), line(23.9, 13, 20.1, 7, ACC, 1.5),
    dot(24, 13, 3.2), node(16, 26), node(32, 26)]
I[cv + "copy_along_vectors.svg"] = lambda: [path("M5,38 C14,6 32,44 43,10", GREY, 2, dash="3 3")] + [rect(x - 4, y - 4, 8, 8, 1, ACC, 2.5, "#ffe9d2") for x, y in [(8, 30), (17, 20), (26, 25), (35, 26), (42, 14)]]
I[cv + "create_fillets.svg"] = lambda: [pl([(8, 6), (8, 40), (42, 40)], GREY, 2, dash="3 3"), path("M8,6 V24 A16,16 0 0 0 24,40 H42", INK, SW), path("M8,24 A16,16 0 0 0 24,40", ACC, 4)]
I[cv + "dimensions.svg"] = lambda: [rect(8, 22, 32, 18, 0, INK, 2.5, LIGHT), line(8, 8, 8, 20, GREY, 1.5), line(40, 8, 40, 20, GREY, 1.5), darrow(9, 13, 39, 13)]
I[cv + "distort_object.svg"] = lambda: [path("M6,8 Q24,2 42,8 Q48,24 42,40 Q24,46 6,40 Q0,24 6,8 Z", INK, SW, LIGHT),
                                        path("M18,6 Q13,24 18,42 M30,6 Q35,24 30,42 M5,19 Q24,13 43,19 M5,30 Q24,36 43,30", INK, 1.5)] + [node(x, y, 4, ACC) for x, y in [(6, 8), (42, 8), (42, 40), (6, 40)]]
I[cv + "document_variables.svg"] = lambda: [path("M16,6 Q9,6 9,13 V19 Q9,24 4,24 Q9,24 9,29 V35 Q9,42 16,42", INK, SW), path("M32,6 Q39,6 39,13 V19 Q39,24 44,24 Q39,24 39,29 V35 Q39,42 32,42", INK, SW), text("X", 18, 17, 14, ACC, 3.5)]
FIT_PTS = [(5, 41), (9, 5), (25, 5), (30, 42), (42, 42), (44, 8)]
# The straight-sided vector stays solid with its corners marked; the fitted curve rides over it.
I[cv + "fit_curves_to_vectors.svg"] = lambda: [
    pl(FIT_PTS, INK, 2.5), *(node(x, y, 4) for x, y in FIT_PTS),
    path("M5,41 C9,10 21,4 27,22 S39,40 44,8", ACC, 2.5)]
I[cv + "group.svg"] = lambda: [rect(4, 4, 40, 40, 4, ACC, 2.5, None, "5 3"), rect(10, 10, 15, 15, 1.5, INK, 2.5, LIGHT), circ(30, 30, 8, INK, 2.5, LIGHT)]
I[cv + "measure_inspect.svg"] = lambda: [group([rect(-21, -6, 42, 12, 2, WOODD, 2.5, "#fbe3a9")] + [line(x, -6, x, -6 + (6 if x % 10 == 0 else 3), WOODD, 1.5) for x in range(-16, 20, 5)], "translate(24,24) rotate(-45)")]
I[cv + "rounded_shape_span.svg"] = lambda: [path("M15,14 H33 A10,10 0 0 1 33,34 H15 A10,10 0 0 1 15,14 Z", INK, SW, LIGHT), path("M33,14 A10,10 0 0 1 33,34", ACC, 4)]
I[cv + "shapes.svg"] = lambda: [circ(15, 15, 9, INK, 2.5, LIGHT), rect(26, 6, 17, 17, 1.5, INK, 2.5, LIGHT), pl([(6, 42), (15, 27), (24, 42)], INK, 2.5, LIGHT, True), pl(star_pts(34, 35, 9, 4), ACC, 2.5, "#ffe9d2", True)]
I[cv + "trace_bitmap.svg"] = lambda: [rect(6 + 6 * c, 6 + 6 * r, 6, 6, 0, None, 0, "#9aa6b8" if (r - 2.5) ** 2 + (c - 2.5) ** 2 < 7 else "#e3e7ee") for r in range(6) for c in range(6)] + [circ(24, 24, 13, ACC, 3)]
I[cv + "vector_boundary.svg"] = lambda: [rect(10, 12, 12, 12, 1.5, INK, 2.5, LIGHT), circ(31, 30, 8, INK, 2.5, LIGHT), path("M7,9 H25 L42,24 V38 Q42,41 39,41 H10 Q7,41 7,38 Z", ACC, 2.5, dash="4 3")]
I[cv + "vector_texture.svg"] = lambda: [rect(5, 5, 38, 38, 2, INK, 2.5, LIGHT)] + [path(f"M8,{y} q5,-4 10,0 t10,0 t10,0", INK, 2) for y in (13, 21, 29, 37)]
I[cv + "vector_validator.svg"] = lambda: [pl(poly_pts(20, 22, 15), INK, SW, LIGHT, True), badge(35, 35, 10, TP, tick(35, 35, 0.75, WHITE))]

# edit objects
eo = "drawing/edit_objects/"
I[eo + "select.svg"] = lambda: [path("M12,5 V38 L20,30 L26,43 L32,40 L26,28 H37 Z", DARK, 2.5, WHITE)]
I[eo + "node_edit.svg"] = lambda: [path("M5,36 C10,10 30,10 36,30", INK, SW), node(5, 36), node(36, 30), node(20, 16, 5, ACC), line(12, 18, 28, 14, ACC, 1.5),
                                   group([path("M0,0 V18 L5,13 L9,21 L13,19 L9,11 H16 Z", DARK, 2, WHITE)], "translate(28,24)")]
I[eo + "transform.svg"] = lambda: [rect(10, 10, 28, 28, 0, INK, 2.5, LIGHT, "4 3")] + [rect(x - 3, y - 3, 6, 6, 0, ACC, 2, WHITE) for x in (10, 24, 38) for y in (10, 24, 38) if (x, y) != (24, 24)] + [dot(24, 24, 2.5, ACC)]
I[eo + "move.svg"] = lambda: [arrow(24, 24, 24, 5), arrow(24, 24, 24, 43), arrow(24, 24, 5, 24), arrow(24, 24, 43, 24), dot(24, 24, 4, DARK)]
I[eo + "rotate.svg"] = lambda: [rect(14, 14, 20, 20, 1.5, INK, 2.5, LIGHT), path("M8,18 A17,17 0 0 1 36,9", ACC, SW), pl([(30, 4), (37, 9), (31, 15)], ACC, SW)]
I[eo + "mirror.svg"] = lambda: [line(24, 4, 24, 44, GREY, 2, "4 3"), pl([(20, 10), (20, 38), (6, 38)], INK, 2.5, LIGHT, True), pl([(28, 10), (28, 38), (42, 38)], ACC, 2.5, "#ffe9d2", True)]
I[eo + "set_size.svg"] = lambda: [rect(6, 14, 26, 26, 1.5, INK, 2.5, LIGHT), darrow(6, 8, 32, 8), darrow(40, 14, 40, 40)]
I[eo + "job_size_and_position.svg"] = lambda: [material(6, 10, 36, 30), dot(6, 40, 4, ACC), darrow(6, 5, 42, 5, INK), darrow(46, 10, 46, 40, INK)]
I[eo + "interactive_vector_trim.svg"] = lambda: [line(6, 14, 42, 14, INK, SW), line(24, 6, 24, 42, INK, SW), line(24, 14, 42, 14, RED, 4, "3 3"),
                                                 group(scissors(), "translate(20.9 21) scale(0.58) rotate(-60 24 22)")]
I[eo + "weld_cross.svg"] = lambda: [pl([(18, 6), (30, 6), (30, 18), (42, 18), (42, 30), (30, 30), (30, 42), (18, 42), (18, 30), (6, 30), (6, 18), (18, 18)], INK, SW, LIGHT, True)]
I[eo + "crop_bitmap.svg"] = lambda: [picture(8, 8, 32, 26), pl([(4, 12), (12, 12), (12, 38), (38, 38)], ACC, 4), pl([(10, 4), (36, 4), (36, 30), (44, 30)], ACC, 4)][::1]
I[eo + "edit_picture.svg"] = lambda: [picture(4, 4, 34, 28), line(8, 41, 42, 41, GREY, 2.5), circ(26, 41, 4, ACC, 2.5, WHITE)]

# join
I["drawing/join/join_straight_line.svg"] = lambda: [path("M5,10 Q14,28 17,34", INK, SW), path("M43,10 Q34,28 31,34", INK, SW), line(17, 34, 31, 34, ACC, 4), node(17, 34), node(31, 34)]
I["drawing/join/join_smooth_curve.svg"] = lambda: [path("M5,10 Q12,24 16,30", INK, SW), path("M43,10 Q36,24 32,30", INK, SW), path("M16,30 Q24,44 32,30", ACC, 4), node(16, 30), node(32, 30)]
I["drawing/join/join_move_end_points.svg"] = lambda: [path("M5,10 Q14,28 24,34", INK, SW), path("M43,10 Q36,24 31,30", GREY, 2, dash="3 3"), path("M43,10 Q34,28 24,34", INK, SW), node(24, 34, 5, ACC)]

# weld and offset
wo = "drawing/weld_and_offset/"
I[wo + "weld_vectors.svg"] = lambda: [
    solid("M18,32 A12,12 0 1 1 26,12 A12,12 0 1 1 18,32 Z", LIGHT), circ(17, 20, 12, INK, SW, LIGHT), circ(30, 28, 12, INK, SW, LIGHT), solid("M17,20 m-9.5,0 a9.5,9.5 0 1,0 19,0 a9.5,9.5 0 1,0 -19,0", LIGHT),
    solid("M30,28 m-9.5,0 a9.5,9.5 0 1,0 19,0 a9.5,9.5 0 1,0 -19,0", LIGHT)]
I[wo + "subtract_vectors.svg"] = lambda: [circ(18, 20, 13, INK, SW, LIGHT), circ(31, 29, 12, ACC, 2.5, WHITE, "4 3")]
I[wo + "overlap_intersect.svg"] = lambda: [circ(18, 21, 12, GREY, 2), circ(30, 27, 12, GREY, 2),
                                           path("M24.0,12.9 A12,12 0 0 1 24.0,35.1 A12,12 0 0 1 24.0,12.9 Z", INK, SW, LIGHT)]
I[wo + "offset_vectors.svg"] = lambda: [rect(14, 14, 20, 20, 2, INK, SW, LIGHT), rect(6, 6, 36, 36, 9, ACC, 2.5, None, "4 3")]
I[wo + "vector_texture_fill.svg"] = lambda: [circ(24, 24, 18, INK, SW, LIGHT)] + [line(10 + i * 6, 12 + abs(i - 2) * 1.5, 10 + i * 6, 36 - abs(i - 2) * 1.5, ACC, 2.5) for i in range(5)]

# dialog pictures (were Aspire's bitmaps): grey shape, the dimension in orange
df = "drawing_forms/"
I[df + "circle_center.svg"] = lambda: [circ(24, 24, 16, GREY, 2.5, "#f1f3f6"), line(17, 24, 31, 24, ACC, 2.5), line(24, 17, 24, 31, ACC, 2.5), dot(24, 24, 3.5)]
I[df + "circle_radius.svg"] = lambda: [circ(24, 24, 16, GREY, 2.5, "#f1f3f6"), dot(24, 24, 2.5, GREY), arrow(24, 24, 40, 24)]
I[df + "width.svg"] = lambda: [rect(8, 16, 32, 24, 0, GREY, 2.5, "#f1f3f6"), darrow(8, 9, 40, 9)]
I[df + "height.svg"] = lambda: [rect(8, 8, 24, 32, 0, GREY, 2.5, "#f1f3f6"), darrow(39, 8, 39, 40)]
I[df + "corner_square.svg"] = lambda: [path("M8,40 V8 H40", GREY, 2.5), path("M8,22 V8 H22", ACC, 4)]
I[df + "corner_rounded.svg"] = lambda: [path("M8,40 V22 A14,14 0 0 1 22,8 H40", GREY, 2.5), path("M8,24 V22 A14,14 0 0 1 22,8 H24", ACC, 4)]
I[df + "corner_notched.svg"] = lambda: [path("M8,40 V22 H22 V8 H40", GREY, 2.5), path("M8,24 V22 H22 V8 H24", ACC, 4)]
I[df + "polygon_center.svg"] = lambda: [pl(poly_pts(24, 24, 17), GREY, 2.5, "#f1f3f6", True), line(18, 24, 30, 24, ACC, 2.5), line(24, 18, 24, 30, ACC, 2.5), dot(24, 24, 3.5)]
I[df + "polygon_radius.svg"] = lambda: [pl(poly_pts(24, 24, 17), GREY, 2.5, "#f1f3f6", True), dot(24, 24, 2.5, GREY), arrow(24, 24, 24, 7)]
I[df + "star_center.svg"] = lambda: [pl(star_pts(24, 25, 19, 8), GREY, 2.5, "#f1f3f6", True), line(19, 25, 29, 25, ACC, 2.5), line(24, 20, 24, 30, ACC, 2.5), dot(24, 25, 3.5)]
I[df + "star_outer_radius.svg"] = lambda: [pl(star_pts(24, 25, 19, 8), GREY, 2.5, "#f1f3f6", True), dot(24, 25, 2.5, GREY), arrow(24, 25, 24, 6)]
I[df + "star_inner_radius.svg"] = lambda: [pl(star_pts(24, 25, 19, 8), GREY, 2.5, "#f1f3f6", True), dot(24, 25, 2.5, GREY),
                                           arrow(24, 25, 24 + 8 * math.cos(math.radians(-54)), 25 + 8 * math.sin(math.radians(-54)), ACC, 2.5, 5)]

# file
fl = "file/"
I[fl + "new.svg"] = lambda: [page(10, 5, 26, 36), badge(35, 35, 9, TP, plus(35, 35, 4.5, WHITE, 3))]
I[fl + "open.svg"] = I["common/open.svg"]
I[fl + "open_folder.svg"] = lambda: [folder(4, 8, 40, 32)]
I[fl + "open_clipart_folder.svg"] = lambda: [folder(4, 8, 40, 32), dome(24, 36, 18, 11)]
I[fl + "save.svg"] = lambda: [floppy()]
I[fl + "undo.svg"] = lambda: [path("M14,20 H30 A10,10 0 0 1 30,40 H18", ACC, SW), pl([(20, 12), (12, 20), (20, 28)], ACC, SW)]
I[fl + "redo.svg"] = lambda: [path("M34,20 H18 A10,10 0 0 0 18,40 H30", ACC, SW), pl([(28, 12), (36, 20), (28, 28)], ACC, SW)]
I[fl + "copy.svg"] = lambda: [page(6, 4, 22, 28, 6, LIGHT, INK), page(20, 16, 22, 28, 6, WHITE, INK)]
I[fl + "cut.svg"] = lambda: scissors()
I[fl + "paste.svg"] = lambda: [clipboard(), page(20, 18, 20, 26, 6, LIGHT, INK)]
I[fl + "paste_in_place.svg"] = lambda: [clipboard(), line(24, 20, 24, 38, ACC, 2.5), line(15, 29, 33, 29, ACC, 2.5), circ(24, 29, 5, ACC, 2.5)]
I[fl + "notes.svg"] = lambda: [rect(9, 6, 30, 36, 2, WOODD, 2.5, "#fff3c4")] + [line(14, y, 34, y, GREY, 2) for y in (16, 23, 30, 37)] + [line(9, 11, 39, 11, ACC, 3)]
I[fl + "job_setup.svg"] = lambda: [slab(4, 24, 30, 10, 9), darrow(4, 40, 34, 40, INK, 2), darrow(40, 22, 40, 32, INK, 2)]
I[fl + "import_vectors.svg"] = lambda: [pl(star_pts(30, 28, 13, 6), INK, 2.5, LIGHT, True), arrow(4, 8, 18, 22)]
I[fl + "export_vectors.svg"] = lambda: [pl(star_pts(18, 28, 13, 6), INK, 2.5, LIGHT, True), arrow(28, 20, 43, 5)]
I[fl + "import_vectors_bitmap.svg"] = lambda: [picture(14, 14, 30, 26), arrow(3, 4, 16, 17)]
I[fl + "export_as_3d_clipart.svg"] = lambda: [dome(18, 40, 28, 18), arrow(28, 18, 43, 4)]
I[fl + "facebook_link.svg"] = lambda: [rect(5, 5, 38, 38, 9, None, 0, "#1877f2"), path("M27,41 V25 H33 L34,19 H27 V15 Q27,12 30,12 H34 V7 H29 Q21,7 21,15 V19 H16 V25 H21 V41", WHITE, 0, WHITE)]
I[fl + "instagram_link.svg"] = lambda: [rect(6, 6, 36, 36, 10, "#d6249f", 4), circ(24, 24, 8, "#d6249f", 4), dot(34, 14, 2.5, "#d6249f")]
I[fl + "pinterest_link.svg"] = lambda: [badge(24, 24, 19, "#e60023"), path("M21,42 L24,28 M24,28 Q19,29 18,24 Q16,13 26,13 Q34,13 34,21 Q34,30 26,30 Q24,30 24,28", WHITE, 3.5)]
I[fl + "twitter_link.svg"] = lambda: [rect(5, 5, 38, 38, 9, None, 0, "#14171a"), line(14, 13, 34, 36, WHITE, 4), line(34, 13, 14, 36, WHITE, 2.5)]
I[fl + "youtube_link.svg"] = lambda: [rect(4, 10, 40, 28, 9, None, 0, "#ff0000"), pl([(20, 17), (32, 24), (20, 31)], None, 0, WHITE, True)]

# import (dialog pictures)
I["import/initial_orientation.svg"] = lambda: [arrow(14, 36, 14, 6, TP), arrow(14, 36, 42, 36, RED), arrow(14, 36, 34, 22, INK), dome(30, 34, 16, 10)]
I["import/model_size_xyz.svg"] = lambda: [slab(6, 20, 26, 16, 10), darrow(6, 42, 32, 42, RED, 2), darrow(44, 12, 44, 34, TP, 2), darrow(35, 42, 45, 32, INK, 2)]
I["import/position_relative_to_modeling_plane.svg"] = lambda: [dome(24, 32, 30, 22), solid("M4,30 H44 L40,38 H0 Z", "#f1d6a8cc"), path("M4,30 H44 L40,38 H0 Z", WOODD, 2), darrow(46, 10, 46, 30, ACC, 2)]

# job setup
js = "job_setup/"


def datum(cx, cy):
    return [material(6, 10, 36, 28), dot(cx, cy, 4.5)]


I[js + "xy_datum_bottom_left.svg"] = lambda: datum(6, 38)
I[js + "xy_datum_bottom_right.svg"] = lambda: datum(42, 38)
I[js + "xy_datum_top_left.svg"] = lambda: datum(6, 10)
I[js + "xy_datum_top_right.svg"] = lambda: datum(42, 10)
I[js + "xy_datum_centre.svg"] = lambda: datum(24, 24)
I[js + "z_zero_material_surface.svg"] = lambda: [line(2, 44, 46, 44, DARK, 2.5), slab(6, 26, 30, 14, 8), arrow(44, 6, 44, 17, ACC), line(36, 18, 47, 18, ACC, 2.5)]
I[js + "z_zero_machine_bed.svg"] = lambda: [line(2, 44, 46, 44, ACC, 4), slab(6, 26, 30, 14, 8), arrow(44, 28, 44, 41, ACC)]
I[js + "material_size_xyz.svg"] = lambda: [slab(5, 22, 28, 12, 10), darrow(5, 40, 33, 40, RED, 2), darrow(40, 22, 40, 34, TP, 2), darrow(5, 18, 15, 8, INK, 2)]
I[js + "single_sided_job.svg"] = lambda: [slab(5, 24, 32, 12, 9), solid("M5,24 L14,15 H46 L37,24 Z", "#ffd19a"), path("M5,24 L14,15 H46 L37,24 Z", ACC, 2.5)]
I[js + "double_sided_job.svg"] = lambda: [slab(5, 20, 32, 12, 9), path("M5,32 H37", ACC, 3), path("M5,20 L14,11 H46 L37,20 Z", ACC, 2.5), curved_arrow(42, 36, 6, 200, 450, INK, 2.5, 5)]
I[js + "rotary_job.svg"] = lambda: [path("M10,14 H36 A5,10 0 0 1 36,34 H10 A5,10 0 0 1 10,14 Z", WOODD, 2.5, WOOD), ell(10, 24, 5, 10, WOODD, 2.5, "#e2bd83"),
                                    line(2, 24, 46, 24, GREY, 1.5, "4 2"), curved_arrow(40, 24, 9, 290, 430, ACC, 2.5, 5)]


def gap_icon(label_z):
    tool = bit("end_mill", 24, 2, 18, 4.5)
    if label_z == 1:
        return [slab(4, 32, 32, 8, 6), tool, darrow(42, 19, 42, 31, ACC, 2)]
    return [slab(4, 32, 32, 8, 6), bit("end_mill", 24, 10, 26, 4.5), darrow(42, 27, 42, 31.5, ACC, 2), line(36, 26.5, 47, 26.5, ACC, 1.5, "2 2")]


I[js + "clearance_z1.svg"] = lambda: gap_icon(1)
I[js + "plunge_z2.svg"] = lambda: gap_icon(2)

# layers
ly = "layers/"
I[ly + "visible.svg"] = lambda: [eye(24, 24, 1.05)]
I[ly + "hidden.svg"] = lambda: [eye(24, 22, 1.05, GREY, True)]
I[ly + "locked.svg"] = lambda: [lock()]
I[ly + "unlocked.svg"] = lambda: [lock(open_=True, c=GREY)]
I[ly + "bitmap_visible.svg"] = lambda: [picture(4, 6, 34, 28), badge(36, 37, 10, WHITE, eye(36, 37, 0.42))]
I[ly + "bitmap_hidden.svg"] = lambda: [greyed(picture(4, 6, 34, 28)), badge(36, 37, 10, WHITE, eye(36, 35, 0.42, GREY, True))]
I[ly + "component_visible.svg"] = lambda: [dome(20, 32, 32, 20), badge(36, 37, 10, WHITE, eye(36, 37, 0.42))]
I[ly + "component_hidden.svg"] = lambda: [greyed(dome(20, 32, 32, 20)), badge(36, 37, 10, WHITE, eye(36, 35, 0.42, GREY, True))]

# modeling
md = "modeling/"


def profile(kind, x0=6, x1=42, base=38, h=24, c=INK, fill=MODEL):
    w = x1 - x0
    if kind == "round":
        d = f"M{x0},{base} A{w / 2},{h} 0 0 1 {x1},{base} Z"
    elif kind == "angled":
        d = f"M{x0},{base} L{x0 + w / 2},{base - h} L{x1},{base} Z"
    elif kind == "flat":
        d = f"M{x0},{base} L{x0 + 7},{base - h * 0.7} H{x1 - 7} L{x1},{base} Z"
    else:  # smooth
        d = f"M{x0},{base} C{x0 + w * 0.35},{base} {x0 + w * 0.3},{base - h} {x0 + w / 2},{base - h} S{x1 - w * 0.35},{base} {x1},{base} Z"
    return path(d, c, SW, fill)


I[md + "create_shape.svg"] = lambda: [plane(40), profile("smooth", 6, 42, 38, 24), pencil(34, 16, 0.6)]
for k in ("round", "angled", "flat", "smooth"):
    I[md + f"create_shape_{k}.svg"] = (lambda k=k: [plane(40), profile(k, 6, 42, 38, 26)])
I[md + "add_zero_plane.svg"] = lambda: [path("M4,30 L14,18 H44 L34,30 Z", INK, 2.5, LIGHT), plus(38, 38, 6, ACC, 3.5)]
I[md + "add_draft_to_model.svg"] = lambda: [plane(40), path("M8,38 L16,12 H32 L40,38 Z", INK, SW, MODEL), line(16, 12, 16, 38, ACC, 2, "3 2"), line(32, 12, 32, 38, ACC, 2, "3 2")]
I[md + "bake_components.svg"] = lambda: [dome(16, 30, 20, 14), dome(30, 30, 20, 14), arrow(24, 32, 24, 42, ACC, 2.5, 6)]
I[md + "component_levels.svg"] = lambda: [path(f"M4,{y} L14,{y - 8} H44 L34,{y} Z", INK, 2.5, c) for y, c in ((40, LIGHT), (30, MODEL), (20, "#ffe9d2"))]
I[md + "create_component_from_bitmap.svg"] = lambda: [picture(3, 4, 22, 18), arrow(14, 25, 22, 32, ACC, 2.5, 6), dome(32, 44, 26, 16)]
I[md + "create_component_from_visible_model.svg"] = lambda: [eye(16, 12, 0.55), dome(26, 42, 34, 22)]
I[md + "create_cross_section_vector.svg"] = lambda: [plane(42), profile("round", 6, 42, 40, 26, GREY, "#e6e8ec"), path("M6,40 A18,26 0 0 1 42,40", ACC, 4)]
I[md + "create_texture_area.svg"] = lambda: [rect(5, 5, 38, 38, 2, ACC, 2.5, None, "4 3")] + [circ(13 + 11 * c, 13 + 11 * r, 4, MODELD, 2, MODEL) for r in range(3) for c in range(3)]
I[md + "create_vector_boundary_around_selected_components.svg"] = lambda: [dome(24, 34, 30, 18), rect(4, 8, 40, 32, 5, ACC, 2.5, None, "4 3")]
I[md + "emboss_component.svg"] = lambda: [plane(40), path("M6,38 C12,26 18,34 24,26 S36,30 42,38 Z", INK, SW, MODEL), arrow(24, 6, 24, 20, ACC, 2.5, 6)]
I[md + "export_as_stl.svg"] = lambda: [pl([(6, 40), (18, 14), (30, 40)], INK, 2.5, MODEL, True), line(18, 14, 18, 40, INK, 1.5), line(12, 27, 24, 27, INK, 1.5), arrow(28, 20, 43, 5)]
I[md + "extrude_and_weave.svg"] = lambda: [path("M4,30 C12,14 20,14 24,24 S36,34 44,18", INK, 7), path("M4,30 C12,14 20,14 24,24 S36,34 44,18", MODEL, 3)]
I[md + "import_component_3d_model.svg"] = lambda: [dome(30, 42, 28, 20), arrow(4, 6, 18, 20)]
I[md + "offset_model.svg"] = lambda: [plane(42), path("M4,40 A20,28 0 0 1 44,40", ACC, 2.5, dash="4 3"), profile("round", 10, 38, 40, 20)]
I[md + "replace_below.svg"] = lambda: [plane(42), profile("round", 6, 42, 40, 30), solid("M5,40 V30 H43 V40 Z", "#ffffffcc"), line(3, 30, 45, 30, ACC, 2.5, "4 3")]
I[md + "reset_inside_selected_vectors.svg"] = lambda: [dome(24, 38, 38, 26), circ(24, 26, 7, ACC, 2.5, WHITE, "3 2")]
I[md + "reset_outside_selected_vectors.svg"] = lambda: [rect(3, 6, 42, 36, 2, ACC, 2.5, WHITE, "3 2"), circ(24, 26, 11, INK, SW, MODEL)]
I[md + "scale_model_z_height.svg"] = lambda: [plane(42), profile("round", 4, 32, 40, 18), darrow(40, 8, 40, 38)]
I[md + "sculpting.svg"] = lambda: [plane(42), path("M4,40 C10,24 16,34 22,24 S34,28 40,40 Z", INK, SW, MODEL),
                                   group([rect(-3, -16, 6, 16, 1, DARK, 2, WOOD), path("M-4,0 H4 L2,7 Q0,10 -2,7 Z", DARK, 2, ACC)], "translate(34,16) rotate(30)")]
I[md + "slice_model.svg"] = lambda: [plane(42), profile("round", 6, 42, 40, 30)] + [line(4, y, 44, y, ACC, 2, "4 2") for y in (18, 26, 34)]
I[md + "split_selected_component.svg"] = lambda: [plane(42), path("M6,40 A16,26 0 0 1 22,14 V40 Z", INK, SW, MODEL), path("M26,40 V14 A16,26 0 0 1 42,40 Z", INK, SW, MODEL), line(24, 6, 24, 44, ACC, 2.5, "4 3")]
I[md + "texture.svg"] = lambda: [path("M4,38 " + " ".join(f"Q{6 + i * 8},{24 if i % 2 else 30} {10 + i * 8},{30 - (i % 3)}" for i in range(5)) + " V42 H4 Z", INK, 2.5, MODEL)]
I[md + "turn_and_spin.svg"] = lambda: [line(24, 3, 24, 45, GREY, 1.5, "4 2"), path("M24,8 H30 Q34,14 30,20 Q38,30 32,40 H24", INK, SW), path("M24,8 H18 Q14,14 18,20 Q10,30 16,40 H24", MODELD, 2, dash="3 2"),
                                       curved_arrow(24, 44, 10, 200, 340, ACC, 2.5, 5)]
I[md + "two_rail_sweep.svg"] = lambda: [path("M4,14 Q24,4 44,14", INK, SW), path("M4,36 Q24,46 44,36", INK, SW), path("M24,9 Q34,24 24,41", ACC, 3), path("M10,13 Q16,24 10,35", MODELD, 2, dash="3 2"), path("M38,13 Q44,24 38,35", MODELD, 2, dash="3 2")]

# snapping (on in colour, off greyed)
sn = "snapping/"


def snap(kind):
    if kind == "grid":
        return [rect(5, 5, 38, 38, 0, GREY, 1.5)] + [line(5, v, 43, v, GREY, 1.5) for v in (17.7, 30.3)] + [line(v, 5, v, 43, GREY, 1.5) for v in (17.7, 30.3)] + [dot(30.3, 17.7, 4.5)]
    if kind == "geometry":
        return [path("M6,40 L20,12 L42,26", INK, SW), node(20, 12, 6, ACC), path("M30,40 A6,6 0 0 1 30,28 H36", DARK, 3), line(30, 34, 22, 20, ACC, 1.5, "2 2")]
    return [line(4, 30, 44, 30, ACC, 1.5, "3 3"), line(30, 4, 30, 44, ACC, 1.5, "3 3"), rect(6, 6, 14, 14, 1, INK, 2.5, LIGHT), dot(30, 30, 4.5, INK)]


for kind in ("grid", "geometry", "smart"):
    I[sn + f"{kind}_snapping_on.svg"] = (lambda k=kind: snap(k))
    I[sn + f"{kind}_snapping_off.svg"] = (lambda k=kind: [greyed("".join(snap(k)))])

# tool database (dialog pictures): the cutter, its diameter in orange
td = "tool_database/"
for k in ("end_mill", "ball_nose", "radiused_end_mill", "v_bit"):
    kind = {"radiused_end_mill": "radiused"}.get(k, k)
    I[td + f"{k}.svg"] = (lambda kind=kind: [bit(kind, 24, 3, 40, 10), darrow(14, 45, 34, 45, ACC, 2, 4)])

# toolpaths: green is the tool's path
tp = "toolpaths/"
I[tp + "profile_toolpath.svg"] = lambda: [rect(13, 13, 22, 22, 2, INK, 2.5, LIGHT), rect(6, 6, 36, 36, 8, TP, SW), dot(6, 24, 4, TP)]
I[tp + "pocket_toolpath.svg"] = lambda: [rect(6, 6, 36, 36, 2, INK, 2.5, LIGHT), rect(11, 11, 26, 26, 4, TP, 2.5), rect(16, 16, 16, 16, 3, TP, 2.5), rect(21, 21, 6, 6, 1, TP, 2.5)]
I[tp + "circular_pocket.svg"] = lambda: [circ(24, 24, 18, INK, 2.5, LIGHT), path("M24,24 m0,-3 a3,3 0 1,1 -3,3 a6,6 0 1,1 6,6 a9,9 0 1,1 -9,-9 a12,12 0 1,1 12,12", TP, 2.5)]
I[tp + "drilling_toolpath.svg"] = lambda: [material(4, 26, 40, 16)] + [circ(x, 34, 3.5, TP, 2.5, WHITE) for x in (12, 24, 36)] + [bit("end_mill", 24, 2, 22, 3.5), arrow(36, 6, 36, 20, TP, 2.5, 5)]
I[tp + "vcarve_engrave_toolpath.svg"] = lambda: [path("M4,14 H18 L24,32 L30,14 H44", WOODD, 2.5, WOOD), pl([(18, 14), (24, 32), (30, 14)], TP, 3)]
I[tp + "vcarve_inlay.svg"] = lambda: [path("M4,22 H14 L20,36 H28 L34,22 H44 V42 H4 Z", WOODD, 2.5, WOOD), path("M14,6 H34 L28,20 H20 Z", INK, 2.5, "#c97b45"), arrow(40, 6, 40, 18, TP, 2.5, 5)]
I[tp + "engraving.svg"] = lambda: [material(4, 8, 40, 32), text("A", 15, 14, 20, TP, 3)]
I[tp + "quick_engrave.svg"] = lambda: [material(4, 14, 40, 28), path("M10,30 Q18,20 24,30 T38,30", TP, 3), pl([(28, 2), (20, 14), (27, 14), (21, 25)], ACC, 2.5)]
I[tp + "fluting_toolpath.svg"] = lambda: [material(4, 6, 40, 36)] + [path(f"M{x},12 Q{x},10 {x + 2},10 Q{x + 4},10 {x + 4},12 V36 Q{x + 4},38 {x + 2},38 Q{x},38 {x},36 Z", TP, 2, TPL) for x in (11, 22, 33)]
I[tp + "chamfer_toolpath.svg"] = lambda: [path("M4,40 V20 L14,10 H44 V40 Z", WOODD, 2.5, WOOD), line(4, 20, 14, 10, TP, 4), bit("v_bit", 22, 0, 14, 3.5)]
I[tp + "rounding_toolpath.svg"] = lambda: [path("M4,40 V22 A12,12 0 0 1 16,10 H44 V40 Z", WOODD, 2.5, WOOD), path("M4,22 A12,12 0 0 1 16,10", TP, 4)]
I[tp + "keyhole_toolpath.svg"] = lambda: [material(4, 6, 40, 36), path("M17,20 A7,7 0 1 1 31,20 A7,7 0 1 1 17,20 Z", TP, 2.5, TPL), path("M21,26 L19,38 H29 L27,26", TP, 2.5, TPL)]
I[tp + "moulding_toolpath.svg"] = lambda: [path("M4,40 C14,40 18,28 24,28 C30,28 34,10 44,10", INK, 2.5), path("M4,40 C14,40 18,28 24,28 C30,28 34,10 44,10", TP, 2.5, dash="3 4"),
                                           path("M6,12 V22 Q10,22 10,18 Q14,18 14,12 Z", INK, 2, MODEL)]
I[tp + "prism_carving.svg"] = lambda: [path("M4,40 L14,12 H34 L44,40 Z", WOODD, 2.5, WOOD), line(14, 12, 24, 26, TP, 2.5), line(34, 12, 24, 26, TP, 2.5), line(4, 40, 24, 26, TP, 2.5), line(44, 40, 24, 26, TP, 2.5)]
I[tp + "3d_roughing_toolpath.svg"] = lambda: [path("M4,40 V34 H10 V28 H16 V22 H32 V28 H38 V34 H44 V40 Z", WOODD, 2.5, WOOD), path("M8,22 A16,16 0 0 1 40,22", INK, 2, dash="3 2"), pl([(10, 34), (10, 28), (16, 28), (16, 22), (32, 22), (32, 28), (38, 28), (38, 34)], TP, 2.5)]
I[tp + "3d_finishing_toolpath.svg"] = lambda: [plane(42), profile("round", 4, 44, 40, 30, INK, MODEL)] + [path(f"M{6 + i * 6},40 V{40 - math.sqrt(max(0, 1 - ((6 + i * 6 - 24) / 20) ** 2)) * 30 + 2}", TP, 2) for i in range(7)]
I[tp + "texture_toolpath.svg"] = lambda: [material(4, 6, 40, 36)] + [line(x, y, x + 6, y + 2, TP, 2.5) for x, y in ((8, 10), (20, 12), (32, 9), (12, 20), (26, 22), (34, 18), (8, 31), (19, 33), (31, 30), (12, 38))]
I[tp + "sketch_carving.svg"] = lambda: [material(4, 6, 40, 36), path("M8,32 L14,20 L18,28 L24,14 L28,26 L34,18 L40,30", TP, 2), path("M9,35 L15,23 L19,31 L25,17 L29,29 L35,21 L40,34", TP, 1.5)]
I[tp + "laser_cut_fill.svg"] = lambda: [material(4, 24, 40, 18), path("M20,2 L24,22 L28,2", RED, 0, "#ff6b6b55"), line(24, 4, 24, 24, RED, 2.5), dot(24, 26, 3.5, "#ffb020")]
I[tp + "laser_engrave_text.svg"] = lambda: [material(4, 24, 40, 18), line(36, 2, 36, 24, RED, 2.5), text("T", 10, 27, 12, DARK, 3), dot(36, 26, 3, "#ffb020")]
I[tp + "laser_picture.svg"] = lambda: [picture(4, 22, 30, 20), line(38, 2, 38, 24, RED, 2.5), dot(38, 26, 3, "#ffb020")]
I[tp + "array_copy_toolpath.svg"] = lambda: [rect(4 + 15 * c, 6 + 15 * r, 11, 11, 3, TP, 2.5, TPL if (r, c) != (0, 0) else WHITE) for r in range(3) for c in range(3)][:6] + [rect(4 + 15 * c, 36, 11, 7, 2, TP, 2, TPL) for c in range(3)][:0]
I[tp + "merge_toolpaths.svg"] = lambda: [path("M4,10 Q18,10 24,24", TP, SW), path("M4,38 Q18,38 24,24", TP, SW), arrow(24, 24, 44, 24, TP)]
I[tp + "preview_toolpaths.svg"] = lambda: [material(4, 6, 40, 36), path("M12,30 Q20,14 36,22", TP, 2.5, dash="3 2"), badge(33, 33, 10, ACC, pl([(30, 28), (38, 33), (30, 38)], None, 0, WHITE, True))]
I[tp + "recalculate_toolpath.svg"] = lambda: [path("M14,24 H34", TP, 3), path("M8,18 A17,17 0 0 1 38,14", ACC, SW), pl([(33, 8), (39, 15), (31, 18)], ACC, SW), path("M40,30 A17,17 0 0 1 10,34", ACC, SW), pl([(15, 40), (9, 33), (17, 30)], ACC, SW)]
I[tp + "edit_toolpath.svg"] = lambda: [path("M4,30 Q12,14 22,24 T40,18", TP, SW), pencil(16, 44, 0.95)]
I[tp + "delete_toolpath.svg"] = lambda: [path("M4,24 Q12,8 22,18 T42,12", TP, SW), badge(33, 34, 11, RED, cross(33, 34, 5, WHITE, 3.5))]
I[tp + "hide_toolpaths.svg"] = lambda: [path("M4,18 Q12,4 22,12 T42,8", "#9fd9b9", SW), eye(24, 30, 0.95, GREY, True)]
I[tp + "keep_out_zones.svg"] = lambda: [material(4, 6, 40, 36), rect(14, 14, 20, 20, 2, RED, 2.5, "#fde2e2")] + [line(14 + i * 5, 34, 14 + i * 5 + 20 if i == 0 else 34, 14 + i * 5 if i else 14, RED, 1.5) for i in range(0)] + [circ(24, 24, 7, RED, 2.5), line(19, 29, 29, 19, RED, 2.5)]
I[tp + "toolpath_tiling.svg"] = lambda: [rect(4 + 14 * c, 6 + 13 * r, 12, 11, 1, TP, 2, TPL) for r in range(3) for c in range(3)] + [line(16, 4, 16, 44, ACC, 1.5, "2 2"), line(30, 4, 30, 44, ACC, 1.5, "2 2")]
I[tp + "toolpath_summary.svg"] = lambda: [page(6, 4, 28, 38, 7)] + [line(11, y, 27, y, GREY, 2) for y in (16, 23, 30)] + [circ(36, 35, 9, DARK, 2.5, WHITE), line(36, 35, 36, 29, DARK, 2), line(36, 35, 40, 37, DARK, 2)]
I[tp + "create_job_sheet.svg"] = lambda: [page(8, 4, 30, 40, 7), rect(13, 12, 18, 12, 1, WOODD, 1.5, WOOD), path("M15,22 Q21,14 29,19", TP, 2)] + [line(13, y, 31, y, GREY, 2) for y in (30, 36)]
I[tp + "load_toolpath_template.svg"] = lambda: [page(14, 6, 28, 36, 7, TPL, TP), arrow(3, 6, 16, 19, ACC)]
I[tp + "save_toolpath_template.svg"] = lambda: [page(4, 4, 26, 34, 7, TPL, TP), floppy(26, 26, 18)]
I[tp + "save_all_as_template.svg"] = lambda: [page(4, 2, 22, 30, 6, TPL, TP), page(10, 8, 22, 30, 6, TPL, TP), floppy(26, 26, 18)]
I[tp + "save_toolpaths.svg"] = lambda: [floppy(), path("M12,32 Q20,26 24,32 T36,32", TP, 2.5)]
I[tp + "tool_database.svg"] = I["common/tool_database.svg"]

# view
vw = "view/"
I[vw + "2d_view.svg"] = lambda: [rect(4, 8, 40, 32, 2, INK, 2.5, LIGHT), text("2D", 11, 16, 16, INK, 3.5)]
I[vw + "3d_view.svg"] = lambda: [path("M24,4 L42,13 V35 L24,44 L6,35 V13 Z", INK, 2.5, LIGHT), path("M6,13 L24,22 L42,13 M24,22 V44", INK, 2.5), path("M24,4 L42,13 L24,22 L6,13 Z", INK, 2.5, MODEL)]
I[vw + "pan.svg"] = lambda: [path("M16,26 V12 A3,3 0 0 1 22,12 V24 V8 A3,3 0 0 1 28,8 V24 V11 A3,3 0 0 1 34,11 V26 V17 A3,3 0 0 1 40,17 V30 Q40,44 28,44 H24 Q17,44 13,37 L8,28 A3,3 0 0 1 13,25 Z", DARK, 2.5, WHITE)]
I[vw + "zoom_box.svg"] = lambda: [rect(4, 4, 24, 20, 0, ACC, 2.5, None, "4 3"), magnifier(26, 26, 10)]
I[vw + "zoom_selected.svg"] = lambda: [magnifier(20, 20, 14), pl(star_pts(20, 20, 9, 4), ACC, 2.5, "#ffe9d2", True)]
I[vw + "zoom_to_job.svg"] = lambda: [magnifier(20, 20, 14), rect(12, 14, 16, 12, 1, WOODD, 2, WOOD)]
I[vw + "layers.svg"] = lambda: [path(f"M24,{y} L44,{y + 9} L24,{y + 18} L4,{y + 9} Z", INK, 2.5, c) for y, c in ((20, LIGHT), (13, MODEL), (6, "#ffe9d2"))]
I[vw + "component_levels.svg"] = I[md + "component_levels.svg"]
I[vw + "expand.svg"] = lambda: [rect(6, 6, 36, 36, 4, DARK, 2.5, WHITE), plus(24, 24, 10, DARK, 4)]
I[vw + "collapse.svg"] = lambda: [rect(6, 6, 36, 36, 4, DARK, 2.5, WHITE), line(14, 24, 34, 24, DARK, 4)]
I[vw + "interface_layout.svg"] = lambda: [rect(4, 6, 40, 36, 2, DARK, 2.5, WHITE), rect(4, 6, 40, 7, 0, DARK, 2.5, DARK), rect(4, 13, 11, 29, 0, DARK, 2, LIGHT), rect(33, 13, 11, 29, 0, DARK, 2, TPL)]
I[vw + "split_side_by_side.svg"] = lambda: [rect(4, 6, 40, 36, 2, DARK, 2.5, WHITE), rect(4, 6, 20, 36, 0, DARK, 2, LIGHT), rect(24, 6, 20, 36, 0, DARK, 2, MODEL)]
I[vw + "split_over_and_under.svg"] = lambda: [rect(4, 6, 40, 36, 2, DARK, 2.5, WHITE), rect(4, 6, 40, 18, 0, DARK, 2, LIGHT), rect(4, 24, 40, 18, 0, DARK, 2, MODEL)]
I[vw + "2d_toolpath_drawing_on.svg"] = lambda: [rect(10, 10, 28, 28, 2, INK, 2.5, LIGHT), rect(5, 5, 38, 38, 8, TP, SW)]
I[vw + "2d_toolpath_drawing_off.svg"] = lambda: [greyed(rect(10, 10, 28, 28, 2, INK, 2.5, LIGHT) + rect(5, 5, 38, 38, 8, TP, SW))]
I[vw + "solid_2d_toolpath_drawing.svg"] = lambda: [path("M5,13 A8,8 0 0 1 13,5 H35 A8,8 0 0 1 43,13 V35 A8,8 0 0 1 35,43 H13 A8,8 0 0 1 5,35 Z M12,12 V36 H36 V12 Z", None, 0, "#7fd1a6"),
                                                   rect(12, 12, 24, 24, 1, INK, 2.5, LIGHT)]


def render(name):
    els = I[name]()
    body = "".join(e for e in els if e)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48">{body}</svg>\n'


# every icon the app ships: the SVGs it had, and its dialog pictures (once bitmaps) as SVGs
EXPECTED = None  # filled from assets/icons by main, so a missing drawing is an error


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else "assets/icons"
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "icons")
    wanted = set()
    for root, _, files in os.walk(here):
        for f in files:
            rel = os.path.relpath(os.path.join(root, f), here).replace(os.sep, "/")
            if f.endswith((".svg", ".bmp", ".png")):
                wanted.add(rel[:-4] + ".svg")
    # credited third-party icons, and pictures rendered from models (scripts/model_pictures.py)
    wanted -= {"file/github_link.svg", "file/thingiverse_link.svg", "import/pointing_hand.svg"}
    missing = sorted(wanted - set(I))
    extra = sorted(set(I) - wanted)
    if missing:
        sys.exit("no drawing for: " + ", ".join(missing))
    for name in sorted(I):
        dest = os.path.join(out, name)
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        with open(dest, "w") as fh:
            fh.write(render(name))
    print(f"{len(I)} icons written to {out}" + (f" ({len(extra)} new: {', '.join(extra)})" if extra else ""))


if __name__ == "__main__":
    main()
