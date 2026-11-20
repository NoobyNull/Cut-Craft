# src/app/gcode_view.rs
gcode-view-check-before-cutting = Check before cutting
gcode-view-check-the-toolpaths-and-save = Check the toolpaths and save the files for the machine
gcode-view-copy-all = Copy All
gcode-view-copy-this-file-s-g = Copy this file's G-code, to paste somewhere else
gcode-view-don-t-save = Don't Save
gcode-view-don-t-send = Don't Send
gcode-view-file = File:
gcode-view-go-ahead-you-have-read = Go ahead: you have read these and they are what you want
gcode-view-make-the-g-code-again = Make the G-code again from the toolpaths as they are now
gcode-view-post-processor = Post-processor:
gcode-view-refresh = Refresh
gcode-view-save-anyway = Save Anyway
gcode-view-save-toolpaths = Save Toolpaths…
gcode-view-save-nothing-to-put-the = Save nothing, to put the toolpaths right first
gcode-view-save-the-toolpaths-anyway = Save the toolpaths anyway?
gcode-view-saving-makes-these-files-run = Saving makes these files, run in this order: pick one to read
gcode-view-send-anyway = Send Anyway
gcode-view-there-is-nothing-to-cut = There is nothing to cut in the toolpaths shown.
gcode-view-what-saving-would-write-through = What saving would write, through the machine's post-processor. Nothing is saved; the checks before cutting run when you save.
# $time is when the G-code was made, as 21:04:09.
gcode-view-lines-made-at = { $count ->
    [one] { $count } line, made at { $time }
   *[other] { $count } lines, made at { $time }
}
