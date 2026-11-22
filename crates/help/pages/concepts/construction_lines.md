---
reviewed: 2026-11-22
see-also: make_construction_line, make_real_line, embellish, import_into_shape
---

# Construction lines

A construction line is a vector you draw only to **place** things. Tools that place or bend
things use it like any other vector, but it is never cut and never saved into another file.
It is drawn as a soft blue dash-dot line, so you can always tell it from the lines that will
be carved.

Use one when you want something to follow a path you don't want carved, for example:

- [Embellish](help:embellish) a border along a curve that isn't part of the sign.
- Bend lettering along an arch with [Import Along This Line](help:import_into_shape).
- Snap shapes to a centre line or a circle you only need as a guide.

## Making one

![A centre line made into a construction line: drawn dashed, never cut](pictures/make_construction_line/step-2.png)

1. Draw the line or shape with any drawing tool, and select it.
2. Right-click it and pick **Make Construction Line**
   ([Make Construction Line](help:make_construction_line)). It turns dashed blue.
3. To carve it after all, select it, right-click and pick **Make Real Line**
   ([Make Real Line](help:make_real_line)).

## What it does and doesn't do

- **Placing tools use it**: Embellish, Import Along This Line, snapping, Node Editing,
  moving, copying and every drawing tool treat it as a normal vector. A copy of a
  construction line is a construction line too.
- **Toolpaths never cut it**: if a construction line is selected with other vectors, the
  toolpath leaves it out. If only construction lines are selected, the toolpath says so
  and tells you how to turn one back. Vector Selector filters never pick them either.
- **Toolpaths already made from it go out of date**: turning a vector a toolpath was made
  from into a construction line marks that toolpath with ⚠, so you know to calculate it
  again.
- **Exports leave it out**: SVG, DXF, PDF and the other exported files, the cut list and
  part labels never include it. It is kept in your saved job, of course.
- **The 3D view doesn't show it**: it isn't part of what is carved.

## Hiding them

**View ▸ Show Construction Lines** hides or shows every construction line. While a job has
any, the same switch sits over the top left of the 2D view: an open eye over a dashed line
while they are shown, a closed eye while they are hidden. Hidden construction lines can't be
picked or snapped to, and any that were selected are let go.
