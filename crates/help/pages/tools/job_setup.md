---
reviewed: 2026-11-17
see-also: draw_rectangle, draw_circle, draw_text
---

## Steps

![The steps, one after another](pictures/job_setup/tour.gif)

1. Press **New** (or File ▸ New) to start a job. The first thing it asks about is your material.

   ![Step 1](pictures/job_setup/step-1.png)

2. Measure your board and type its **Width**, **Height** and **Thickness**. Get the thickness right: it decides how deep a cut must go to come through.

   ![Step 2](pictures/job_setup/step-2.png)

3. Choose where **Z zero** is: the top of the material (most common) or the machine bed. Zero the bit in the same place on your machine before cutting.

   ![Step 3](pictures/job_setup/step-3.png)

4. Pick the **XY zero** point: the corner (or the middle) of the board you will set as X0 Y0 on the machine.

   ![Step 4](pictures/job_setup/step-4.png)

5. Pick what the board is under **Material** (Pine, Hard Maple, MDF…), from the Materials Library. Toolpaths over the 3D model take their bits' feeds for it. Layers name their own material in [Layer Properties](help:layer_properties): a layer left without one cuts as the hardest class, so name it on each layer you cut (see [Toolpaths](help:toolpaths)).

6. Press **OK**. The sheet on screen is now your board, ready to draw on.

   ![Step 6](pictures/job_setup/step-5.png)

Next, draw what you want to cut: [Draw Rectangle](help:draw_rectangle), [Draw Circle](help:draw_circle) or [Draw Text](help:draw_text).
