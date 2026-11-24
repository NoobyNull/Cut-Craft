---
reviewed: 2026-11-24
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

## File names

When the toolpaths go into more than one file, each file's name is the name you typed, then
its number in cutting order, then what it holds:

| Saved as | Example |
|---|---|
| A separate file for each bit | `sign_1_6mm End Mill.nc`, `sign_2_90 V-Bit.nc` |
| Multiple Files (one per toolpath) | `sign_1_Pocket_6mm End Mill.nc`, `sign_2_Text_90 V-Bit.nc` |
| Two-sided, with the side added | `sign_1_Pocket_6mm End Mill_Top.nc` |
| Tiled | `sign_1_Pocket_6mm End Mill_tile2.nc` |

- Run the files in number order. With ten or more, the numbers start with a 0 (`01`, `02` …
  `12`) so your file list and the controller show them in that order.
- Characters Windows will not save (`< > : " / \ | ? *`) become `_`, and each part is kept to
  64 characters. Spaces are kept as you typed them.
- One file only is saved under the name you typed, with nothing added.

Two settings in **Edit ▸ Options ▸ Toolpaths** change this:

- **Underscores for spaces in G-code file names**: turn it on if your controller or its USB
  reader cannot open names with spaces (`My_Sign_1_End_Mill.nc`). Off by default.
- **Bit name in multiple file names**: on by default, so a Multiple Files name says which bit
  to fit. Turn it off for the toolpath's name alone (`sign_1_Pocket.nc`).

Saving again under the same name finds the files the earlier save made (numbered, sided and
tiled ones too) and asks whether to **Replace** them or **Archive** them in a dated folder
under `Archive`. See [Confirmation Choices](help:confirmation_choices) to answer once for good.
