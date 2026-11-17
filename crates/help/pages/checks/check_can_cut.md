---
reviewed: 2026-11-17
see-also: checks_before_cutting
---

# Settings that can cut

![A toolpath with no feed rate cannot move](pictures/checks/can-cut.png)

## What it looks for

Every toolpath saved has moves to make, a **feed rate** (how fast it moves while cutting),
a **plunge rate** (how fast it goes down into the material) and a **spindle speed**.

## Why it matters

With no moves there is nothing to cut. A feed or plunge rate of 0 leaves the machine
standing still at the material, and with no spindle speed the spindle may never turn on,
so the bit is dragged through the wood without spinning and snaps.

## How to fix it

- **No moves**: recalculate the toolpath, or untick it in the toolpath list so it is not
  saved.
- **No feed, plunge rate or spindle speed**: open the toolpath, and set them in its tool
  settings, or pick the bit again with **Select…** to bring the Tool Database's values.

See also [Checks before cutting](help:checks_before_cutting).
