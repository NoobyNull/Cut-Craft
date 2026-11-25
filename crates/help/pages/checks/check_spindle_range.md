---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_machine_speeds
---

# Spindle speeds the spindle can turn

![A spindle speed outside the spindle's range](pictures/checks/spindle-range.png)

## What it looks for

Each bit's spindle speed (rpm) against the slowest and fastest the machine's spindle
turns. Lasers are left out: they have no spindle.

## Why it matters

A spindle asked for a speed it cannot do turns at its nearest speed instead. The bit then
takes bigger or smaller bites than planned: too big can break it, too small rubs and burns
the wood.

## How to fix it

Change the spindle speed in the toolpath's bit settings, and the feed rate with it so
each tooth still takes the same bite (the chip load). If it says *not checked*, give the
machine its spindle range in Machine Configuration.

See also [Checks before cutting](help:checks_before_cutting).
