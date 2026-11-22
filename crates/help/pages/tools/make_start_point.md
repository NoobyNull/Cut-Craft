---
reviewed: 2026-11-22
see-also: node_edit, reverse_direction, profile_toolpath
---

**Make Start Point** makes a closed vector start at the node you right-click. Toolpaths made from
it start, and plunge, at that node. It is on node editing's right-click menu.

## Steps

![The steps, one after another](pictures/make_start_point/tour.gif)

1. Select the closed shape and press **Node Edit**, then right-click the node the cut should start from.

   ![Step 1](pictures/make_start_point/step-1.png)

2. Choose **Make Start Point**. Toolpaths made from it now start, and plunge, at that node.

   ![Step 2](pictures/make_start_point/step-2.png)

## Good to know

- It counts when the profile's **Start At** is **Keep Current Start Points**: the default,
  Optimize Start Points, picks its own.
- Put the start where a plunge mark won't show, such as an inside corner.
- Choose which way it runs with [Reverse Direction](help:reverse_direction).
