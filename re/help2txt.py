#!/usr/bin/env python3
"""Extract the data-pagefind-body text of each help page. usage: help2txt.py <help_dir> <out_dir>"""
import glob, os, sys
from html.parser import HTMLParser

VOID = {"br", "img", "hr", "meta", "link", "input", "source", "col", "area", "wbr"}
BLOCK = {"p", "div", "li", "tr", "h1", "h2", "h3", "h4", "h5", "table", "ul", "ol"}

class P(HTMLParser):
    def __init__(self):
        super().__init__(); self.depth = 0; self.out = []; self.skip = 0
    def handle_starttag(self, tag, attrs):
        if tag in VOID: return
        if self.depth: self.depth += 1
        elif "data-pagefind-body" in dict(attrs): self.depth = 1
        if self.depth:
            if tag in ("script", "style", "svg"): self.skip += 1
            if tag in BLOCK: self.out.append("\n")
            if tag in ("h1", "h2", "h3", "h4"): self.out.append("#" * int(tag[1]) + " ")
            if tag == "li": self.out.append("- ")
            if tag in ("td", "th"): self.out.append(" | ")
    def handle_endtag(self, tag):
        if tag in VOID or not self.depth: return
        if tag in ("script", "style", "svg"): self.skip -= 1
        self.depth -= 1
    def handle_data(self, d):
        if self.depth and not self.skip: self.out.append(d.replace("\n", " "))

src, out = sys.argv[1:3]
for f in glob.glob(os.path.join(src, "*", "*", "index.html")):
    p = P(); p.feed(open(f, encoding="utf-8", errors="replace").read())
    text = "\n".join(" ".join(l.split()) for l in "".join(p.out).splitlines())
    text = "\n".join(l for l in text.splitlines() if l.strip())
    name = os.path.basename(os.path.dirname(f))
    open(os.path.join(out, name + ".txt"), "w").write(text + "\n")
