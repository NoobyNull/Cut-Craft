---
reviewed: 2026-11-25
see-also: rotary_finishing_toolpath, rotary_locked_angles_toolpath, rotary_roughing_toolpath, 3d_pencil_toolpath, rotary_firmware, job_setup, collisions, save_toolpaths, tool_database
---

**Rotary Details** runs a smaller bit after the finish on a rotary job. It cuts only what the
bigger bits before it could not reach, such as grooves, tight corners and fine detail, so the
small bit doesn't spend hours going over the whole part again.

## Steps

1. Finish the part first, for example with [Rotary Finishing](help:rotary_finishing_toolpath)
   and a bigger ball nose.
2. On the Toolpaths page, press **Rotary Details** (under 3D Toolpaths).
3. Pick the **detail bit**: a ball nose smaller than the bits that finished before.
4. Under **Bits that finished before**, add the bigger bits that finished the part.
5. Set **Most angles**, then press **Calculate**.

## How it works

1. Each bit that finished before is run over the part as a finish all the way round, in
   thought, and what it reached is taken off.
2. The detail bit goes round the part ring by ring, wherever some material is still left.
3. Last, A locks at the angles that reach the most of what the rings could not, up to **Most
   angles**, as in [Rotary Locked Angles](help:rotary_locked_angles_toolpath). It stops sooner
   once nothing worth another angle is left.

It also cuts only what the toolpaths **above it** in the list leave, so a finish above it
counts even if you don't add its bit under Bits that finished before. It never cuts air. When a
toolpath above it changes, or the list's order changes above it, it goes out of date (⚠) until
it is recalculated.

**Advanced rotary mode** (Edit ▸ Options ▸ Toolpath Settings) starts it from the whole blank
instead of what the toolpaths above leave. The bits that finished before still count, so it
still cuts only what they could not reach.

The locked angles hold the bit to one side of the turning axis, so they need a post that moves
the bit across the axis: the **Rotary A** posts do. Set Most angles to 0 for rings alone.

## Settings

- **Tool**: the detail bit, a small ball nose. Its **finishing stepover** spaces the passes.
- **Bits that finished before**: the bigger bits it follows.
- **Most angles**: the most angles A locks at after the rings, from 0 to 24.
- **Allowance**: material left on the model for a later pass. 0 for the finish.
- **Start** and **End**: where along the axis it cuts, from the job's start (the chuck end).
  End 0 runs to the job's end.

## What it says in red

- **Nothing is left for the detail bit**: the bits before already reach all of the model. Pick
  a smaller detail bit.
- **Still left rough**: neither the rings nor the angles allowed reach it. Raise **Most
  angles**, or add a smaller detail bit after.
- **The collet nut and spindle can't get down to some of the model**: the holder (see
  [Collisions](help:collisions)) is kept clear of the material and out of the **Chuck** and
  **Tailstock**, so that part is left. A bit that sticks out further would reach it.
