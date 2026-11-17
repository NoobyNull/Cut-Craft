---
reviewed: 2026-11-17
see-also: checks_before_cutting
---

# Toolpaths up to date

![A toolpath marked out of date in the list](pictures/checks/up-to-date.png)

## What it looks for

Whether each toolpath was made from the vectors and settings as they are now. A toolpath
goes **out of date** when you move, resize or edit what it was made from after making it.
It is marked ⚠ in the toolpath list.

## Why it matters

An out-of-date toolpath still cuts the old shape: the part comes out where it used to be,
or the size it used to be.

## How to fix it

Press **Recalculate All Toolpaths** on the Analyse tab, or open the toolpath and press
**Calculate**.

See also [Checks before cutting](help:checks_before_cutting).
