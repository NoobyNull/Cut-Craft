---
reviewed: 2026-11-19
see-also: post_processor_management, job_setup, save_toolpaths, check_rotary_setup
---

# The 4th Axis and Your Firmware

A **4th axis** (A) is a rotary that turns the material while the bit cuts. Your machine can only
use one if the **firmware** on its controller can drive a 4th motor together with X, Y and Z.
Cut Craft asks which firmware you have, so it never writes a file your machine will refuse
part-way through a job.

| Firmware | A axis? |
|---|---|
| **GRBL 1.1 (standard)**, the usual one on an Arduino board | No: X, Y and Z only. A line with an A in it stops the job with `error:20`. |
| **grblHAL** | Yes, if its firmware was built with 4 or more axes. Set `$376` to make A rotary. |
| **FluidNC** | Yes, once an A axis is added to its config file. |
| **Mach3 / Mach4** | Yes, up to 6 axes. |
| **Other** | Taken to have the axes you use. |

## Setting it

![Machine Configuration on standard GRBL: the 4th axis greyed out, with the warning](pictures/concepts/rotary_firmware.png)

1. Open **Machine ▸ Machine Configuration** and pick your machine.
2. Choose its **Firmware**. Until you choose, Cut Craft reads it from the **Controller** box
   (for example "grblHAL" or "FluidNC"). If that names none, it takes **standard GRBL**,
   which has no A axis.
3. Tick **4th axis**. On standard GRBL the tick is greyed out, with this warning:
   *"Standard GRBL supports only 3 axes (X, Y and Z). For an A axis, use custom firmware or
   your controller's own firmware."* Its link takes you to the Firmware choice.
4. Press **Apply**.

Ticking **4th axis** adds Cut Craft's rotary posts for your machine, named **Rotary A, cylinder
along X** and **along Y**. GRBL's are named **grblHAL**, since standard GRBL can't run them.
Choose posts in [Post-Processor Management](help:post_processor_management), and set up a
rotary job in [Job Setup](help:job_setup).

## Finding the centre line

Once the 4th axis is ticked, Machine Configuration shows **Rotary (4th axis)**. Set it once:

![Rotary (4th axis) in Machine Configuration, before the centre line is found](pictures/concepts/rotary_axis_settings.png)

1. **Lies along**: X if the rotary runs left to right on your bed (most kits), Y if front to
   back.
2. Put a straight, round pin (a drill rod or a dowel) in the chuck.
3. Jog the bit (or a probe) to touch one side of the pin, and note the reading across the bed
   (Y for a rotary along X). Touch the other side and note that reading too.
4. Halfway between the two is the centre line. Jog there and set work zero across the bed
   (Y0) there. Then **Centre line Y** is 0. If you zero somewhere else, type the distance from
   your zero to the centre line instead.
5. For Z, zero where Job Setup says: on the axis (touch the top of the pin, then go down by
   half its diameter) or on the top of the cylinder. Nothing to type here: the job knows which.
6. Tick **Centre line found**.
7. Jog A by +10° and watch which way the chuck turns, seen from the chuck end. Set **+A turns**
   to match.
8. Set **A top speed** from the controller's settings, and how far the **Chuck** and
   **Tailstock** reach along the axis.

The [Rotary set up](help:check_rotary_setup) check before cutting asks until **Centre line
found** is ticked.

If you change the firmware back to standard GRBL, the tick is kept but doesn't count, and
[Save Toolpaths](help:save_toolpaths) won't save a rotary job for that machine.
