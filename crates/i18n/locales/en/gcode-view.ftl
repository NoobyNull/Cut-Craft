# src/app/gcode_view.rs
gcode-view-check-before-cutting = Check before cutting
gcode-view-checks-cancelled = The checks before cutting were cancelled: press Refresh to run them again.
gcode-view-checks-found = What the checks before cutting found. It is your call what to change: viewing saves nothing.
gcode-view-checks-passed = ✔ The checks before cutting found nothing to put right.
gcode-view-checks-running = Checking the toolpaths before cutting…
gcode-view-check-the-toolpaths-and-save = Check the toolpaths and save the files for the machine
gcode-view-copy-all = Copy All
gcode-view-error = ✖ Must fix: { $message }
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
gcode-view-warning = ⚠ { $message }
gcode-view-what-saving-would-write-through = What saving would write, through the machine's post-processor. Nothing is saved.
# $time is when the G-code was made, as 21:04:09.
gcode-view-lines-made-at = { $count ->
    [one] { $count } line, made at { $time }
   *[other] { $count } lines, made at { $time }
}
