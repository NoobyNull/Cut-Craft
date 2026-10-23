#!/usr/bin/env python3
"""Extract PNG / BMP / ICO image resources from a PE (e.g. Aspire's Icons_Light.dll) using REA's
inspect_pe_resources JSON, and write label.html for naming them by hand.
usage: icons.py <rea_json> <pe_path> <out_dir> <rust_src>...
Output stays local (re/extracted/ is gitignored): the images are Vectric's artwork."""
import html, json, os, re, struct, sys

rea, pe, out, *rust_src = sys.argv[1:]
res = json.load(open(rea))["normalized_result"]["resources"]
blob = open(pe, "rb").read()

def key(x):
    return x["name"] if x["kind"] == "name" else x["id"]

def payload(r):
    loc = r["payload"]["location"]
    return blob[loc["offset"]:loc["offset"] + loc["bytes"]]

def bmp_file(dib):
    size, w, h, _, bits, comp = struct.unpack_from("<IiiHHI", dib, 0)
    used, = struct.unpack_from("<I", dib, 32) if size >= 36 else (0,)
    palette = (used or 1 << bits) * 4 if bits <= 8 else 0
    masks = 12 if comp == 3 and size == 40 else 0
    offset = 14 + size + palette + masks
    return struct.pack("<2sIHHI", b"BM", 14 + len(dib), 0, 0, offset) + dib, w, abs(h)

icons = {key(r["name"]): payload(r) for r in res if key(r["type"]) == 3}

def ico_file(group):
    _, kind, count = struct.unpack_from("<HHH", group, 0)
    entries, images, offset = [], [], 6 + 16 * count
    for n in range(count):
        w, h, colors, _, planes, bits, size, nid = struct.unpack_from("<BBBBHHIH", group, 6 + 14 * n)
        data = icons[nid]
        entries.append(struct.pack("<BBBBHHII", w, h, colors, 0, planes, bits, len(data), offset))
        images.append(data)
        offset += len(data)
    largest = max(struct.unpack_from("<BB", group, 6 + 14 * n) for n in range(count))
    return struct.pack("<HHH", 0, kind, count) + b"".join(entries) + b"".join(images), *(v or 256 for v in largest)

items = []
for r in res:
    t, name = key(r["type"]), key(r["name"])
    if t == "PNG":
        data = payload(r)
        w, h = struct.unpack_from(">II", data, 16)
        ext = "png"
    elif t == 2:
        data, w, h = bmp_file(payload(r))
        ext = "bmp"
    elif t == 14:
        data, w, h = ico_file(payload(r))
        ext = "ico"
    else:
        continue
    os.makedirs(f"{out}/{ext}", exist_ok=True)
    path = f"{ext}/{name}.{ext}"
    open(f"{out}/{path}", "wb").write(data)
    items.append({"key": f"{ext}/{name}", "src": path, "w": w, "h": h})
    # Codejock icon strips: square slots side by side, one per command. Slice the largest set.
    if ext == "png" and w % h == 0 and w // h >= 4 and h >= 42:
        from PIL import Image
        os.makedirs(f"{out}/slot", exist_ok=True)
        strip = Image.open(f"{out}/{path}")
        for n in range(w // h):
            slot = f"slot/{name}_{n:02d}.png"
            strip.crop((n * h, 0, (n + 1) * h, h)).save(f"{out}/{slot}")
            items.append({"key": slot[:-4], "src": slot, "w": h, "h": h})

order = {"slot": 0, "png": 1, "bmp": 2, "ico": 3}
items.sort(key=lambda i: (order[i["key"].split("/")[0]], [int(p) for p in i["key"].split("/")[1].split("_") if p.isdigit()]))

# Suggestions for the label box: title-case string literals from our own UI code.
names = set()
for f in rust_src:
    names |= set(re.findall(r'"([A-Z][A-Za-z0-9 /()&.\'-]{1,60})"', open(f).read()))

template = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "label_template.html")).read()
page = (template
        .replace("/*ITEMS*/[]", json.dumps(items))
        .replace("<!--NAMES-->", "".join(f'<option value="{html.escape(n)}">' for n in sorted(names))))
open(f"{out}/label.html", "w").write(page)
print(len(items), "images ->", out)
