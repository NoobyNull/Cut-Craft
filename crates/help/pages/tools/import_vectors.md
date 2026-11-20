---
reviewed: 2026-11-20
see-also: import_vectors_bitmap, vector_validator, join_vectors, layer_properties, show_hide
---

## Steps

![The steps, one after another](pictures/import_vectors/tour.gif)

1. Bring in a drawing made in another program: DXF or SVG.

   ![Step 1](pictures/import_vectors/step-1.png)

2. Under **Import**, press **Import Vectors** and pick the file. A DXF asks which units it was drawn in.

   ![Step 2](pictures/import_vectors/step-2.png)

3. Each layer of a DXF comes in as a layer of your job, with its name and colour. Open the **Layers** tab to see them, show or hide them, or change their colour.

   ![Step 3](pictures/import_vectors/step-3.png)

## Layers

A DXF or SVG often keeps its drawing on layers: the outline of each part on one, the holes
of each size on another, notes on a third. Every one of them comes in as a layer of your
job, with the same name and colour, so the **Layers** tab looks like the layer list in the
program that made the file:

- Each vector goes on the layer it was on in the file, and the layers are listed in the
  same order as in that program.
- Every layer comes in, even one with nothing on it, so the list matches the program that
  made the file.
- A layer the file had turned off or frozen comes in hidden: its vectors are there, but not
  drawn and not selected. Press its eye in the **Layers** tab to show it.
- If your job already has a layer of the same name (capitals or not), the vectors go on
  that one instead of a second copy, and it keeps its own colour.
- The small square before each layer's name is the colour its vectors are drawn in. Click
  it to change it in [Layer Properties](help:layer_properties). The colour only changes how
  vectors look: it never changes how they are cut.

Layers drawn in white or black in the file use the usual vector colour, so they show on a
light or dark screen alike.

Where the layers come from:

- **DXF**: the file's own layer list, with each layer's colour.
- **SVG**: the layers made in most drawing programs. A layer's colour is the colour most of its lines are drawn
  in. Shapes not on any layer go on the current layer.
- **Job files** (`.crv3d`): each vector goes on its layer by name. Their colours are not
  read yet, so they use the usual vector colour.
