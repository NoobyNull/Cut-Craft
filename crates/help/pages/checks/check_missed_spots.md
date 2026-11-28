---
reviewed: 2026-11-28
see-also: checks_before_cutting, check_reach
---

# Bit fits every corner

![The corners a round bit cannot get into](pictures/checks/missed-spots.svg)

## What it looks for

For Pocket, Profile and 3D Finishing toolpaths, the spots the bit is too wide to get into:
narrow gaps, small details and, on a 3D model, dips narrower than the bit. Ordinary
rounded inside corners are not counted.

## Why it matters

What the bit cannot reach is left uncut, and the part does not come out as drawn.

## How to fix it

Use a smaller bit, or add a second toolpath with a smaller bit to clean up after it
(rest machining). A Pocket's **Clearance Bits** can do this for you.

See also [Checks before cutting](help:checks_before_cutting).
