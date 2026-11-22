---
reviewed: 2026-11-22
see-also: contents, settings
---

# Screen readers

Cut Craft tells screen readers what is on the screen: the name of each button, box and
choice, and what it is set to. It works with the screen reader your computer already has:

| Computer | Screen reader |
| --- | --- |
| Windows | Narrator (built in, **Ctrl+Windows+Enter**), NVDA or JAWS |
| Linux | Orca (on most desktops, **Super+Alt+S**) |
| Mac | VoiceOver (**⌘+F5**) |

Start the screen reader before or after Cut Craft; either way it picks Cut Craft up.

## Moving round with the keyboard

- **Tab** goes to the next button or box, **Shift+Tab** to the one before. A thick outline
  shows where you are.
- **Enter** or **Space** presses the button you are on.
- In a number box, type a number (or a sum such as `12/2`), or press the **up and down
  arrow keys** to step it. The small up and down buttons beside the box do the same with
  the mouse, so Tab skips them.
- **Esc** closes the open form or small window.
- **Ctrl+Plus** and **Ctrl+Minus** make everything bigger or smaller.
  [Edit ▸ Options ▸ Appearance ▸ Text size](help:settings) sets the size it starts at.

![Text size, at the foot of Edit ▸ Options ▸ Appearance](pictures/options-appearance.png)

## What you hear in a form

Each number box is read with its label and its unit, so a toolpath form's depth box is
heard as **"Cut Depth, mm"** (or **"Cut Depth, in"** in an inch job), followed by its value.
Its arrows are **"Increase Cut Depth"** and **"Decrease Cut Depth"**.

## What does not work yet

Drawing and picking shapes in the 2D and 3D views still need the mouse. The drawing tools
that take numbers (Draw Rectangle, Draw Circle, Draw Polyline and others) can be used by
typing their position and size instead. Some pictures, check boxes and side tabs are not
named yet, and messages in the status bar are not read out on their own.
