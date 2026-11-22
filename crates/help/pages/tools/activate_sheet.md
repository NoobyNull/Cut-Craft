---
reviewed: 2026-11-22
see-also: add_sheet, sheet_properties, send_to_sheet, delete_sheet
---

**Activate** makes another sheet of material the one you are working on. The view shows only
the active sheet, with its own vectors, components and toolpaths, and [Job Setup](help:job_setup)
changes its size.

## Steps

![The steps, one after another](pictures/activate_sheet/tour.gif)

1. A job can hold several sheets of material. Open **Sheets** at the top of the Design page to see them listed.

   ![Step 1](pictures/activate_sheet/step-1.png)

2. Click a sheet's name, or right-click it and choose **Activate**. The view now shows that sheet and what is on it.

   ![Step 2](pictures/activate_sheet/step-2.png)

## Good to know

- Each sheet keeps its own size and thickness, so one job can hold a box's sides cut from 12 mm
  board and its back from 6 mm.
- Saving toolpaths saves the active sheet's. Activate each sheet in turn to save its file.
- Add sheets with [Add New Sheet](help:add_sheet), and move vectors between them with
  [Copy / Move to Sheet](help:send_to_sheet).
