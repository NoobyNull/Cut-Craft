---
reviewed: 2026-11-20
see-also: profile_toolpath, vcarve_engrave_toolpath, stepover
---

## Steps

![The steps, one after another](pictures/pocket_toolpath/tour.gif)

1. Draw the shape to hollow out on the Design page. Here it is a circle.

   ![Step 1](pictures/pocket_toolpath/step-1.png)

2. Click the shape to select it. A pocket clears everything inside the selected shapes.

   ![Step 2](pictures/pocket_toolpath/step-2.png)

3. On the Toolpaths page, press **Pocket** (under Cut Out and Clear).

   ![Step 3](pictures/pocket_toolpath/step-3.png)

4. Set the **Cut Depth** (D): how far below the top of the material the floor of the pocket ends up. Here it is 10 mm.

   ![Step 4](pictures/pocket_toolpath/step-4.png)

5. Check the bit. **Select…** picks another one from your Tool Database; its feeds, speeds and depth per pass come with it.

   ![Step 5](pictures/pocket_toolpath/step-5.png)

6. Press **Calculate** to make the toolpath.

   ![Step 6](pictures/pocket_toolpath/step-6.png)

7. The orange lines are the path the bit will follow, round and round until the inside is cleared.

   ![Step 7](pictures/pocket_toolpath/step-7.png)

8. Press **Preview All Toolpaths** to watch it cut in 3D before you send it to the machine.

   ![Step 8](pictures/pocket_toolpath/step-8.png)

The bit clears the inside in rows that overlap a little. How far over each row moves is the [stepover](help:stepover), set with the bit in the Tool Database.

With clearance bits added, a big pocket can take a while to calculate. The progress window
gives each bit its share of the bar and says about how long is left
([Long jobs and the time left](help:progress)).
