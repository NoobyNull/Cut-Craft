---
reviewed: 2026-11-26
see-also: interface_layout, layout_ribbon, layout_rail, layout_classic, favourites, find_tool, toolbars
---

# Canvas layout

The whole window is your drawing. A thin header holds the tabs and a box to find a tool;
everything else floats over the view, so you see the most of your job.

![The Canvas layout, light theme on the left and dark on the right](pictures/layouts/canvas-light-and-dark.png)

## Finding a tool

1. Pick **Design**, **Model** or **Toolpaths** in the header. The palette down the left
   shows that tab's tool groups, one icon for each. In a short window the groups go on to
   a second column, so every group shows.
2. Point at an icon. The group's tools are listed beside the palette by icon and name,
   with what the tool you point at does under them.
3. Click a tool. Its settings float on the right, and the list closes. Moving the pointer
   away from the palette and the list closes it too.

To open a group's list only when you click its icon, set **Options ▸ General ▸ Canvas tool
lists open** to **On Click**. Then click the same icon again to close its list.

Draw Rectangle and Draw Circle are listed once each: **Draw from**, at the top of their
settings, picks how the shape is drawn ([Settings](help:settings) can list each way on its
own instead).

To find a tool by name, click the **Find a tool** box in the header, or press **Ctrl+K**,
and type part of its name. The tools that match drop down under it: click one, or press
**Enter** for the first. **Esc** clears the box. See [Find Tool](help:find_tool).

## Moving the palette

The palette is a bar like your [own toolbars](help:toolbars). The row of dots at its top is
its **grip**:

- **Drag the dots** to move the palette anywhere over the drawing. Dragging a group's icon
  never moves it.
- **Right-click the dots** to **Snap to Top**, **Bottom**, **Left** or **Right**, to
  **Float** it again, to **Lock Position** so a stray drag leaves it where it is, or to
  **Turn** it so it runs across instead of down.

A floating palette keeps its distance from the window corner it is nearest, so it stays in
place when the window changes size. See [Toolbars of Your Own](help:toolbars) for how
snapped bars and the drawing share the window.

## Sheets, layers and toolpaths

Along the foot of the view sit **Sheets**, **Layers**, **Components**, **Library**,
**Toolpaths** and **Checks**. Click one to open its list over the drawing, and click it
again to close it. **Checks** shows what would stop or spoil the cut before you save it.

## Your tools at hand

Right-click a tool in a list, or a group's icon, and pick **Add to Favourites**. Your
favourites float over the drawing as a bar of their own. Right-click its dots and pick
**Attach to Header** to make them a **Favourites** tab in the header instead, which lists
them in place of the palette. Once you install a gadget or add-on, a
**Gadgets** tab comes last. See [Favourites](help:favourites).

The view's own bar (2D or 3D, the split, the sheet, layer and level, and snapping) stays at
the top of the view, as in every layout.

To pick another layout, press [Interface Layout](help:interface_layout).
