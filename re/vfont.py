#!/usr/bin/env python3
"""Decode a Vectric .vfont (OLE compound file holding MFC-serialized glyphs).

Usage: vfont.py FONT.vfont [OUT.svg]  -- prints a summary; with OUT.svg draws every glyph.
Layout (worked out from the 23 samples in Aspire's Fonts/) is written up in re/SPEC.md.
Output stays under re/extracted/ (gitignored).
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import cfb  # noqa: E402


class Archive:
    """Just enough of MFC's CArchive object reading: classes and objects share one index."""

    def __init__(self, b, readers):
        self.b, self.o, self.idx, self.readers = b, 0, [None], readers

    def rd(self, f):
        v = struct.unpack_from(f, self.b, self.o)
        self.o += struct.calcsize(f)
        return v[0] if len(v) == 1 else v

    def obj(self):
        t = self.rd("<H")
        if t == 0:
            return None
        if t == 0x7FFF:  # big tag
            t = self.rd("<I")
            big = True
        else:
            big = False
        if t == 0xFFFF:
            self.rd("<H")  # schema
            n = self.rd("<H")
            cls = self.b[self.o:self.o + n].decode()
            self.o += n
            self.idx.append(cls)
        elif t & (0x80000000 if big else 0x8000):
            cls = self.idx[t & (0x7FFFFFFF if big else 0x7FFF)]
        else:
            raise ValueError(f"object back-reference {t} at {self.o:#x}")
        self.idx.append(("obj", cls))
        return self.readers[cls](self)


def char(a):
    a.rd("<I")
    code, adv = a.rd("<I"), a.rd("<d")
    return dict(code=code, advance=adv, contours=a.obj() or [])


def group(a):
    ver = a.rd("<I")
    n = a.rd("<I")
    cs = [a.obj() for _ in range(n)]
    if ver >= 2:
        a.rd("<B")
    return cs


def contour(a):
    ver = a.rd("<I")
    a.rd("<i"), a.rd("<d"), a.rd("<d"), a.rd("<H")
    n = a.rd("<I")
    spans = [a.obj() for _ in range(n)]
    if ver >= 2:
        a.o += 16
    return spans


def span(kind):
    def f(a):
        a.rd("<I")
        a.rd("<I")
        x0, y0, _, x1, y1, _ = a.rd("<6d")
        a.rd("<I")
        if kind == "C":
            c = a.rd("<4d")
            return ("C", (x0, y0), (c[0], c[1]), (c[2], c[3]), (x1, y1))
        if kind == "A":
            return ("A", (x0, y0), (x1, y1), a.rd("<d"))  # bulge: tan(sweep / 4)
        return ("L", (x0, y0), (x1, y1))
    return f


READERS = dict(txtVectricChar=char, vdContourGroup=group, vdContour=contour,
               vdBezierSpan=span("C"), vdLineSpan=span("L"), vdArcSpan=span("A"))


def glyphs(data):
    a = Archive(data, READERS)
    a.rd("<I")
    out = []
    while a.o < len(data):
        c = a.obj()
        if c:
            out.append(c)
    return out


def load(path):
    s = dict(cfb.read(path))
    gl = []
    for name, d in sorted(s.items()):
        if name.startswith("VectricFontData/"):
            gl += glyphs(d)
    return s, gl


def svg(gl, path):
    cell = max(g["advance"] for g in gl) * 1.2
    cols = 16
    parts = []
    for i, g in enumerate(gl):
        ox, oy = (i % cols) * cell, (i // cols + 1) * cell
        d = []
        for spans in g["contours"]:
            for k, s in enumerate(spans):
                p = s[1]
                if k == 0:
                    d.append(f"M{ox + p[0]:.0f},{oy - p[1]:.0f}")
                if s[0] == "A":
                    (x0, y0), (x1, y1), bulge = s[1:]
                    r = math.hypot(x1 - x0, y1 - y0) * (1 + bulge * bulge) / (4 * abs(bulge) or 1)
                    large = int(abs(bulge) > 1)
                    sweep = int(bulge < 0)  # svg y points down
                    d.append(f"A{r:.0f},{r:.0f} 0 {large} {sweep} {ox + x1:.0f},{oy - y1:.0f}")
                    continue
                pts = s[2:]
                d.append(s[0] + " ".join(f"{ox + x:.0f},{oy - y:.0f}" for x, y in pts))
        parts.append(f'<path d="{" ".join(d)}"/>')
    rows = (len(gl) + cols - 1) // cols + 1
    Path(path).write_text(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {cols * cell} {rows * cell}">'
        f'<rect width="100%" height="100%" fill="white"/>'
        f'<g fill="none" stroke="black" stroke-width="{cell / 200:.3g}">{"".join(parts)}</g></svg>')


if __name__ == "__main__":
    streams, gl = load(sys.argv[1])
    for n, d in streams.items():
        print(f"{len(d):8d} {n}")
    spans = sum(len(s) for g in gl for s in g["contours"])
    print(f"{len(gl)} glyphs, {spans} spans, codes {gl[0]['code']}..{gl[-1]['code']}")
    if len(sys.argv) > 2:
        svg(gl, sys.argv[2])
