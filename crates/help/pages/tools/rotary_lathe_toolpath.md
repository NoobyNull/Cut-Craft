---
reviewed: 2026-11-23
see-also: rotary_machining_toolpath, orientate_3d_model, rounding_toolpath, rotary_firmware, job_setup, collisions, save_toolpaths
---

**Rotary Lathe** works a rotary job like a lathe: A spins the material steadily while the bit
stays straight over the turning axis and steps down a level at a time. Use it to square off the
ends, to turn the blank round to the model's outline before the carving, or to part the
finished piece off.

## Steps

![The steps, one after another](pictures/rotary_lathe_toolpath/tour.gif)

1. Rotary Lathe spins the material like a lathe while the bit stays over the turning axis. Start a **rotary** job: here, a bead in a 60 mm bar.

   ![Step 1](pictures/rotary_lathe_toolpath/step-1.png)

2. On the Toolpaths page, press **Rotary Lathe** (under 3D Toolpaths).

   ![Step 2](pictures/rotary_lathe_toolpath/step-2.png)

3. Pick a square end mill: it leaves a flat face. Its pass depth sets each step down, and its stepover the turns along the part.

   ![Step 3](pictures/rotary_lathe_toolpath/step-3.png)

4. Choose what it does: **Face** an end down to a stub, **Turn** the part round to its outline, or **Part off** with one groove.

   ![Step 4](pictures/rotary_lathe_toolpath/step-4.png)

5. Say where along the part it cuts with **Start** and **End**, measured from the chuck end.

   ![Step 5](pictures/rotary_lathe_toolpath/step-5.png)

6. **Holding stub** is how thick a stub is left on the axis, keeping the part joined on until it is cut free.

   ![Step 6](pictures/rotary_lathe_toolpath/step-6.png)

7. Press **Calculate**: a turn round the part at each step along it, a level at a time, only where there is material.

   ![Step 7](pictures/rotary_lathe_toolpath/step-7.png)

When it looks right, [save it for your machine](help:save_toolpaths) through a **Rotary A** post.

## What it does

![Face, Turn and Part off, seen from the side](pictures/rotary_lathe_toolpath/operations.png)

- **Face** takes everything between **Start** and **End** down to the **Holding stub**: the
  waste at an end of the material, or room beside the part so the collet nut can get down to
  it later.
- **Turn** takes everything between **Start** and **End** down to the model's outline, round:
  as far out as any of the model reaches at each point along, plus the **Allowance**. A full
  3D model (see [Orientate 3D Model](help:orientate_3d_model)) counts as it really is in
  space, wherever Material Setup's flat-job model position says. Where there is no model it
  goes down to the stub. It is the quickest way to rough a round part
  before [4-Axis Rotary Machining](help:rotary_machining_toolpath) carves it.
- **Part off** cuts one groove, a bit wide, at **Start**, down to the stub, so the finished
  piece can be snapped or sawn free.

## Settings

- **Tool**: a square end mill leaves a flat face. Its **pass depth** (in the
  [Tool Database](help:tool_database)) is how far each level steps down, and its **stepover**
  how far apart the turns are along the part.
- **Operation**: Face, Turn or Part off (above).
- **Start** and **End**: where along the axis it cuts, from the job's start (the chuck end).
  End 0 runs to the job's end. For Part off, Start is where the groove goes. They count from
  the material's start whatever XY zero [Job Setup](help:job_setup) uses: zeroed on the middle,
  the toolpath is written where the material is, just as with a corner.
- **Holding stub**: how thick a stub is left on the turning axis, joining the part to the
  rest of the material until it is cut free. 0 cuts right through. Turn comes down to it too,
  wherever there is no model.
- **Allowance** (Turn): material left on the model's outline for the carving after.

## What it keeps clear of

It knows where the material is: it never cuts air, and each level only turns where there is
still material to take off. It keeps out of the **Chuck** and **Tailstock** set in
[Machine Configuration](help:rotary_firmware), and keeps the collet nut and spindle (see
[Collet and Holder Collisions](help:collisions)) above the material left beside the cut.

The toolpath is always made, and anything it can't do is said in red at the foot of the form:

- Where the holder, or the chuck or tailstock beside the cut, would not let the bit down as far
  as it should go, the cut stops short there. Face the material beside it down first, keep
  Start and End further from the chuck and tailstock, or use a bit that sticks out further.
- When End is not past Start, or there is less room between them than the bit is wide,
  nothing is cut.
- Turn with no model in the job cuts nothing: place a model first, or use Face.
