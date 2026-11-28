---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_makers_figures
---

# Bits from a maker's catalogue

![A generic bit, called out](pictures/checks/checks-found.png)

## What it looks for

Whether each bit comes from a **maker's catalogue** in the Bit Library (it has a
manufacturer, such as Amana or Whiteside), or is a **generic** bit like the ones PowerWhittle
starts with. It also says when a toolpath's bit is no longer in the Bit Library at all.

## Why it matters

A maker's feeds and speeds are worked out for that bit. A generic bit's are only safe
starting values, and the bit in your collet may want something different.

## How to fix it

This is a reminder, not a fault: tick it if you know your bit. To use a maker's figures,
import the maker's tool library (**Import Bits** in the Bit Library), copy your bit into My Bits,
and pick it with the toolpath's **Select…**. Until then, watch the first cut with a hand
near the stop button.

See also [Checks before cutting](help:checks_before_cutting).
