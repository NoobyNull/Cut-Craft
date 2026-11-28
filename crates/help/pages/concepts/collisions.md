---
reviewed: 2026-11-17
see-also: tool_database, save_toolpaths, keep_out_zones, toolpaths, check_collisions
---

# Collet and Holder Collisions

Only the bit's flutes cut. Above them come the plain shank, the **collet nut** that holds
the bit, and the **spindle** (or router) body. On a deep cut any of them can run into wood
still standing beside the cut, such as the wall round a narrow slot or uncut stock round a
deep profile. That can ruin the part, damage the nut, or pull the bit out of the collet.

PowerWhittle checks for this for you. It cuts every toolpath in order on the same material as
the preview, so wood that an earlier toolpath already cleared away does not count, and
wood left standing does.

![Left: the collet nut sinks into the wood beside a deep, narrow cut and the wood in its way is red. Right: the same cut with the wood round it cleared first, so nothing hits](pictures/collet-hits.png)

## What you see

- **Preview**: after Preview All (or any preview), the wood in the way of the collet nut,
  spindle or shank is painted **red** in the 3D view, and the status bar says how many
  toolpaths hit.
- **Toolpaths list**: each toolpath that hits shows **⚠** and its name in red. Hover over
  it to read what hits, where (X and Y), and how to fix it.
- **Save Toolpaths and Send to Machine**: the checks list it as **⚠ Check** and always ask
  "Save the toolpaths anyway?" before writing the file. This question cannot be switched
  off in Options, because a collision can break things.

Nothing is refused: the toolpath is still made and you can still save it once you have
read the warning.

## Telling PowerWhittle your sizes

The check is only as good as the sizes it knows. Measure them once with a ruler.

1. Open **Machine ▸ Machine Configuration** and click **Spindle and Collet Nut** to open it. Set
   **Collet nut across** (an ER20 nut is about 35 mm, a trim router's about 25 mm),
   **Collet nut height** (from the nut's bottom face up to the spindle body) and
   **Spindle across** (65 mm or 80 mm for most spindles).
2. Open **Toolpaths ▸ Bit Library**, pick each bit you use for deep cuts, and set its
   **Stick-out**: how far the bit pokes out below the collet nut, from its tip up to the
   nut's face, once it is tightened in.
3. Set the bit's **Flute length** and **Shank diameter** too, so a narrow bit on a wider
   shank (a 1/8 in bit with a 1/4 in shank) is checked for its shank rubbing.

Until you set a bit's Stick-out, PowerWhittle guesses it, and the warning says so. The guess
is the length that cuts (the flutes, or the V's height on a V-bit), plus the neck if the bit
has one, plus 5 mm of shank showing. A bit with none of these is taken as 25 mm. The check
uses this guess to decide how high the nut sits, so measure and set the real figure.

### Tool changers: Holder per bit

On a machine with a tool changer, each bit sits in a holder of its own. Tick
**Holder per bit** in Machine Configuration, then open the bit's **Own holder** in the
Bit Library and give it its own collet nut and spindle sizes. A bit without its own
holder uses the machine's. With Holder per bit not ticked, every bit uses the machine's.

## Putting it right

- **Clear the wood round the cut first**: a wider pocket or clearing toolpath before the
  deep one leaves room for the nut, as in the picture.
- **Let the bit stick out further** (keep at least three quarters of the shank in the
  collet), then set the new Stick-out in the Bit Library.
- **Use a longer bit**, or cut less deep.
- For a shank that rubs: use a bit whose flutes are as long as the cut is deep.

## Clamps

Keep-out zones already keep the bit away from your clamps, but the collet nut and spindle
are much wider than the bit. Beside a deep cut they can come down over a clamp the bit
never touches. So each keep-out zone has a height: how tall the clamp stands above the
top of the material (measure to its highest point; a screw head counts).

- For clamps on this job, set **Clamp Height** in **Keep-Out Zones** when you make the
  zones.
- For clamps fixed to the bed, set **Tall** beside each zone in **Machine Configuration ▸
  Keep-Out Zones**.

A zone with no height is taken as 20 mm tall, and the warning says so. When the nut or
spindle would come down lower than the clamp's top while over it, the toolpath is marked ⚠
just as for the material, saying which clamp and where. Move the clamp further from the
cut, or let the bit stick out further.

See also [Keep-Out Zones](help:keep_out_zones), [Checks before cutting](help:toolpaths)
and [Save Toolpaths](help:save_toolpaths).
