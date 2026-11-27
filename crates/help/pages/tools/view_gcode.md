---
reviewed: 2026-11-27
see-also: save_toolpaths, machine_files, checks_before_cutting
---

## Steps

1. Make at least one toolpath, and tick the ones to look at in the toolpath list.
2. Press **View G-code** (in **Save for the Machine** on the Toolpaths bar, or on the
   Output tab of the Toolpaths page in the Classic layout).
3. A window shows the G-code that [Save Toolpaths](help:save_toolpaths) would write for
   them, worked out through your machine's post processor, line by line. Nothing is saved.
   Select lines and copy them if you need them somewhere else.

![The View G-code window: the post-processor, what the checks before cutting found, the file, its lines numbered, and Copy All, Save Toolpaths… and Refresh](pictures/view-gcode.png)

## Good to know

- It reads the same file saving writes, so it is a good way to check the first lines (the
  units, the spindle speed, the first move) before the machine runs it.
- Above the G-code, the [checks before cutting](help:checks_before_cutting) list what they
  found, the same checks saving runs. Nothing is stopped here.
- **Copy All** copies the G-code, **Save Toolpaths…** goes on to save it, and **Refresh**
  makes it again after you change a toolpath. See
  [Looking at the G-code](help:toolpaths) for more.
- Greyed out until the job has a toolpath.
