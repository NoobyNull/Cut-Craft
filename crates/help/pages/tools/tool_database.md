---
reviewed: 2026-11-17
see-also: pocket_toolpath, profile_toolpath, stepover, collisions
---

## Steps

![The steps, one after another](pictures/tool_database/tour.gif)

1. The Tool Database keeps your bits, each with its size, shape, feeds and speeds. On the Toolpaths page, press **Tool Database** (under Job and Tools).

   ![Step 1](pictures/tool_database/step-1.png)

2. **My Tools** are the bits you own; the makers' catalogues below are for copying from. Click a bit to see its drawing and settings, or add your own. Every toolpath's **Select…** picks from My Tools, and brings that bit's feeds, speeds and depth per pass with it.

   ![Step 2](pictures/tool_database/step-2.png)

Each bit's stepover and depth per pass decide how fast it cuts: see [what stepover is](help:stepover).

## Stick-out and Own holder

Under **Tool Shape**, **Stick-out** is how far the bit pokes out below the collet nut, from
its tip up to the nut's face. Measure it once the bit is tightened in. Until you set it,
Cut Craft guesses it from the length that cuts (flutes, or a V-bit's V height) plus any neck
plus 5 mm, or 25 mm when none of those are set, and says so.
On a deep cut the collet nut or spindle can hit wood beside the cut: see
[Collet and Holder Collisions](help:collisions).

**Own holder** (folded below it) gives this bit its own collet nut and spindle sizes. It is
only used on a machine with **Holder per bit** ticked in Machine Configuration, such as one
with a tool changer; otherwise every bit uses the machine's sizes.
