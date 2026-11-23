---
reviewed: 2026-11-23
see-also: checks_before_cutting
---

# Keeps out of keep-out zones

![A cut running into a clamp's keep-out zone](pictures/checks/keep-out.png)

## What it looks for

Whether any toolpath cuts into, or passes over, a **keep-out zone**: the job's own (made
with Keep-Out Zones) and the machine's (from Machine Configuration). The edge of the bit
counts, not just its centre.

## Why it matters

Keep-out zones mark clamps, screws and anything else the bit must never touch. A bit
that hits a clamp breaks, and can throw the clamp or the work.

## How to fix it

Move the part or the zone, then recalculate the toolpath. To have toolpaths lift over
the zones and leave what is under them uncut, tick **Edit ▸ Options ▸ Toolpath Settings ▸
Clip toolpaths at keep-out zones**.

This is a **Must fix**: Save asks every time while a cut goes into a zone, and you tick it
to save anyway. See [Saving with errors](help:checks_before_cutting).

See also [Checks before cutting](help:checks_before_cutting).
