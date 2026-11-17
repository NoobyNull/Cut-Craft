---
reviewed: 2026-11-14
see-also: chamfer_toolpath, 3d_finishing_toolpath, stepover
---

## Steps

![The steps, one after another](pictures/moulding_toolpath/tour.gif)

1. A moulding sweeps a shape (the profile) along a path (the rail), like a picture frame's edge. Draw the rail, then the profile as an open line: across is its width, up is its height.

   ![Step 1](pictures/moulding_toolpath/step-1.png)

2. Select the rail first, then Shift-click the profile last: the open line picked last is the profile.

   ![Step 2](pictures/moulding_toolpath/step-2.png)

3. On the Toolpaths page, press **Moulding** (under Edges and Inlays).

   ![Step 3](pictures/moulding_toolpath/step-3.png)

4. Check the bit, usually a ball nose. Its stepover sets how close the passes run.

   ![Step 4](pictures/moulding_toolpath/step-4.png)

5. Press **Calculate**. The passes follow the rail, each as low as the profile allows.

   ![Step 5](pictures/moulding_toolpath/step-5.png)

6. Press **Preview All Toolpaths** to see the moulding.

   ![Step 6](pictures/moulding_toolpath/step-6.png)

When it looks right in the preview, [save it for your machine](help:save_toolpaths).
