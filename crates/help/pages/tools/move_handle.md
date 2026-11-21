---
reviewed: 2026-11-21
see-also: span_to_bezier, node_edit, move_node, smooth_point
---

**Move Handle** bends a curved side by dragging one of its handles. You do it in
[Node Edit](help:node_edit): there is no button for it.

## Steps

1. Select the shape and press **Node Edit** (or N).
2. If the side is still straight or an arc, right-click it and pick
   [Convert Span to Bezier](help:span_to_bezier). Two round handles appear on it.
3. Drag a round handle (not a square: squares are the corners). The side bends towards
   where you let go; its corners stay put.

![Dragging a handle bends the side](pictures/span_to_bezier/step-4.png)

## How it behaves

- Each curved side has two handles. The one nearer a corner shapes the curve as it leaves
  that corner: a longer handle pulls the curve further out.
- At a smooth corner (made with [Smooth Point](help:smooth_point)) the handles either side
  line up. Dragging one turns the other the opposite way, keeping its length, so the corner
  stays smooth. At a sharp corner the other handle stays where it is.
- If a handle sits right on its corner, a drag there moves the corner; drag the handle out
  from somewhere just beside it instead, or use [Node Position](help:move_node).
- Undo (Ctrl+Z) takes back a drag.

## Settings

- **The handle to move**: which shape, where you pressed, and how close you had to be.
  Node Edit fills these in from your drag.
- **Where it goes**: where you let go.
