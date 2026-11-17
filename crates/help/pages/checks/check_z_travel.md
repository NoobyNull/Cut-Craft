---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_bed
---

# Fits the Z axis travel

![From the clearance height down to the deepest cut](pictures/checks/z-travel.png)

## What it looks for

How far the toolpaths go up and down: from the highest move (the clearance height, Z1)
to the deepest cut, against how far the machine's Z axis travels.

## Why it matters

If the job needs more travel than the Z axis has, the spindle hits the top of its travel
on the way up, or cannot reach the depth on the way down.

## How to fix it

- Lower the **Clearance (Z1)** in Job Setup: a few millimetres above clamps is plenty.
- Cut less deep, or set the bit to stick out further.
- If it says *not checked*, give the machine its Z travel in Machine Configuration.

See also [Checks before cutting](help:checks_before_cutting).
