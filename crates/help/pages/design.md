---
reviewed: 2026-11-28
see-also:
---

# Design

![Design down the left, where you draw; Toolpaths down the right, where you say how to cut it](pictures/basics/design_then_toolpaths.svg)

The Design page, down the left, draws and edits vectors, then models them in 3D. Its side
tabs: **Design** (these tools), **Sheets**, **Layers**, **Components** and **Library** (your own folders of models, pictures and drawings: pick a folder in the tree, or All folders, then double-click or drag one in; right-click for more).
Pick a tool to open its form on the page. Drag on the canvas to draw, or type the numbers
and press **Create**. Once drawn, the shape changes as you change the form, so there is
nothing more to press; **New Shape** leaves it as it is to type another. Tools that change
what is selected say **Apply**, toolpaths say **Calculate**. **Close** (or Esc) puts the
tool away, keeping what is there; **Cancel** takes back what the tool did.

On the **Layers** tab, the eye shows or hides a layer and the padlock locks it against
changes. Neither touches anything else about the layer: its name and its material (set
with right-click ▸ Rename, which toolpaths use to pick feeds and speeds) stay as they are.
A layer belongs to the whole job, so one that is empty on this sheet may hold work on
another: Delete ▸ Empty leaves it, Delete Layer tells you which sheet still uses it, and
Merge Visible brings the other sheets' vectors onto the layer it merges into.

Hover over a tool's button to see what it does, and what to do next, in the box under the
buttons. The buttons stay where they are while you hover; tools that do one job different
ways sit together in a box named for that job.

**Notes** keeps notes with the job, with headings, lists, check boxes, links and live
values (the material's thickness, each toolpath's bit and run time…) picked from its
Variable Explorer. See [Notes](help:notes).
