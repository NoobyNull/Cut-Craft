---
reviewed: 2026-11-19
see-also: import_component_3d_model, scale_model_z_height, slice, view_3d, material_setup
---

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

The alignment is kept. Tick **Scale model to fit material**, untick it again to go back
to the file's size (1:1), or type a new **Model Size**, and the model is lined up again at
its new size, measured from the material's corner (its 0,0). So a model aligned to the
right side stays against the right side as it grows, and a centred one stays centred
with nothing of it under the modeling plane.

Type a **position** X or Y, or a **Depth Below Top**, and that number is used as you
typed it instead: the model's corner stays there when it is resized. Press an Align in
Material button again to go back to lining it up.

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

## See also

- [Material Setup](help:material_setup), and **Import Component / 3D Model...** in the
  Import tools, whose form has the same setting
- [3D View ▸ Changing a model's height](help:view_3d): the height buttons on a picked
  model, to change the model itself rather than where it sits
