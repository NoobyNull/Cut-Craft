---
reviewed: 2026-11-23
see-also: move_handle, node_edit, span_to_line, span_to_arc, smooth_point
---

A span is one side of a shape: the piece of line between two nodes (the corners). **Convert
Span to Bezier** makes that one side a curve you can bend with two handles. The corners stay
where they are.

## Steps

![The steps, one after another](pictures/span_to_bezier/tour.gif)

1. Convert Span to Bezier turns one side of a shape (a span) into a curve you can bend. Select the shape.

   ![Step 1](pictures/span_to_bezier/step-1.png)

2. On the Design page, under **Edit**, press **Node Edit** (or press N). Small squares appear on the shape's corners: its nodes.

   ![Step 2](pictures/span_to_bezier/step-2.png)

3. Right-click the side you want to curve and pick **Convert Span to Bezier**. Two round handles appear on it, each joined by a line to the corner it pulls from.

   ![Step 3](pictures/span_to_bezier/step-3.png)

4. Drag a round handle. The side bends towards it while the corners stay put. Drag the other handle to shape the other end of the curve.

   ![Step 4](pictures/span_to_bezier/step-4.png)

## What you see

- **Squares** are nodes. Drag one to move a corner ([Move Node](help:move_node)).
- **Round dots on a thin line** are handles. Drag one to bend the curve
  ([Move Handle](help:move_handle)). A handle only shows while its shape is selected in Node
  Edit.
- A straight side becomes a curve that still looks straight: its handles sit a third of the
  way along it, on the line. It only bends once you drag a handle.
- An arc becomes one or more curves that follow it so closely you can't see the difference,
  each with its own two handles.

## Settings

There are none. Right-click on the side itself: the side nearest the pointer is the one
converted. To make it straight again, use [Convert Span to Line](help:span_to_line); for a
round bend with no handles, use [Convert Span to Arc](help:span_to_arc).

## Tips

- To round a corner smoothly, right-click the corner node and pick
  [Smooth Point](help:smooth_point): both sides become curves whose handles line up. Dragging
  one of those handles turns the other with it, so the corner stays smooth.
- Undo (Ctrl+Z) takes back a convert or a handle drag.
