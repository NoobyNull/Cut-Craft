---
reviewed: 2026-11-19
see-also: rounding_toolpath, 3d_finishing_toolpath, rotary_firmware, check_rotary_setup, job_setup, save_toolpaths
---

**4-Axis Rotary Machining** finishes a 3D model all the way round a turning cylinder. The bit
moves along the part and up and down while A turns the material under it: X, Z and A move
together.

It is worked out round the turning axis itself, not on the unrolled sheet. Deep in the part, the
bit reaches further round than it does at the surface, so a toolpath made on the flat sheet can
nick the side of a narrow ridge. This one keeps the bit clear of all of it.

## Steps

![The steps, one after another](pictures/rotary_machining_toolpath/tour.gif)

1. 4-Axis Rotary Machining finishes a model all the way round a turning cylinder. Start a
   **rotary** job and place a full 3D model in it: here, a turned bead.

   ![Step 1](pictures/rotary_machining_toolpath/step-1.png)

2. On the Toolpaths page, press **4-Axis Rotary Machining** (under 3D Toolpaths).

   ![Step 2](pictures/rotary_machining_toolpath/step-2.png)

3. Pick a ball nose bit: its finishing stepover spaces the rings round the part.

   ![Step 3](pictures/rotary_machining_toolpath/step-3.png)

4. Say where along the part the rings **Start** and **End**, clear of the chuck and the
   tailstock.

   ![Step 4](pictures/rotary_machining_toolpath/step-4.png)

5. Tick **Registration mark** for a small notch at A=0, so the part can go back exactly as it
   was if the job is started again.

   ![Step 5](pictures/rotary_machining_toolpath/step-5.png)

6. Press **Calculate** to make the toolpath: a ring round the part at each step along it.

   ![Step 6](pictures/rotary_machining_toolpath/step-6.png)

When it looks right, [save it for your machine](help:save_toolpaths) through a **Rotary A** post.

## Settings

- **Tool**: a ball nose follows curves best. A flat-bottomed bit works too, resting on the
  highest point under it. Its **finishing stepover** (in the Tool Database) is the distance
  from one ring to the next.
- **Strategy**: **Radial** keeps the bit straight over the turning axis. (Cutting from off the
  axis, for undercuts, and indexed faces come later.)
- **Start** and **End**: where along the axis the rings go, from the job's start (the chuck
  end). End 0 runs to the job's end. Keep them clear of the chuck and tailstock you set in
  [Machine Configuration](help:rotary_firmware).
- **Allowance**: material left all the way round for a later pass.
- **Registration mark**: a notch 0.5 mm deep at A=0, a bit's width past the last ring. If a
  job stops part way, turn A until the notch is under the bit and zero A there before starting
  again. Without one, pencil a line along the material at A=0 before you start.
  [Rotary set up](help:check_rotary_setup) reminds you when there is neither.

## How the rings join up

Every ring starts at A=0 and goes once round. Between rings the bit lifts clear. How A gets to
the start of the next ring is set once for the machine, under **A can turn** in
[Machine Configuration](help:rotary_firmware):

- **Freely**: A carries on round the same way, like a lathe. A long job can wind A hundreds of
  turns; past 500, [Rotary set up](help:check_rotary_setup) suggests a limit, because many
  controllers place A less finely so far out.
- **Up to N turns each way** with **Cut both ways**: the next ring is cut back the other way.
- **Up to N turns each way** with **Lift and spin back**: A spins back before the next ring.
