---
reviewed: 2026-11-17
see-also: checks_before_cutting, profile_toolpath, pocket_toolpath, collisions
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

If the collet nut, spindle or shank would hit the wood beside a deep cut, the
checks list it as **⚠ Check** and Save always asks before writing the file, whatever
Options say. See [Collet and Holder Collisions](help:collisions).
