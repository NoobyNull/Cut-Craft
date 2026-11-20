---
reviewed: 2026-11-19
see-also: setup_material, material_setup, settings, draw_rectangle
---

## Steps

![The steps, one after another](pictures/job_setup/tour.gif)

1. Job Setup holds every setting of this job's material. Press **Setup Material** (under File Operations).

   ![Step 1](pictures/job_setup/step-1.png)

2. Press **More settings for this job…** under the size: the rest of Job Setup opens below it.

   ![Step 2](pictures/job_setup/step-2.png)

3. Choose where **Z zero** is: the top of the material (most common) or the machine bed. Zero the bit in the same place on your machine before cutting.

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
