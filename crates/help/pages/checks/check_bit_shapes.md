---
reviewed: 2026-11-27
see-also: checks_before_cutting, check_tool_changer, check_bit_changes, tool_database
---

# Each toolpath cut with its own bit

## What it looks for

A job keeps a full copy of every toolpath's bit: its type, diameter, angle, tip radius,
taper, flute length, shank and feeds. It opens and cuts the same on another computer, even
one whose Bit Library has never seen the bit. But bits are changed and loaded **by name**,
so this check looks for a name that stands for two different bits:

- **The Bit Library's bit of the same name has another shape.** The toolpath was made for a
  6 mm end mill, say, and this computer's Bit Library has a 1/4" one under the same name, or
  the bit was edited after the toolpath was made.
- **A tool changer holder has that other bit.** The holder is loaded by name, so the machine
  would pick up the Bit Library's bit for a toolpath made for another. This one must be
  ticked and saved with Save Anyway.
- **Two toolpaths use different bits with one name.** A bit change goes in only where the
  name changes, so the second toolpath would be cut with the first one's bit. This one must
  be ticked and saved with Save Anyway too.

Names, feeds, speeds and bit numbers are not compared, only the shape: a size one of them was
saved without (a flute length from before flute lengths were kept) does not count.

## Why it matters

A toolpath is worked out for its bit's size and shape. Cut with a wider bit, a profile cuts
the part too small; with a V-bit of another angle, a V-carving goes too deep or too shallow.

## How to fix it

- If the toolpath's own bit is right, put that bit in the machine. Where a tool changer
  holds the other one, give one of the two bits another name in the
  [Bit Library](help:tool_database) and load the right one.
- If the Bit Library's bit is right, open the toolpath, press **Select…** on its Bit and pick
  it again, then press **Calculate**. The form says when its bit is not the Bit Library's,
  with **Use the Bit Library's** to do this in one go.
- For two toolpaths with different bits of one name, rename one of the bits in the Bit
  Library, pick it again in that toolpath, and press Calculate.

See also [Checks before cutting](help:checks_before_cutting).
