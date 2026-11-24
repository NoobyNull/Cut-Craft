---
reviewed: 2026-11-23
see-also: save, open, save_toolpaths, setup_material, settings, updates
icon: common/save.png
---

# Saving and opening jobs

A job is one file (`.ccjob`) holding everything: the material, your drawing, the 3D model
and the toolpaths. [Save](help:save) keeps it; [Open](help:open) brings it back. This page
covers the ways Cut Craft keeps your work safe in between.

## Increment and Save: a new copy each time

![Increment and Save makes a new numbered copy each time](pictures/basics/increment_save.png)

**File ▸ Increment and Save** (**Ctrl+Alt+S**) saves a new copy with the next number on
its name, and leaves the file you had as it was:

1. Your job is `sign.ccjob`. Press **Ctrl+Alt+S**: it saves as `sign_1.ccjob`, and that is
   the job now open.
2. Press it again later: `sign_2.ccjob`, then `sign_3.ccjob`.

So you can always go back to an earlier version. A name that already ends in a number
(`sign_4` or `sign-4`) counts on from it. A year or a date on the end, such as
`Smith-2024` or `order_2024-11-15`, is part of the name, so it gets `_1` after it.
A job never saved yet asks for a name first, as **Save As** does. **Ctrl+S** saves over
the open file as usual.

## The recovery copy: work kept if the power goes

![A recovery copy is kept every 30 seconds, so a power cut loses little](pictures/basics/recovery_copy.png)

Cut Craft copies your unsaved work every 30 seconds. If it closes without saving (a crash,
a power cut), it opens that work again the next time it starts. The copy is kept until
you save or close the job.

The work comes back under the job's own name, still marked as not saved, so **Save** puts
it back in its own file. (A job you never saved comes back as **Untitled**.) Installing an
update keeps your unsaved work the same way, without asking: see
[Updates](help:updates).

- **Edit ▸ Options ▸ General Settings ▸ Keep a recovery copy every** sets how often, from
  10 seconds to 10 minutes. Shorter loses less work; longer writes to the disk less often.
- Each open window keeps its own copy: a second Cut Craft window does not take the first
  one's work. This works on network and other shared settings folders too.
- If the settings folder can't be written at all, the status bar warns that no copy is
  being kept: save often.
- If one copy can't be written (a full disk, a network folder that dropped out), the
  status bar says so and Cut Craft tries again at the next interval, even if you have not
  changed anything since.

A recovery copy is a safety net, not a save: press **Save** for a file you can open,
copy and keep.

## When you close with unsaved changes

When you open, start or close a job while the one you have open has unsaved changes,
Cut Craft asks **Save changes to (file name)?**: **Save** saves them, **Don't Save** lets
them go, **Cancel** keeps working. If you press **Don't Save**, then cancel the file
picker (or the file will not open), nothing is lost: your job stays open with its unsaved
changes, still marked with **\*** in the title and still kept for recovery.

## Opening a file from your file manager

Open a file **with Cut Craft** from your file manager (**Open With**, or drag it onto Cut
Craft's icon) and Cut Craft starts with it:

- A job (`.ccjob`) opens.
- A drawing or picture (DXF, SVG, PNG, JPG and the rest) is brought into a new, blank job.
  Change the material's size to fit it with [Setup Material](help:setup_material).
- With **Classic Startup Flow** ticked (**Edit ▸ Options ▸ Job Defaults**), Job Setup asks
  first, and **OK** makes the job and imports the file into it. **Cancel** goes back to
  the start page without it.

## A job that won't open

A job that will not open says why, and stays in **File ▸ Recent Files** so you can try
again. Only a file that is no longer there is taken off the list.

- **Damaged** means the file was cut short or broken, often by a copy or a cloud sync that
  did not finish. Open a backup or an earlier copy (an Increment and Save copy is ideal),
  or wait for the sync to finish and try again.
- **Saved by a newer version** means Cut Craft needs updating first: **Help ▸ Check for
  Updates**. See [Updates](help:updates).
- If a newer version's job does open but has settings this version does not know, a
  warning names them. Saving it here leaves them out, so update first to keep them. Each
  save (Save, Save As, Increment and Save) gives the warning again.

## Names and file types

- A name you type with a dot in it keeps all of it: `plaque v1.2` saves as
  `plaque v1.2.ccjob`. When Cut Craft adds the extension for you and a file of that name is
  already there, it asks before replacing it. The same goes for exports, templates and
  other saved files.
- Every file window starts on the kind of file you asked for, so the list only shows
  files that will work. **File ▸ Open** shows Cut Craft jobs; **Import Component / 3D
  Model** shows 3D models (.stl, .obj, .ccclip); **Import Vectors / Bitmap** shows drawings
  and pictures (.dxf, .svg, .png, .jpg and the rest); **Import Cut List** shows .csv
  spreadsheets; saving G-code shows your post-processor's file type. To pick a file of
  another kind, open the file-type list at the bottom of the window (beside the file name
  on Windows): the import windows also list each format on its own, and **All supported
  files** for anything Cut Craft can import. On Linux, a file with a capital-letter
  extension such as `PART.STL` shows too.
