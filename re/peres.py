#!/usr/bin/env python3
"""Decode MENU / ACCELERATOR / STRINGTABLE / TOOLBAR payloads using REA's resource ranges.
usage: peres.py <rea_json> <pe_path> <out_dir>"""
import json, struct, sys, os

rea, pe, out = sys.argv[1:4]
res = json.load(open(rea))["normalized_result"]["resources"]
blob = open(pe, "rb").read()
os.makedirs(out, exist_ok=True)

def key(x):
    return x["name"] if x["kind"] == "name" else x["id"]

def payload(r):
    loc = r["payload"]["location"]
    return blob[loc["offset"]:loc["offset"] + loc["bytes"]]

def wstr(b, i):
    j = i
    while b[j:j + 2] != b"\0\0":
        j += 2
    return b[i:j].decode("utf-16le"), j + 2

strings = {}
for r in res:
    if key(r["type"]) == 6:
        b, i, base = payload(r), 0, (key(r["name"]) - 1) * 16
        for n in range(16):
            ln, = struct.unpack_from("<H", b, i); i += 2
            if ln:
                strings[base + n] = b[i:i + ln * 2].decode("utf-16le")
            i += ln * 2

def menu_std(b, i, depth, lines):
    while i < len(b):
        flags, = struct.unpack_from("<H", b, i); i += 2
        cid = None
        if not flags & 0x10:
            cid, = struct.unpack_from("<H", b, i); i += 2
        text, i = wstr(b, i)
        lines.append("  " * depth + (text or "----") + (f"  [{cid}]" if cid else ""))
        if flags & 0x10:
            i = menu_std(b, i, depth + 1, lines)
        if flags & 0x80:
            break
    return i

def menu_ex(b, i, depth, lines):
    while i < len(b):
        i = (i + 3) & ~3
        typ, state, cid, flags = struct.unpack_from("<IIIH", b, i); i += 14
        text, i = wstr(b, i)
        lines.append("  " * depth + (text or "----") + (f"  [{cid}]" if cid else ""))
        if flags & 1:
            i = (i + 3) & ~3; i += 4
            i = menu_ex(b, i, depth + 1, lines)
        if flags & 0x80:
            break
    return i

with open(f"{out}/menus.txt", "w") as f:
    for r in res:
        if key(r["type"]) == 4:
            b, lines = payload(r), []
            ver, off = struct.unpack_from("<HH", b, 0)
            (menu_ex(b, 4 + off, 0, lines) if ver == 1 else menu_std(b, 4 + off, 0, lines))
            f.write(f"== MENU {key(r['name'])}\n" + "\n".join(lines) + "\n\n")

VK = {0x08: "Backspace", 0x09: "Tab", 0x0D: "Enter", 0x1B: "Esc", 0x20: "Space", 0x21: "PgUp", 0x22: "PgDn",
      0x23: "End", 0x24: "Home", 0x25: "Left", 0x26: "Up", 0x27: "Right", 0x28: "Down", 0x2D: "Ins", 0x2E: "Del",
      0x6B: "Num+", 0x6D: "Num-", 0xBB: "=", 0xBD: "-", 0xDB: "[", 0xDD: "]"}
VK.update({0x70 + n: f"F{n + 1}" for n in range(12)})
with open(f"{out}/accelerators.txt", "w") as f:
    for r in res:
        if key(r["type"]) == 9:
            b = payload(r)
            for i in range(0, len(b), 8):
                fl, k, cid, _ = struct.unpack_from("<HHHH", b, i)
                mods = "".join(m for bit, m in ((8, "Ctrl+"), (0x10, "Alt+"), (4, "Shift+")) if fl & bit)
                kn = VK.get(k, chr(k) if 0x20 < k < 0x7F else hex(k)) if fl & 1 else repr(chr(k))
                f.write(f"{mods}{kn}\t[{cid}]\t{strings.get(cid, '').split(chr(10))[-1]}\n")
                if fl & 0x80:
                    break

with open(f"{out}/toolbars.txt", "w") as f:
    for r in res:
        if key(r["type"]) == 241:
            b = payload(r)
            _, w, h, n = struct.unpack_from("<HHHH", b, 0)
            ids = struct.unpack_from(f"<{n}H", b, 8)
            f.write(f"== TOOLBAR {key(r['name'])} ({w}x{h})\n")
            for cid in ids:
                f.write(("  ----" if not cid else f"  [{cid}] {strings.get(cid, '?').split(chr(10))[-1]}") + "\n")

with open(f"{out}/strings.txt", "w") as f:
    for k in sorted(strings):
        f.write(f"{k}\t{strings[k]!r}\n")

with open(f"{out}/names.txt", "w") as f:
    for r in res:
        t = key(r["type"])
        if t in (5, "RT_XAML", "AFX_DIALOG_LAYOUT", 23, "TEXTFILE", "PNG", 2, 240):
            f.write(f"{t}\t{key(r['name'])}\t{r['payload']['location']['bytes']}\n")

print(len(strings), "strings")
