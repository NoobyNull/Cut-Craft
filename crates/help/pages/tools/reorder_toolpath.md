---
reviewed: 2026-11-25
see-also: rotary_roughing_toolpath, arrange_toolpaths, group_toolpaths, toolpath_summary
---

**Move Up** and **Move Down** above the Toolpath List move the picked toolpath one place. The list
is the order the toolpaths are cut and saved in, top first.

## Steps

![The steps, one after another](pictures/reorder_toolpath/tour.gif)

1. Click the toolpath to move in the Toolpath List: they cut from the top down.

   ![Step 1](pictures/reorder_toolpath/step-1.png)

2. Press **Move Up** to cut it sooner, or **Move Down** to cut it later.

   ![Step 2](pictures/reorder_toolpath/step-2.png)

3. It has moved one place, and the file you save cuts it there.

   ![Step 3](pictures/reorder_toolpath/step-3.png)

## Good to know

- Inside a group, a toolpath moves only inside that group; it steps past a whole group inside
  it in one press.
- A toolpath that cuts only what the toolpaths above it leave ([Rotary Roughing](help:rotary_roughing_toolpath))
  goes out of date (⚠) when a move changes what is above it. PowerWhittle says so, with **Put it
  back** to undo the move, or **Keep the new order** to recalculate it later.
- Dragging a toolpath in the list by its grip (≡) moves it too: see
  [Arrange Toolpaths](help:arrange_toolpaths).
