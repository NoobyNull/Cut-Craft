---
reviewed: 2026-11-25
see-also: checks_before_cutting, protect_the_machine
---

# Keeps out of keep-out zones

![A cut running into a clamp's keep-out zone](pictures/checks/keep-out.png)

## What it looks for

Whether any toolpath cuts into, or passes over, a **keep-out zone**: the job's own (made
with Keep-Out Zones) and the machine's (from Machine Configuration). The edge of the bit
counts, not just its centre. In a tiled job, the machine's zones (fixed to the bed) are
checked against each tile's cuts where its file puts them.

## Why it matters

Keep-out zones mark clamps, screws and anything else the bit must never touch. A bit
that hits a clamp breaks, and can throw the clamp or the work.

## How to fix it

Move the part or the zone, then recalculate the toolpath. To have toolpaths lift over
the zones and leave what is under them uncut, tick **Edit ▸ Options ▸ Toolpath Settings ▸
Clip toolpaths at keep-out zones**.

This is **⛔ Could damage the machine**: never saved, sent or made through the Web API while
a cut goes into a zone, with no tick and no Save Anyway. The only way past is switching
**Keep-out zones** off in the machine's [Protect the Machine](help:protect_the_machine).

See also [Checks before cutting](help:checks_before_cutting).
