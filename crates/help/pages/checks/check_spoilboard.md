---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_reach
---

# Not cutting into the spoilboard

![A cut going well below the material](pictures/checks/spoilboard.png)

## What it looks for

How far below the bottom of the material the deepest cut goes. Up to 1 mm is allowed,
since cutting a part out cleanly goes a little into the spoilboard on purpose. Flattening
the spoilboard itself (Surfacing, Spoilboard) is a reminder, not a problem.

## Why it matters

Cutting deep into the spoilboard (the board under your work) wears it away, and if it is
thin, cuts into the machine's bed. It usually means the material thickness or the cut
depth is wrong.

## How to fix it

Measure the material and check the **Thickness** in Job Setup, then the toolpath's
**Cut Depth**.

See also [Checks before cutting](help:checks_before_cutting).
