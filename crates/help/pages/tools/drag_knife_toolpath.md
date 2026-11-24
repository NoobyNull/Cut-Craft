---
reviewed: 2026-11-23
see-also: profile_toolpath, quick_engrave_toolpath, surfacing_toolpath
---

## Steps

![The steps, one after another](pictures/drag_knife_toolpath/tour.gif)

1. A drag knife cuts thin sheet (vinyl, card, thin plastic) with a blade that trails behind like a castor. Select the shapes to cut.

   ![Step 1](pictures/drag_knife_toolpath/step-1.png)

2. On the Toolpaths page, press **Drag Knife** (under Cut Out and Clear).

   ![Step 2](pictures/drag_knife_toolpath/step-2.png)

3. Type the **Blade Offset** from the knife's maker: how far the tip trails behind the pivot. The knife swings round sharp corners by this much so the blade faces the new way.

   ![Step 3](pictures/drag_knife_toolpath/step-3.png)

4. Set the **Pressure**: how hard the blade presses while it cuts. Use more for thicker or tougher sheet.

   ![Step 4](pictures/drag_knife_toolpath/step-4.png)

5. **Overshoot** runs each cut on a little past a sharp corner, so the cuts meet and the corner peels away cleanly.

   ![Step 5](pictures/drag_knife_toolpath/step-5.png)

6. Press **Calculate** to make the toolpath. Switch the spindle off: the knife does not spin.

   ![Step 6](pictures/drag_knife_toolpath/step-6.png)

7. The path follows each line, with small loops at the sharp points where the blade swivels round.

   ![Step 7](pictures/drag_knife_toolpath/step-7.png)

A knife needs a flat surface: make it the first toolpath, or put it straight after a [Surfacing](help:surfacing_toolpath) toolpath. When it looks right in the preview, [save it for your machine](help:save_toolpaths).
