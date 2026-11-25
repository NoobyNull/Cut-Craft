---
reviewed: 2026-11-25
see-also: toolpaths, save_toolpaths, protect_the_machine
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

- **⛔ Could damage the machine** (red): a cut into a keep-out zone, past the bed or the Z
  travel, or too deep into the spoilboard. It has **no tick**: nothing is saved or sent
  while it is so. Put it right, or see [Protect the Machine](help:protect_the_machine).
- **✖ Must fix** (red): something that breaks a bit or stops the job, such
  as a rapid move that would crash into the material. Put it right if you can. If you
  mean it, **tick it** and save anyway (see below).
- **⚠ To read** (amber): something you should know before cutting. Read it and **tick it**.
- **✔ Passed** (green): nothing found.
- **– Not checked** (grey): the check could not be made, and it says why, such as *Shop
  Router has no Z travel in Machine Configuration*. Fill in what it asks for and it is
  checked next time.

Point at a check's name to see what it protects you from. Press its **?** for its own page,
with how to fix what it found.

## Ticking each warning and error

![Every warning ticked, Save Toolpaths ready](pictures/checks/checks-ticked.png)

**Save Toolpaths...** and **Send to Machine** stay greyed until every ✖ and ⚠ is ticked.
Ticking says *I have read this, and it is what I want*: you decide. Only a ⛔ Could
damage the machine is refused, whatever is ticked.

## Saving with errors

![Save Anyway with an error to tick](pictures/checks/save-anyway-errors.png)

A ✖ Must fix ticked in the list is not the end of it. Every time you save or send,
**Save Anyway** asks again and lists:

1. A red line at the top, *The job as it stands will not run on the machine*, when it would
   stop on the machine as saved (a toolpath with no feed rate or plunge rate, say).
2. Each error in red with a tick box of its own. **Save Anyway** stays greyed until each
   one is ticked. The ones you ticked in the list come ticked.
3. The warnings, to read.

Press **Save Anyway** (or **Send Anyway**) to save, or **Go Back** (**Don't Save** when
the job would still run) to put it right first. The answer is never remembered: the next
save asks again.

A file saved past errors says so at the top, before its first command, in comments the
machine skips:

```
; SAVED WITH ERRORS: the checks before cutting found these, and Save Anyway was chosen
; Profile: End Mill has no feed rate, so the machine would not move while cutting...
```

So whoever runs the file later sees what it was saved with.

A toolpath with no moves at all has nothing to write: it is left out of the files, and the
status bar names it once saved. The ticks cannot be switched off, so nothing slips past unread. (Earlier versions
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
- [Each bit its own number](help:check_tool_numbers)
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
