---
reviewed: 2026-11-25
see-also: checks_before_cutting, check_reach, protect_the_machine
---

# Not cutting into the spoilboard

![A cut going well below the material](pictures/checks/spoilboard.png)

## What it looks for

How far below the bottom of the material the deepest cut goes, against the machine's
**Allowed into spoilboard** depth (1 mm unless set). Cutting a part out cleanly goes a
little into the spoilboard on purpose. Flattening the spoilboard itself (Surfacing,
Spoilboard) is a reminder, not a problem.

- Deeper than the depth allowed: **⛔ Could damage the machine**, never saved, with no tick.
- More than 1 mm in but within the depth allowed: a ⚠ to read and tick, unless the
  machine's **Warn about cuts more than 1 mm into the spoilboard** is unticked.

Both are set per machine in [Protect the Machine](help:protect_the_machine), where
**Spoilboard depth** can also be switched off.

## Why it matters

Cutting deep into the spoilboard (the board under your work) wears it away, and if it is
thin, cuts into the machine's bed. It usually means the material thickness or the cut
depth is wrong.

## How to fix it

Measure the material and check the **Thickness** in Job Setup, then the toolpath's
**Cut Depth**.

See also [Checks before cutting](help:checks_before_cutting).
