import os
import sys
import math

I = {}

def rect(x, y, w, h, rx=0, s="black", sw=2, f="white", dash=None):
    r_attr = f'rx="{rx}" ry="{rx}" ' if rx else ""
    d_attr = f'stroke-dasharray="{dash}" ' if dash else ""
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" {r_attr}stroke="{s}" stroke-width="{sw}" fill="{f}" {d_attr}/>'

def path(d, s="black", sw=2, f="none", dash=None):
    d_attr = f'stroke-dasharray="{dash}" ' if dash else ""
    return f'<path d="{d}" stroke="{s}" stroke-width="{sw}" fill="{f}" {d_attr}/>'

def circ(cx, cy, r, s="black", sw=2, f="none", dash=None):
    d_attr = f'stroke-dasharray="{dash}" ' if dash else ""
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" stroke="{s}" stroke-width="{sw}" fill="{f}" {d_attr}/>'

def text(t, x, y, s="black", size=16, weight="normal"):
    return f'<text x="{x}" y="{y}" font-family="sans-serif" font-size="{size}" font-weight="{weight}" fill="{s}">{t}</text>'

# Toolpath icons
I["profile_toolpath.svg"] = lambda: [
    path("M10,10 H38 V38 H10 Z", "#f39c12", 4, "#fcf3cf"),
    path("M6,6 H42 V42 H6 Z", "#c0392b", 2, "none", "4,4")
]

I["pocket_toolpath.svg"] = lambda: [
    rect(8, 8, 32, 32, 4, "#2980b9", 2, "#d4e6f1"),
    path("M16,16 H32 V32 H16 Z", "#2980b9", 4, "#85c1e9")
]

I["drilling_toolpath.svg"] = lambda: [
    rect(6, 6, 36, 36, 2, "#7f8c8d", 2, "#ecf0f1"),
    circ(16, 16, 6, "#c0392b", 2, "#e74c3c"),
    circ(32, 16, 6, "#c0392b", 2, "#e74c3c"),
    circ(24, 32, 6, "#c0392b", 2, "#e74c3c")
]

I["vcarve_engrave_toolpath.svg"] = lambda: [
    path("M4,14 H18 L24,32 L30,14 H44", "#d35400", 2.5, "#e67e22"),
    path("M18,14 L24,32 L30,14", "#c0392b", 3, "none")
]

I["fluting_toolpath.svg"] = lambda: [
    path("M10,10 Q24,40 38,10", "#9b59b6", 4, "none"),
    path("M16,10 Q24,30 32,10", "#8e44ad", 3, "none"),
    path("M22,10 Q24,20 26,10", "#8e44ad", 2, "none")
]

I["texture_toolpath.svg"] = lambda: [
    path("M8,16 Q16,8 24,16 T40,16", "#16a085", 3, "none"),
    path("M8,24 Q16,16 24,24 T40,24", "#1abc9c", 3, "none"),
    path("M8,32 Q16,24 24,32 T40,32", "#1abc9c", 3, "none")
]

I["preview_toolpaths.svg"] = lambda: [
    path("M6,24 A18,18 0 0 1 42,24 A18,18 0 0 1 6,24", "#34495e", 4, "none"),
    circ(24, 24, 8, "#3498db", 0, "#3498db"),
    circ(24, 24, 4, "#2980b9", 0, "#2980b9")
]

I["save_toolpaths.svg"] = lambda: [
    rect(6, 6, 36, 36, 4, "#27ae60", 3, "#2ecc71"),
    path("M16,24 L24,32 L32,16", "#2c3e50", 4, "none")
]

# Editing icons
I["undo.svg"] = lambda: [
    path("M28,12 A16,16 0 0 0 12,28 A16,16 0 0 0 28,44", "#34495e", 4, "none"),
    path("M28,12 L20,4 M28,12 L20,20", "#34495e", 4, "none")
]

I["redo.svg"] = lambda: [
    path("M20,12 A16,16 0 0 1 36,28 A16,16 0 0 1 20,44", "#34495e", 4, "none"),
    path("M20,12 L28,4 M20,12 L28,20", "#34495e", 4, "none")
]

