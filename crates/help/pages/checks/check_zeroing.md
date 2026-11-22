---
reviewed: 2026-11-22
see-also: checks_before_cutting, setup_material, material_setup
---

# Zero set where the job says

## What it looks for

![Material Setup says where Z zero and X0 Y0 are for this job](pictures/setup_material/step-4.png)

Whether you looked at where this job's zero is. A new job takes its **Z zero** (on top of
the material or on the machine bed) and its **X0 Y0** (a corner or the middle of the board)
from **Edit ▸ Options ▸ Job Defaults**. [Material Setup](help:material_setup) opens by
itself before your first toolpath, so you see them for each job. If it never opened (the
toolpaths came from a template, say), this check says where the job expects the bit to be
zeroed, for you to read and tick.

## Why it matters

The machine cuts every depth from where you zeroed the bit. Zeroed on the bed when the job
expects the top of the board, it cuts in the air; zeroed on top when the job expects the
bed, it plunges deep into the board and the spoilboard. X0 Y0 in the wrong corner puts the
whole job off the board.

## How to fix it

Zero the bit on the machine where the message says. To zero it somewhere else for this job,
open **Material Setup** on the Toolpaths page (or [Setup Material](help:setup_material) ▸
**More settings for this job…**), change **Z Zero** or **XY Datum Position**, and save the
toolpaths again. Once Material Setup has been opened for the job, this check passes.

See also [Checks before cutting](help:checks_before_cutting).
