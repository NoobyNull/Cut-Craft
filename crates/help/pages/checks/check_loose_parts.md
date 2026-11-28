---
reviewed: 2026-11-28
see-also: checks_before_cutting, tabs
---

# Cut-out parts held down

![Tabs holding a part as it is cut out](pictures/checks/loose-parts.svg)

## What it looks for

Profiles that cut right through the material with no [tabs](help:tabs), shapes with
placed tabs that got none, and inlays cut right through (inlays have no tabs).

## Why it matters

A part cut free can be grabbed by the spinning bit and thrown, or pulled into the bit,
spoiling the part and breaking the bit.

## How to fix it

Add tabs in the toolpath's form, or hold the part down another way: tape, screws or a
vacuum table. For an inlay, make the Cut Depth a little less than the material and cut
the thin skin by hand.

See also [Checks before cutting](help:checks_before_cutting).
