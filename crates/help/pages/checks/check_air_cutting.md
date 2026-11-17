---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_rapids
---

# Not cutting air

![A second pass over a pocket already cleared](pictures/checks/air-cutting.png)

## What it looks for

While cutting the job in order, how much of each toolpath's cutting is where the material
is already gone. It is mentioned when it is at least 30% of the toolpath and more than
100 mm, with roughly how much time it wastes.

## Why it matters

Cutting air does no harm, but it wastes time: often a toolpath that repeats one before
it, or starts its passes above the material.

## How to fix it

If an earlier toolpath cleared this area, cut less with this one: a smaller area, fewer
passes, or a deeper **Start Depth**. If two toolpaths do the same job, delete one.

See also [Checks before cutting](help:checks_before_cutting).
