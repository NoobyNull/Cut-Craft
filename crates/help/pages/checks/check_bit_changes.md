---
reviewed: 2026-11-24
see-also: checks_before_cutting, save_toolpaths, check_tool_changer, check_zeroing
---

# Bit changes you can make

## What it looks for

Jobs with more than one bit, when the bits are changed by hand rather than by a tool
changer:

- **Z zero again with each bit.** It reminds you, every time, how many bit changes there
  are and in what order (for example "End Mill, then V-Bit, then End Mill").
- **A pause you can zero at.** If you save one file that pauses at each change on a machine
  with standard GRBL, it warns that GRBL cannot jog while paused.
- **The cutting order.** If **Group by bit** is on in Machine Configuration, it lists the
  order the toolpaths will really be cut in.

## Why it matters

A new bit never sits at quite the same height in the collet. Every cut after the change is
measured from where you set Z zero, so a bit you forget to zero cuts too deep or in the air.

Standard GRBL holds at `M0` and will not jog until you resume. You cannot bring the new bit
down to the material to zero it, so a file that pauses is hard to use there.

Group by bit saves bit changes, but it can cut a part free before the work inside it is done.

## How to fix it

- Read the bit order, tick the warning, and set Z zero the same way as the first bit after
  each change (Job Setup says where: on top of the material or on the bed).
- On standard GRBL, choose **Separate file for each tool** in Save Toolpaths. To make that
  the default, use **Machine ▸ Machine Configuration ▸ Bit Changes**. If your controller is
  not standard GRBL, pick its **Firmware** there instead.
- If Group by bit puts a cut-out before the cuts inside it, turn **Group by bit** off, or
  give the cut-out its own bit.

## Settings for bit changes

**Machine ▸ Machine Configuration ▸ Bit Changes** (for a machine without a tool changer):

- **A separate file for each bit** (the default) or **One file that pauses at each bit
  change**: what Save Toolpaths starts on for this machine.
- **Go to a bit change position at each pause**: the head lifts and goes to the X and Y you
  give (from the job's X0 Y0), out of the way, before it stops. Z is the height above the
  material, and it is never lower than the safe Z. Off, the bit only lifts to the safe Z.
- **Spindle spin-up after a pause**: how many seconds the spindle gets to reach speed once
  you resume. 0 keeps the post's own wait (3 seconds in Cut Craft's posts).
- **Group by bit**: off by default, so the toolpaths are cut in list order.

See also [Checks before cutting](help:checks_before_cutting) and
[Save Toolpaths](help:save_toolpaths).
