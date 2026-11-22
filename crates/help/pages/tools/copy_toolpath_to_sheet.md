---
reviewed: 2026-11-22
see-also: add_sheet, duplicate_toolpath, toolpath_tiling
---

**Copy to Sheet** puts a copy of a toolpath on another sheet, with copies of the vectors it was
made from, so you don't set it up again. Right-click the toolpath in the Toolpath List.

## Steps

![The steps, one after another](pictures/copy_toolpath_to_sheet/tour.gif)

1. Right-click a toolpath in the Toolpath List and choose **Copy to Sheet**, then the sheet it goes on.

   ![Step 1](pictures/copy_toolpath_to_sheet/step-1.png)

2. The toolpath and copies of its vectors are now on that sheet too, ready to recalculate there.

   ![Step 2](pictures/copy_toolpath_to_sheet/step-2.png)

## Good to know

- The copy is out of date on its new sheet until you recalculate it there.
- It is greyed out while the job has only one sheet: see [Add New Sheet](help:add_sheet).
