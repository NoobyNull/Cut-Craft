---
reviewed: 2026-11-24
see-also: 3d_roughing_toolpath, create_shape, stepover
---

## Steps

![The steps, one after another](pictures/3d_finishing_toolpath/tour.gif)

1. 3D finishing runs a ball nose bit over a 3D model in fine rows, as low as it can go without cutting into it. Make or load the model first: here, a dome.

   ![Step 1](pictures/3d_finishing_toolpath/step-1.png)

2. On the Toolpaths page, press **3D Finishing** (under 3D Toolpaths).

   ![Step 2](pictures/3d_finishing_toolpath/step-2.png)

3. On the **3D Finishing** tab, choose the **Strategy**: **Raster** rows back and forth, or **Offset** rings following the outline.

   ![Step 3](pictures/3d_finishing_toolpath/step-3.png)

4. Pick a ball nose bit. Its stepover sets how close the rows are: closer rows leave a smoother surface but take longer. Add more bits to finish the fine detail the first one cannot reach.

   ![Step 4](pictures/3d_finishing_toolpath/step-4.png)

5. Press **Calculate** to make the toolpath.

   ![Step 5](pictures/3d_finishing_toolpath/step-5.png)

6. Press **Preview All Toolpaths** to see the smooth dome.

   ![Step 6](pictures/3d_finishing_toolpath/step-6.png)

On a tall model, clear most of the wood first with [3D Roughing](help:3d_roughing_toolpath). When it looks right in the preview, [save it for your machine](help:save_toolpaths).

The **Bit** box on the form says which bit cuts it: **Select…** picks another from the [Bit Library](help:tool_database), and **Edit…** changes its sizes, feeds and speeds there.