I["cut.svg"] = lambda: [
    path("M16,36 A6,6 0 1 1 28,36 A6,6 0 1 1 16,36 Z", "#c0392b", 3, "none"),
    path("M32,36 A6,6 0 1 0 44,36 A6,6 0 1 0 32,36 Z", "#c0392b", 3, "none"),
    path("M26,30 L10,8 M34,30 L40,8", "#7f8c8d", 4, "none"),
    circ(30, 24, 2, "#2c3e50", 2, "#2c3e50")
]

I["copy.svg"] = lambda: [
    rect(12, 12, 20, 24, 2, "#2980b9", 2, "#ecf0f1"),
    rect(20, 20, 20, 24, 2, "#2980b9", 2, "#d4e6f1")
]

I["paste.svg"] = lambda: [
    path("M12,12 H36 V40 H12 Z", "#f39c12", 2, "#fcf3cf"),
    rect(20, 6, 8, 8, 2, "#c0392b", 2, "#e74c3c"),
    path("M18,24 H30 M18,32 H30", "#34495e", 2, "none")
]

I["group_objects.svg"] = lambda: [
    rect(8, 8, 12, 12, 0, "#8e44ad", 2, "#e8daef"),
    rect(28, 8, 12, 12, 0, "#8e44ad", 2, "#e8daef"),
    rect(18, 28, 12, 12, 0, "#8e44ad", 2, "#e8daef"),
    rect(4, 4, 40, 40, 4, "#9b59b6", 2, "none", "4,4")
]

I["ungroup_objects.svg"] = lambda: [
    rect(8, 8, 12, 12, 0, "#3498db", 2, "#d6eaf8"),
    rect(28, 8, 12, 12, 0, "#3498db", 2, "#d6eaf8"),
    rect(18, 28, 12, 12, 0, "#3498db", 2, "#d6eaf8"),
    path("M6,6 L12,12 M42,6 L36,12 M24,42 L24,36", "#e74c3c", 3, "none")
]

I["align_left.svg"] = lambda: [
    path("M8,4 V44", "#e74c3c", 2, "none"),
    rect(14, 8, 24, 12, 2, "#3498db", 2, "#d6eaf8"),
    rect(14, 28, 16, 12, 2, "#3498db", 2, "#d6eaf8")
]

I["align_center_x.svg"] = lambda: [
    path("M24,4 V44", "#e74c3c", 2, "none"),
    rect(12, 8, 24, 12, 2, "#3498db", 2, "#d6eaf8"),
    rect(16, 28, 16, 12, 2, "#3498db", 2, "#d6eaf8")
]

I["align_right.svg"] = lambda: [
    path("M40,4 V44", "#e74c3c", 2, "none"),
    rect(10, 8, 24, 12, 2, "#3498db", 2, "#d6eaf8"),
    rect(18, 28, 16, 12, 2, "#3498db", 2, "#d6eaf8")
]

I["align_top.svg"] = lambda: [
    path("M4,8 H44", "#e74c3c", 2, "none"),
    rect(8, 14, 12, 24, 2, "#3498db", 2, "#d6eaf8"),
    rect(28, 14, 12, 16, 2, "#3498db", 2, "#d6eaf8")
]

I["align_center_y.svg"] = lambda: [
    path("M4,24 H44", "#e74c3c", 2, "none"),
    rect(8, 12, 12, 24, 2, "#3498db", 2, "#d6eaf8"),
    rect(28, 16, 12, 16, 2, "#3498db", 2, "#d6eaf8")
]

I["align_bottom.svg"] = lambda: [
    path("M4,40 H44", "#e74c3c", 2, "none"),
    rect(8, 10, 12, 24, 2, "#3498db", 2, "#d6eaf8"),
    rect(28, 18, 12, 16, 2, "#3498db", 2, "#d6eaf8")
]

I["align_center.svg"] = lambda: [
    path("M24,4 V44 M4,24 H44", "#e74c3c", 2, "none"),
    rect(12, 12, 24, 24, 2, "#3498db", 2, "#d6eaf8")
]

# Better 2D view icon
I["2d_view.svg"] = lambda: [
    rect(4, 8, 40, 32, 4, "#2c3e50", 3, "#ecf0f1"),
    path("M16,18 L16,28 L24,28", "#2c3e50", 3, "none"),
    path("M28,18 L28,28 Q36,28 36,23 Q36,18 28,18", "#2c3e50", 3, "none")
]

