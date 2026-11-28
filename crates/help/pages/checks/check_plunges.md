---
reviewed: 2026-11-28
see-also: checks_before_cutting, check_drilling
---

# Going down into the material

![A ramp in, and a straight plunge](pictures/checks/plunges.svg)

## What it looks for

Where a toolpath had to go down into the material another way than its **Entry** setting
asked, there being no room for the ramp or helix: a straight plunge, or a zig-zag. It
says where, by X and Y.

## Why it matters

A straight plunge is hard on the bit, and a bit that does not cut at its centre cannot do
it at all: it burns or snaps. On a V-carve, short straight plunges are usual, and the
message says there is nothing to fix.

## How to fix it

Use a smaller bit, which has room to ramp, or check the shape is meant to be that small.
To change how the bit goes in, open the toolpath's **Entry** settings.

See also [Checks before cutting](help:checks_before_cutting).
