---
reviewed: 2026-11-23
see-also: checks_before_cutting, check_air_cutting
---

# Rapid moves clear of the material

![A rapid move through material that is still there](pictures/checks/rapids.png)

## What it looks for

PowerWhittle cuts the whole job, in order, from a block of material, and watches every
**rapid move** (the moves at full speed between cuts) for one that goes into material that
is still there.

## Why it matters

Rapid moves are not meant to cut: at full speed, into wood, the bit snaps or the machine
loses its place. This is a crash.

## How to fix it

- Raise the **Clearance (Z1)** in Job Setup so moves between cuts stay above the material.
- Recalculate the toolpath: an out-of-date toolpath can run its rapids where the material
  now is.
- Check the toolpath order: a toolpath that relies on another having cut first must come
  after it in the list.

This is a **Must fix**: Save asks every time, and you tick it to save anyway. A rotary job is not cut through in order yet, so this check says *not checked* there: watch its first moves.

See also [Checks before cutting](help:checks_before_cutting).
