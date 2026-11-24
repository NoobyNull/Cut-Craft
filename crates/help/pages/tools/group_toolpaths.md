---
reviewed: 2026-11-23
see-also: ungroup_toolpaths, rename_toolpath_group, arrange_toolpaths, show_toolpath_group
---

Toolpath groups are folders in the Toolpath List, and a group can hold groups of its own. A
group's toolpaths are cut one after another and move up and down the list together, with
everything inside it. Pick toolpaths and press **Group**, or right-click one ▸ **Group** for
more ways.

## Steps

![The steps, one after another](pictures/group_toolpaths/tour.gif)

1. In the Toolpath List, click one toolpath and Ctrl-click the others to put together.

   ![Step 1](pictures/group_toolpaths/step-1.png)

2. Press **Group** above the list. The group is cut one toolpath after another, and moves as one.

   ![Step 2](pictures/group_toolpaths/step-2.png)

3. The group is listed with its toolpaths under it. Drag another toolpath onto its name to add it.

   ![Step 3](pictures/group_toolpaths/step-3.png)

4. Groups can hold groups. Pick toolpaths already in a group and press **Group**, or drag a group by its grip onto another group's name. The lines show which group holds what.

   ![Step 4](pictures/group_toolpaths/step-4.png)

5. Click a group's arrow to close it to its name and count, and again to open it. A group moves and cuts with everything inside it.

   ![Step 5](pictures/group_toolpaths/step-5.png)

## Reading the list

- **≡** at the left of every row is its grip: drag it to change the cut order (see
  [Arrange Toolpaths](help:arrange_toolpaths)).
- **▼** / **▶** opens or closes a group. A closed group shows how many toolpaths it holds.
  Closing a group only changes what you see, so it is not a step to undo.
- The thin lines join each toolpath or group to the group holding it.
- The tick beside a group's name shows or hides everything in it: see
  [Show or Hide Toolpath Group](help:show_toolpath_group).

## The Group menu

Right-click a toolpath ▸ **Group**:

- **Group This**: this toolpath into a new group, inside the group it is in.
- **Group Visible**: every ticked toolpath into a new group.
- **Create Empty Group**: a new group; toolpaths made while it is picked join it.
- **Move to Group**: this toolpath into another group. A group inside another is listed with
  its path, such as Outside ▸ Tabs.
- **Remove from Group**: this toolpath out of its group, up one level.

## Buttons with a group picked

- **Group** puts the picked group inside a new group.
- **New Group** makes an empty group inside the picked group.
- **Move Up** / **Move Down** move the group, with everything inside it, past the next toolpath
  or group.

Rename a group with [Rename Toolpath Group](help:rename_toolpath_group) and take one apart with
[Ungroup Toolpaths](help:ungroup_toolpaths).
