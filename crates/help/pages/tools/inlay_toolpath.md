---
reviewed: 2026-11-24
see-also: vcarve_inlay_toolpath, pocket_toolpath, profile_toolpath
---

## Steps

![The steps, one after another](pictures/inlay_toolpath/tour.gif)

1. An inlay is a part and the hole it fits, both cut with one bit. Select the shape.

   ![Step 1](pictures/inlay_toolpath/step-1.png)

2. On the Toolpaths page, press **Inlay** (under Edges and Inlays).

   ![Step 2](pictures/inlay_toolpath/step-2.png)

3. Choose which half this toolpath cuts: **Male** is the insert (straight, or stepped with a shelf round its base), **Female** is the pocket or hole it fits in. Make one toolpath for each half.

   ![Step 3](pictures/inlay_toolpath/step-3.png)

4. Set the **Allowance**: the insert is cut this much smaller (or the pocket this much bigger) so they fit.

   ![Step 4](pictures/inlay_toolpath/step-4.png)

5. Press **Calculate**. The star's inside corners are rounded to the bit's radius, so the two halves match.

   ![Step 5](pictures/inlay_toolpath/step-5.png)

6. The toolpath runs round the shape with the corners rounded.

   ![Step 6](pictures/inlay_toolpath/step-6.png)

For sharp corners, use a [V-Carve Inlay](help:vcarve_inlay_toolpath) instead. When it looks right in the preview, [save it for your machine](help:save_toolpaths).

The **Bit** box on the form says which bit cuts it: **Select…** picks another from the [Bit Library](help:tool_database), and **Edit…** changes its sizes, feeds and speeds there.
