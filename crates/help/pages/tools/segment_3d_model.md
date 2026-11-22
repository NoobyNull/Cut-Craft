---
reviewed: 2026-11-22
see-also: import_component_3d_model, slice, orientate_3d_model
---

**Segment 3D Model** cuts a model into pieces with flat planes, so a model with undercuts can be
machined in parts on a 3-axis machine and glued together. Choose **File ▸ Import ▸ Segment 3D
Model...**. The pieces are laid side by side as components, their tops at the material's surface.

## Steps

![The steps, one after another](pictures/segment_3d_model/tour.gif)

1. Choose **File ▸ Import ▸ Segment 3D Model...** and pick the model's STL file.

   ![Step 1](pictures/segment_3d_model/step-1.png)

2. Pick the axis the planes are square to (**Segmenting plane square to**), and the **Number of cuts**.

   ![Step 2](pictures/segment_3d_model/step-2.png)

3. Tick **Turn the second segment over** so its far side faces up to be machined.

   ![Step 3](pictures/segment_3d_model/step-3.png)

4. Press **Apply**. The segments are laid side by side as components, their tops at the material's surface.

   ![Step 4](pictures/segment_3d_model/step-4.png)

## Settings

- **File** and **Units**: the STL file and the units it was drawn in.
- **Segmenting plane square to**: the planes cut across the model's length (X), width (Y) or height (Z).
- **Number of cuts** and **Position**: one cut where you say, or several in equal slices.
- **Turn the second segment over**: turns the piece beyond the plane over, so their far sides face up.
- **Gap**: space between the laid-out pieces.
- **Cut segments again** and **Segment orientations**: cut a piece again, or turn up another
  face of it.

A piece that still has undercuts says so.
