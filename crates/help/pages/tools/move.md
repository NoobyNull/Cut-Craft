---
reviewed: 2026-11-24
see-also: transform, rotate, center_in_material, orientate_3d_model
---

On a rotary job, a full 3D model (imported with [Orientate 3D Model](help:orientate_3d_model)
as a full model) moves with its undercuts: moving it along the axis moves it along the part,
and moving it across the sheet turns it round the axis. It can't be scaled, rotated or
mirrored on the sheet, as the unrolled sheet can't carry its undercuts: change its size or
which way it faces in Orientate 3D Model instead. Rotary toolpaths that cut it are marked out
of date when it moves.

## Steps

![The steps, one after another](pictures/move/tour.gif)

1. Move the selection to an exact place, or by an exact distance. Select it.

   ![Step 1](pictures/move/step-1.png)

2. Under **Edit**, press **Move** (under Transform Objects).

   ![Step 2](pictures/move/step-2.png)

3. Type where it goes (X and Y) and which point of it to place there, or how far to move it, then press **Apply**.

   ![Step 3](pictures/move/step-3.png)
