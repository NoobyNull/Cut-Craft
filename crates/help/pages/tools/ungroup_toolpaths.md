---
reviewed: 2026-11-23
see-also: group_toolpaths, rename_toolpath_group, ungroup_merged_toolpath
---

**Ungroup** under the Toolpath List takes the picked toolpaths out of their group, or takes the
picked group apart, one level at a time. The toolpaths stay. Right-click a group ▸ **Delete
Group and Its Toolpaths** deletes the toolpaths too, and any groups inside it.

## Steps

![The steps, one after another](pictures/ungroup_toolpaths/tour.gif)

1. Click a group's name in the Toolpath List.

   ![Step 1](pictures/ungroup_toolpaths/step-1.png)

2. Press **Ungroup** above the list (or right-click the group ▸ **Ungroup**). Its toolpaths stay, moving up one level: out of the group, or into the group around it.

   ![Step 2](pictures/ungroup_toolpaths/step-2.png)

3. The group is gone and its toolpaths are listed on their own again.

   ![Step 3](pictures/ungroup_toolpaths/step-3.png)

## Good to know

- One level: a group inside another hands its toolpaths and groups to the group around it.
  Ungroup again to take that one apart too.
- Nothing changes in the cut order: the toolpaths still cut where they did.
- See [Group Toolpaths](help:group_toolpaths).
