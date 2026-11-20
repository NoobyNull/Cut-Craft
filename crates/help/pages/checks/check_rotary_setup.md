---
reviewed: 2026-11-20
see-also: checks_before_cutting, rotary_firmware, job_setup, rotary_machining_toolpath
---

# Rotary set up

![Seen from the chuck end: the bit right over the turning axis, and the bit off to one side](pictures/checks/rotary-setup.png)

## What it looks for

For a **rotary job** (a cylinder turned by the 4th axis), whether the machine's rotary is set
up in **Machine ▸ Machine Configuration ▸ Rotary (4th axis)**:

- **Centre line found** is ticked, so the app knows where the turning axis is. It matters most
  for [Off centre and Indexed](help:rotary_machining_toolpath) toolpaths, which hold the bit a
  measured distance from the axis.
- The rotary **lies along** the same direction (X or Y) as the job's cylinder in Job Setup.
- When **A can turn** is **Freely**, the toolpaths don't wind A more than 500 turns one way.
- When **A can turn** is **Up to … turns each way**, no single pass (the cutting between two
  lifts) turns A further than that. One that does is asked about before every save, like a
  collision, because a cable or hose on the chuck would be wound past its limit: allow more
  turns, or choose **Freely** if nothing stops the chuck.
- A [4-Axis Rotary Machining](help:rotary_machining_toolpath) job cuts a **registration mark**
  at A=0, so the part can go back exactly as it was if the job is started again.
- The material has **room to turn**: a square's corners or a board's edges swing out further
  than its top, and must clear the spoilboard below (**Axis height**) and the bit lifted all the
  way up (**Room above**). See [Room to turn](help:rotary_firmware).
- No toolpath **turns A with the bit lower** than the material swings, and none lifts the bit
  higher than Z can go.

## Why it matters

A rotary job cuts round the turning axis. If the bit is not right over that axis, every
cut is a little off to one side, and a round carving comes out lumpy or lopsided. If the
rotary lies the other way from the job, the carving is wrapped the wrong way round.

Many controllers keep A's position as a single-precision number. Hundreds of turns out, it
can only be placed to about a hundredth of a degree, and coarser further on, so fine detail is
lost. A registration mark lets you line the part up again after a stop.

## How to fix it

1. Find the centre line on your machine, as shown in
   [The 4th Axis and Your Firmware](help:rotary_firmware).
2. In Machine Configuration, open **Rotary (4th axis)**, type the two **Centre line** values
   and tick **Centre line found**.
3. If the job lies the other way, change the cylinder's direction in
   [Job Setup](help:job_setup), or **Lies along** in Machine Configuration.
4. If A winds too far, set **A can turn** to **Up to** a few turns each way (Machine
   Configuration ▸ Rotary).
5. If the material hits the spoilboard, take the spoilboard out from under the rotary and tick
   **Taken out**, or use smaller material. Measure **Axis height** and **Room above** first if
   they are 0.
6. If a toolpath turns A too low, raise **Clearance (Z1)** in Job Setup to the height the
   warning gives, and calculate the toolpath again.
7. For a registration mark, tick **Registration mark** in 4-Axis Rotary Machining, or pencil a
   line along the material at A=0 before you start.

This is a **warning**: you can still save, once you have checked.

See also [Checks before cutting](help:checks_before_cutting).
