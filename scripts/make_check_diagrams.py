#!/usr/bin/env python3
# Usage: scripts/make_check_diagrams.py crates/help/pictures/checks
"""Side-view diagrams for the checks-before-cutting help pages: a wrong and a right panel."""
import sys
from PIL import Image, ImageDraw, ImageFont

OUT = sys.argv[1]
F = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
FB = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
font = ImageFont.truetype(F, 15)
small = ImageFont.truetype(F, 13)
bold = ImageFont.truetype(FB, 17)
W, H = 760, 330
WOOD, WOOD_EDGE = (222, 184, 135), (160, 120, 70)
SPOIL = (190, 190, 175)
BIT, SHANK = (120, 130, 145), (170, 175, 185)
RED, GREEN, BLUE, AMBER = (200, 40, 40), (40, 140, 60), (40, 100, 200), (215, 130, 0)
INK = (40, 40, 40)

def canvas():
    im = Image.new("RGB", (W, H), (250, 250, 248))
    return im, ImageDraw.Draw(im)

def panels(d, left, right):
    d.line([(W // 2, 20), (W // 2, H - 20)], fill=(200, 200, 200), width=2)
    d.text((20, 12), "✖ " + left, font=bold, fill=RED)
    d.text((W // 2 + 20, 12), "✔ " + right, font=bold, fill=GREEN)

def block(d, x0, x1, top, bottom, spoil=True):
    if spoil:
        d.rectangle([x0 - 10, bottom, x1 + 10, bottom + 22], fill=SPOIL, outline=(150, 150, 140))
        d.text((x0 - 5, bottom + 3), "spoilboard", font=small, fill=INK)
    d.rectangle([x0, top, x1, bottom], fill=WOOD, outline=WOOD_EDGE, width=2)

def bit(d, x, tip, flutes=50, width=16, shank=70, colour=BIT):
    d.rectangle([x - width // 2, tip - flutes, x + width // 2, tip], fill=colour)
    for k in range(tip - flutes + 6, tip, 10):
        d.line([(x - width // 2, k), (x + width // 2, k - 6)], fill=(90, 95, 105), width=2)
    d.rectangle([x - width // 2, tip - flutes - shank, x + width // 2, tip - flutes], fill=SHANK)

def arrow(d, a, b, colour, width=3, dash=False):
    if dash:
        n = 14
        for i in range(n):
            if i % 2 == 0:
                p = (a[0] + (b[0] - a[0]) * i / n, a[1] + (b[1] - a[1]) * i / n)
                q = (a[0] + (b[0] - a[0]) * (i + 1) / n, a[1] + (b[1] - a[1]) * (i + 1) / n)
                d.line([p, q], fill=colour, width=width)
    else:
        d.line([a, b], fill=colour, width=width)
    import math
    ang = math.atan2(b[1] - a[1], b[0] - a[0])
    for s in (2.6, -2.6):
        d.line([b, (b[0] + 12 * math.cos(ang + s), b[1] + 12 * math.sin(ang + s))], fill=colour, width=width)

def note(d, xy, text, colour=INK, f=None):
    d.multiline_text(xy, text, font=f or font, fill=colour, spacing=3)

def save(im, name):
    im.save(f"{OUT}/{name}.png", optimize=True)

def rapids():
    im, d = canvas()
    panels(d, "rapid move into the material", "rapids stay above it")
    for off, bad in ((0, True), (W // 2, False)):
        block(d, off + 40, off + 340, 190, 280)
        d.line([(off + 40, 150), (off + 340, 150)], fill=BLUE, width=1)
        note(d, (off + 250, 128), "Clearance (Z1)", BLUE, small)
        if bad:
            bit(d, off + 90, 230, 30, 14, 50)
            arrow(d, (off + 100, 230), (off + 300, 230), RED, 4, dash=True)
            note(d, (off + 120, 245), "full speed, not cutting:\na crash", RED)
        else:
            bit(d, off + 90, 150, 30, 14, 50)
            arrow(d, (off + 100, 150), (off + 300, 150), GREEN, 4, dash=True)
            note(d, (off + 110, 200), "clear of the material", GREEN)
    save(im, "rapids")

def spoilboard():
    im, d = canvas()
    panels(d, "well below the material", "a little into the spoilboard")
    for off, depth, colour, text in ((0, 40, RED, "wrong thickness or depth:\ncuts the spoilboard away"), (W // 2, 4, GREEN, "up to 1 mm: a clean cut-out")):
        block(d, off + 40, off + 340, 130, 230)
        d.rectangle([off + 40 - 10, 230, off + 350, 280], fill=SPOIL, outline=(150, 150, 140))
        d.rectangle([off + 160, 130, off + 180, 230 + depth], fill=(250, 250, 248))
        bit(d, off + 170, 230 + depth, 60, 18, 40)
        note(d, (off + 45, 290), text, colour)
    save(im, "spoilboard")

def reach():
    im, d = canvas()
    panels(d, "cut deeper than the flutes", "flutes reach the depth")
    for off, flutes, colour, text in ((0, 40, RED, "the smooth shank rubs:\nburns, and can snap"), (W // 2, 90, GREEN, "only the flutes cut")):
        block(d, off + 60, off + 320, 130, 260)
        d.rectangle([off + 170, 130, off + 196, 230], fill=(250, 250, 248))
        bit(d, off + 183, 230, flutes, 24, 140 - flutes)
        if colour == RED:
            d.rectangle([off + 168, 130, off + 198, 230 - flutes], outline=RED, width=3)
        note(d, (off + 210, 150), text, colour)
    save(im, "reach")

def keep_out():
    im, d = canvas()
    panels(d, "a cut runs into a clamp's zone", "the cut keeps clear")
    for off, bad in ((0, True), (W // 2, False)):
        d.rectangle([off + 40, 70, off + 340, 300], fill=WOOD, outline=WOOD_EDGE, width=2)
        d.rectangle([off + 230, 140, off + 320, 230], outline=RED, width=3)
        d.rectangle([off + 245, 160, off + 305, 210], fill=(110, 110, 120))
        note(d, (off + 232, 108), "Keep-out: Clamp 1", RED, small)
        if bad:
            d.rounded_rectangle([off + 80, 110, off + 280, 260], 14, outline=AMBER, width=5)
            note(d, (off + 90, 268), "the bit would hit the clamp", RED)
        else:
            d.rounded_rectangle([off + 70, 110, off + 200, 260], 14, outline=AMBER, width=5)
            note(d, (off + 60, 268), "part moved clear of the zone", GREEN)
    save(im, "keep-out")

def bed():
    im, d = canvas()
    panels(d, "bigger than the bed", "fits, or split into tiles")
    for off, bad in ((0, True), (W // 2, False)):
        d.rectangle([off + 60, 80, off + 300, 260], outline=INK, width=3)
        note(d, (off + 65, 265), "machine bed", INK, small)
        if bad:
            d.rectangle([off + 90, 110, off + 350, 230], outline=RED, width=4)
            note(d, (off + 100, 290), "runs into its end stops", RED)
        else:
            d.rectangle([off + 90, 110, off + 190, 230], outline=GREEN, width=4)
            d.rectangle([off + 190, 110, off + 280, 230], outline=GREEN, width=4)
            note(d, (off + 100, 160), "tile 1      tile 2", GREEN)
    save(im, "bed")

def z_travel():
    im, d = canvas()
    panels(d, "more than the Z axis travels", "within the Z travel")
    for off, top, colour in ((0, 40, RED), (W // 2, 120, GREEN)):
        d.rectangle([off + 300, 60, off + 330, 300], outline=INK, width=2)
        note(d, (off + 255, 302), "Z travel", INK, small)
        block(d, off + 40, off + 260, 200, 270, spoil=False)
        arrow(d, (off + 150, 245), (off + 150, top), colour, 3)
        arrow(d, (off + 150, top), (off + 150, 245), colour, 3)
        note(d, (off + 160, top), "clearance (Z1)", colour, small)
        if colour == RED:
            d.line([(off + 280, 60), (off + 350, 60)], fill=RED, width=3)
            note(d, (off + 20, 280), "lower Z1, or cut less deep", RED)
    save(im, "z-travel")

def air():
    im, d = canvas()
    panels(d, "a second pass over a cleared pocket", "each pass takes material")
    for off, bad in ((0, True), (W // 2, False)):
        block(d, off + 40, off + 340, 140, 240, spoil=False)
        d.rectangle([off + 100, 140, off + 280, 200], fill=(250, 250, 248))
        for y in range(150, 200, 14):
            d.line([(off + 105, y), (off + 275, y)], fill=AMBER if bad else GREEN, width=3)
        if bad:
            note(d, (off + 50, 262), "cuts where the wood is gone:\nwasted minutes", RED)
        else:
            d.rectangle([off + 100, 200, off + 280, 230], fill=(250, 250, 248))
            note(d, (off + 50, 262), "a deeper start depth", GREEN)
    save(im, "air-cutting")

def plunges():
    im, d = canvas()
    panels(d, "straight down, no room to ramp", "ramps in along the cut")
    for off, bad in ((0, True), (W // 2, False)):
        block(d, off + 40, off + 340, 160, 250, spoil=False)
        if bad:
            bit(d, off + 180, 200, 40, 18, 60)
            arrow(d, (off + 215, 100), (off + 215, 190), RED, 4)
            note(d, (off + 45, 268), "hard on the bit; a bit that does not\ncut at its centre burns or snaps", RED)
        else:
            d.line([(off + 80, 160), (off + 260, 200)], fill=GREEN, width=4)
            bit(d, off + 260, 200, 40, 18, 60)
            note(d, (off + 60, 268), "the Entry setting: ramp or helix", GREEN)
    save(im, "plunges")

def drilling():
    im, d = canvas()
    panels(d, "a drill moved sideways", "drills go straight down")
    for off, bad in ((0, True), (W // 2, False)):
        block(d, off + 40, off + 340, 160, 250, spoil=False)
        d.polygon([(off + 172, 200), (off + 188, 200), (off + 180, 212)], fill=BIT)
        bit(d, off + 180, 200, 50, 16, 50)
        if bad:
            arrow(d, (off + 200, 185), (off + 300, 185), RED, 4)
            note(d, (off + 45, 268), "snaps, or tears the work", RED)
        else:
            arrow(d, (off + 210, 110), (off + 210, 190), GREEN, 4)
            note(d, (off + 45, 268), "holes come out the drill's size", GREEN)
    save(im, "drilling")

def missed():
    im, d = canvas()
    panels(d, "bit too wide for the narrow part", "a smaller bit reaches")
    for off, r, colour in ((0, 26, RED), (W // 2, 10, GREEN)):
        pts = [(off + 80, 90), (off + 300, 90), (off + 300, 260), (off + 200, 260), (off + 200, 150), (off + 180, 150), (off + 180, 260), (off + 80, 260)]
        d.polygon(pts, fill=WOOD, outline=WOOD_EDGE)
        d.ellipse([off + 120 - r, 200 - r, off + 120 + r, 200 + r], outline=colour, width=3)
        d.ellipse([off + 190 - r, 120 - r, off + 190 + r, 120 + r], outline=colour, width=3)
        if colour == RED:
            note(d, (off + 30, 280), "the gap between is left uncut", RED)
        else:
            note(d, (off + 30, 280), "or clean up with a second, smaller bit", GREEN)
    save(im, "missed-spots")

def loose():
    im, d = canvas()
    panels(d, "cut right through, no tabs", "tabs hold the part")
    for off, bad in ((0, True), (W // 2, False)):
        d.rectangle([off + 40, 70, off + 340, 280], fill=WOOD, outline=WOOD_EDGE, width=2)
        d.rounded_rectangle([off + 100, 110, off + 280, 240], 12, outline=(80, 60, 40), width=8)
        if bad:
            note(d, (off + 110, 160), "loose: can be thrown\nor pulled into the bit", RED)
        else:
            for x, y in ((off + 190, 110), (off + 190, 240), (off + 100, 175), (off + 280, 175)):
                d.rectangle([x - 8, y - 8, x + 8, y + 8], fill=WOOD)
            note(d, (off + 135, 165), "4 tabs", GREEN)
    save(im, "loose-parts")

def on_material():
    im, d = canvas()
    panels(d, "copies off the material", "every copy on it")
    for off, n, colour in ((0, 4, RED), (W // 2, 3, GREEN)):
        d.rectangle([off + 40, 90, off + 280, 280], fill=WOOD, outline=WOOD_EDGE, width=2)
        for i in range(n):
            for j in range(2):
                d.ellipse([off + 60 + i * 75, 200 - j * 80, off + 110 + i * 75, 250 - j * 80], outline=colour if (i * 75 + 110 > 240 and colour == RED) else AMBER, width=4)
    save(im, "on-material")

def gauge(name, left, right, unit, lo, hi, value, maker=None, limit=None):
    im, d = canvas()
    panels(d, left, right)
    for off, val, good in ((0, value, False), (W // 2, maker if maker else limit * 0.8, True)):
        x0, x1 = off + 40, off + 340
        d.rectangle([x0, 150, x1, 175], outline=INK, width=2)
        pos = lambda v: x0 + (x1 - x0) * (v - lo) / (hi - lo)
        if limit:
            d.line([(pos(limit), 135), (pos(limit), 190)], fill=INK, width=3)
            note(d, (pos(limit) - 60, 195), f"the machine's\nlimit {limit:g} {unit}", INK, small)
        if maker:
            d.line([(pos(maker), 135), (pos(maker), 190)], fill=BLUE, width=3)
            note(d, (pos(maker) - 40, 110), f"maker's {maker:g}", BLUE, small)
        colour = GREEN if good else RED
        d.rectangle([x0 + 2, 152, pos(val), 173], fill=colour)
        note(d, (x0, 240), f"set: {val:g} {unit}", colour)
    save(im, name)

rapids(); spoilboard(); reach(); keep_out(); bed(); z_travel(); air(); plunges(); drilling(); missed(); loose(); on_material()
gauge("machine-speeds", "faster than the machine moves", "within its top speed", "mm/min", 0, 8000, 7000, limit=5000)
gauge("spindle-range", "slower than the spindle turns", "inside its range", "rpm", 0, 30000, 6000, limit=10000)
gauge("makers-figures", "40% above the maker's feed", "near the maker's figure", "mm/min", 0, 3000, 2520, maker=1800)

# no feed rate: a bar at zero
im, d = canvas()
panels(d, "feed rate 0: never moves", "every rate set")
for off, vals in ((0, (0, 500, 18000)), (W // 2, (1500, 500, 18000))):
    for k, (label, v, top) in enumerate(zip(("Feed rate", "Plunge rate", "Spindle speed"), vals, (3000, 1000, 24000))):
        y = 90 + k * 65
        note(d, (off + 40, y), label, INK)
        d.rectangle([off + 40, y + 22, off + 340, y + 42], outline=INK, width=2)
        colour = RED if v == 0 else GREEN
        d.rectangle([off + 42, y + 24, off + 42 + 296 * v / top, y + 40], fill=colour)
        if v == 0:
            note(d, (off + 50, y + 24), "0: not set", RED, small)
save(im, "can-cut")

# out of date: the old shape and the moved one
im, d = canvas()
panels(d, "toolpath of the old shape", "recalculated after the change")
for off, bad in ((0, True), (W // 2, False)):
    d.rectangle([off + 40, 70, off + 340, 280], fill=WOOD, outline=WOOD_EDGE, width=2)
    d.ellipse([off + 170, 120, off + 300, 250], outline=INK, width=2)
    if bad:
        d.ellipse([off + 70, 100, off + 200, 230], outline=AMBER, width=5)
        note(d, (off + 60, 285), "⚠ cuts where it used to be", RED)
    else:
        d.ellipse([off + 166, 116, off + 304, 254], outline=AMBER, width=5)
        note(d, (off + 60, 285), "cuts what the design shows", GREEN)
save(im, "up-to-date")

# tool changer: slots
im, d = canvas()
panels(d, "the bit is in no slot", "loaded in slot T3")
for off, loaded in ((0, False), (W // 2, True)):
    for i in range(5):
        x = off + 50 + i * 60
        d.rectangle([x, 120, x + 44, 200], outline=INK, width=2)
        note(d, (x + 10, 205), f"T{i + 1}", INK, small)
        if i < 2 or (loaded and i == 2):
            bit(d, x + 22, 190, 30, 10, 30, BIT)
    note(d, (off + 50, 250), "machine stops and asks" if not loaded else "changed for you", RED if not loaded else GREEN)
save(im, "tool-changer")
print("ok")
