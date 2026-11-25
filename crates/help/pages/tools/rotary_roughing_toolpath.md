---
reviewed: 2026-11-25
see-also: rotary_finishing_toolpath, rotary_locked_angles_toolpath, rotary_machining_toolpath, rotary_lathe_toolpath, rotary_firmware, job_setup, collisions, save_toolpaths, tool_database
---

**Rotary Roughing** takes the bulk off a carved part on a rotary job: a level at a time from the
outside in, all the way round at each level before the next, down to a little above the model.
The finish after it then has only a thin, even layer left to take off.

## Steps

1. Start a **rotary** job (see [Job Setup](help:job_setup)) and put the model in it.
2. On the Toolpaths page, press **Rotary Roughing** (under 3D Toolpaths). It shows once
   **4th axis** is ticked in Machine Configuration (see
   [The 4th axis and your firmware](help:rotary_firmware)).
3. Pick a square end mill: it clears the bulk quickest. Its **pass depth** (in the
   [Bit Library](help:tool_database)) is how deep each level goes, and its **stepover** how
   far apart the passes are at the outside of the blank.
4. Choose the **Direction** (below) and the **Cut direction**.
5. Set the **Allowance**: how much is left on the model for the finish.
6. Press **Calculate**.

When it looks right, finish the part with [Rotary Finishing](help:rotary_finishing_toolpath),
then [save them for your machine](help:save_toolpaths) through a **Rotary A** post.

## Direction

- **Along the part** (the usual): the bit runs the whole length of the part with A held still,
  then A turns a step and it runs back, like a typewriter. Best for clearing the bulk, and for
  long parts. Some call this axial roughing.
- **Round the part**: A turns the bit all the way round the part, then it steps along. Best for
  short parts and round shapes. Some call this radial roughing.

**Cut direction** says which way the bit's edge meets the material, with the spindle turning
clockwise seen from above:

- **Both** (the usual): back and forth, the bit staying down between passes. The quickest.
- **Climb**: every pass the same way, the edge biting into the material as it comes to it. A
  cleaner cut on most woods; the bit lifts back to start each pass.
- **Conventional**: every pass the same way the other way round. Gentler on a light machine with
  some play in it.

## It cuts only what is left

The toolpath list is the cutting order. Rotary Roughing works out what the toolpaths **above it**
leave of the blank, by cutting their moves from it in order, and cuts only that. Put a
[Rotary Lathe](help:rotary_lathe_toolpath) Turn above it, and the roughing starts where the
turning stopped; put a second Rotary Roughing with a smaller bit or allowance under the first,
and it takes off only the steps the first one's levels left.

If nothing is left to cut, it says so in red. When a toolpath above it is calculated again, or
the list's order changes above it, it goes out of date (⚠) until it is recalculated: moving
toolpaths in the list asks first, with **Put it back** to undo the move.

**Advanced rotary mode** (Edit ▸ Options ▸ Toolpath Settings, or **Toolpaths ▸ Turn On Advanced
Rotary Mode...**, which opens it with the setting ringed) turns this off for every rotary
toolpath: each then cuts the whole blank, as if nothing were above it, and you decide what it
cuts with its **Start**, **End** and **Allowance**.

## Settings

- **Tool**: an end mill (above).
- **Direction** and **Cut direction** (above).
- **Allowance**: material left on the model, all the way round, for the finish to take off.
- **Start** and **End**: where along the axis it cuts, from the job's start (the chuck end).
  End 0 runs to the job's end.
- **Machining limit**: where on the material it may cut. **Material** (the usual) is all of
  it; **Model**, a **Selected Level** or **Selected Vector(s)** keep it within their outline,
  grown by **Boundary Offset**. Vectors drawn on the job wrap round the part, just as the job
  does. Selected vectors are kept with the toolpath, so it recalculates the same.

## What it keeps clear of

It never cuts air. It keeps out of the **Chuck** and **Tailstock** set in
[Machine Configuration](help:rotary_firmware), and keeps the collet nut and spindle (see
[Collisions](help:collisions)) clear of the material beside the cut. Where they would not let the
bit down as far as it should go, it stops short there and says so in red: rough the material
beside it down first, or use a bit that sticks out further. Material outside the
**Machining limit** is never cut, so the holder keeps clear of it too, all the way round.
