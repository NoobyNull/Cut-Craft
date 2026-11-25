---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_spoilboard
---

# Bit long enough for the depth

![Flutes shorter than the cut](pictures/checks/reach.png)

## What it looks for

How deep each toolpath cuts against how deep its bit can cut: its **Flute Length**, or for
a V-bit the height of its V. A bit with no flute length set is taken to have flutes as
long as any cut, so it is never flagged: set its real Flute Length to have it checked.

## Why it matters

Past its flutes, the bit's smooth shank rubs on the wood. It heats, burns the wood and
can snap the bit.

## How to fix it

- Pick a bit with longer flutes, or cut less deep.
- Set the bit's real **Flute Length** in the Bit Library (measure the cutting part).
- For a V-bit, a deeper cut needs a wider V-bit.

See also [Checks before cutting](help:checks_before_cutting).
