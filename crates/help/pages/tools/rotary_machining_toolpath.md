---
reviewed: 2026-11-24
see-also: rotary_lathe_toolpath, rounding_toolpath, 3d_finishing_toolpath, rotary_firmware, check_rotary_setup, collisions, job_setup, save_toolpaths, tool_database, web_api
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
- **Strategy**: how the bit goes round the part (see Four ways round, below):
  - **Radial** keeps the bit straight over the turning axis while A turns.
  - **Off centre** holds the bit to one side of the axis while A turns.
  - **Indexed** turns A to each side in turn and holds it still while the bit cuts across.
  - **Sides, then details** clears the bulk from each side with A held still, finishes each
    side, then locks A at a few more angles for the detail the sides could not reach.
- **Offset** (Off centre): how far to one side of the axis the bit is held. A third to a half
  of the bit's width suits most parts. Positive is the way the part comes up towards the bit
  as A turns back; a negative offset holds it the other side.
- **Faces** (Indexed, Sides then details): how many sides the part is cut from, evenly round it
  from A=0. Four suits most parts for Indexed; two (top and bottom) suits Sides then details
  for flat-ish parts such as a plane with its wings level.
- **Roughing allowance** (Sides then details): how much the bulk clearing leaves on the model
  for each side's finish to take off.
- **Tilts** (Sides then details): the most extra angles A locks at after the sides, for the
  details. Each is picked where it reaches the most of what is still left. 6 suits most parts;
  0 finishes with the sides alone.
- **Tilt angles** (Sides then details): leave empty to have the angles picked for you, or type
  your own in degrees, in order, such as `15, 165, 45`.
- **Start** and **End**: where along the axis the rings go, from the job's start (the chuck
  end). End 0 runs to the job's end. Keep them clear of the chuck and tailstock you set in
  [Machine Configuration](help:rotary_firmware). They count from the material's start
  whatever XY zero [Job Setup](help:job_setup) uses: zeroed on the middle, the toolpath is
  written where the material is, just as with a corner.
- **Allowance**: material left all the way round for a later pass.
- **Cut Away the Support**: cuts away the centre stock [Job Setup](help:job_setup) leaves
  along the axis (Holding the Part), down to the model. Tick it only on the last toolpath:
  the ends the part is kept joined at still hold it. Left unticked, every way round cuts round
  the centre stock and leaves the material whole past the model at a joined end.
- **Registration mark**: a notch 0.5 mm deep at A=0, a bit's width past the last ring. If a
  job stops part way, turn A until the notch is under the bit and zero A there before starting
  again. It is only cut where there is waste past **End** to cut it in: when the part runs
  right to the end of the material, it is left out and a note says so, rather than notch the
  part. Bring **End** in a little to make room, or pencil a line along the material at A=0
  before you start.
  [Rotary set up](help:check_rotary_setup) reminds you when there is neither.

## Four ways round

![Radial, Off centre and Indexed, seen down the turning axis](pictures/rotary_machining_toolpath/strategies.png)

- **Radial** is the one to start with: the bit points straight at the axis, so it reaches
  everything a round part shows from outside.
- **Off centre**: right on the axis, the very tip of a ball nose barely turns, so it rubs more
  than it cuts. Held to one side, the faster side of the ball does the cutting for a cleaner
  finish, and walls that face sideways are reached square on.
- **Indexed**: A stops at each side and holds still while the bit goes back and forth across
  it, along the part, like a flat 3D finish. Use it for flat faces, square parts, and machines
  whose A holds still better than it turns smoothly.
