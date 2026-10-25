#!/usr/bin/env python3
"""Dump every DIALOG(EX) and XAML resource of a PE file as text. Stdlib only.

Usage: dialogs.py Resources_ENU.dll OUTDIR
Writes OUTDIR/dialog_<id>.txt (one control per line: class, id, text, x y w h)
and OUTDIR/xaml_<name>.xaml. OUTDIR belongs under re/extracted/ (gitignored).
"""
import struct
import sys
from pathlib import Path

CLASSES = {0x80: "BUTTON", 0x81: "EDIT", 0x82: "STATIC", 0x83: "LISTBOX", 0x84: "SCROLLBAR", 0x85: "COMBOBOX"}


def sections(b):
    pe = struct.unpack_from("<I", b, 0x3C)[0]
    nsec, = struct.unpack_from("<H", b, pe + 6)
    opt = struct.unpack_from("<H", b, pe + 20)[0]
    magic = struct.unpack_from("<H", b, pe + 24)[0]
    ddir = pe + 24 + (112 if magic == 0x20B else 96)
    rsrc_rva = struct.unpack_from("<I", b, ddir + 2 * 8)[0]
    secs = []
    for i in range(nsec):
        s = pe + 24 + opt + 40 * i
        vsize, va, rsize, raw = struct.unpack_from("<IIII", b, s + 8)
        secs.append((va, max(vsize, rsize), raw))
    off = lambda rva: next(raw + rva - va for va, sz, raw in secs if va <= rva < va + sz)
    return rsrc_rva, off


def resources(b):
    rsrc, off = sections(b)
    base = off(rsrc)

    def name(v):
        if v & 0x80000000:
            p = base + (v & 0x7FFFFFFF)
            n, = struct.unpack_from("<H", b, p)
            return b[p + 2:p + 2 + 2 * n].decode("utf-16-le")
        return v

    def walk(p, path):
        named, ids = struct.unpack_from("<HH", b, p + 12)
        for i in range(named + ids):
            nv, ov = struct.unpack_from("<II", b, p + 16 + 8 * i)
            if ov & 0x80000000:
                yield from walk(base + (ov & 0x7FFFFFFF), path + [name(nv)])
            else:
                rva, size = struct.unpack_from("<II", b, base + ov)
                o = off(rva)
                yield path + [name(nv)], b[o:o + size]

    yield from walk(base, [])


def sz_or_ord(b, i):
    w, = struct.unpack_from("<H", b, i)
    if w == 0xFFFF:
        return struct.unpack_from("<H", b, i + 2)[0], i + 4
    j = i
    while b[j:j + 2] != b"\0\0":
        j += 2
    return b[i:j].decode("utf-16-le"), j + 2


def align(i):
    return (i + 3) & ~3


def dialog(b):
    ex = b[:4] == b"\x01\x00\xff\xff"
    if ex:
        _, _, _, _, style, n, x, y, w, h = struct.unpack_from("<HHIIIHhhhh", b, 0)
        i = 26
    else:
        style, _, n, x, y, w, h = struct.unpack_from("<IIHhhhh", b, 0)
        i = 18
    menu, i = sz_or_ord(b, i)
    cls, i = sz_or_ord(b, i)
    title, i = sz_or_ord(b, i)
    lines = [f"DIALOG {title!r} {x} {y} {w} {h}"]
    if style & 0x40:  # DS_SETFONT
        i += 6 if ex else 2
        _, i = sz_or_ord(b, i)
    for _ in range(n):
        i = align(i)
        if ex:
            _, _, style, x, y, w, h, cid = struct.unpack_from("<IIIhhhhI", b, i)
            i += 24
        else:
            style, _, x, y, w, h, cid = struct.unpack_from("<IIhhhhH", b, i)
            i += 18
        c, i = sz_or_ord(b, i)
        t, i = sz_or_ord(b, i)
        extra, = struct.unpack_from("<H", b, i)
        i += 2 + extra
        c = CLASSES.get(c, c)
        lines.append(f"  {c:18s} id={cid & 0xFFFF:<6d} style={style:#010x} {t!r} {x} {y} {w} {h}")
    return "\n".join(lines)


if __name__ == "__main__":
    data = Path(sys.argv[1]).read_bytes()
    out = Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    nd = nx = 0
    for path, d in resources(data):
        kind, rid = path[0], path[1]
        if kind == 5:
            try:
                (out / f"dialog_{rid}.txt").write_text(dialog(d))
            except Exception as e:  # noqa: BLE001 - report and carry on
                print("dialog", rid, "failed:", e)
            nd += 1
        elif isinstance(kind, str) and "XAML" in kind.upper():
            (out / f"xaml_{rid}.xaml").write_bytes(d)
            nx += 1
    print(nd, "dialogs,", nx, "xaml")
