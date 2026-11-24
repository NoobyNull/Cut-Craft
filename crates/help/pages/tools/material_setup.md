---
reviewed: 2026-11-23
see-also: job_setup, save_toolpaths, view_3d, orientate_3d_model
---

## Steps

![The steps, one after another](pictures/material_setup/tour.gif)

1. Before the first toolpath, tell Cut Craft about the board on the machine. On the Toolpaths page, press **Material Setup** (under Job and Tools). It also opens by itself the first time you make a toolpath.

   ![Step 1](pictures/material_setup/step-1.png)

2. Check the **Thickness**: measure the board, because every depth is worked out from it.

   ![Step 2](pictures/material_setup/step-2.png)

3. Choose where the bit's **Z zero** is: on the top of the board, or on the machine bed under it. Touch the bit off at the same place on the machine.

   ![Step 3](pictures/material_setup/step-3.png)

4. Choose the **XY datum**: the corner (or the middle) of the board that is X0, Y0 on the machine.

   ![Step 4](pictures/material_setup/step-4.png)

5. Check how high the bit lifts between cuts (**Rapid Z gaps**), then press **Apply**.

   ![Step 5](pictures/material_setup/step-5.png)

Job Setup sets the same thickness and zero when you start a job: see [Job Setup](help:job_setup).

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
