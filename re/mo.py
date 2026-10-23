#!/usr/bin/env python3
"""Dump English msgids from each gettext .mo catalog. usage: mo.py <dir_with_mo> <out_dir>"""
import gettext, glob, os, sys
src, out = sys.argv[1:3]
for p in sorted(glob.glob(os.path.join(src, "*.mo"))):
    with open(p, "rb") as f:
        cat = gettext.GNUTranslations(f)._catalog
    ids = sorted({(k[0] if isinstance(k, tuple) else k) for k in cat} - {""})
    name = os.path.basename(p)[:-3]
    open(os.path.join(out, name + ".txt"), "w").write("\n".join(ids) + "\n")
    print(f"{len(ids):6d}  {name}")
