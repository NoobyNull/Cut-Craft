---
reviewed: 2026-11-27
see-also: interface_layout, layout_ribbon, layout_rail, layout_classic, favourites, find_tool, toolbars, save_toolpaths, view_gcode, machine_files
---

# Canvas layout

The whole window is your drawing. A thin header holds a box to find a tool; everything
else floats over the view, so you see the most of your job. Two bars of tools float over
it, both at once: the **Modeling** bar on the left, for drawing and modeling, and the
**Toolpaths** bar on the right, for making toolpaths and saving them for your machine.

![The Canvas layout, light theme on the left and dark on the right](pictures/layouts/canvas-light-and-dark.png)

## Finding a tool

1. Look on the left for drawing and modeling tools, or on the right for toolpaths. Each
   bar shows one icon for each group of tools: the Modeling bar has the drawing groups
   then the modeling ones, the Toolpaths bar every kind of toolpath, then templates,
   reports and **Save for the Machine**. In a short window a bar goes on to a second
   column, so every group shows.
2. Point at an icon. The group's tools are listed beside the bar, on the drawing's side,
   by icon and name, with what the tool you point at does under them.
3. Click a tool. Its settings float on the right, beside the Toolpaths bar, and the list
   closes. Moving the pointer away from the bar and the list closes it too.

**Save for the Machine** holds [Save Toolpaths](help:save_toolpaths),
[View G-code](help:view_gcode) and [Machine Files](help:machine_files), so the last step of
every job is on the Toolpaths bar too.

Toolpaths that need something your machine lacks (a 4th axis for the rotary ones, a laser
for the laser ones) are left off until its Machine Configuration has it, as in every layout.

To open a group's list only when you click its icon, set **Options ▸ General ▸ Canvas tool
lists open** to **On Click**. Then click the same icon again to close its list.

Draw Rectangle and Draw Circle are listed once each: **Draw from**, at the top of their
settings, picks how the shape is drawn ([Settings](help:settings) can list each way on its
own instead).

To find a tool by name, click the **Find a tool** box in the header, or press **Ctrl+K**,
and type part of its name. The tools that match drop down under it: click one, or press
**Enter** for the first. **Esc** clears the box. See [Find Tool](help:find_tool).

## Moving the bars

The Modeling and Toolpaths bars are bars like your [own toolbars](help:toolbars). The row
of dots at a bar's top is its **grip**:

- **Drag the dots** to move a floating bar anywhere over the drawing. A snapped bar stays
  on its side: pick **Float** from its menu first. Dragging a group's icon never moves it.
- **Right-click the dots** to **Snap to Top**, **Bottom**, **Left** or **Right**, to
  **Float** it again, to **Lock Position** so a stray drag leaves it where it is, or to
  **Turn** it so it runs across instead of down.

A floating bar starts just past any toolbar snapped to its side, and keeps its distance
from the window corner it is nearest, so it stays in place when the window changes size.
Snapped to the foot, a group's list opens upward. A bar grows to show every group it
holds, up to the window's edge. See [Toolbars of Your Own](help:toolbars) for how snapped
bars and the drawing share the window.

To hide a bar, untick **Options ▸ General ▸ Canvas modeling bar** or **Canvas toolpaths
bar**. Find a Tool (**Ctrl+K**) still finds every tool.

## Sheets, layers and toolpaths

Along the foot of the view sit **Sheets**, **Layers**, **Components**, **Library**,
**Toolpaths** and **Checks**. Click one to open its list over the drawing, and click it
again to close it. **Checks** shows what would stop or spoil the cut before you save it.

## Your tools at hand

Right-click a tool in a list, or a group's icon, and pick **Add to Favourites**. Your
favourites float over the drawing as a bar of their own. Right-click its dots and pick
**Attach to Header** to make them a **Favourites** tab in the header instead: click it to
list them beside the Modeling bar, and click it again to close the list. Once you install a
gadget or add-on, a **Gadgets** tab comes last, and opens and closes its list the same way. See [Favourites](help:favourites).

The view's own bar (2D or 3D, the split, the sheet, layer and level, and snapping) stays at
the top of the view, as in every layout.

To pick another layout, press [Interface Layout](help:interface_layout).
