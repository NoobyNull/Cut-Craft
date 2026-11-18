---
reviewed: 2026-11-18
see-also: addons, relief_from_photo
---

**Open My Gadgets Folder** opens the folder where your own gadgets go, in your computer's
file manager. Cut Craft makes the folder the first time if it is not there yet.

1. Pick **Gadgets ▸ My Gadgets ▸ Open My Gadgets Folder**.
2. Copy the gadget's `.lua` file into the folder that opens.
3. Restart Cut Craft. The gadget is listed in **Gadgets ▸ My Gadgets** and in the
   **Gadgets** section of the tool buttons.

A gadget you put here yourself is not signed, so Cut Craft asks before it first runs each
session. A file that is not a working gadget is skipped, and the status bar says why.

The folder is called `gadgets`, inside Cut Craft's settings folder:

- **Linux**: `~/.config/cut-craft/gadgets`
- **Windows**: `%APPDATA%\cut-craft\gadgets`
- **macOS**: `~/Library/Application Support/cut-craft/gadgets`

To get gadgets other people have published on GitHub, use [Add-ons](help:addons) instead:
it installs them for you.
