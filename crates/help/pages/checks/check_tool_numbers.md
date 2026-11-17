---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_tool_changer
---

# Each bit its own tool number

![The Tool Database warns when a tool number is taken](pictures/checks/tool-number-taken.png)

## What it looks for

Whether two **different** bits in the job have the same tool number. The same bit used by
several toolpaths may of course keep its number. **Tool number 0** means the bit is changed
by hand, so any number of bits may share 0.

## Why it matters

A tool changer picks bits by number (T1, T2...). Two bits with one number means one of
them is cut with the wrong bit: the wrong size, the wrong shape, the wrong depth.

## How to fix it

Open the **Tool Database**, pick one of the bits, and give it a **Tool Number** of its
own, or 0 if you change it by hand. The Tool Database says in red when you type a number
another bit already has. A bit loaded in a tool changer takes its slot's number: move it to
another slot instead.

See also [Checks before cutting](help:checks_before_cutting).
