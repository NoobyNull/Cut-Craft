---
reviewed: 2026-11-23
see-also: laser_picture_toolpath, profile_toolpath
---

## Steps

![The steps, one after another](pictures/laser_cut_fill_toolpath/tour.gif)

1. A laser can cut shapes out or mark them with a hatched fill. Select the shapes.

   ![Step 1](pictures/laser_cut_fill_toolpath/step-1.png)

2. On the Toolpaths page, press **Laser Cut / Fill** (under Laser). It shows once **Laser fitted** is ticked in Machine Configuration.

   ![Step 2](pictures/laser_cut_fill_toolpath/step-2.png)

3. Choose the **Strategy**: cut outside, inside or on the line (the beam's width is allowed for), or **Hatch fill** to mark the inside with lines.

   ![Step 3](pictures/laser_cut_fill_toolpath/step-3.png)

4. Set the **Power** and the number of **Passes**: more passes cut through thicker material. Try them on a scrap first.

   ![Step 4](pictures/laser_cut_fill_toolpath/step-4.png)

5. Press **Calculate** to make the toolpath.

   ![Step 5](pictures/laser_cut_fill_toolpath/step-5.png)

6. The hatch lines fill the star, with an outline round it for a crisp edge.

   ![Step 6](pictures/laser_cut_fill_toolpath/step-6.png)

When it looks right, [save it for your laser](help:save_toolpaths).

## No Laser group on the Toolpaths page?

The laser toolpaths show only for a machine with a laser. Open **Machine ▸ Machine
Configuration**, tick **Laser fitted** for your machine and press **OK**: the **Laser** group
appears. A laser toolpath already in a job stays in the list either way. If the active
machine has no laser, its form says so in red at the foot, and you can still edit it.

**Select…** on a laser toolpath shows only the lasers in the [Tool Database](help:tool_database);
on any other toolpath it shows only bits.
