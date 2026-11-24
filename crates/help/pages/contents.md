---
reviewed: 2026-11-23
see-also:
---

# Cut Craft Help

Cut Craft draws 2D vectors and 3D models on a sheet of material, then makes the toolpaths
a CNC machine cuts them with.

![The Help window: search along the top, the pages listed, related pages down the side](pictures/help-contents.png)

- [Design](help:design): the page down the left. Draw and edit vectors, then model in 3D.
- [Toolpaths](help:toolpaths): the page down the right. Turn vectors into toolpaths,
  preview them and save them for the machine.
- [3D View](help:view_3d): turning, zooming and fitting the 3D view, changing a model's
  height with its height buttons, and Auto-rotate to see the job from every side.
- [Groups](help:groups): step inside a group to change one thing in it; text in a
  group stays editable.
- [Checks before cutting](help:checks_before_cutting): what Cut Craft checks before
  it saves your toolpaths, what each check protects you from, and how to fix what it finds.
- [Changing settings](help:settings): Edit ▸ Options, and how Apply, Cancel and Close
  keep or throw away your changes.
- [Did You Know?](help:tips): short tips by subject, from drawing to cutting safely, one
  shown each time Cut Craft starts. Each links to the page that says more.
- [Updates](help:updates): install a new version in one click; Cut Craft reopens on
  your job.
- [Reporting a bug](help:bug_reports): right-click ▸ Report a Bug Here…, what each report
  sends (the details of your computer included), and how to switch those off.
- [Collet and Holder Collisions](help:collisions): the collet nut, spindle or shank
  hitting wood beside a deep cut, shown red in the preview and asked about before saving.
- [Web API](help:web_api): let a program or script drive Cut Craft, off until you
  switch it on in Options.
- [How long the job takes to cut](help:cutting_time): what the machining time counts, and
  the machine settings that make it match your machine.
- [Long jobs and the time left](help:progress): the progress window, how its "about 2
  minutes left" is worked out, and Cancel.
- [Using Cut Craft in another language](help:language): install a language pack and
  switch to it in Options.

## Everyday basics

- [Your first job](help:getting_started): the sample project, Design then Toolpaths, a
  first sign step by step, millimetres or inches, and undo.
- [Finding help](help:finding_help): notes under the pointer, F1, Search help, and finding
  a tool by its icon.
- [Find Tool](help:find_tool): Ctrl+K opens any tool by typing part of its name.
- [Keyboard shortcuts](help:keyboard_shortcuts): the keys worth learning first, and
  changing them.
- [Moving around the 2D view](help:view_2d): zooming and moving with the mouse, F2 and F3.
- [Number fields](help:number_fields): the arrows, dragging, and typing sums such as W/2.
- [Snapping](help:snapping) and [Guide lines](help:guide_lines): lining things up exactly.
- [Favourites](help:favourites): the tools you use most, kept at hand.
- [Saving and opening jobs](help:saving_jobs): Increment and Save, the recovery copy, and
  a job that won't open.
- [Colors on screen](help:colors_on_screen): color-blind friendly colors, or your own.
- [First-Time Setup](help:first_time_setup): your machine and usual material, asked again.
- [Confirmation Choices](help:confirmation_choices): what happens when something looks
  wrong.
- [Screen readers](help:screen_readers): using Cut Craft by keyboard or with a screen
  reader (Narrator, NVDA, Orca, VoiceOver).

## Windows that open beside Cut Craft

Some tools open in a window of their own: Did You Know?, Options, the Tool Database,
Machine Configuration, View G-code, Report a Bug and others. Each one stays in front of
Cut Craft's main window until you close it, so clicking the main window never hides it.
You can still use the main window while it is open. It does not cover other programs,
and it is minimised along with Cut Craft.

A file or folder window opened from one of these windows (Save, Open, Browse...) belongs
to that window and opens in front of it.

On a Wayland desktop with **Edit ▸ Options ▸ Advanced ▸ Use Wayland directly** switched
on, and on a Mac, the desktop decides which window is in front. If a window goes behind,
click it in the taskbar or dock to bring it back.

## Printing

**File ▸ Print (opens a PDF)...** opens the 2D view as a PDF in your computer's PDF viewer.
Check it there, then print from the viewer as you would any PDF. To print what the machine
operator needs, use **Toolpaths ▸ Print Job Sheet** or **Toolpaths ▸ Print Part Labels** instead.

## Newer post-processors

Cut Craft's own post-processors come with Cut Craft, so **Help ▸ Check for Updates** brings the
newest ones. Post-processors you added yourself are read from their folder each time, so
there is nothing to update; see **Machine ▸ Post-Processor Management**.

## Updates

Every time Cut Craft starts, it asks GitHub whether there is a new version. If there is, it
asks you: "A new version is available. Download, install and relaunch?" **Install Now**
downloads it, checks it is genuine, installs it, and reopens Cut Craft on your job, with
any unsaved changes kept as they were. **Help ▸ Check for Updates**, or right-clicking the
version number in the bottom-right corner, asks straight away. See
[Updates](help:updates) for every step, what to do if it goes wrong, and how copies
installed with apt, dnf or Flatpak are updated.
