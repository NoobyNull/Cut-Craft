---
reviewed: 2026-11-19
see-also: post_processor_management, job_setup, save_toolpaths
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

If you change the firmware back to standard GRBL, the tick is kept but doesn't count, and
[Save Toolpaths](help:save_toolpaths) won't save a rotary job for that machine.