- **Sides, then details** works bulk to fine detail, like a two-sided job the machine turns
  over by itself:
  1. A turns to the top (A=0) and holds still while the bulk is cleared a level at a time,
     down to the **Roughing allowance** above the model, the way a flat 3D roughing does.
     Then A turns to the next side and does the same, each side in turn, evenly round the part:
     with **Faces** 2, the top and then the other side (A=180), turned over like a two-sided
     job; with 4, every quarter turn.
  2. Each side is finished, A still held.
  3. Last, A locks at a few more angles, a tilt at a time, for what no side could reach: the
     back edge of a wing, under a tail, into a hollow. Each angle is picked where it reaches
     the most of what is left, up to **Tilts** of them, or you type them in **Tilt angles**.
     At every angle A holds still while the bit cuts, so the machine moves only X, Y and Z.

  ![The bulk cleared from the top, then the other side, then locked tilts for the details](pictures/rotary_machining_toolpath/sides-then-details.png)

  It knows where the material is the whole way through: the blank you set in
  [Job Setup](help:job_setup) (round, square or a board), less what each pass has already cut.
  Wherever the bit would cut nothing it lifts and moves over instead, and where the way down is
  clear it comes down at the rapid rate to just above the material before it plunges. One limit:
  each side comes down to the turning axis and no further, so where nothing of the model is
  between the sides, a thin ridge between the lines can be left along the axis. Sand it off.

Every way round leaves out the air: a pass that would only cut air beyond the blank (past a
round blank's edge, or where the model is the blank's own surface) is skipped.

Off centre and Indexed hold the bit away from the axis, so the rotary's
[centre line](help:rotary_firmware) must be found first: [Rotary set up](help:check_rotary_setup)
warns until it is. The preview shows where the tip really cuts, and the file moves your
machine's own axis across the rotary (Y beside a rotary along X) as well as A. Cut Craft's
**Rotary A** posts write it; a post of your own needs `[ACROSS]` in its move lines.

## Kept clear of the holder, chuck and tailstock

The bit is not the only thing that comes down into the work. **Sides, then details** keeps the
collet nut and spindle above every bit of material left at the angle A is locked at, not just
the bit's tip off the model, so a deep cut beside a tall wing never brings the nut down onto it.
The sizes come from your machine: the holder from [Machine Configuration](help:collisions)
(the usual ER20 nut and a 65 mm spindle if none is set), and how far the bit sticks out from its
stick-out in the [Tool Database](help:tool_database).

It also keeps out of the **Chuck** and **Tailstock** set in Machine Configuration's rotary
settings: nothing is cut where they hold the material. (The other ways round cut between
**Start** and **End**, so keep those clear of the chuck and tailstock yourself.)

When something can't be reached, the toolpath is still made, and a red note at the foot of the
form says what and where. The toolpath keeps the note: the checks before saving the G-code say
it again under **The bit fits every corner and detail it is asked to cut**, and so does the
[Web API](help:web_api)'s list of toolpaths, so it isn't lost once the form is closed. In the
preview, what was left looks like blocks and steps: that is the bulk clearing's levels, which
no finish could get down to.

- **The model is too near the chuck or tailstock**: the collet nut is wider than the bit, so it
  needs room beside the chuck to get down to the model. The note says how much to leave. Move
  the model along, or use longer material, so a spacer of waste sits between the chuck and the
  model. The same goes for the tailstock end.
- **Some of the model is left rough**: no angle tried let the holder get down there. The note
  says where along the part, and which angles A locked at. A bit that sticks out further, a
  slimmer holder, more **Tilts**, or the model sliced into parts would reach it. If the
  [Tool Database](help:tool_database) doesn't say how far the bit sticks out of the collet
  (its stick-out, or its flute length), 25 mm is taken and the note says so. That guess is
  often what leaves a detailed model rough: measure the bit in the collet and set it.
- A model joined to the material at the chuck or tailstock end (Job Setup's **Keep Joined
  At**) needs no room beside it there, and nothing is said about it.

To take the waste off the ends, or turn the blank round before the carving, use
[Rotary Lathe](help:rotary_lathe_toolpath).

## Cut round the model as it really is

A full 3D model imported into a rotary job (see [Orientate 3D Model](help:orientate_3d_model))
is cut round as it really is, triangle by triangle, exactly where it sits in space: the same
model the 3D view shows. Material Setup's **Model Position in Material** is for flat jobs and
does not move it; where the material reaches further out than the model, it is cut down to the
model. With
**Off centre** the side of the ball can reach under a ledge or into a hollow that a bit pointing
at the axis would miss, and it is never let into the model. A model drawn or sculpted in Cut
Craft, or one sunk into another, is cut from its heights as before.

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
