---
reviewed: 2026-11-20
see-also: setup_material, material_setup, settings, draw_rectangle, rotary_firmware, check_rotary_setup, rotary_machining_toolpath
---

## Steps

![The steps, one after another](pictures/job_setup/tour.gif)

1. Job Setup holds every setting of this job's material. Press **Setup Material** (under File Operations).

   ![Step 1](pictures/job_setup/step-1.png)

2. Press **More settings for this job…** under the size: the rest of Job Setup opens below it.

   ![Step 2](pictures/job_setup/step-2.png)

3. Choose where **Z zero** is: the top of the material (most common) or the machine bed. Zero the bit in the same place on your machine before cutting. In the [3D view](help:view_3d) the red, green and blue arrows move to the zero you choose, on the top or the bottom of the material.

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
   - **Round**: a dowel, a turned spindle or a log. **Diameter (D)** is across it.
   - **Square**: a face flat on top at A=0. **Diameter (D)** is its side.
   - **Board**: a flat board lying flat at A=0, for carving both faces with A turning it over.
     **Diameter (D)** is its thickness, and **Board Width (W)** how wide it is.

   ![Round, square and board seen down the axis: D straight up through the axis, and the circle each swings through](pictures/job_setup/rotary_shapes.png)

3. Pick the **Orientation**: whether the axis runs along X or along Y.
4. Pick where **Z zero** is: on top of the material, or on the turning axis.

A square's corners and a board's edges swing out further than its top as A turns, out to the
red circle. Toolpaths lift the bit that much higher before A turns, so it never knocks into
them, and the [Rotary set up](help:check_rotary_setup) check sees that they clear the
spoilboard and the bit.

