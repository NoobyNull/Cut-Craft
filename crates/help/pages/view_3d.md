---
reviewed: 2026-11-21
see-also: job_setup, check_zeroing, 3d_finishing_toolpath, 3d_roughing_toolpath, create_shape, preview_detail
---

# 3D View

The 3D view (**F3**) shows the job as a block of material, with your models and, once
previewed, the cuts your [toolpaths](help:toolpaths) make in it. How finely a preview is drawn is
up to the [Preview detail](help:preview_detail) slider on the Toolpaths page's Preview tab.

## Moving the view

- **Drag** with the left or right button to turn the job; **drag with the wheel** (or
  Ctrl and the right button) to slide it; **roll the wheel** (or pinch on a trackpad) to
  zoom where the pointer is. If a direction feels backwards, turn it the other way: see
  [Moving the views](help:view_controls).
- **Double-click** off the job, or press **F**, to fit the whole job in the view.
- The cube in the top corner turns the view too: click a face, edge or corner to look
  from there, or drag it.
- **Top**, **Front**, **Right** and **Iso** along the top jump to those views; **Fit**
  fits the job; **Perspective** makes near things larger, as your eye sees them; **Vectors**
  shows or hides the vectors drawn over the model.

## Where zero is

Three short arrows, red for X, green for Y and blue for Z, start at the job's zero: the
spot where you will zero the bit on your machine. They sit at the **XY zero** you picked in
[Job Setup](help:job_setup), at the height its **Z Zero Position** says:

- **Material Surface**: on the top of the material.

  ![Material Surface: the arrows start on the material's top corner](pictures/z-zero-top.png)

- **Machine Bed**: on the bottom of the material, where it lies on the bed.

  ![Machine Bed: the arrows start on the bottom corner, where the material meets the bed](pictures/z-zero-bed.png)

Switch it and press **Apply**, and the arrows move. Your model stays where it is in the
material: only the Z numbers the machine is told change.

Where the material hides part of an arrow, that part is drawn thin and faint, so you can
still see it. With Machine Bed and the XY zero in the **Centre**, the arrows start inside
the block, under the middle of the material: X and Y show faint, and the blue Z arrow pokes
up through the top.

![Machine Bed with the XY zero in the centre: faint arrows inside the block, under its middle](pictures/z-zero-bed-centre.png)

**On a [rotary job](help:rotary_firmware)**, Machine Bed is the turning axis instead: the
middle of the blank, half its thickness under its top (20 mm under for a 40 mm blank).
The arrows are not drawn while the job is wrapped round its axis: untick View ▸
**Auto-Wrapping View** to see the job flat, with the arrows on it.

While [Import 3D Model](help:import_component_3d_model) is open, the arrows sit on the material's
outline drawn round the model: on its top, or on its bottom for Machine Bed.

To hide the arrows, untick View ▸ **Draw Origin**.

## Changing a model's height

Every model (component) has round buttons for changing its height right in the 3D view.

![Click the model: it is outlined in blue and its height buttons appear; drag one up or down](pictures/model-height-buttons.gif)

1. In the 3D view, **click** the model. A thick blue line goes round it to show it is
   picked, and a row of round buttons appears along the front edge of its box, with the
   move, scale and rotate handles over it (this is **Transform** mode).
2. Drag a button **up** to make the model taller, **down** to make it lower. The model
   follows as you drag, and its size shows in the status bar along the bottom. Let go when
   it looks right. **Ctrl+Z** undoes the whole drag in one go; **Esc** while dragging
   cancels it.

![The height buttons: Total Height, Shape Height, Base Height and Settings](pictures/model-height-buttons.png)

Each picture shows the model from the side: the orange part is what the button changes,
and the blue arrow how far it reaches.

- **Total Height**: the whole model, shape and base together, taller or lower.
- **Shape Height**: only the raised shape, as a percentage of its own height (100% is as
  it was made). Drag up a quarter of the view to double it.
- **Base Height**: the flat, solid layer the shape stands on, thicker or thinner.
- **Settings**: click it for the rest: how it combines with the models under it, fade,
  tilt and its color.

**Shift**-click adds another model to the ones picked, or takes one away. Click anywhere
off the models to let go of them. The buttons also go away when you press **Esc** or click
**Transform** again; click the model to bring them back. While another view tool such as
Measure is in use, a click picks the model (blue outline) without changing the tool.

The 2D view never shows these buttons. To set the heights by typing numbers instead,
right-click the model in the 3D view and choose **Component ▸ Properties** (or pick it and
use **Model ▸ Selected Component Properties**, or right-click it in the **Components** tab
and choose **Properties**): **Shape Height** and **Base Height** there are the same
settings. To change the height of every model at once, use **Scale Model Z Height** on the
Design page.

## Auto-rotate

**Auto-rotate**, at the end of the row along the top, turns the job round and round on
the spot, so you can see every side of it. Use it to check cuts that the lighting or the
angle hides: a shallow pocket, a groove on the far side, a V-carve in shadow.

![Auto-rotate on: the job turns round on its own, with its speed beside the button](pictures/auto-rotate.gif)

1. Click **Auto-rotate**. It lights up and the job starts to turn.
2. Drag the **Speed** slider beside it: slower to look closely, faster for a quick look all
   round, from 2 to 90 degrees a second. The speed is kept for next time, and can be set in
   Edit ▸ Options ▸ 3D View as **Auto-rotate speed** too.
3. To stop, click **Auto-rotate** again, or just drag in the view (or on the cube) to take
   over: the job stays where it stopped. Top, Front, Right and Iso stop it too.

A flat job turns about its own upright, keeping the tilt you gave it, so tilt the view
first to look at the job from lower down or higher up while it turns.

A **rotary job** (a cylinder on a 4th axis) turns round the cylinder's own long axis
instead, as the rotary axis on your machine turns it: the axis stays where it is on the
screen and the whole way round the cylinder comes past you, so you can check every cut on
its round side. With View ▸ Auto-Wrapping View off, the rotary job is drawn flat and turns on
the spot like a flat job. On a slower computer the
model is drawn a little coarser while it turns, and sharp again once you stop it.

See also: [Design page](help:design), [Toolpaths page](help:toolpaths).