# Better 3D view icon
I["3d_view.svg"] = lambda: [
    path("M24,4 L42,13 V35 L24,44 L6,35 V13 Z", "#2c3e50", 3, "#ecf0f1"),
    path("M6,13 L24,22 L42,13 M24,22 V44", "#2c3e50", 3, "none"),
    path("M24,4 L42,13 L24,22 L6,13 Z", "#2980b9", 3, "#3498db")
]

I["split_side_by_side.svg"] = lambda: [
    rect(4, 6, 40, 36, 2, "#2c3e50", 3, "#ecf0f1"),
    path("M24,6 V42", "#2c3e50", 3, "none"),
    rect(8, 14, 12, 20, 1, "#3498db", 2, "#d6eaf8"),
    path("M34,14 L40,24 L34,34 Z", "#e74c3c", 2, "#fadbd8")
]

I["split_over_and_under.svg"] = lambda: [
    rect(4, 6, 40, 36, 2, "#2c3e50", 3, "#ecf0f1"),
    path("M4,24 H44", "#2c3e50", 3, "none"),
    rect(14, 10, 20, 10, 1, "#3498db", 2, "#d6eaf8"),
    path("M18,36 L24,28 L30,36 Z", "#e74c3c", 2, "#fadbd8")
]

# Better file operations icons
I["create_new_file.svg"] = lambda: [
    path("M10,4 H30 L40,14 V44 H10 Z", "#2c3e50", 3, "#ecf0f1"),
    path("M30,4 V14 H40", "#2c3e50", 3, "none"),
    path("M25,20 V36 M17,28 H33", "#27ae60", 4, "none")
]

I["open_file.svg"] = lambda: [
    path("M6,12 H18 L22,18 H42 V40 H6 Z", "#e67e22", 3, "#f39c12"),
    path("M10,24 L42,24 L38,40 H6 Z", "#d35400", 2, "#e67e22")
]

I["save_file.svg"] = lambda: [
    rect(6, 6, 36, 36, 4, "#2c3e50", 3, "#34495e"),
    rect(12, 6, 24, 12, 0, "#2c3e50", 2, "#ecf0f1"),
    rect(16, 24, 16, 18, 0, "#2c3e50", 2, "#bdc3c7"),
    rect(16, 8, 8, 8, 0, "#2c3e50", 0, "#95a5a6")
]

# Better drawing icons
I["draw_circle.svg"] = lambda: [
    circ(24, 24, 16, "#e74c3c", 4, "#fadbd8"),
    circ(24, 24, 2, "#c0392b", 4, "#c0392b")
]

I["draw_rectangle.svg"] = lambda: [
    rect(8, 12, 32, 24, 4, "#3498db", 4, "#d6eaf8"),
    circ(8, 12, 3, "#2980b9", 2, "#2980b9"),
    circ(40, 12, 3, "#2980b9", 2, "#2980b9"),
    circ(8, 36, 3, "#2980b9", 2, "#2980b9"),
    circ(40, 36, 3, "#2980b9", 2, "#2980b9")
]

I["draw_polygon.svg"] = lambda: [
    path("M24,6 L40,16 L34,36 H14 L8,16 Z", "#9b59b6", 4, "#e8daef"),
    circ(24, 6, 2, "#8e44ad", 2, "#8e44ad")
]

I["draw_ellipse.svg"] = lambda: [
    rect(4, 12, 40, 24, 12, "#f1c40f", 4, "#fcf3cf"),
    path("M24,12 V36 M4,24 H44", "#f39c12", 2, "none", "2,2")
]

I["draw_star.svg"] = lambda: [
    path("M24,4 L30,16 L44,18 L34,28 L36,42 L24,36 L12,42 L14,28 L4,18 L18,16 Z", "#f1c40f", 4, "#fcf3cf")
]

I["draw_polyline.svg"] = lambda: [
    path("M6,36 L18,12 L30,30 L42,16", "#34495e", 4, "none"),
    circ(6, 36, 3, "#2c3e50", 2, "#2c3e50"),
    circ(18, 12, 3, "#2c3e50", 2, "#2c3e50"),
    circ(30, 30, 3, "#2c3e50", 2, "#2c3e50"),
    circ(42, 16, 3, "#2c3e50", 2, "#2c3e50")
]

