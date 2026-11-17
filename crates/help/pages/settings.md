---
reviewed: 2026-11-17
see-also: web_api, bug_reports, contents
---

# Changing settings

**Edit ▸ Options** holds Cut Craft's settings, page by page down the left: how it looks,
the mouse and shortcuts, toolpath previews, the Web API and more.

A change you make there does **not** take effect straight away. It waits until you press
**Apply**, so you can try a few changes, look them over, and still back out.

![Options with a change not applied yet: the note at the foot and Apply ready to press](pictures/options-not-applied.png)

## The buttons along the foot

- **Apply** uses your changes now and keeps the window open. It is greyed out until you
  change something.
- **Cancel** throws away every change you have not applied and closes the window. The
  settings stay as they were.
- **Close** closes the window. If some changes are not applied yet it asks first, so
  nothing is lost or kept by surprise.

While changes are waiting, the foot of the window says **● Changes not applied yet**. Once
everything is applied it says **No changes waiting**.

**Reset to defaults** on each page puts that page's settings back as Cut Craft first had
them. That is a change too: press **Apply** to use it.

## When you close with changes waiting

Closing the window, with **Close** or its own close box, while changes are waiting asks:

![Apply your changes? with Apply and Close, Discard Changes and Keep Editing](pictures/options-apply-question.png)

1. **Apply and Close** uses the changes and closes.
2. **Discard Changes** throws them away and closes.
3. **Keep Editing** takes you back to the settings. Pressing Esc does the same.

## Snapping Options, Machine Configuration and the Tool Database

The other windows that hold settings work the same way:

- **Snapping Options** (F4) has the same **Close**, **Cancel** and **Apply** along its foot.
  What drawing snaps to changes only once you press **Apply**.

![Snapping Options with a change not applied yet](pictures/snapping-not-applied.png)

- **Machine Configuration**, the **Tool Database** and the **Materials Library** keep their
  **OK**, **Cancel** and **Apply** where they always were. **Apply** is greyed out until you
  change something, the same **● Changes not applied yet** note shows beside it while
  changes wait, and **OK** keeps them and closes.
- Closing any of them with its own close box while changes wait asks **Apply your
  changes?** first, just as Options does.

**Material Setup** and the other forms on the Design and Toolpaths pages are not windows:
their **Apply** button uses what you typed, as before.

## Shortcuts too

A shortcut you set or remove in **Mouse and Shortcuts** also waits for **Apply**. Until
then the old shortcut still runs.

See also: [Web API](help:web_api), [3D View](help:view_3d), [Did You Know?](help:tips).
