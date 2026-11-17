---
reviewed: 2026-11-17
see-also: checks_before_cutting, check_tool_numbers
---

# Bits loaded in the tool changer

![The tool changer's slots in Machine Configuration](pictures/checks/tool-changer.png)

## What it looks for

When **Tool changer (ATC)** is chosen under Tool changes, every bit the job uses must be
loaded in one of the tool changer's slots in Machine Configuration. It also warns when
Tool changer is chosen for a machine with no tool changer set up.

## Why it matters

A bit that is not loaded makes the machine stop half-way and ask for it, or, on some
controllers, carry on with whatever bit it is holding.

## How to fix it

Load the bit into a slot in **Machine Configuration**. If you change bits by hand, pick
**Separate file for each tool** or **One file, pause to change the tool** instead. If the
bit is no longer in the Tool Database, pick it again with the toolpath's **Select…**.

Without a tool changer this check says *not checked: you change the bits*.

See also [Checks before cutting](help:checks_before_cutting).
