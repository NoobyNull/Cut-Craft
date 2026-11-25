---
reviewed: 2026-11-23
see-also: import_toolpaths_file, load_template, save_toolpaths
---

**Import Toolpaths (G-code)** reads a G-code file back as toolpaths, one for each tool it uses, so you can
preview it, check it and save it again through another post-processor. Choose **File ▸ Import ▸
Import Toolpaths (G-code)...**.

## Steps

![The steps, one after another](pictures/import_toolpaths/tour.gif)

1. Choose **File ▸ Import ▸ Import Toolpaths (G-code)...** and pick a G-code file your machine runs.

   ![Step 1](pictures/import_toolpaths/step-1.png)

2. Pick the **Tool** the file cuts with: its shape and size draw the preview.

   ![Step 2](pictures/import_toolpaths/step-2.png)

3. Press **Apply**. The file is a toolpath in the list now, to preview, check and save again.

   ![Step 3](pictures/import_toolpaths/step-3.png)

## Settings

- **File**: the G-code file (.nc, .tap, .gcode, .ngc, .cnc or .txt).
- **Tool**: the bit the file cuts with. Its shape and size draw the preview. The file's own
  feed rate, plunge rate, spindle speed and tool number are used where it gives them.
- **Part**: with several tools in the file, which tool's part to read (0: every part, a
  toolpath each).
- **Name**: the toolpath's name.

## What it reads

Straight moves and arcs (G0 to G3, in any of the three planes), millimetres or inches, absolute or
relative moves, F, S and tool changes. Drilling cycles (G81 to G89) are refused with a message.
Rotary axis words are left out with a note, so a rotary file is read flat.
