---
reviewed: 2026-11-23
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

- **No moves** (⚠): the toolpath is left out of the files, and the status bar names it
  once saved. Recalculate it, or untick it in the toolpath list. If nothing is left to
  cut, nothing is saved and an earlier save is kept as it was.
- **No feed, plunge rate or spindle speed**: open the toolpath, and set them in its bit
  settings, or pick the bit again with **Select…** to bring the Bit Library's values.

See also [Checks before cutting](help:checks_before_cutting).
