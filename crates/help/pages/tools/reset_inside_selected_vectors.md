---
reviewed: 2026-11-23
see-also: reset_outside_selected_vectors, create_shape, sculpt
---

## Steps

![The steps, one after another](pictures/reset_inside_selected_vectors/tour.gif)

1. Reset Inside clears the model inside the selected shapes back to flat. Select the closed shapes.

   ![Step 1](pictures/reset_inside_selected_vectors/step-1.png)

2. On the Design page, under **Model**, press **Reset Inside Selected Vectors**.

   ![Step 2](pictures/reset_inside_selected_vectors/step-2.png)

3. Inside the shape, the model is flat again.

   ![Step 3](pictures/reset_inside_selected_vectors/step-3.png)

## Which parts of the model change

- If you have picked components (clicked them in the Components panel or the 3D view), only
  those are reset inside the shapes. The others are left exactly as they are.
- If no component is picked, every shown component is reset inside the shapes.
- Hidden components are never changed.

See also [Reset Outside Selected Vectors](help:reset_outside_selected_vectors).
