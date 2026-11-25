---
reviewed: 2026-11-25
see-also: checks_before_cutting, check_z_travel, protect_the_machine
---

# Fits on the machine's bed

![Toolpaths wider than the bed](pictures/checks/bed.png)

## What it looks for

How far across (X) and along (Y) the toolpaths go, against the bed size of the machine
picked in **Machine Configuration**. In a tiled job, each tile is checked on its own,
since each is saved as its own file.

## Why it matters

A job bigger than the bed runs the machine into its end stops part-way through. It
loses its place, and everything after that is cut in the wrong spot.

## How to fix it

- Make the job smaller, or split it with **Toolpaths ▸ Toolpath Tiling**.
- Check the right machine is picked at the top of Save Toolpaths.
- If it says *not checked*, give the machine its bed size in Machine Configuration.

This is **⛔ Could damage the machine**: never saved, sent or made through the Web API while
it is so, with no tick and no Save Anyway. The only way past is switching **Bed size** off
in the machine's [Protect the Machine](help:protect_the_machine).

See also [Checks before cutting](help:checks_before_cutting).
