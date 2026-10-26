#!/usr/bin/env python3
"""Build our SVG icon set from the extracted Aspire icons (re/extracted, see icons.py).
Each icon is traced in colour layers with potrace and named after its label in re/icon_labels.json.
usage: make_icons.py <extracted_dir> <icon_labels.json> <out_dir> <manifest.json>
The SVGs are derived from Vectric's artwork: out_dir stays gitignored; the manifest (ids + names) is ours."""
import glob, json, os, re, shutil, subprocess, sys, tempfile
from PIL import Image

SCALE_TO = 384      # trace at this many px on the longest side: smoother curves from small rasters
COLORS = 8          # colour layers per icon; shaded art gets posterized to this many
TURDSIZE = 40       # potrace: drop specks smaller than this many px (at SCALE_TO)
RAW = {"import"}    # dialog pictures: Aspire's bitmaps copied as they are, not traced
SAME_ART = 12       # feature distance below which a larger single PNG is the same art as a strip slot

src_dir, labels_f, out_dir, manifest_f = sys.argv[1:5]
labels = json.load(open(labels_f))

# Strips -> folders, mirroring where they sit in the app. Toolpaths uses the 56px set.
STRIPS = {
    "35063": "drawing/create_vectors", "35064": "drawing/edit_objects", "35065": "drawing/weld_and_offset",
    "35066": "drawing/join", "35067": "drawing/align", "35068": "modeling", "35069": "file", "35070": "toolpaths",
}
SKIP = re.compile(r"(Vectric|Design & Make|forum) link$|logo$|^Account$|^Offline|\(alt\)")
SINGLES = {
    "view": ["ico/35153", "ico/35154", "png/35096", "png/35097", "png/35098", "png/35099", "png/35091", "png/35092",
             "png/35093", "ico/57651", "ico/33848", "png/34562", "png/34563",
             "ico/57652", "ico/33843", "ico/33847"],
    "snapping": ["ico/33883", "ico/33884", "ico/33885", "ico/33886", "ico/33887", "ico/33888"],
    "layers": ["ico/414", "ico/415", "ico/418", "ico/419", "png/35146", "png/35147", "png/35148", "png/35149"],
    "job_setup": ["png/34896", "png/34897", "png/34898", "png/34899", "png/34900", "png/34556", "png/34557",
                  "png/2572", "png/2573", "png/2696", "png/34905", "png/34902", "png/34903"],
    "common": ["png/34241", "png/34243", "png/34245", "png/34246", "png/34247", "png/34248", "png/34278",
               "png/34242", "png/35075", "png/34829", "ico/35038"],
    "import": ["bmp/4018", "bmp/4023", "bmp/34746"],
}

def load(key):
    f = glob.glob(f"{src_dir}/{key}.*")[0]
    im = Image.open(f)
    if f.endswith(".ico"):
        im.size = max(im.info["sizes"])
    return im.convert("RGBA")

def feat(im):
    bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
    rgb = Image.alpha_composite(bg, im).convert("RGB")
    box = rgb.convert("L").point(lambda v: 0 if v > 245 else 255).getbbox()
    return list((rgb.crop(box) if box else rgb).resize((20, 20), Image.LANCZOS).get_flattened_data())

def dist(a, b):
    return sum(abs(x - y) for p, q in zip(a, b) for x, y in zip(p, q)) / (len(a) * 3)

singles = {os.path.relpath(f, src_dir).rsplit(".", 1)[0] for f in glob.glob(f"{src_dir}/png/*.png")}
big = {}
for k in singles:
    im = load(k)
    if im.width == im.height and im.width >= 64:
        big[k] = (im, feat(im))

def best_source(key):
    """Largest single PNG with the same art as this strip slot, else the slot itself."""
    im = load(key)
    f = feat(im)
    match = min(big, key=lambda k: dist(f, big[k][1]), default=None)
    if match and dist(f, big[match][1]) < SAME_ART:
        return match, big[match][0]
    return key, im

def trace(im):
    """Layered colour trace: each layer covers its colour plus every later one, so no seams show."""
    z = SCALE_TO / max(im.size)
    im = im.resize((round(im.width * z), round(im.height * z)), Image.LANCZOS)
    opaque = im.getchannel("A").point(lambda a: 255 if a > 127 else 0)
    flat = Image.new("RGB", im.size, (255, 255, 255))
    flat.paste(im.convert("RGB"), mask=opaque)
    q = flat.quantize(colors=COLORS, method=Image.Quantize.MEDIANCUT)
    palette = q.getpalette()
    idx, alpha = list(q.get_flattened_data()), list(opaque.get_flattened_data())
    area = {}
    for i, a in zip(idx, alpha):
        if a:
            area[i] = area.get(i, 0) + 1
    order = sorted(area, key=lambda i: -area[i])
    layers = []
    with tempfile.TemporaryDirectory() as tmp:
        for n, colour in enumerate(order):
            members = set(order[n:])
            mask = Image.new("1", im.size, 1)
            mask.putdata([0 if a and i in members else 1 for i, a in zip(idx, alpha)])
            pbm, svg = f"{tmp}/m.pbm", f"{tmp}/m.svg"
            mask.save(pbm)
            subprocess.run(["potrace", "-s", "-t", str(TURDSIZE), "-o", svg, pbm], check=True)
            g = re.search(r"<g transform=\"[^\"]+\"[\s\S]*?</g>", open(svg).read())
            if g:
                r, gr, b = palette[colour * 3:colour * 3 + 3]
                layers.append(re.sub(r'fill="#000000"', f'fill="#{r:02x}{gr:02x}{b:02x}"', g.group(0)))
    w, h = im.size
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">\n'
            + "\n".join(layers) + "\n</svg>\n")

def slug(label):
    return re.sub(r"[^a-z0-9]+", "_", label.lower().replace("?", "")).strip("_")

jobs = []  # (folder, source key, label)
for sid, folder in STRIPS.items():
    for f in sorted(glob.glob(f"{src_dir}/slot/{sid}_*.png")):
        key = os.path.relpath(f, src_dir)[:-4]
        if not SKIP.search(labels.get(key, "")):
            jobs.append((folder, key, labels[key]))
for folder, keys in SINGLES.items():
    jobs += [(folder, k, labels[k]) for k in keys]

manifest, used = [], set()
for folder, key, label in jobs:
    name = slug(label.split(": ", 1)[-1] if folder == "drawing/align" else label)
    ext = os.path.splitext(glob.glob(f"{src_dir}/{key}.*")[0])[1] if folder in RAW else ".svg"
    path, n = f"{folder}/{name}{ext}", 2
    while path in used:
        path, n = f"{folder}/{name}_{n}{ext}", n + 1
    used.add(path)
    os.makedirs(f"{out_dir}/{folder}", exist_ok=True)
    if folder in RAW:
        source = key
        shutil.copyfile(glob.glob(f"{src_dir}/{key}.*")[0], f"{out_dir}/{path}")
    else:
        source, im = best_source(key) if key.startswith("slot/") else (key, load(key))
        open(f"{out_dir}/{path}", "w").write(trace(im))
    manifest.append({"path": path, "label": label.rstrip(" ?"), "guess": label.endswith("?"), "slot": key, "source": source})
json.dump(manifest, open(manifest_f, "w"), indent=1)
print(len(manifest), "icons ->", out_dir)
