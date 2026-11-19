#!/usr/bin/env python3
"""Packs PNG pictures of the app icon into one Windows .ico (each picture kept as PNG, which
Windows reads from Vista on). Stdlib only. scripts/windows-icon makes the pictures from
assets/cut-craft.svg and runs this.
usage: python3 -I scripts/make_windows_icon.py out.ico 16.png 24.png ... 256.png"""
import struct
import sys


def size_of(png):
    if png[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit("not a PNG")
    return struct.unpack(">II", png[16:24])


def main():
    out, *names = sys.argv[1:]
    pictures = [open(n, "rb").read() for n in names]
    pictures.sort(key=lambda p: size_of(p)[0])
    header = struct.pack("<HHH", 0, 1, len(pictures))
    offset = 6 + 16 * len(pictures)
    entries, data = b"", b""
    for png in pictures:
        w, h = size_of(png)
        # 256 is written as 0
        entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(png), offset)
        data += png
        offset += len(png)
    with open(out, "wb") as f:
        f.write(header + entries + data)


main()
