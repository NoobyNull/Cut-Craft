---
reviewed: 2026-11-25
see-also: find_tool, view_2d, settings, getting_started, screen_readers
---

# Keyboard shortcuts

Most tools have a key. **Help ▸ Keyboard Shortcuts** lists the key for every tool and
menu item, so you can work without hunting through the menus.

![Keyboard shortcuts: the key for every tool](pictures/basics/keyboard.png)

On a Mac, read **⌘** (Command) where this help says **Ctrl**, and **⌥** (Option) for
**Alt**. The menus show each shortcut the Mac way, ⌘S for Save, and the delete key
deletes the selection.

## The keys worth learning first

- **Ctrl+Z** undoes the last change, as many times as you need. **Ctrl+Y** puts it back
  (on a Mac **⇧⌘Z** does too).
- **Esc** closes the open tool's form without making anything.
- **E** edits the selected shape in the tool that made it, with the settings you used.
- **Ctrl+K** [Find Tool](help:find_tool): type part of a tool's name and press Enter.
- **F1** the open tool's help; with no tool open, Help Contents.
- **F2** the 2D view, **F3** the 3D view, **F4** Snapping Options.
- **Ctrl+S** saves; **Ctrl+Alt+S** [Increment and Save](help:saving_jobs).

![Esc closes the open form without making anything](pictures/basics/esc_close.png)

## More about some keys

- **Esc** closes a small window in front (Find Tool, a library file's Properties) on its
  own, else the open form. On Import 3D Model's form it is Cancel: the model is not
  imported. Pressed while dragging in the 2D or 3D view, it cancels the drag and changes
  nothing (see [Moving around the 2D view](help:view_2d)).
- **Esc** in Move, Rotate, Set Size or another Transform Objects tool opened over a form
  you were filling in goes back to that form, as you left it. What you moved stays moved,
  and toolpaths it changes are out of date until recalculated.
- **E** edits the selected shape (or Create Shape component) in the tool that made it.
  Once a Create Shape component has been sculpted, smoothed, embossed or cut back with
  Replace Below or Reset, it is kept as you left it: E no longer remakes it from its
  settings, so your hand work is never thrown away.

  ![Select a shape and press E to change it in the tool that made it](pictures/basics/edit_again.png)
- **F11** the designing layout, **F12** the machining layout.
- The arrow keys nudge the selection one square of the snap grid. Hold **Shift** for half
  a square, **Ctrl+Shift** for a tenth, or **Ctrl** for the **Fixed nudge distance** set in
  [Snapping Options](help:snapping).
- **Ctrl+Q** starts Quick Engrave, except on a Mac, where **⌘Q** quits: set it another key
  if you use it.
- Using PowerWhittle by keyboard alone, or with a screen reader (Narrator, NVDA, Orca,
  VoiceOver)? See [Screen readers](help:screen_readers).

## Changing a shortcut

1. Open **Edit ▸ Options ▸ Mouse and Shortcuts**.
2. Type part of an action's name (or a key, such as Ctrl+Z) in **Find** to find its row.
3. Press **Add** beside it, then press the key you want, with Ctrl, Alt or Shift if you
   like. A mouse button works too: the wheel click, or a thumb button (Back and Forward).
4. If that key already runs something else, PowerWhittle says so: **Move it here** takes it
   from the other action, and **Pick another** waits for a different key.
5. Press **Apply** at the foot of Options. Until then the old shortcut still runs.

Press **×** on a shortcut to take it away, and **Reset** to go back to PowerWhittle's own.
**Pan (hold and drag)**, at the top of the list, is the mouse button that moves the view
when held and dragged: the wheel to start with. Ctrl+C, Ctrl+X and Ctrl+V only ever copy,
cut and paste. See [Changing settings](help:settings) for Apply, Cancel and Close.
