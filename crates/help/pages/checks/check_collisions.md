---
reviewed: 2026-11-28
see-also: checks_before_cutting, collisions, check_reach
---

# Collet and spindle clear of the material

![The collet nut running into the wood beside a deep slot](pictures/collet-hits.svg)

## What it looks for

While cutting the job in order, after every move deep enough to matter, whether the parts
above the flutes run into wood that is still there: the plain shank (where it is wider than
the cut), the **collet nut** that holds the bit, and the **spindle** body above it, and
whether they come down over a clamp (a keep-out zone given a height). It uses
the sizes in Machine Configuration ▸ Spindle and Collet Nut, and each bit's **Stick-out**
from the Bit Library.

## Why it matters

Only the flutes cut. A collet nut rubbing the wall of a narrow slot, or a spindle body
dropping onto uncut stock beside a deep profile, can spoil the part, damage the nut, or pull
the bit out of the collet.

## Ticking it

It shows as ⚠ like the other warnings, and you tick it to say you have read it. Unlike the
others, Save Toolpaths and Send to Machine still ask **Save Anyway?** once it is ticked, so a
crash into the work is never saved by a tick alone. Nothing is refused: you decide.

## How to fix it

- Use a bit with longer flutes, or one that sticks out further from the collet nut, and set
  its **Stick-out** in the Bit Library to what you measure.
- Cut less deep in one toolpath, or clear the wood beside the cut first with a wider pocket.
- Check the nut and spindle sizes in Machine Configuration: until you set them, an ER20 nut
  (35 mm across, 20 mm tall) and a 65 mm spindle are assumed.

See [Collet and Holder Collisions](help:collisions) for the sizes in detail, and
[Checks before cutting](help:checks_before_cutting) for the rest of the list.
