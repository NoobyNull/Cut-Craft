---
reviewed: 2026-11-24
see-also: 3d_finishing_toolpath, create_shape
---

## Steps

![The steps, one after another](pictures/3d_roughing_toolpath/tour.gif)

1. 3D roughing clears away most of the wood above a 3D model quickly, leaving a little for the finishing toolpath. Make or load the model first: here, a dome.

   ![Step 1](pictures/3d_roughing_toolpath/step-1.png)

2. On the Toolpaths page, press **3D Roughing** (under 3D Toolpaths).

   ![Step 2](pictures/3d_roughing_toolpath/step-2.png)

3. Set the **Allowance**: the wood left on the model for the finishing toolpath to take off cleanly.

   ![Step 3](pictures/3d_roughing_toolpath/step-3.png)

4. Choose the **Strategy**: **Z level** clears flat steps, one pass depth at a time; **3D raster** rides over the model in rows.

   ![Step 4](pictures/3d_roughing_toolpath/step-4.png)

5. Use a big end mill to clear quickly, then press **Calculate**.

   ![Step 5](pictures/3d_roughing_toolpath/step-5.png)

6. Press **Preview All Toolpaths**: the dome comes out in steps, ready for finishing.

   ![Step 6](pictures/3d_roughing_toolpath/step-6.png)

Then make a [3D Finishing](help:3d_finishing_toolpath) toolpath to take off the allowance. When it looks right in the preview, [save it for your machine](help:save_toolpaths).
