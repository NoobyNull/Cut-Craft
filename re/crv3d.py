#!/usr/bin/env python3
"""Read the vectors out of a Vectric .crv3d job (OLE compound file, MFC-serialized streams).

Usage: crv3d.py JOB.crv3d [OUT.svg]  -- lists layers and contours; with OUT.svg draws them.
Only the vector contours are decoded (layer name, lines, arcs, Béziers); see re/SPEC.md, Job files.
Output stays under re/extracted/ (gitignored).
"""
import re
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import cfb  # noqa: E402

# every record sits behind a "Vectric__Version" wrapper: u32 16, the name, u32 1, u32 36, a GUID
# string (which record it belongs to; not fixed), u32 wrapper version; a vdContour follows some
CONTOUR_KEY = b"Vectric__Version"
KINDS = {0: "L", 1: "A", 2: "C"}


def layers(data):
    """(offset, name) of each vcCadLayer record, from its UTF-16 name."""
    out = []
    for m in re.finditer(rb"\xff\xfe\xff(.)", data, re.S):
        n = m.group(1)[0]
        o = m.end()
        if n == 0xFF:
            n = struct.unpack_from("<H", data, o)[0]
            o += 2
        name = data[o:o + 2 * n]
        try:
            name = name.decode("utf-16-le")
        except UnicodeDecodeError:
            continue
        # a layer record holds a GUID right before its name; contours put the name elsewhere
        if data[m.start() - 16:m.start()] and 0 < n < 64 and name.isprintable():
            out.append((m.start(), name))
    return out


def contours(data):
    """(offset, [span, ...]); a span is ('L', from, to), ('A', from, to, bulge) or
    ('C', from, c1, c2, to) in job units."""
    out = []
    for m in re.finditer(re.escape(CONTOUR_KEY), data):
        o = m.end()
        if o + 8 + 36 + 4 > len(data):
            break
        _ver, n = struct.unpack_from("<II", data, o)
        o += 8
        if n != 36:
            continue
        guid = data[o:o + 36]
        o += 36
        if guid == b"00000000-0000-0000-0000-000000000000":
            continue  # a component's 2D preview outline, not a drawn vector
        wrap, = struct.unpack_from("<I", data, o)
        o += 4
        if wrap != 2:
            continue  # another record kind shares the wrapper
        tag, = struct.unpack_from("<H", data, o)
        o += 2
        if tag == 0xFFFF:
            o += 2
            n, = struct.unpack_from("<H", data, o)
            o += 2 + n
        ver, = struct.unpack_from("<I", data, o)
        o += 4
        if ver != 7:
            continue
        _, _tol, _z, _flag, n = struct.unpack_from("<iddHI", data, o)
        o += 4 + 8 + 8 + 2 + 4
        spans = []
        for _ in range(n):
            kind = data[o]
            o += 1
            _sv, _typ = struct.unpack_from("<II", data, o)
            o += 8
            x0, y0, _, x1, y1, _ = struct.unpack_from("<6d", data, o)
            o += 48
            o += 4  # flags
            if kind == 1:
                bulge, = struct.unpack_from("<d", data, o)
                o += 8
                spans.append(("A", (x0, y0), (x1, y1), bulge))
            elif kind == 2:
                c = struct.unpack_from("<4d", data, o)
                o += 32
                spans.append(("C", (x0, y0), (c[0], c[1]), (c[2], c[3]), (x1, y1)))
            else:
                spans.append(("L", (x0, y0), (x1, y1)))
        out.append((m.start(), spans))
    return out


def by_layer(data):
    """Each contour with the name of the layer record before it."""
    names = layers(data)
    out = []
    for at, spans in contours(data):
        layer = [n for o, n in names if o < at]
        out.append((layer[-1] if layer else "?", spans))
    return out


def load(path):
    streams = dict(cfb.read(path))
    return by_layer(streams["VectorData/2dDataV2"])


if __name__ == "__main__":
    items = load(sys.argv[1])
    for layer, spans in items:
        kinds = "".join(s[0] for s in spans)
        print(f"{layer:20s} {len(spans):3d} spans {kinds[:40]} from {spans[0][1]}")
    if len(sys.argv) > 2:
        import math

        parts = []
        for _, spans in items:
            d = [f"M{spans[0][1][0]},{-spans[0][1][1]}"]
            for s in spans:
                if s[0] == "A":
                    (x0, y0), (x1, y1), bulge = s[1:]
                    r = math.hypot(x1 - x0, y1 - y0) * (1 + bulge * bulge) / (4 * abs(bulge) or 1)
                    d.append(f"A{r},{r} 0 {int(abs(bulge) > 1)} {int(bulge < 0)} {x1},{-y1}")
                else:
                    d.append(s[0] + " ".join(f"{x},{-y}" for x, y in s[2:]))
            parts.append(f'<path d="{" ".join(d)}"/>')
        xs = [p[0] for _, sp in items for s in sp for p in s[1:] if isinstance(p, tuple)]
        ys = [-p[1] for _, sp in items for s in sp for p in s[1:] if isinstance(p, tuple)]
        box = f"{min(xs) - 1} {min(ys) - 1} {max(xs) - min(xs) + 2} {max(ys) - min(ys) + 2}"
        Path(sys.argv[2]).write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{box}"><rect x="{min(xs) - 1}" y="{min(ys) - 1}" '
            f'width="100%" height="100%" fill="white"/><g fill="none" stroke="black" stroke-width="0.05">'
            f'{"".join(parts)}</g></svg>')