I["draw_text.svg"] = lambda: [
    path("M12,40 L24,8 L36,40 M18,28 H30", "#34495e", 5, "none"),
    path("M10,40 H14 M34,40 H38 M22,8 H26", "#34495e", 5, "none")
]

I["trace_bitmap.svg"] = lambda: [
    rect(6, 6, 36, 36, 0, "#95a5a6", 2, "#ecf0f1"),
    path("M6,42 L20,24 L28,32 L42,16", "#34495e", 2, "none"),
    path("M12,38 L24,20 L32,28 L42,12", "#1abc9c", 3, "none", "4,2")
]

I["weld_vectors.svg"] = lambda: [
    path("M8,24 A12,12 0 1 1 20,12 H28 A12,12 0 1 1 40,24 V32 H28 A12,12 0 0 0 20,44 H8 Z", "#3498db", 3, "#d6eaf8")
]

I["subtract_vectors.svg"] = lambda: [
    circ(20, 20, 14, "#e74c3c", 3, "#fadbd8"),
    path("M28,28 A14,14 0 1 0 42,42 H28 Z", "#c0392b", 3, "white")
]

I["overlap_vectors.svg"] = lambda: [
    circ(20, 20, 14, "#95a5a6", 2, "none", "2,2"),
    circ(34, 34, 14, "#95a5a6", 2, "none", "2,2"),
    path("M28,20 A14,14 0 0 1 34,26 A14,14 0 0 1 20,34 A14,14 0 0 1 28,20", "#34495e", 3, "#bdc3c7")
]

# Modeling icons
I["create_shape.svg"] = lambda: [
    circ(24, 24, 16, "#2ecc71", 4, "#a9dfbf"),
    path("M8,24 Q24,4 40,24 Q24,44 8,24", "#27ae60", 2, "none")
]

I["two_rail_sweep.svg"] = lambda: [
    path("M8,12 Q24,4 40,12", "#e74c3c", 4, "none"),
    path("M8,36 Q24,44 40,36", "#e74c3c", 4, "none"),
    path("M16,10 V38 M32,10 V38", "#c0392b", 2, "none", "2,2")
]

I["extrude_and_weave.svg"] = lambda: [
    path("M16,12 L32,12 L32,36 L16,36 Z", "#3498db", 4, "none"),
    path("M24,4 V44", "#2980b9", 2, "none", "4,4")
]

I["turn_and_spin.svg"] = lambda: [
    path("M24,8 C12,8 12,40 24,40 Z", "#f39c12", 4, "#fcf3cf"),
    path("M24,4 V44", "#e67e22", 2, "none", "4,4"),
    path("M16,16 L12,24 L16,32", "#d35400", 2, "none")
]

I["smooth_components.svg"] = lambda: [
    path("M6,24 Q15,10 24,24 T42,24", "#3498db", 4, "none"),
    circ(15, 17, 3, "#2980b9", 0, "#2980b9"),
    circ(33, 31, 3, "#2980b9", 0, "#2980b9")
]

I["scale_z_height.svg"] = lambda: [
    path("M12,24 L24,8 L36,24", "#e74c3c", 4, "none"),
    path("M16,24 L24,40 L32,24", "#e74c3c", 4, "none"),
    path("M24,8 V40", "#c0392b", 2, "none", "2,2")
]

I["slice_model.svg"] = lambda: [
    path("M8,16 L40,16 L32,32 L16,32 Z", "#f1c40f", 4, "#fcf3cf"),
    path("M6,24 H42", "#c0392b", 2, "none", "2,2")
]

I["add_draft.svg"] = lambda: [
    path("M16,40 L16,16 L32,8 L32,40", "#9b59b6", 4, "#e8daef"),
    path("M6,40 H42", "#8e44ad", 3, "none")
]

def render(name):
    els = I[name]()
    body = "".join(e for e in els if e)
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" width="48" height="48">{body}</svg>\n'

def main():
    out = "assets/icons/jules icons"
    os.makedirs(out, exist_ok=True)
    for name in sorted(I):
        dest = os.path.join(out, name)
        with open(dest, "w") as fh:
            fh.write(render(name))
    print(f"{len(I)} icons written to {out}")

if __name__ == "__main__":
    main()
