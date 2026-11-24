---
reviewed: 2026-11-23
see-also: component_levels, scale_model_z_height, delete_component
---

**Component Properties** changes one 3D component: its name, how it combines with the model
under it, whether it is part of the model, and its height, base, fade and tilt. Right-click the
component in **Components** ▸ **Properties**, or **Model ▸ Selected Component Properties**.

## Steps

![The steps, one after another](pictures/component_properties/tour.gif)

1. Open **Components** at the top of the Design page, right-click the component and choose **Properties**.

   ![Step 1](pictures/component_properties/step-1.png)

2. **Shape height** makes it taller or lower, as a percentage; **Base height** raises it on a flat base.

   ![Step 2](pictures/component_properties/step-2.png)

3. **Combine Mode** says how it meets the model under it: Add, Subtract, Merge, Low or Multiply.

   ![Step 3](pictures/component_properties/step-3.png)

## Settings

- **Name**: what it is called in the Components list.
- **Combine Mode**: **Add** raises it on top of what is under it, **Subtract** sinks it in,
  **Merge** keeps the higher of the two, **Low** the lower, and **Multiply** scales what is
  under it.
- **Visible**: hidden components are left out of the model.
- **Shape Height**: its height as a percentage of its own (50 % is half as tall).
- **Base Height**: raises it on a flat base this thick.
- **Fade** and **Tilt**: fade it down towards one side, or tilt it.
- **Bake**: makes the height, base, fade and tilt permanent and resets them.
- **Appearance**: its colour in the 3D view, its own or its group's.

See [Group Properties](help:group_properties) to set one mode on a whole group.
