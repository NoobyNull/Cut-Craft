---
reviewed: 2026-11-23
see-also: save_toolpaths, check_tool_changer, job_setup, rotary_firmware
---

A **post-processor** (post for short) turns your toolpaths into G-code written the way your
machine's controller reads it. PowerWhittle comes with many: one for each controller, in
millimetres or inches, with arcs, straight lines only, or a 4th axis (rotary). The filters at
the top of **Machine ▸ Post-Processor Management** narrow the list to the ones that suit you.

## Finding your post

![Post-Processor Management, filtered to millimetre posts with arcs or straight lines](pictures/post_processor_management/filters.png)

1. Open **Machine ▸ Post-Processor Management**. The filters start from your machine: its
   units (**mm** or **inch**), **Arcs** and **Straight** ticked, and **Rotary (A)** ticked only
   if the machine has a 4th axis.
2. Tick or untick the filters. A post shows when its units **and** its moves are ticked:
   - **mm** / **inch**: the units the G-code is written in. Use the units your controller is
     set to.
   - **Arcs**: curves written as arcs (G2 / G3). Smaller files and smoother curves. Pick these
     if your controller does arcs (grbl, grblHAL, FluidNC and Mach3 all do).
   - **Straight**: lines only (G1), for controllers that can't do arcs.
   - **Rotary (A)**: for a 4th axis. The side that goes round the cylinder is written as A, in
     degrees. These posts are named **Rotary A, cylinder along X** (or **Y**): pick the one
     that matches which way the rotary lies on your bed. **G93** at a name's end means its
     feeds are in inverse time. Standard GRBL has no A axis, so GRBL's rotary posts are named
     **grblHAL** (see [The 4th axis and your firmware](help:rotary_firmware)).
3. Pick your controller in **Any controller** to see only its posts, or type part of a name
   in **Search posts…**.
4. The posts are grouped: **Installed by you** first, then PowerWhittle's own under their
   controller. Hover a post to read what it is for.
5. Click a post and press **Select** to make it the one your machine saves toolpaths through.

If nothing is left, the list says so: tick more filters, or press **Show All**. The filters
start afresh each time the window opens.

![The same window with only Rotary (A) ticked and GRBL / grblHAL picked](pictures/post_processor_management/rotary.png)

The same filters sit at the top of **Add a post-processor** in **Machine ▸ Machine
Configuration**, so you can find a post to add to a machine the same way.

## A post from somewhere else

A post you install (**Install Post-Processor**) is sorted by what its own lines say: its
units, whether it has arc moves, whether it wraps onto a rotary axis, and the controller its
name or notes mention. If that is wrong, or it names no controller (it then shows under
**Other**), correct it:

![Edit Capabilities: the controller, units and moves the filters see](pictures/post_processor_management/capabilities.png)

1. Right-click the post, or open its **Inspect ▾**, and choose **Edit Capabilities…**.
2. Pick its **Controller**, **Units** and **Moves**, and tick **Inverse-time feeds (G93)** for a
   rotary post that uses them. Add a line under **What it is for**; it shows when you hover
   the post.
3. Press **Save**. **Read From the Post** forgets the correction.

This only changes how the filters see the post, never the post itself. Corrections are kept
in `capabilities.json` in your custom posts' folder.

Saving with the post your machine uses is covered in [Save Toolpaths](help:save_toolpaths); a
rotary job is set up in [Job Setup](help:job_setup).
