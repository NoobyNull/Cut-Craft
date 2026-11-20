# src/app/panels.rs
panels-add-new-sheet = Add New Sheet...
panels-appearance = Appearance
panels-auto-rotate = Auto-rotate
panels-check-again = Check Again
panels-checking-the-toolpaths-a-big = Checking the toolpaths: a big 3D toolpath takes a little while
panels-close = Close
panels-color = Color...
panels-combine-with-other-components = Combine with other components ...
panels-cut-craft-s-version-help = Cut Craft's version (Help ▸ About)
panels-drop-here-to-cut-it = Drop here to cut it last, in no group
panels-fade = Fade
panels-faster = Faster
panels-fit = Fit
panels-hide-these-controls-the-preview = Hide these controls: the preview stays as it is
panels-how-quickly-the-job-turns = How quickly the job turns, degrees a second
panels-leave-out-the-layers-with = Leave out the layers with nothing on the active sheet
panels-make-a-toolpath-or-tick = Make a toolpath (or tick one in the list) to check it here
panels-more-previews = More Previews
panels-more-detail = More detail
panels-more = More…
panels-no-components-yet = No components yet
panels-no-favourites-yet-right-click = No favourites yet: right-click a tool ▸ Add to Favourites
panels-no-toolpaths-yet-each-one = No toolpaths yet: each one you make is listed here
panels-out-of-date = Out of date
panels-perspective = Perspective
panels-preview-watch-it-cut = Preview: watch it cut
panels-run-the-checks-before-cutting = Run the checks before cutting on these toolpaths again
panels-run-to-retract = Run to Retract
panels-save-the-file-your-machine = Save: the file your machine runs
panels-set = Set
panels-sheets-list = Sheets List
panels-show-all-tools = Show All Tools
panels-show-only-layers-used-on = Show only layers used on this sheet
panels-show-or-hide-the-vectors = Show or hide the vectors drawn over the model (View ▸ Draw Vectors in 3D View)
panels-show-or-hide-this-toolpath = Show or hide this toolpath; Undo takes it back in one step
panels-single-step = Single Step
panels-stop-the-preview-here-keeping = Stop the preview here, keeping what is cut so far (Esc)
panels-stop = Stop
panels-the-checks-were-cancelled-so = The checks were cancelled, so nothing has been checked yet
panels-tilt = Tilt
panels-toolpath-list = Toolpath List
panels-vectors = Vectors
panels-by-rule = by rule
panels-every-toolpath-matches-what-it = ✔ Every toolpath matches what it was made from
panels-version = v{ $version }

# The buttons under the Toolpath List.
toolpaths-move-up = Move Up
    .tooltip = Cut the picked toolpath, or the picked group with all its toolpaths, sooner
toolpaths-move-down = Move Down
    .tooltip = Cut the picked toolpath, or the picked group with all its toolpaths, later
toolpaths-delete = Delete
    .tooltip = Delete Toolpath: take the picked toolpath out of the job. Del does the same just after you click it in the list
toolpaths-group = Group
    .tooltip = Put the picked toolpaths (Ctrl-click to pick several) in a new group, which cuts them one after another and moves them together
toolpaths-ungroup = Ungroup
    .tooltip = Take the picked toolpaths out of their group, or take the picked group apart, keeping its toolpaths
toolpaths-new-group = New Group
    .tooltip = Create Empty Group: toolpaths made while it is picked join it; drag toolpaths onto its name to put them in it
panels-move-up = Move Up
panels-move-down = Move Down
panels-a-toolpath-group-its-toolpaths = A toolpath group: its toolpaths cut one after another. Toolpaths made while it is picked join it, and so do toolpaths dragged onto its name
panels-check-the-toolpaths-and-write = Check the toolpaths and write the G-code file your machine runs, using its post processor
panels-give-the-machine-a-folder = Give the machine a folder or web address to send to in Machine Configuration first
panels-how-finely-the-3d-view = How finely the 3D view draws the model, and how many pictures a second it draws while it moves (Options ▸ 3D View)
panels-turn-the-job-round-and = Turn the job round and round to see it from every side. Drag in the view, or click here again, to stop
panels-1-plays-at-the-machine = ×1 plays at the machine's own speed, as the Toolpath Summary times it: feeds, rapids, speeding up, slowing down and corners. ×10 plays ten times faster
# The Components list's heading: the active sheet's name.
panels-components-of-sheet = Components ({ $sheet })
# The status bar's units button. $other is the units it switches to.
panels-switch-to-inches = Switch to inches (Edit ▸ Units)
panels-switch-to-millimetres = Switch to millimetres (Edit ▸ Units)
panels-current-tool = Tool: { $tool }
# A layer in the Layers list: how many vectors are on it.
panels-layer-vectors = { $count ->
    [one] { $count } vector
   *[other] { $count } vectors
}
# A toolpath in the list, out of date. $tool is its tool's name.
panels-toolpath-out-of-date = { $tool }
    Out of date: what it was made from has changed since. Recalculate it.
panels-shank-guessed = ⚠ { $name } has no shank diameter set, so its shank is drawn from a guess: set it in the Tool Database
