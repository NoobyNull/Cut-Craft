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
- **Strategy**: how the bit goes round the part (see Three ways round, below):
  - **Radial** keeps the bit straight over the turning axis while A turns.
  - **Off centre** holds the bit to one side of the axis while A turns.
  - **Indexed** turns A to each side in turn and holds it still while the bit cuts across.
- **Offset** (Off centre): how far to one side of the axis the bit is held. A third to a half
  of the bit's width suits most parts. Positive is the way the part comes up towards the bit
  as A turns back; a negative offset holds it the other side.
- **Faces** (Indexed): how many sides the part is cut from, evenly round it from A=0. Four
  suits most parts; more sides leave a smoother finish where they meet.
- **Start** and **End**: where along the axis the rings go, from the job's start (the chuck
  end). End 0 runs to the job's end. Keep them clear of the chuck and tailstock you set in
  [Machine Configuration](help:rotary_firmware).
- **Allowance**: material left all the way round for a later pass.
- **Registration mark**: a notch 0.5 mm deep at A=0, a bit's width past the last ring. If a
  job stops part way, turn A until the notch is under the bit and zero A there before starting
  again. Without one, pencil a line along the material at A=0 before you start.
  [Rotary set up](help:check_rotary_setup) reminds you when there is neither.

## Three ways round

![Radial, Off centre and Indexed, seen down the turning axis](pictures/rotary_machining_toolpath/strategies.png)

- **Radial** is the one to start with: the bit points straight at the axis, so it reaches
  everything a round part shows from outside.
- **Off centre**: right on the axis, the very tip of a ball nose barely turns, so it rubs more
  than it cuts. Held to one side, the faster side of the ball does the cutting for a cleaner
  finish, and walls that face sideways are reached square on.
- **Indexed**: A stops at each side and holds still while the bit goes back and forth across
  it, along the part, like a flat 3D finish. Use it for flat faces, square parts, and machines
  whose A holds still better than it turns smoothly.

Off centre and Indexed hold the bit away from the axis, so the rotary's
[centre line](help:rotary_firmware) must be found first: [Rotary set up](help:check_rotary_setup)
warns until it is. The preview shows where the tip really cuts, and the file moves your
machine's own axis across the rotary (Y beside a rotary along X) as well as A. Cut Craft's
**Rotary A** posts write it; a post of your own needs `[ACROSS]` in its move lines.

## How the rings join up

Every ring starts at A=0 and goes once round. Between rings the bit lifts clear: by
**Clearance (Z1)** above the top on round material, and on a square or a board (its **Shape**
in [Job Setup](help:job_setup)) higher still, clear of its corners or edges as they swing
round. How A gets to
the start of the next ring is set once for the machine, under **A can turn** in
[Machine Configuration](help:rotary_firmware):

- **Freely**: A carries on round the same way, like a lathe. A long job can wind A hundreds of
  turns; past 500, [Rotary set up](help:check_rotary_setup) suggests a limit, because many
  controllers place A less finely so far out.
- **Up to N turns each way** with **Cut both ways**: the next ring is cut back the other way.
- **Up to N turns each way** with **Lift and spin back**: A spins back before the next ring.
