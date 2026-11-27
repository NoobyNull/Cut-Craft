---
reviewed: 2026-11-27
see-also: draw_text, save_toolpaths
---

## Steps

![The steps, one after another](pictures/vcarve_engrave_toolpath/tour.gif)

1. Select the letters or shapes to carve. V-carving suits lettering and shapes with sharp corners.

   ![Step 1](pictures/vcarve_engrave_toolpath/step-1.png)

2. On the Toolpaths page, press **V-Carve** (under Carve and Engrave).

   ![Step 2](pictures/vcarve_engrave_toolpath/step-2.png)

3. Check the bit is a **V-bit**: first in the form, drawn with its point beside its name. **Select…** picks one from your Bit Library; its angle sets how deep each part is carved.

   ![Step 3](pictures/vcarve_engrave_toolpath/step-3.png)

4. Choose how to carve. **Fill** carves the whole shape, as deep as each part's width needs, so the sides meet in a sharp crease.

   ![Step 4](pictures/vcarve_engrave_toolpath/step-4.png)

5. Press **Calculate**. Wide parts are carved deeper, narrow points shallower.

   ![Step 5](pictures/vcarve_engrave_toolpath/step-5.png)

6. Press **Preview All Toolpaths** to see the carving in 3D.

   ![Step 6](pictures/vcarve_engrave_toolpath/step-6.png)

V-carving needs a V-bit; pick one from the [Bit Library](help:tool_database).

The picture under **Cutting Depths** marks **Start depth** (D1) and **Flat depth** (D) in
magenta, with the same letters beside the settings.

Carving lots of letters or a detailed picture can take a while to calculate. The progress
window moves on as each shape is carved, gives each clearance bit its share, and says about
how long is left ([Long jobs and the time left](help:progress)). Cancel stops it with
nothing changed.

## Carving style: how the passes are cut

A Fill, Inside or Outside carve deeper than the bit's pass depth goes down in passes. The
**Carving style** drop-down sets how, and the picture above it shows the one picked. Every
style carves the same shape; they differ in how many moves the G-code has and how the bit
gets from one cut to the next. In each picture the left half is a pass on the way down and
the right half the last pass; orange dots are where the bit goes down, dashed lines where it
lifts and moves over.

**Stay down** (new toolpaths): each pass on the way down cuts one ring round its floor, and
the centre lines out to the points are carved once, in the last pass. The bit goes down once
for each shape and stays down until it is done. **Order** sets which centre line comes next:

- **Round the shape** goes round each shape the same way every time, out and back along each
  branch in turn, so every letter and point is cut alike.

  ![Stay down, round the shape](pictures/vcarve_engrave_toolpath/style-stay-down-round.png)

- **Shortest path** takes the nearest cut next.

  ![Stay down, shortest path](pictures/vcarve_engrave_toolpath/style-stay-down-shortest.png)

**Lift between cuts** cuts each stretch of centre line once, lifting between them: the
fewest moves of all. **Direction** sets which end each stretch starts at:

- **Shallow in** starts at the surface, where the shape narrows to a point, and goes down.
  The bit eases in and never plunges deep, which keeps the points from chipping.

  ![Lift between cuts, shallow in](pictures/vcarve_engrave_toolpath/style-lift-shallow-in.png)

- **Depth out** goes straight down at the deep end and comes up to the surface at the point.

  ![Lift between cuts, depth out](pictures/vcarve_engrave_toolpath/style-lift-depth-out.png)

**Light passes** clears the whole floor in every pass and follows every centre line in every
pass, so no pass cuts deeper than the bit's pass depth: the most moves, for hard woods and
light machines. Toolpaths made before there was a choice keep it.

![Light passes](pictures/vcarve_engrave_toolpath/style-light-passes.png)

The style new V-Carve toolpaths start with is set in Options ▸ Toolpath Settings (**V-Carve
carving style**, with its order and direction); each toolpath keeps its own.

## When the bit is too short

A V-bit carves to a point only as deep as its V is tall (a 1/2" 90° bit: 6.35 mm). Where a
Fill shape is wider than that, the middle is cut flat at that depth, and the form says so in
red. **Clearance Bits** clear that flat bottom quicker with an end mill; a wider V-bit carves
it deeper.
