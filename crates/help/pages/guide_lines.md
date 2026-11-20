---
reviewed: 2026-11-20
see-also: snapping, view_2d, make_construction_line, dimensions
icon: snapping/smart_snapping_on.png
---

# Guide lines

A guide line is a straight line across the 2D view that you line things up against. It is
never cut and never saved as G-code: it is only there to draw and place shapes by. Drawing
[snaps](help:snapping) to guides, so a row of shapes dropped on one sits in a perfect line.

![Two guide lines pulled out of the rulers, and Guide Properties open on one](pictures/basics/guide-lines.png)

## Making a guide

1. Point at the **ruler** along the top of the 2D view (for a guide across the sheet) or
   down its left side (for one up and down).
2. Hold the left mouse button down and **drag** into the view. A line follows the pointer.
3. Let go where you want it.

Or right-click on the sheet and pick **Guidelines ▸ Insert Horizontal Guide**, **Insert
Vertical Guide** or **Insert Angled Guide…**: the guide goes through the spot you clicked.

## Moving, setting and deleting a guide

- **Move it**: point at the guide where no shape lies on it (the pointer shows a two-way
  arrow) and drag it.
- **Put it exactly**: right-click the guide and pick **Guide Properties…**. Type its
  **Position** (where it crosses the X or Y axis) and its **Angle**, then press **Apply**.
- **Make more**: in Guide Properties, under **New guide**, type a **Position**
  (**Absolute**) or an **Offset** from this guide (**Relative**) and a **Count**, then press
  **Create New Guide**. Five guides 20 mm apart take one click.
- **Stop it moving**: right-click it and pick **Lock Guide**. A locked guide can't be
  dragged by accident. **Unlock Guide** frees it again.
- **Delete it**: drag it back onto a ruler, or right-click it and pick **Delete Guide**.
- Started dragging the wrong one? Press **Esc** before you let go, and it stays where it
  was.

## All guides at once

**View ▸ Guide Lines**:

- **Guides Visible** shows or hides them all.
- **Lock All Existing Guides** and **Unlock All Existing Guides**.
- **Delete All Guides** clears them away.

Guides are saved with the job, and adding, moving or deleting one can be undone with
**Ctrl+Z**. Want a line that stays with the drawing, at any angle or shape, but is never
cut? Use a [construction line](help:make_construction_line) instead.
