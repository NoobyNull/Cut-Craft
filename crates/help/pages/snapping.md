---
reviewed: 2026-11-25
see-also: guide_lines, settings, draw_polyline, keyboard_shortcuts
icon: snapping/grid_snapping_on.png
---

# Snapping

When you draw or drag a point near something worth lining up with, Cut Craft pulls the
point onto it: the end of a line, the middle of a line, the centre of a circle, a corner of
the material, a [guide line](help:guide_lines) or the grid. That pull is **snapping**. It
makes shapes meet exactly, with no tiny gaps that spoil a toolpath.

![The pointer snapping to the end of a line, and F4 for what it snaps to](pictures/basics/snapping.png)

## What you see

A small marker shows where the point will land, with a word beside it: **End**,
**Middle**, **Center**, **Shape center**, **Material**, **Tangent**, **Perpendicular**,
**Guide**, **Lined up** or **Grid**. The status bar says what it means, such as "halfway
along a line or curve".

- To place a point **freely**, hold **Shift** while you click or drag. Snapping is off
  while Shift is held.
- A point that snapped onto a shape stays on it: move that shape and the point goes with
  it, until you move the point on its own.

## Turning snapping on and off

Three buttons at the top right of the 2D view switch the main kinds on and off straight
away: **Geometry Snapping** (points on your shapes), **Smart Snapping** (lining up across
or up and down with those points) and **Grid Snapping**. A pressed-in button is on.

## Snapping Options (F4)

**Edit ▸ Snap Options** (**F4**) chooses exactly what drawing snaps to:

![Snapping Options, with a change waiting for Apply](pictures/snapping-not-applied.png)

- **Snap to grid** and its **Grid spacing**: points land on a grid of that size. Off to
  start with. The arrow keys nudge the selection by one square of it.
- **Snap to job center and corners**: the material's corners and middle.
- **Geometry snapping**, and the points it uses: **Object centers** (the middle of each
  shape's outline box), **End points**, **Middle points**, **Arc centers**, and **Tangent
  and perpendicular** (while drawing a line, where it just touches a curve, or meets a line
  square on).
- **Smart snapping**: lines the pointer up with the snap points, across or up and down,
  when none is near.
- **Snapping radius**: how near, in screen pixels, the pointer must come to a point to snap
  to it. Raise it if snapping is hard to catch, lower it if points jump about.
- **Fixed nudge distance**: how far **Ctrl** and an arrow key move the selection.
- **Rotate snap angle with Shift** and **with Ctrl**: the angles a rotate drag clicks to.

Changes here wait for **Apply**, like every settings window: see
[Changing settings](help:settings).
