---
reviewed: 2026-11-22
see-also: import_component_3d_model, generate_border, scale_model_z_height, slice, view_3d, material_setup, rotary_machining_toolpath, rotary_lathe_toolpath
---

## Borders from the Library

A border made with [Generate Border](help:generate_border) comes in here too, like a 3D model:
double-click it in the Library under **Generated ▸ Borders**. It arrives at the length,
height and depth it was made at, shown in your job's units, ready to turn, size and place.

## Turning the model with the rings

While you import a model, three rings round it in the 3D view turn it: the **red** ring
about **X**, the **green** ring about **Y** and the **blue** ring about **Z**, the same colours
as the **X**, **Y** and **Z** boxes under **Interactive Rotation** and the arrows on the view
cube. Drag the grey ball in the middle to turn it about all three.

Each ring has a pair of arrows, marked **X+** and **X−** on the red ring (Y+ and Y− on the
green one, and so on). Drag along the ring the way the **+** arrow points and the angle in
that ring's box goes up; drag the **−** way and it goes down. Only that box changes.

Before you turn anything, each ring is square to the job's own X, Y or Z. Once you have
turned the model about Z, the green Y ring turns with it, and the red X ring follows the Z
and Y turns. That way each ring always goes round the axis its own box turns the model
about, so the model turns exactly the way the ring's arrows show.

![The red, green and blue rings round a model, each with its + and − arrows](pictures/turn-rings.png)

Steps:

1. Press **Import 3D Model** and pick your file. The model shows in the 3D view with the
   rings round it.
2. Drag a ring to turn the model about that axis, or type the angle into its box.
3. Changed your mind? Press **↺ Reset rotation** at the foot of Interactive Rotation. The
   model goes back to how its file has it: its top up, every turn back to 0.
4. Moved it with **Align in Material**, a position or Depth Below Top? Press
   **⌖ Reset position** at the foot of **Position relative to Modeling Plane**. The model goes
   back to where it came in: centred on the material, sitting on the zero plane (on a
   rotary job, back on the axis).

Nothing is imported until you press **Import**, so the reset buttons need no **Undo**. Undo
takes back an import that is already made. Each reset button is greyed while there is
nothing to put back.

## Where the model sits in the material

A 3D model is usually thinner than the board you carve it from. **Model Position in
Material** says where in the board's thickness the model goes. It is in **Material Setup**
(opened from the Toolpaths page) and on **Import 3D Model**'s form; both change the same
setting for the whole job.

The **gap above** and the **gap below** the model always add up to the material's
thickness less the model's. A 10 mm model in 20 mm material leaves 10 mm to share: type
one gap and the other is what is left. Both are shown under the setting, with a picture of
the model in the board drawn to scale.

![Top at material top: the model's highest point flush with the board's top, the rest of the board cut down round it](pictures/model-top-at-top.png)

## The four ways to place it

- **All the way up**: press **⬆ Top at material top** (Gap Above Model, gap 0). The
  model's highest point is the board's top, so the least material is cut away. The usual
  choice.
- **A set distance under the top**: pick **Gap Above Model** and type the gap. The whole
  model is cut that much lower.
- **Bottom on the bottom**: press **⬇ Bottom on material bottom** (Gap Below Model, gap
  0). The model's lowest point is the board's bottom. If your Z zero is the **Machine
  Bed**, that is Z 0.
- **A set distance over the bottom**: pick **Gap Below Model** and type the gap, to leave
  that much solid board under the model.

![Bottom on material bottom: everything round the model is cut down to the board's bottom](pictures/model-bottom-on-bottom.png)

Press **Apply** and the 3D view shows the model where the toolpaths will cut it: inside
the board, with what is round it cut down to the model's base. A toolpath preview carves
the same place.

## Lining it up on the material, then resizing it

**Align in Material** on the import form lines the model up with the material: against
its left or right side, its front or back edge, or centred, and in height on the zero
plane, half way up, or with its top at the material's top. A new model starts centred,
its bottom on the zero plane.

The alignment is kept. Resize the model (below) and it is lined up again at its new size,
measured from the material's corner (its 0,0). So a model aligned to the right side stays
against the right side as it grows, and a centred one stays centred with nothing of it
under the modeling plane.

Type a **position** X or Y, or a **Depth Below Top**, and that number is used as you
typed it instead: the model's corner stays there when it is resized. Press an Align in
Material button again to go back to lining it up.

## Changing the model's size

A model comes in at the size its file says. Its size is **locked**, so turning it or
nudging a setting never resizes it without you knowing:

1. Under **Model Size**, tick **Change size**. Until you do, X, Y, Z and **Lock XYZ
   ratio** are greyed out, and so is the button at the foot of the form.
2. Either type a new **X**, **Y** or **Z** (with Lock XYZ ratio ticked the other two
   follow), or press **Scale to Fit Material**, the last button on the form. It makes the
   model as big as the material allows (on a rotary job, as long as the job and out to
   the material's radius), turned the way it is now.
3. Untick **Change size** to lock the new size in.

Either way the model is scaled **once**. Turn it afterwards with **Interactive Rotation**
and it keeps that size: it is not fitted again at every step of the turn. If a turn makes
it poke out of the material, press **Scale to Fit Material** again.

![A rotary import: Change size ticked under Model Size and the model scaled to fit, shown round the axis inside the material](pictures/rotary-import-scale-to-fit.png)

## When it doesn't fit

A gap bigger than the room left is kept as you typed it, never refused: the model is drawn
in red where it pokes out, and a red note at the foot of the form says by how much and the
largest gap that fits.

![A gap too big: the model pokes out of the top, drawn in red, with the red note at the foot of the form](pictures/model-sticks-out.png)

- Out of the **top**: the cutter would only cut air there, so the top of the model comes
  out flat.
- Out of the **bottom**: the bit would cut into your spoilboard.
- A model **taller than the material** never fits: use thicker material, lower the model
  (Material Setup's **Model Thickness**, or
  [Scale Model Z Height](help:scale_model_z_height)), or cut it into boards with
  [Slice Model](help:slice) and glue them up.

## On a rotary job

On a [rotary](help:job_setup) job, a **Full** 3D model is a real 3D shape in space: its
length along the job's axis, the axis through its middle (move it off with **Rotation Axis
Movement**). It comes in at its file's size; press **Scale to Fit Material** (above) to fill
the material.

The 3D view always shows it exactly as it is, both while you import it and after you press
**Import**: the same model, in the same place, inside the outline of the material.
[4-Axis Rotary Machining](help:rotary_machining_toolpath) and the
[Rotary Lathe](help:rotary_lathe_toolpath)'s Turn cut round that same shape, so a bit held
off centre can reach under a ledge or into a hollow.

**Model Position in Material** (top at material top, gaps) is for flat jobs and does not
move a full 3D model on a rotary job: it is cut where it sits. Where the material reaches
further out than the model, the toolpath cuts the material down to the model.

![After Import on a rotary job: the model drawn as it really is round the axis, the same as while it was imported](pictures/rotary-imported-as-it-is.png)

Cut Craft also keeps a model's heights round the axis, for the tools that work on
heights. Keeping the model makes the job file bigger, by about the size of the model's
file.

## See also

- [Material Setup](help:material_setup), and **Import Component / 3D Model...** in the
  Import tools, whose form has the same setting
- [3D View ▸ Changing a model's height](help:view_3d): the height buttons on a picked
  model, to change the model itself rather than where it sits
