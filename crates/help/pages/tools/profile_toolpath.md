---
reviewed: 2026-11-27
see-also: tabs, pocket_toolpath, save_toolpaths
---

## Steps

![The steps, one after another](pictures/profile_toolpath/tour.gif)

1. Draw the part you want to cut out, and click it to select it.

   ![Step 1](pictures/profile_toolpath/step-1.png)

2. On the Toolpaths page, press **Profile** (under Cut Out and Clear).

   ![Step 2](pictures/profile_toolpath/step-2.png)

3. Check the **Bit**, first in the form. The drawing beside its name shows its shape: orange is the part that cuts. **Select…** picks another one from your Bit Library.

   ![Step 3](pictures/profile_toolpath/step-3.png)

4. Set the **Cut Depth** (D, marked in magenta on the picture above it). To cut a part free, make it the thickness of the material.

   ![Step 4](pictures/profile_toolpath/step-4.png)

5. Choose which side the bit runs: **Outside** cuts out a part the size you drew, **Inside** cuts a hole that size, **On** follows the line.

   ![Step 5](pictures/profile_toolpath/step-5.png)

6. Add **Tabs**: small bridges that hold the part in the board until the last pass, so it cannot fly loose.

   ![Step 6](pictures/profile_toolpath/step-6.png)

7. Press **Calculate** to make the toolpath.

   ![Step 7](pictures/profile_toolpath/step-7.png)

8. The new line is where the middle of the bit will travel: just outside the shape, so the part comes out the size you drew.

   ![Step 8](pictures/profile_toolpath/step-8.png)

9. Press **Preview All Toolpaths** to watch it cut in 3D before you send it to the machine.

   ![Step 9](pictures/profile_toolpath/step-9.png)

When it looks right in the preview, [save it for your machine](help:save_toolpaths).

The **Bit** box on the form says which bit cuts it: **Select…** picks another from the [Bit Library](help:tool_database), and **Edit…** changes its sizes, feeds and speeds there.
