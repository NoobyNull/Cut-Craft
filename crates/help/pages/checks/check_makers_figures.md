---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_generic_tool, check_machine_speeds
---

# Feeds and speeds as the maker gives

![A feed rate changed from the maker's figure](pictures/checks/makers-figures.png)

## What it looks for

For a bit from a maker's catalogue, whether the toolpath's feed rate, plunge rate or
spindle speed is more than 10% away from the maker's figure for the material picked in
the Bit Library. It says how far, and which way.

## Why it matters

Faster than the maker says can break the bit or overload the machine. Slower rubs, heats
the bit and burns the wood. Either may be what you want, but it should never be a typing
slip.

## How to fix it

If you meant it, tick it. If not, pick the bit again with the toolpath's **Select…**: the
maker's figures come back.

See also [Checks before cutting](help:checks_before_cutting).
