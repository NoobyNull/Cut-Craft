---
reviewed: 2026-11-24
see-also: toolpaths, save_toolpaths
---

# Checks before cutting

![Save Toolpaths checking each thing in turn](pictures/checks/checks-running.png)

Before Cut Craft saves your toolpaths or sends them to the machine, it checks them for the
things that break bits, spoil the work or stop the machine half-way. It does this in
**Save Toolpaths** and on the Toolpaths page's **Analyse** tab (the **Checks** tab in the
Rail and Ribbon [window layouts](help:settings)).

While the checks run, every check is listed by name:

- ○ waiting its turn
- a spinner while it runs
- a grey ✔ once it has been checked

A big 3D toolpath takes a little while, because Cut Craft cuts the whole job in order from
a block of material to look for crashes. **Cancel** stops it; **Check Again** runs it again.

## When the checks are done

![What the checks found](pictures/checks/checks-found.png)

The line at the top counts them up, such as *✔ 15 passed · ⚠ 3 to read · 2 not checked*.
Each check then shows how it came out, with what needs you first:

- **✖ Must fix** (red): the toolpaths are not saved until it is put right, such as a rapid
  move that would crash into the material.
- **⚠ To read** (amber): something you should know before cutting. Read it and **tick it**.
- **✔ Passed** (green): nothing found.
- **– Not checked** (grey): the check could not be made, and it says why, such as *Shop
  Router has no Z travel in Machine Configuration*. Fill in what it asks for and it is
  checked next time.

Point at a check's name to see what it protects you from. Press its **?** for its own page,
with how to fix what it found.

## Ticking each warning

![Every warning ticked, Save Toolpaths ready](pictures/checks/checks-ticked.png)

**Save Toolpaths...** and **Send to Machine** stay greyed until every ⚠ is ticked. Ticking
says *I have read this, and it is what I want*. Nothing is refused for a warning: you
decide. The ticks cannot be switched off, so nothing slips past unread. (Earlier versions
had a *Save and tell* choice in Options ▸ Confirmations that saved without asking; it is
gone, and options saved with it still open.)

A tick belongs to what the warning says. If you change the job and the same warning comes
back, it stays ticked; a new or changed warning needs a new tick.

## Every check

**Can it cut?**

- [Settings that can cut](help:check_can_cut)
- [Toolpaths up to date](help:check_up_to_date)

**Does it fit the machine?**

- [Zero set where the job says](help:check_zeroing)
- [Fits on the machine's bed](help:check_bed)
- [Fits the Z axis travel](help:check_z_travel)
- [Feeds the machine can reach](help:check_machine_speeds)
- [Spindle speeds the spindle can turn](help:check_spindle_range)
- [Bits loaded in the tool changer](help:check_tool_changer)
- [Each bit its own tool number](help:check_tool_numbers)
- [Bit changes you can make](help:check_bit_changes)
- [Keeps out of keep-out zones](help:check_keep_out)
- [Rotary set up](help:check_rotary_setup)

**Cutting the job in order**

- [Rapid moves clear of the material](help:check_rapids)
- [Not cutting air](help:check_air_cutting)
- [Collet and spindle clear of the material](help:check_collisions)

**The bit and the cut**

- [Bit long enough for the depth](help:check_reach)
- [Not cutting into the spoilboard](help:check_spoilboard)
- [Going down into the material](help:check_plunges)
- [Drills and holes](help:check_drilling)
- [Bit fits every corner](help:check_missed_spots)
- [Cut-out parts held down](help:check_loose_parts)
- [Stays on the material](help:check_on_material)

**Feeds and speeds**

- [Bits from a maker's catalogue](help:check_generic_tool)
- [Feeds and speeds as the maker gives](help:check_makers_figures)

## What the checks do not do

They read your toolpaths and settings; they do not see your machine. Always check that the
material is held down, the right bit is in the collet, and X, Y and Z zero are set where the
job expects them.
