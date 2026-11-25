---
reviewed: 2026-11-25
see-also: reorder_toolpath, group_toolpaths, toolpath_summary
---

The Toolpath List is the order the toolpaths are cut in, top first, and the order they are
saved in. **Move Up**, **Move Down** and dragging a row by its grip (≡) change that order. A
group moves with everything inside it, its own groups too, and always cuts as one block.

## Steps

![The steps, one after another](pictures/arrange_toolpaths/tour.gif)

1. The Toolpath List cuts from the top down. Click the toolpath you want to cut at another time.

   ![Step 1](pictures/arrange_toolpaths/step-1.png)

2. Press **Move Up** or **Move Down** above the list, or drag the toolpath by its grip (≡) at the left of its row: a blue line shows where it will land.

   ![Step 2](pictures/arrange_toolpaths/step-2.png)

3. It now cuts in its new place: the file you save runs the toolpaths in this order.

   ![Step 3](pictures/arrange_toolpaths/step-3.png)

## Good to know

- Cut the small inside details before the outline that frees the part, so the part is still
  held while they are cut.
- Every row has a grip (≡) at its left. Drag by the grip or by the name.
- Where it lands, as the blue line shows:
  - onto a toolpath: just before it, in the same group;
  - onto a group's name: at the end of that group (a dragged group becomes a group inside it);
  - onto the top edge of a group's row: just before that group, beside it;
  - into the space under the list: last, in no group.
- A group can't be dropped inside itself or inside one of its own groups.
- Drag a toolpath onto a group's name to put it in that group: see
  [Group Toolpaths](help:group_toolpaths).
- One **Undo** takes back one move.
