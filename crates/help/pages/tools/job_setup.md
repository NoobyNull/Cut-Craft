---
reviewed: 2026-11-25
see-also: setup_material, material_setup, settings, draw_rectangle, rotary_firmware, check_rotary_setup, rotary_machining_toolpath, view_2d
---

## Steps

![The steps, one after another](pictures/job_setup/tour.gif)

1. Job Setup holds every setting of this job's material. Press **Setup Material** (under File Operations).

   ![Step 1](pictures/job_setup/step-1.png)

2. Press **More settings for this job…** under the size: the rest of Job Setup opens below it.

   ![Step 2](pictures/job_setup/step-2.png)

3. Choose where **Z zero** is: the top of the material (most common) or the machine bed. Zero the bit in the same place on your machine before cutting. In the [3D view](help:view_3d) the red, green and blue arrows move to the zero you choose, on the top or the bottom of the material (on a rotary job, the turning axis).

   ![Step 3](pictures/job_setup/step-3.png)

4. Pick the **XY zero** point: the corner (or the middle) of the board you will set as X0 Y0 on the machine.

   ![Step 4](pictures/job_setup/step-4.png)

5. Press **Apply**. Only this job changes: new jobs start with Edit ▸ Options ▸ Job Defaults.

   ![Step 5](pictures/job_setup/step-5.png)

Pick what the board is under **Material** (Pine, Hard Maple, MDF…), from the Materials Library. Toolpaths over the 3D model take their bits' feeds for it. Layers name their own material in [Layer Properties](help:layer_properties): a layer left without one cuts as the hardest class, so name it on each layer you cut (see [Toolpaths](help:toolpaths)).

## The same settings for every new job

**Edit ▸ Options ▸ Job Defaults** holds these settings for every new job: set them once for
your machine and each blank job starts with them. **Set as Default** here does the same from
this job's settings. Changing either leaves the job that is open as it is.

## Classic Startup Flow

Cut Craft opens straight to a blank job. If you would rather start each job by filling in
Job Setup, as Cut Craft used to, tick **Classic Startup Flow** on the Job Defaults page:
Cut Craft then opens on the start page, and **New** asks for every setting here, with **OK**
and **Cancel**, before the job opens.

Next, draw what you want to cut: [Draw Rectangle](help:draw_rectangle), [Draw Circle](help:draw_circle) or [Draw Text](help:draw_text).

## Rotary jobs

Choose **Rotary** under **Job Type** to carve material turned by a 4th axis (see
[The 4th Axis and Your Firmware](help:rotary_firmware)). Then:

1. Type the **Length (L)** of material between the chuck and the tailstock.
2. Pick its **Shape**, seen down the axis:
   - **Round**: a dowel, a turned spindle or a log. Type its **Diameter (D)**.
   - **Square**: a face flat on top at A=0. Type its **Side (D)**.
   - **Board**: a flat board lying flat at A=0, for carving both faces with A turning it over.
     Type its **Thickness (D)**, and **Board Width (W)**, how wide it is.

   The size's name changes with the shape you pick. Whatever its name, it is the
   material's size straight up through the axis at A=0.

   ![Round, square and board seen down the axis: D straight up through the axis, and the circle each swings through](pictures/job_setup/rotary_shapes.png)

3. Pick the **Orientation**: whether the axis runs along X or along Y.
4. Pick where **Z zero** is: **Material Top** (touch the bit off on the top of the
   material at A=0) or **Turning Axis** (on the axis the rotary turns it round).

The 3D view shows the material as you described it: a round blank, a square or a board, seen
round its axis. A toolpath preview cuts into that same shape, so a shallow cut takes a
square's corners off just as it will on the machine. Depths are measured down from the round
that touches a square's or board's flat faces, the same round the design is wrapped onto.

The 2D view shows the design unrolled: one turn of the material between two blue seams, with
a shaded band past each seam showing what wraps round from the other side. See
[Moving around the 2D view](help:view_2d).

5. Under **Holding the Part**, say what of the material is left to hold the part while it is
   cut (see below). Leave **Keep Joined At** on **Neither** and **Centre Stock Diameter** at 0
   to cut the part free all round, as before.

### Holding the part

A long or thin part, such as a figure lying along the axis, can flex or snap off part way
through when everything round it is cut away. Two settings keep it held:

![Left: the part kept joined at the chuck end and held at the head by centre stock. Right: the last toolpath cuts the centre stock away, and the chuck end still holds the part, ready to saw off flat](pictures/job_setup/holding-the-part.png)

- **Keep Joined At**: the end (or ends) of the material the part stays joined to. Past the
  model at that end the material is left whole, so that end of the model comes out as a flat
  face: saw the part free there when it is done. A statue joined at the chuck gets a flat base.
  It also needs no room beside the chuck for the collet nut.
- **Centre Stock Diameter**: a round beam of material left along the turning axis, from the
  chuck to the tailstock, through the model and past it. Every rotary toolpath cuts round it,
  so the far end of the part (a statue's head) stays held by the tailstock. 0 leaves none.
  It must be less than the material's diameter. 6 to 10 mm suits most parts in wood.

The centre stock is centred on the turning axis wherever the model sits, so chuck the material
so that it turns true (centred) on the axis. Where the model is thinner than the centre stock
(a torch, a crown's points) the centre stock shows through until it is cut away.

To finish, make the last toolpath a [4-Axis Rotary Machining](help:rotary_machining_toolpath)
pass with **Cut Away the Support** ticked: it cuts the centre stock down to the model, and the
joined end still holds the part. Then saw it off at the joined end. The checks before saving
warn if any toolpath comes after it. [Rotary Lathe](help:rotary_lathe_toolpath) Face and Turn
leave the centre stock too; Part off cuts through it.

A square's corners and a board's edges swing out further than its top as A turns, out to the
red circle. Toolpaths lift the bit that much higher before A turns, so it never knocks into
them, and the [Rotary set up](help:check_rotary_setup) check sees that they clear the
spoilboard and the bit.

Throughout the forms, the **bit** is what is in the spindle; the tools are the buttons you press.
