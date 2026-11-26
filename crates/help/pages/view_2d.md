---
reviewed: 2026-11-26
see-also: view_3d, pan, zoom_box, zoom_to_job, keyboard_shortcuts, interface_layout, job_setup, settings
icon: view/2d_view.png
---

# Moving around the 2D view

The 2D view is the flat drawing of your material, where you draw and pick shapes. You can
zoom in to work on a small detail and move about the sheet without changing anything in
the job.

## Zooming and moving with the mouse

![Roll the wheel to zoom about the pointer; drag with the wheel pressed to move around](pictures/basics/mouse_zoom.png)

1. Point at the part of the drawing you want a closer look at.
2. **Roll the mouse wheel** away from you to zoom in, towards you to zoom out. The view
   zooms about the pointer, so what you point at stays under it.
3. To move around, press the **wheel** down like a button, hold it, and **drag**.
4. Tilting the wheel left or right (or swiping sideways on a touchpad) moves the view
   sideways. **Edit ▸ Options ▸ Mouse and Shortcuts** chooses which way, or turns it off.

No wheel button, or prefer another? In **Edit ▸ Options ▸ Mouse and Shortcuts**, set
**Pan (hold and drag)** to a thumb button instead. See
[Keyboard shortcuts](help:keyboard_shortcuts).

## Zoom tools

- [Zoom to Job](help:zoom_to_job) fits the whole sheet in the view.
- **View ▸ Zoom to Drawing** (**F6**) fits everything you have drawn.
- [Zoom to Selection](help:zoom_selected) fits the shapes you picked.
- [Zoom Box](help:zoom_box): drag a box round what you want to fill the view.
- [Pan](help:pan): drag with the left button to move the view, for a mouse with no wheel.

## Rotary jobs: the seams and the wrap bands

On a [rotary job](help:job_setup) the sheet is the outside of the material unrolled flat:
one full turn, from one **seam** to the other. The two seams are the same line on the real
material, so a shape that runs off one seam carries on in from the other.

The 2D view shows this for you:

![A rotary job in the 2D view, light and dark: two blue seams marked A 0° and A 360°, shaded bands above and below, and two slanted lines that run off the top seam seen coming back in, faded, at the bottom](pictures/rotary-wrap-bands.png)

1. The two seams are drawn as **blue lines**, each with the **A angle** the rotary turns to
   there (for example **A 0°** and **A 360°**).
2. Past each seam is a shaded **wrap band**: what lies just over the seam on the other side.
   On material lying along X the bands are above and below the sheet; along Y they are to
   its left and right.
3. Anything near one seam is drawn again, faded, a turn away: a **ghost**. A line that runs
   off the top of the sheet is seen coming back in at the bottom, where it will really be
   cut.
4. Click a ghost to pick the shape it copies: the shape and its ghost are both picked. When
   you draw, the ends and middles of ghosts snap just as the shapes themselves do, so a new
   line can join up with a shape across the seam.
5. You can draw in a band too. What you draw stays where you drew it, and its ghost shows
   on the other side.

[Zoom to Job](help:zoom_to_job) fits the sheet and both bands, so the seams are always in
view. To change how far the bands reach, open **Edit ▸ Options ▸ Appearance** and set
**Rotary wrap bands** to **Quarter turn** (90°, the one Cut Craft starts with), **Half
turn** (180°: the whole of the other side at once) or **Off**.

Toolpaths are not drawn again in the bands: to see a rotary toolpath all the way round, use
the toolpath preview in the [3D view](help:view_3d).

## 2D or 3D

![F2 shows the 2D view and F3 the 3D view](pictures/basics/views_2d_3d.png)

- **F2** shows the 2D view, for drawing.
- **F3** shows the 3D view, for models and the toolpath preview. See
  [3D View](help:view_3d).
- **F11** lays the window out for designing: the Design page held open, Toolpaths put
  away. **F12** lays it out for machining: the Toolpaths page held open, Design put away.

## Changed your mind mid-drag?

Started dragging the wrong thing, in the 2D or 3D view? Press **Esc** while the mouse
button is still down. The drag is cancelled and nothing changes: whatever you were pulling,
a shape or a guide line, stays where it was.
