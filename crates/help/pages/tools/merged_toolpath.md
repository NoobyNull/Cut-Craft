---
reviewed: 2026-11-25
see-also: array_copy_toolpath, profile_toolpath
---

## Steps

![The steps, one after another](pictures/merged_toolpath/tour.gif)

1. Merged joins toolpaths that use the same bit into one, in the order that travels least. Make the toolpaths first: here a Profile round the circle…

   ![Step 1](pictures/merged_toolpath/step-1.png)

2. …and a Profile round the rectangle, with the same bit.

   ![Step 2](pictures/merged_toolpath/step-2.png)

3. On the Toolpaths page, press **Merged** (under From Other Toolpaths). It merges the toolpaths shown in the list.

   ![Step 3](pictures/merged_toolpath/step-3.png)

4. Tick the orders to try: left to right, bottom to top, in a grid, or nearest next. The quickest is kept. **Merge by part** keeps each part's cuts together.

   ![Step 4](pictures/merged_toolpath/step-4.png)

5. Press **Calculate**: one toolpath now cuts both, and the originals are hidden.

   ![Step 5](pictures/merged_toolpath/step-5.png)

Ungroup takes a merged toolpath apart again. When it looks right in the preview, [save it for your machine](help:save_toolpaths).

Drill bank toolpaths, and rotary toolpaths that hold the bit off the turning axis (Off centre
or Follows the surface finishes, and flat cuts on a locked face), don't merge: their moves only
work in their own order.
