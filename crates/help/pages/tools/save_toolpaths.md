---
reviewed: 2026-11-21
see-also: checks_before_cutting, profile_toolpath, pocket_toolpath, collisions, rotary_machining_toolpath
---

## Steps

![The steps, one after another](pictures/save_toolpaths/tour.gif)

1. When the toolpaths look right in the preview, press **Save Toolpaths** (Save for the Machine).

   ![Step 1](pictures/save_toolpaths/step-1.png)

2. Pick your machine's **post processor** (the kind of file it reads), then press **Save** and choose where the file goes, such as the card or stick you take to the machine.

   ![Step 2](pictures/save_toolpaths/step-2.png)

3. Under **Checks before cutting**, every check is listed with how it came out. Fix anything
   marked ✖ Must fix, then read each ⚠ and tick it. **Save Toolpaths...** and **Send to
   Machine** wait until every ⚠ is ticked.

   ![The checks done: what each found](pictures/checks/checks-found.png)

   See [Checks before cutting](help:checks_before_cutting) for what each check looks for.

Always check the toolpaths in the preview first: what you save is exactly what the machine cuts.

If the collet nut, spindle or shank would hit the wood (or a clamp) beside a deep cut, the
checks list it as **⚠ Check** and Save always asks before writing the file, whatever
Options say. See [Collet and Holder Collisions](help:collisions).

When Save asks, it lists every problem the checks found. **Save Anyway** goes on despite
those problems only. If the checks find one it did not list (say you changed a toolpath
in between), Save asks again. A new or opened job always asks afresh.

The checks run in the background, so the window keeps working while they do. If a
toolpath is still calculating when you press Save, the checks wait until it is done: the
status bar says so, and you press **Save** again once it finishes.

## Rotary toolpaths

Saving makes each toolpath's file a little smaller by leaving out points that lie on a
straight line between their neighbours. A [4-Axis Rotary Machining](help:rotary_machining_toolpath)
toolpath that holds the bit to one side of the turning axis (**Off centre**, **Indexed** and
**Sides, then details**) keeps a place across the axis for every point, so it is saved with
every point it has. Its file is bigger, and it is exactly what the turning preview plays.

Files saved with **Indexed** or **Sides, then details** in versions 0.7.0 to 0.7.2 lost that
match and can cut into the model: save them again with this version before cutting.
