---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_spindle_range, check_makers_figures
---

# Feeds the machine can reach

![A feed rate faster than the machine moves](pictures/checks/machine-speeds.png)

## What it looks for

Each toolpath's feed rate against the fastest the machine moves in X and Y, and its plunge
rate against the fastest it moves in Z, as Machine Configuration has them.

## Why it matters

The machine cannot go faster than it can, so it cuts slower than planned. The job takes
longer than the time shown, and the chips come out thinner than the bit wants, which rubs
and heats it.

## How to fix it

Lower the feed or plunge rate in the toolpath's tool settings to what the machine can do.
If it says *not checked*, give the machine its top speeds in Machine Configuration.

See also [Checks before cutting](help:checks_before_cutting).
