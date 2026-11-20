---
reviewed: 2026-11-20
see-also: reset_inside_selected_vectors, create_shape, sculpt
---

## Steps

![The steps, one after another](pictures/reset_outside_selected_vectors/tour.gif)

1. Reset Outside clears the model outside the selected shapes back to flat. Select the closed shapes.

   ![Step 1](pictures/reset_outside_selected_vectors/step-1.png)

2. On the Design page, under **Model**, press **Reset Outside Selected Vectors**.

   ![Step 2](pictures/reset_outside_selected_vectors/step-2.png)

3. Outside the shape, the model is flat again.

   ![Step 3](pictures/reset_outside_selected_vectors/step-3.png)

## Which parts of the model change

- If you have picked components (clicked them in the Components panel or the 3D view), only
  those are reset outside the shapes. The others are left exactly as they are.
- If no component is picked, every shown component is reset outside the shapes.
- Hidden components are never changed.

See also [Reset Inside Selected Vectors](help:reset_inside_selected_vectors).
