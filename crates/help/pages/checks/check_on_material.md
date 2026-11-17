---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_bed
---

# Stays on the material

![Copies running off the edge of the material](pictures/checks/on-material.png)

## What it looks for

Whether the copies made by **Array Copy Toolpath** stay on the material. Copies are put up
and to the right of the first, however much room there is.

## Why it matters

Copies off the material are cut in the air, or into the spoilboard and clamps beside it.

## How to fix it

Use fewer rows or columns, or less space between them, or make the material bigger in
Job Setup.

See also [Checks before cutting](help:checks_before_cutting).
