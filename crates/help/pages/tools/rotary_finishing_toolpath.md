---
reviewed: 2026-11-25
see-also: rotary_roughing_toolpath, rotary_locked_angles_toolpath, rotary_machining_toolpath, rotary_lathe_toolpath, rotary_firmware, job_setup, collisions, save_toolpaths, tool_database
---

**Rotary Finishing** is the last pass over a carved part on a rotary job: the bit stays on the
model all the way round, as low as it goes without touching it, and leaves the finished shape.
Rough the part first with [Rotary Roughing](help:rotary_roughing_toolpath), so the finish has
only a thin, even layer to take off.

## Steps

1. Start a **rotary** job (see [Job Setup](help:job_setup)) and put the model in it.
2. On the Toolpaths page, press **Rotary Finishing** (under 3D Toolpaths). It shows once
   **4th axis** is ticked in Machine Configuration (see
   [The 4th axis and your firmware](help:rotary_firmware)).
3. Pick a ball nose: it follows curves best. Its **finishing stepover** (in the
   [Tool Database](help:tool_database)) is how far apart the passes are: smaller for a
   smoother part.
4. Choose the **Direction**, the **Cut direction** and where the **Bit** is **held** (below).
5. Press **Calculate**.

Then [save the toolpaths for your machine](help:save_toolpaths) through a **Rotary A** post.

## Direction

- **Round the part** (the usual): A turns the bit all the way round the part, then it steps
  along. Best for round shapes and short parts. Some call this radial finishing.
- **Along the part**: the bit runs the whole length of the part with A held still, then A turns
  a step and it runs back, like a typewriter. Best for long parts with detail running along
  them. Some call this axial finishing.

**Cut direction** is the same as on Rotary Roughing: **Both** goes back and forth, staying
down; **Climb** and **Conventional** cut every pass the same way, lifting back to start each one.

## Bit held

- **Over the axis** (the usual): the bit points straight at the turning axis, and its tip cuts.
- **Off centre**: the bit is held **Offset** to one side of the axis, so the side of the ball
  cuts rather than its slow-turning tip. That is a cleaner finish, and walls that face sideways
  are reached. **One side** holds it there for the whole finish; **Both sides** goes over the
  whole part from one side, then again from the other, for walls facing either way, in twice
  the time.

An off-centre finish needs a post that moves the bit across the axis: the Rotary A posts do.

## It cuts only what is left

The toolpath list is the cutting order. Rotary Finishing works out what the toolpaths **above
it** leave of the blank, and goes only where some of it is still there: it never cuts air. If
nothing is left, it says so in red. When a toolpath above it changes, or the list's order
changes above it, it goes out of date (⚠) until it is recalculated.

**Advanced rotary mode** (Edit ▸ Options ▸ Toolpath Settings) turns this off: the finish then
goes over the whole blank, as if nothing were above it.

## Settings

- **Tool**: a ball nose (above).
- **Direction**, **Cut direction**, **Bit held**, **Offset** and **Sides** (above).
- **Allowance**: material left on the model for a later pass. 0 for the finish.
- **Start** and **End**: where along the axis it cuts, from the job's start (the chuck end).
  End 0 runs to the job's end.
- **Registration mark**: a small notch at A=0 just past End, so the part can be put back
  exactly as it was if a job has to be started again.
- **Cut support**: cut away the centre stock Job Setup leaves along the axis, down to the
  model: for the last toolpath, once everything else is cut.

## What it keeps clear of

It keeps out of the **Chuck** and **Tailstock** set in
[Machine Configuration](help:rotary_firmware), and keeps the collet nut and spindle (see
[Collisions](help:collisions)) clear of the material beside the cut. Where they would not let the
bit down onto the model, it stays up there, leaves that bit of the model uncut and says so in
red: rough the material beside it down first, or use a bit that sticks out further.
