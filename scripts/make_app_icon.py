#!/usr/bin/env python3
"""Cut Craft's app icon: a spindle and collet nut holding an end mill that has cut a groove
into an isometric block of wood. The bit sits at the end of its groove, and the groove is
as wide as the bit. Stdlib only.
usage: python3 -I scripts/make_app_icon.py > assets/cut-craft.svg"""
u=(10.73,5.37)   # half groove width along the across-axis (iso)
def add(a,b,k=1): return (a[0]+k*b[0], a[1]+k*b[1])
F=lambda p: f"{p[0]:.1f},{p[1]:.1f}"
s0=(80,174); bit=(132.8,147.6); D=12
A,B=add(s0,u,-1),add(s0,u); C,Dd=add(bit,u),add(bit,u,-1)
dn=lambda p:(p[0],p[1]+D)
rim=f"M{F(A)} L{F(B)} L{F(C)} A15.1 7.6 0 0 0 {F(Dd)} Z"
floor=f"M{F(dn(A))} L{F(dn(B))} L{F(dn(C))} A15.1 7.6 0 0 0 {F(dn(Dd))} Z"
bx=bit[0]; L,R=bx-15.1,bx+15.1
grain='''<g clip-path="url(#top)" fill="none" stroke="#c4904f" stroke-width="2" stroke-linecap="round" opacity=".55">
<path d="M58 150 C 90 136, 120 118, 150 108"/><path d="M110 192 C 140 178, 170 160, 206 146"/><path d="M150 186 C 176 172, 196 160, 222 150"/><path d="M42 146 C 70 134, 96 120, 118 108"/></g>
<g fill="none" stroke="#9a6430" stroke-width="2" opacity=".6"><path d="M40 168 C 60 176, 90 194, 120 210"/><path d="M36 178 C 60 188, 90 204, 124 222"/></g>'''
flutes=f'''<g clip-path="url(#flute)" stroke-linecap="round">
{"".join(f'<path d="M{L-4} {y} C {bx-6} {y-2}, {bx+6} {y-12}, {R+4} {y-20}" fill="none" stroke="#39414b" stroke-width="5"/><path d="M{L-4} {y+5} C {bx-6} {y+3}, {bx+6} {y-7}, {R+4} {y-15}" fill="none" stroke="#f4f7fa" stroke-width="2" opacity=".8"/>' for y in range(104,180,19))}
</g>'''
chips='''<g fill="none" stroke="#d9a35f" stroke-width="4" stroke-linecap="round">
<path d="M164 128 q 8 -8 14 0"/><path d="M180 110 q 4 -9 11 -4"/><path d="M96 124 q -6 -8 -13 -2"/></g>'''
svg=f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
<title>Cut Craft</title>
<defs>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3d4a5d"/><stop offset="1" stop-color="#222a35"/></linearGradient>
<linearGradient id="wood" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f3d49c"/><stop offset="1" stop-color="#d8a862"/></linearGradient>
<linearGradient id="steel" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#5b6470"/><stop offset=".35" stop-color="#eef2f6"/><stop offset=".7" stop-color="#8c96a2"/><stop offset="1" stop-color="#4a525c"/></linearGradient>
<linearGradient id="nut" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#6b7480"/><stop offset=".33" stop-color="#6b7480"/><stop offset=".33" stop-color="#c9d1da"/><stop offset=".67" stop-color="#c9d1da"/><stop offset=".67" stop-color="#8a94a0"/><stop offset="1" stop-color="#8a94a0"/></linearGradient>
<clipPath id="tile"><rect x="8" y="8" width="240" height="240" rx="52"/></clipPath>
<clipPath id="top"><polygon points="32,150 128,102 224,150 128,198"/></clipPath>
<clipPath id="flute"><rect x="{L}" y="96" width="30.2" height="51.6"/><ellipse cx="{bx}" cy="{bit[1]}" rx="15.1" ry="7.6"/></clipPath>
<clipPath id="rim"><path d="{rim}"/></clipPath>
</defs>
<rect x="8" y="8" width="240" height="240" rx="52" fill="url(#bg)"/>
<rect x="9.5" y="9.5" width="237" height="237" rx="50.5" fill="none" stroke="#6f8199" stroke-width="3" opacity=".3"/>
<g clip-path="url(#tile)">
<polygon points="32,150 128,198 128,232 32,184" fill="#b67d3c"/>
<polygon points="128,198 224,150 224,184 128,232" fill="#9a642c"/>
<polygon points="32,150 128,102 224,150 128,198" fill="url(#wood)"/>
{grain}
<path d="{rim}" fill="#5e3f1d"/>
<g clip-path="url(#rim)"><path d="{floor}" fill="#b98a4f"/></g>
<polygon points="{F(A)} {F(B)} {F(dn(B))} {F(dn(A))}" fill="#4a3116"/>
<rect x="88" y="0" width="80" height="40" fill="url(#steel)"/>
<rect x="88" y="34" width="80" height="6" fill="#3a414a"/>
<polygon points="102,40 154,40 158,52 154,64 102,64 98,52" fill="url(#nut)"/>
<rect x="{bx-12}" y="64" width="24" height="36" fill="url(#steel)"/>
<rect x="{bx-12}" y="72" width="24" height="7" fill="#ef8f2a"/>
<g clip-path="url(#flute)"><rect x="{L}" y="96" width="30.2" height="70" fill="url(#steel)"/></g>
{flutes}
{chips}
</g>
</svg>'''
print(svg)
