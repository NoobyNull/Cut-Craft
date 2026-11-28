---
reviewed: 2026-11-23
see-also: addons, relief_from_photo
---

**Open My Gadgets Folder** opens the folder where your own gadgets go, in your computer's
file manager. PowerWhittle makes the folder the first time if it is not there yet.

1. Pick **Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder**.
2. Copy the gadget's `.lua` file into the folder that opens.
3. Restart PowerWhittle. The gadget is listed in **Gadgets ▸ My Gadgets** and in the
   **Gadgets** section of the tool buttons, under its file's name.
4. Pick it. PowerWhittle asks **Do you want to run this unsigned code?** Choose **Run and
   Remember** (don't ask again until the file changes), **Run This Time** (ask again after
   PowerWhittle closes) or **Don't Run**.
5. Once you say yes, it takes its own name and opens its settings.

A gadget you put here yourself is not official (nobody has signed it), so My Gadgets shows
**⚠ (not official)** after its name. None of its code runs until you say yes, not even to
read its name. If you edit the file, you are asked again. A file that is not a working
gadget says why in the status bar when you say yes to it. See
[Running unsigned code](help:addons) for the whole story.

The folder is called `gadgets`, inside PowerWhittle's settings folder:

- **Linux**: `~/.config/powerwhittle/gadgets`
- **Windows**: `%APPDATA%\powerwhittle\gadgets`
- **macOS**: `~/Library/Application Support/powerwhittle/gadgets`

To get gadgets other people have published on GitHub, use [Add-ons](help:addons) instead:
it installs them for you.

## Steps

![The steps, one after another](pictures/open_gadgets_folder/tour.gif)

1. Gadgets you install show on the Design page under **Gadgets**. **Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder** opens the folder they go in.

   ![Step 1](pictures/open_gadgets_folder/step-1.png)
