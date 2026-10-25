#!/usr/bin/env python3
"""List and dump the streams of an OLE compound file (.vfont, .crv3d, ...).

Stdlib only. Usage: cfb.py FILE [OUTDIR]  -- lists streams; with OUTDIR writes each one there.
Output goes under re/extracted/ (gitignored): stream contents are Vectric data.
"""
import struct
import sys
from pathlib import Path

FREE, END = 0xFFFFFFFF, 0xFFFFFFFE


def read(path):
    b = Path(path).read_bytes()
    assert b[:8] == bytes.fromhex("d0cf11e0a1b11ae1"), "not a compound file"
    ssz = 1 << struct.unpack_from("<H", b, 0x1E)[0]
    msz = 1 << struct.unpack_from("<H", b, 0x20)[0]
    nfat, dir0, _, cutoff, mini0, nmini, difat0, ndifat = struct.unpack_from("<IIIIIIII", b, 0x2C)
    sec = lambda i: b[(i + 1) * ssz:(i + 2) * ssz]
    difat = list(struct.unpack_from("<109I", b, 0x4C))
    d = difat0
    for _ in range(ndifat):
        ids = struct.unpack("<%dI" % (ssz // 4), sec(d))
        difat += ids[:-1]
        d = ids[-1]
    fat = []
    for s in difat[:nfat]:
        fat += struct.unpack("<%dI" % (ssz // 4), sec(s))

    def chain(start, table):
        out, i = [], start
        while i not in (FREE, END) and len(out) <= len(table):
            out.append(i)
            i = table[i]
        return out

    big = lambda start: b"".join(sec(i) for i in chain(start, fat))
    dirs = big(dir0)
    entries = []
    for o in range(0, len(dirs), 128):
        e = dirs[o:o + 128]
        nlen = struct.unpack_from("<H", e, 64)[0]
        name = e[:max(nlen - 2, 0)].decode("utf-16-le")
        kind = e[66]
        left, right, child = struct.unpack_from("<III", e, 68)
        start, size = struct.unpack_from("<IQ", e, 116)
        entries.append(dict(name=name, kind=kind, left=left, right=right, child=child, start=start, size=size))
    root = entries[0]
    ministream = big(root["start"])
    minifat = []
    for i in chain(mini0, fat):
        minifat += struct.unpack("<%dI" % (ssz // 4), sec(i))

    def data(e):
        if e["size"] < cutoff:
            raw = b"".join(ministream[i * msz:(i + 1) * msz] for i in chain(e["start"], minifat))
        else:
            raw = big(e["start"])
        return raw[:e["size"]]

    # walk the tree to give each stream its storage path
    paths = {}

    def walk(i, prefix):
        if i == FREE or i >= len(entries):
            return
        e = entries[i]
        walk(e["left"], prefix)
        walk(e["right"], prefix)
        p = prefix + [e["name"]]
        paths[i] = "/".join(p)
        if e["kind"] in (1, 5):
            walk(e["child"], p if e["kind"] == 1 else [])

    walk(0, [])
    return [(paths[i], data(e)) for i, e in enumerate(entries) if e["kind"] == 2 and i in paths]


if __name__ == "__main__":
    streams = read(sys.argv[1])
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    for name, d in streams:
        print(f"{len(d):8d}  {name}")
        if out:
            p = out / name.replace("\x01", "_").replace("\x05", "_")
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_bytes(d)
