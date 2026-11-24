---
reviewed: 2026-11-23
see-also: level_properties, component_levels, vector_boundary
---

**Clipping** keeps a level of the 3D model only inside the selected closed vectors, without
changing its components. Use it to trim a model to a plaque's outline. Right-click the level in
**Components** ▸ **Clipping**.

## Steps

![The steps, one after another](pictures/level_clipping/tour.gif)

1. Draw a closed vector round the part of the model to keep, and select it.

   ![Step 1](pictures/level_clipping/step-1.png)

2. Open **Components**, right-click the level and choose **Clipping ▸ Apply**.

   ![Step 2](pictures/level_clipping/step-2.png)

3. Only the part inside the vector shows; the components are not changed. **Clipping ▸ Remove** shows it all again.

   ![Step 3](pictures/level_clipping/step-3.png)

## The Clipping menu

- **Apply**: clip the level to the selected closed vectors.
- **Update**: clip again, to the selected vectors or else the ones used last.
- **Remove**: show the whole level again.
