# Cut Craft Help

Cut Craft draws 2D vectors and 3D models on a sheet of material, then makes the toolpaths
a CNC machine cuts them with.

- [Design](help:design): the page down the left. Draw and edit vectors, then model in 3D.
- [Toolpaths](help:toolpaths): the page down the right. Turn vectors into toolpaths,
  preview them and save them for the machine.
- [Did You Know?](help:tips): short tips, one shown each time Cut Craft starts.

## Finding help

- **Search help** along the top of this window finds every page with the words you type:
  a tool's name, a setting such as "stepover", or a word such as "tabs".
- **?** at the top of the Design or Toolpaths page lists that page's tools.
- **F1** with a tool's form open goes straight to that tool; with none open, to this page.
- Hover over a setting's name in a form for a short note about it.

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

- **F2** the 2D view, **F3** the 3D view, **F4** Snapping Options
- **F11** the designing layout, **F12** the machining layout
- **Esc** closes the open form
- **E** edits the selected shape (or Create Shape component) in the tool that made it
- Help ▸ Keyboard Shortcuts lists the rest.

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
