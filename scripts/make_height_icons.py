#!/usr/bin/env python3
"""The 3D view's height button pictures (assets/icons/modeling): a model seen from the side,
the part each button changes in orange, the rest grey, a blue arrow for how far it reaches.
Run: python3 scripts/make_height_icons.py assets/icons/modeling"""
import sys
from PIL import Image, ImageDraw
K=4; S=128*K
ORANGE=(232,135,30,255); ORANGE_EDGE=(120,66,10,255)
GREY=(196,201,208,255); GREY_EDGE=(110,116,126,255)
BLUE=(37,110,230,255)
def s(v): return int(v*K)
def icon(dome_on, base_on, arrow):
    im=Image.new('RGBA',(S,S),(0,0,0,0)); d=ImageDraw.Draw(im)
    # base slab
    fill,edge=(ORANGE,ORANGE_EDGE) if base_on else (GREY,GREY_EDGE)
    d.rectangle((s(10),s(74),s(88),s(108)),fill=fill,outline=edge,width=s(4))
    # dome: upper half ellipse sitting on the slab
    fill,edge=(ORANGE,ORANGE_EDGE) if dome_on else (GREY,GREY_EDGE)
    d.pieslice((s(16),s(36),s(82),s(112)),180,360,fill=fill,outline=edge,width=s(4))
    d.line((s(18),s(74),s(80),s(74)),fill=edge,width=s(4))
    # double arrow on the right
    top,bot=arrow; x=s(110)
    d.line((x,s(top)+s(12),x,s(bot)-s(12)),fill=BLUE,width=s(8))
    h=s(13)
    d.polygon([(x,s(top)),(x-h,s(top)+s(16)),(x+h,s(top)+s(16))],fill=BLUE)
    d.polygon([(x,s(bot)),(x-h,s(bot)-s(16)),(x+h,s(bot)-s(16))],fill=BLUE)
    return im.resize((128,128),Image.LANCZOS)
out=sys.argv[1]
icon(True,True,(30,110)).save(out+'/total_height.png',optimize=True)
icon(True,False,(30,76)).save(out+'/shape_height.png',optimize=True)
icon(False,True,(70,110)).save(out+'/base_height.png',optimize=True)
