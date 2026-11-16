# Cut Craft Help

Cut Craft draws 2D vectors and 3D models on a sheet of material, then makes the toolpaths
a CNC machine cuts them with.

- [Design](help:design): the page down the left. Draw and edit vectors, then model in 3D.
- [Toolpaths](help:toolpaths): the page down the right. Turn vectors into toolpaths,
  preview them and save them for the machine.
- [Did You Know?](help:tips): short tips, one shown each time Cut Craft starts.
- [Web API](help:web_api): let a program or script drive Cut Craft, off until you
  switch it on in Options.

## Finding help

- **Search help** along the top of this window finds every page with the words you type:
  a tool's name, a setting such as "stepover", or any word for what you want to do.
- **?** at the top of the Design or Toolpaths page lists that page's tools.
- **F1** with a tool's form open goes straight to that tool; with none open, to this page.
- Hover over a setting's name in a form for a short note about it.
- Hover over a greyed-out menu item or tool to see what it is waiting for, such as a
  selection, a toolpath picked in the list, or a job open.

## Number fields

Every number field has small up and down arrows on its right. Click one to nudge the value
by a step that suits it: 0.1 mm or 0.01 in for a length, 1 for a percentage, an angle or a
count, 100 mm/min (5 in/min) for a feed rate, 1000 rpm for a spindle speed. Hold the arrow to
keep going, and hold **Shift** for steps ten times as big. The arrows stop at the field's
lowest and highest values. You can also drag sideways on the value (the pointer shows a
two-way arrow there) or just click it and type.

A number field takes a sum as well as a number, worked out when it is entered:
`+ - * / ^` and brackets, and these letters:

- **W** or **X**: the material's width; **H** or **Y**: its height; **T** or **Z**: its thickness
- **P**: π
- **I**: inches to millimetres; **M**: millimetres to inches; `'` after a number: feet
- **{name}**: a [document variable](help:document_variables)

## Keys

- **F1** the open tool's help, **F2** the 2D view, **F3** the 3D view, **F4** Snapping Options. Edit ▸ Options ▸ Mouse and Shortcuts can move these like any other key; the arrow keys always nudge the selection
- **F11** the designing layout, **F12** the machining layout
- **Ctrl+Alt+S** Increment and Save: a new numbered copy (sign_1), leaving the file you opened as it was. **Ctrl+S** saves over it
- **Ctrl+Y** redoes; on a Mac **⇧⌘Z** does too, as in other Mac apps
- **Ctrl+Q** starts Quick Engrave, except on a Mac, where **⌘Q** quits: set it another key in Edit ▸ Options ▸ Mouse and Shortcuts
- **Esc** closes a small window in front (Find Tool, a library file's Properties) on its own, else the open form (on Import 3D Model's form it is Cancel: the model is not imported); pressed while dragging in the 2D or 3D view, it cancels the drag (a guide being pulled out of a ruler too) and changes nothing
- **Esc** in Move, Rotate, Set Size or another Transform Objects tool opened over a form you were filling in goes back to that form, as you left it. What you moved stays moved, and toolpaths it changes are out of date until recalculated
- **E** edits the selected shape (or Create Shape component) in the tool that made it. Once a Create Shape component has been sculpted, smoothed, embossed or cut back with Replace Below or Reset, it is kept as you left it: E no longer remakes it from its settings, so your hand work is never thrown away
- Help ▸ Keyboard Shortcuts lists the rest.

## Saving and opening jobs

- When you open, start or close a job while the one you have open has unsaved changes,
  Cut Craft asks **Save changes to (file name)?**: **Save** saves them, **Don't Save** lets
  them go, **Cancel** keeps working. If you press **Don't Save**, then cancel the file
  picker (or the file will not open), nothing is lost: your job stays open with its unsaved
  changes, still marked with **\*** in the title and still kept for recovery.
- A name you type with a dot in it keeps all of it: `plaque v1.2` saves as
  `plaque v1.2.ccjob`. When Cut Craft adds the extension for you and a file of that name is
  already there, it asks before replacing it. The same goes for exports, templates and
  other saved files.
- Opening a file with Cut Craft from your file manager (**Open With**, or dragging it onto
  Cut Craft's icon) starts Cut Craft with it. A job opens. A drawing or picture starts a
  new job: set up the material in **Job Setup**, and **OK** makes the job and imports the
  file into it (**Cancel** goes back to the start page without it).
- A job that will not open says why. **Damaged** means the file was cut short or broken,
  often by a copy or cloud sync that did not finish: open a backup or an earlier copy.
  **Saved by a newer version** means Cut Craft needs updating first (**Help ▸ Check for
  Updates**). Either way the job stays in **Recent Files**, ready to try again; only a
  file that is no longer there is taken off the list. If a newer version's job does open, but has settings this version does not
  know, a warning names them: saving it here leaves them out, so update first to keep them.
  Each save (Save, Save As, Increment and Save) gives the warning again.
- Unsaved work is copied for recovery every 30 seconds (Edit ▸ Options ▸ General). If
  Cut Craft closes without saving, it opens that work again next time it starts. Each open
  window keeps its own copy: a second Cut Craft window does not take the first one's work.
  This works on network and other shared settings folders too. If the settings folder
  cannot be written at all, the status bar warns that no copy is being kept: save often.

## Updates

Cut Craft asks GitHub about once a day, when it starts, whether there is a new version, and
says so if there is. **Help ▸ Check for Updates** asks straight away.

- **Update Now** downloads the new version, checks it is a genuine Cut Craft release
  (signed by its maker) that arrived complete, and installs it. Nothing is downloaded until
  you press it.
- **Restart Now** closes Cut Craft and opens the new version. If your job has unsaved
  changes, you are asked whether to save them first. **Restart Later** keeps working; the
  new version starts next time.
- **Skip This Version** stops the startup check offering that version.
- To stop the startup check, turn off **Edit ▸ Options ▸ Updates ▸ Check for updates when
  Cut Craft starts**.

The Flatpak and the Mac version cannot replace themselves: the window links to the download
page instead.
