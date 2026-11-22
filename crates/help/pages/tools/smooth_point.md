---
reviewed: 2026-11-22
see-also: node_edit, insert_point, span_to_bezier, move_handle
---

**Smooth Point** (node editing's right-click menu) turns the two sides meeting at a node into
curves whose round handles lie on one straight line through it. Drag either handle in
[Node Edit](help:node_edit) to reshape the bend: the other turns with it, so the node stays
smooth ([Move Handle](help:move_handle)).

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
