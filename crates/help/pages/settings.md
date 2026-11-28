---
reviewed: 2026-11-28
see-also: toolbars, backups, view_2d, performance, setup_material, job_setup, web_api, bug_reports, favourites, keyboard_shortcuts, colors_on_screen, confirmation_choices, screen_readers
---

# Changing settings

**Edit ▸ Options** holds PowerWhittle's settings, page by page down the left: how it looks,
the mouse and shortcuts, toolpath previews, the [language](help:language), the
[icons](help:icons), the Web API
and more.

A change you make there does **not** take effect straight away. It waits until you press
**Apply**, so you can try a few changes, look them over, and still back out.

![Options with a change not applied yet: the note at the foot and Apply ready to press](pictures/options-not-applied.png)

## The buttons along the foot

- **Apply** uses your changes now and keeps the window open. It is greyed out until you
  change something.
- **Cancel** throws away every change you have not applied and closes the window. The
  settings stay as they were.
- **Close** closes the window. If some changes are not applied yet it asks first, so
  nothing is lost or kept by surprise.

While changes are waiting, the foot of the window says **● Changes not applied yet**. Once
everything is applied it says **No changes waiting**.

**Reset to defaults** on each page puts that page's settings back as PowerWhittle first had
them. That is a change too: press **Apply** to use it.

## When you close with changes waiting

Closing the window, with **Close** or its own close box, while changes are waiting asks:

![Apply your changes? with Apply and Close, Discard Changes and Keep Editing](pictures/options-apply-question.png)

1. **Apply and Close** uses the changes and closes.
2. **Discard Changes** throws them away and closes.
3. **Keep Editing** takes you back to the settings. Pressing Esc does the same.

## Job Defaults: what every new job starts with

The **Job Defaults** page holds the settings every new job starts with: the material's kind
and size, the units, where Z zero and X0 Y0 are, the modelling resolution, the material and
its colour, the rapid heights and home. These are [Job Setup](help:job_setup)'s settings,
kept for every new job. Set them once for your machine.

![Options at Job Defaults: Classic Startup Flow, then the settings every new job starts with](pictures/options-job-defaults.png)

Changing them, and pressing **Apply**, changes the **next** job you start, never the one open.
To change the open job, use [Setup Material](help:setup_material) and its **More settings
for this job…**.

At the top, **Classic Startup Flow** brings back the old start: PowerWhittle opens on its start
page (recent files, the sample project), and **New** asks for every setting before the job
opens. Left off, PowerWhittle opens straight to a blank job and New makes another.

**Reset to defaults** on this page puts back PowerWhittle's own starting job: 600 × 400 × 20 mm,
single sided, Z zero on top, X0 Y0 at the bottom left.

## Performance: keeping the computer responsive

The **Performance** page sets how much of the computer calculations may use (80% of the
processor's cores to start with, at a priority that gives way to your other programs), and
holds the speed-or-detail choices for previews and the 3D view. See
[Keeping the computer responsive](help:performance).

## Snapping Options, Machine Configuration and the Bit Library

The other windows that hold settings work the same way:

- **Snapping Options** (F4) has the same **Close**, **Cancel** and **Apply** along its foot.
  What drawing snaps to changes only once you press **Apply**.

![Snapping Options with a change not applied yet](pictures/snapping-not-applied.png)

- **Machine Configuration**, the **Bit Library** and the **Materials Library** keep their
  **OK**, **Cancel** and **Apply** where they always were. **Apply** is greyed out until you
  change something, the same **● Changes not applied yet** note shows beside it while
  changes wait, and **OK** keeps them and closes.
- In **Machine Configuration**, tick **Laser fitted** if your machine has a laser on its head,
  and **4th axis** if it has a rotary. The [laser toolpaths](help:laser_cut_fill_toolpath)
  and the [rotary toolpaths](help:rotary_machining_toolpath) show on the Toolpaths page only
  for a machine that has them, so a machine without them has fewer tools to wade through.
- Closing any of them with its own close box while changes wait asks **Apply your
  changes?** first, just as Options does.

**Material Setup** and the other forms on the Design and Toolpaths pages are not windows:
their **Apply** button uses what you typed, as before.

## Shortcuts too

A shortcut you set or remove in **Mouse and Shortcuts** also waits for **Apply**. Until
then the old shortcut still runs.

**Mouse and Shortcuts** also holds **Trackpad mode** and an **Invert** box for each way of
moving the views: see [Moving the views](help:view_controls).

## How the window looks: four layouts, light or dark

PowerWhittle can arrange its tools four ways: **Ribbon** (the one it starts with), **Rail**,
**Canvas** and **Classic**. Press [Interface Layout](help:interface_layout) under 2D View Control to see
each and pick one, or use **View ▸ Window Layout** or **Options ▸ General ▸ Window layout**.
Every tool is in every layout; only where the buttons sit changes. **Ctrl+K** finds a tool by
name whichever you use.

**General ▸ Customize Toolbars...**, at the top of the page, opens
[Customize Toolbars](help:customize_toolbars) at once, to build your own toolbars.

**General ▸ Show a tip when PowerWhittle starts** shows a [Did You Know?](help:tips) tip you have
not seen yet each time PowerWhittle opens. Which tips you have seen is kept with these settings,
on this computer only.

Each layout also comes light or dark: **Options ▸ Appearance ▸ Theme** picks **Light**,
**Dark**, or **System** to follow your desktop. In the pictures below the light theme is on
the left and the dark theme on the right. **Options ▸ Appearance ▸ Rotary wrap bands** sets
how far past each seam a rotary job's 2D view shows the other side of the material (see
[Moving around the 2D view](help:view_2d)).

