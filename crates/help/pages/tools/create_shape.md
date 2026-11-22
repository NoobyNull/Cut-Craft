---
reviewed: 2026-11-22
see-also: create_shape_round, create_shape_angled, create_shape_flat, create_shape_smooth
---

**Create Shape** raises a 3D component over the selected closed vectors. Its sides rise from the
edges as a round dome, a straight slope, an S curve or a flat top, or follow an open vector you
draw as the profile. The four **Create Shape** buttons on the Model page each start it with one
profile picked.

## Steps

![The steps, one after another](pictures/create_shape/tour.gif)

1. Select one or more closed vectors: the shape rises from their edges.

   ![Step 1](pictures/create_shape/step-1.png)

2. On the Model page, press one of the **Create Shape** buttons, then pick the **Profile**: Round, Angular, Smooth, Flat or Custom.

   ![Step 2](pictures/create_shape/step-2.png)

3. Set the **Angle** the sides rise at, and a **Height** where the top levels off.

   ![Step 3](pictures/create_shape/step-3.png)

4. Press **Apply**. The shape is added to the model as a component.

   ![Step 4](pictures/create_shape/step-4.png)

## Settings

- **Profile**: **Round** rises at the angle and rounds over the top; **Angular** rises at the
  angle all the way; **Smooth** rises in an S; **Flat** is one height all over; **Custom** follows
  an open vector selected with the closed ones.
- **Angle**: how steeply round and angular sides rise.
- **Height**: where the top levels off (0: no limit).
- **Base**: added everywhere inside the shape.
- **Scale** and **Total height**: size the profile, or the whole shape with its base, to a height.
- **Preserve internal corners**: inside corners rise as sharp creases.
- **Blend to inner vectors**: vectors inside the outline are the top, not holes.
- **Combine**: how it meets the model under it.

A ring between two vectors rises from both edges. Press **E** on the shape to change it later.
