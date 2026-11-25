---
reviewed: 2026-11-25
see-also: rotary_roughing_toolpath, rotary_finishing_toolpath, rotary_details_toolpath, rotary_machining_toolpath, rotary_firmware, job_setup, collisions, save_toolpaths, tool_database
---

**Rotary Locked Angles** finishes a carved part on a rotary job with A held still. A turns to
an angle and locks there while the bit finishes the side facing it, back and forth along the
part like a flat 3D finish. Then A turns to the next angle, and the next.

You don't pick the angles. Each one is chosen where it reaches the most of what is left, facing
the bit squarely, so a flat is finished from straight over it. You set the **Most angles**, and
it stops sooner once nothing worth another angle is left: a square part takes four when you
allow at least four, and no more however many you allow.

Use it for flats and faces, and on machines whose A is better at holding still than at moving
smoothly. For round shapes, [Rotary Finishing](help:rotary_finishing_toolpath) keeps A turning
and is usually smoother.

## Steps

1. Start a **rotary** job (see [Job Setup](help:job_setup)) and put the model in it.
2. Rough it first with [Rotary Roughing](help:rotary_roughing_toolpath).
3. On the Toolpaths page, press **Rotary Locked Angles** (under 3D Toolpaths). It shows once
   **4th axis** is ticked in Machine Configuration (see
   [The 4th axis and your firmware](help:rotary_firmware)).
4. Pick a ball nose. Its **finishing stepover** (in the [Bit Library](help:tool_database))
   is how far apart the passes are.
5. Set **Most angles**, then press **Calculate**.

Each side is cut with the bit held to one side of the turning axis, so it needs a post that
moves the bit across the axis: the **Rotary A** posts do.

## It cuts only what is left

The toolpath list is the cutting order. Rotary Locked Angles works out what the toolpaths
**above it** leave of the blank and cuts only that, so it never cuts air. If nothing is left,
it says so in red. When a toolpath above it changes, or the list's order changes above it, it
goes out of date (⚠) until it is recalculated.

**Advanced rotary mode** (Edit ▸ Options ▸ Toolpath Settings) turns this off: it then works on
the whole blank, as if nothing were above it.

## Settings

- **Tool**: a ball nose (above).
- **Most angles**: the most angles A locks at, from 1 to 24 (every 15°).
- **Allowance**: material left on the model for a later pass. 0 for the finish.
- **Start** and **End**: where along the axis it cuts, from the job's start (the chuck end).
  End 0 runs to the job's end.
- **Machining limit**: where on the material it may cut. **Material** (the usual) is all of
  it; **Model**, a **Selected Level** or **Selected Vector(s)** keep it within their outline,
  grown by **Boundary Offset**. Vectors drawn on the job wrap round the part, just as the job
  does. Selected vectors are kept with the toolpath, so it recalculates the same.

## What it says in red

- **Another angle would still reach some of the model**: all the angles allowed were used
  and some is still left rough. Raise **Most angles**, or finish it after with Rotary
  Finishing.
- **The collet nut and spindle can't get down to some of the model at any angle**: it keeps
  the holder (see [Collisions](help:collisions)) clear of the material, and out of the
  **Chuck** and **Tailstock**, so that part is left rough. A bit that sticks out further, or a
  slimmer holder, would reach it. It says so even when the holder keeps it off everything
  left, and no angle cuts at all.