**Options ▸ Appearance ▸ Form pictures** sets where a tool form's measurement pictures go.
**Beside the fields**, the setting it starts with, draws each one small at the right of the
settings it marks, so more of the form fits on the screen. **Full width** draws them across
the form, above their settings. Either way the letters on a picture are the same size as the
form's text, and **Text size** makes both bigger or smaller together.

**Options ▸ Appearance ▸ Tool forms** sets how a toolpath's form is laid out. **Tabs**, the
setting it starts with, splits it into a few short pages across its top: **Bit & Depth**,
the toolpath's own, **Where**, **Movement**, **Finishing** and **Defaults** (see
[Toolpaths](help:toolpaths)). **One page** shows every setting in one long list, as before.

**Options ▸ Appearance ▸ Calculate button** sets where a tabbed form's **Calculate** is.
**Under every tab**, the setting it starts with, keeps it at the foot of every tab, so a
change on any tab can be calculated at once. **Last tab only** puts **Next** there instead
until the last tab, to go through every tab before calculating.

**[Ribbon](help:layout_ribbon)**: Design, Model and Toolpaths tabs across the top, each a row
of tool groups, the sheets, layers and toolpath list on the left, and the open form on the
right. A **Favourites** tab holds the tools you keep at hand.

Each Ribbon group shows the tools that fit under its name as icons. Click a group's name,
such as **Create Vectors ▾**, to list every tool in it; the pin beside each one puts it on
the Ribbon or takes it off. **General ▸ Ribbon icons** set to **Every Tool** shows every
tool as an icon instead (**Compact** is the starting choice). When the window is too narrow for every group on a tab, the groups
carry on along a second row, in the same order, so nothing is hidden.

A shape you can draw more than one way has one button: **Draw Rectangle** (from a corner or
the center) and **Draw Circle** (from the center or an edge). **Draw from**, at the top of its
form, picks the way. In every layout, **General ▸ Other ways to draw a shape** set to **As buttons**
gives [Draw Rectangle from Center](help:draw_rectangle_from_center) and
[Draw Circle from Edge](help:draw_circle_from_edge) buttons of their own again.

In the Canvas layout a group on the Modeling or Toolpaths bar lists its tools as soon as you
point at it. Set **General ▸ Canvas tool lists open** to **On Click** to open a list only
when you click its group. **General ▸ Canvas modeling bar** and **Canvas toolpaths bar**
(both ticked to begin with) show or hide each bar.

Bars of tools (the Canvas Modeling and Toolpaths bars and [your own toolbars](help:toolbars)) can be snapped to a
side of the drawing. **General ▸ Bars snapped to a side** picks whether the drawing makes room
for them (**Push the drawing over**, the first choice) or they lie over its edge (**Over the
drawing**). **General ▸ New toolbars start** picks whether a toolbar you make floats over the
drawing (the first choice) or goes in the layout. **General ▸ Add to menus** picks whether a
tool's right-click menu lists **Add to** each toolbar by name (the first choice; past six
toolbars they go under **Add to Toolbar**) or always puts them under **Add to Toolbar**.

![The Ribbon layout, light theme on the left and dark on the right](pictures/layouts/ribbon-light-and-dark.png)

**[Rail](help:layout_rail)**: one icon for each tool group down the far left. Click one and
that group's tools are listed by name beside it. The open tool's form sits in the inspector
on the right.

![The Rail layout, light theme on the left and dark on the right](pictures/layouts/rail-light-and-dark.png)

**[Canvas](help:layout_canvas)**: the whole window is the drawing. The Modeling bar of tool groups
floats down the left and the Toolpaths bar down the right, the open tool's form floats
beside it, and the sheets, layers and
toolpaths open from chips along the foot. A box in the header finds a tool by name.

![The Canvas layout, light theme on the left and dark on the right](pictures/layouts/canvas-light-and-dark.png)

**[Classic](help:layout_classic)**: the Design page with its tool groups down the left, tabs
down the side for Sheets, Layers, Components and Library, and the Toolpaths page on the
right.

![The Classic layout, light theme on the left and dark on the right](pictures/layouts/classic-light-and-dark.png)

In Rail and Ribbon, the panel that lists sheets, layers, components, the library and the
toolpaths has a **Checks** tab too. It runs the same
[checks before cutting](help:checks_before_cutting) as the Classic Toolpaths page's
**Analyse** tab, so you can see what would spoil the cut before you press Save.

Once you install a gadget, or an add-on that brings a tool, each layout gains a **Gadgets**
section holding it, always after everything else so no button moves: at the bottom of
Classic's Design page (when it shows all tools, not only favourites),
as the last button of the Rail (after ★), and as a **Gadgets** tab at the end of the Ribbon.
See [Add-ons](help:addons).

Each layout shows the tool you point at with what it does and the next step: under the
icons in Classic, under the open list in Rail, and at the foot of the left-hand panel in
Ribbon.

## Favourites: the tools you keep at hand

Right-click any tool button and pick **Add to Favourites**, then show just your favourites:
in Ribbon on the **Favourites** tab, in Rail with **★**, and in Classic from the Design or
Toolpaths side tab's right-click menu. See [Favourites](help:favourites) for every step.

To pick another layout, use [Interface Layout](help:interface_layout).

## Backups

**Options ▸ Backups** lists the backups PowerWhittle has made by itself, and sets how often and
how many. **Help ▸ Back Up Settings** saves everything set up here, and more, in one file. See
[Backing up your settings](help:backups).

See also: [Web API](help:web_api), [3D View](help:view_3d), [Did You Know?](help:tips),
[Interface Layout](help:interface_layout).
