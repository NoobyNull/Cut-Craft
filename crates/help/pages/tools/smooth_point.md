---
reviewed: 2026-11-22
see-also: node_edit, insert_point, span_to_bezier
---

**Smooth Point** makes the node you right-click smooth: its two spans become curves whose handles
lie on one line there, so the outline flows through it with no corner. It is on node editing's right-click menu.

## Steps

![The steps, one after another](pictures/smooth_point/tour.gif)

1. Select the shape and press **Node Edit**, then right-click a sharp node.

   ![Step 1](pictures/smooth_point/step-1.png)

2. Choose **Smooth Point**. The corner rounds into a smooth curve; drag its handles to shape it.

   ![Step 2](pictures/smooth_point/step-2.png)

## Good to know

- Drag the handles to shape the curve either side.
- To make a sharp corner again, convert the spans to lines with
  [Convert Span to Line](help:span_to_line).
