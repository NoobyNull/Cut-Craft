---
reviewed: 2026-11-23
see-also: first_time_setup, pocket_toolpath, profile_toolpath, stepover, collisions, check_tool_numbers, check_makers_figures
---

## Steps

![The steps, one after another](pictures/tool_database/tour.gif)

1. The Bit Library keeps your bits, each with its size, shape, feeds and speeds. On the Toolpaths page, press **Bit Library** (under Job and Bits).

   ![Step 1](pictures/tool_database/step-1.png)

2. **My Bits** are the bits you own; the makers' catalogues below are for copying from. Click a bit to see its drawing and settings, or add your own. Every toolpath's **Select…** picks from My Bits, and brings that bit's feeds, speeds and depth per pass with it. A laser toolpath's **Select…** lists only lasers, and every other toolpath's lists only bits.

   ![Step 2](pictures/tool_database/step-2.png)

Each bit's stepover and depth per pass decide how fast it cuts: see [what stepover is](help:stepover).

## Bit numbers

A tool changer picks bits by their **Bit Number** (T1, T2...). Give each bit its own number;
**0** means you change the bit by hand, and any number of bits may share 0. When you type a
number another bit already has, the Bit Library says so in red under it. Nothing is
refused, and the checks before cutting call it out again when you save
([Each bit its own number](help:check_tool_numbers)).

![The red note under a bit number another bit has](pictures/checks/tool-number-taken.png)

## A maker's feeds and speeds

A bit copied from a maker's catalogue keeps the maker's feeds and speeds. When a toolpath's
feed rate, plunge rate or spindle speed ends up more than a tenth away from them, the checks
before cutting ask you to tick it, so a typing slip is not cut
([Feeds and speeds as the maker gives](help:check_makers_figures)).

## Stick-out and Own holder

Under **Bit Shape**, **Stick-out** is how far the bit pokes out below the collet nut, from
its tip up to the nut's face. Measure it once the bit is tightened in. Until you set it,
Cut Craft guesses it from the length that cuts (flutes, or a V-bit's V height) plus any neck
plus 5 mm, or 25 mm when none of those are set, and says so.
On a deep cut the collet nut or spindle can hit wood beside the cut: see
[Collet and Holder Collisions](help:collisions).

**Own holder** (folded below it) gives this bit its own collet nut and spindle sizes. It is
only used on a machine with **Holder per bit** ticked in Machine Configuration, such as one
with a tool changer; otherwise every bit uses the machine's sizes.
