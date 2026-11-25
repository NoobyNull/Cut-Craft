# The status line along the window's foot: what the app says it did, or why it did not.
# A value put in ({ $tool }, { $path }, { $error }) is filled in by Cut Craft: keep it as it is.

## Undo, the preview, gadgets and imports
# $problems: what went wrong with each, separated by semicolons.
status-gadgets-not-loaded = Some gadgets could not be loaded: { $problems }
status-undone = Undone
status-redone = Redone
status-no-machine-yet = No machine yet: add one in Machine ▸ Machine Configuration
status-undid-last-preview = Undid the last preview
status-preview-reset = Preview reset: Undo Last Preview brings it back
# $path: the file it was saved in.
status-saved-brush = Saved sculpting brush { $path }
# $path: the imported file.
status-import-changed = Warning: { $path } has changed since it was imported; imported again as it is now
# $name: the toolpath's name.
status-notes-kept = Notes kept with { $name }

## Work done in the background
# $what: what was being worked out, such as a toolpath's name.
status-not-used-job-changed = { $what } was not used: the job changed while it worked. Try again
# $what: what was being worked out.
status-cancelled-nothing-changed = { $what } cancelled: nothing changed
status-already-up-to-date = Already up to date: nothing changed, so nothing was calculated

## Tool forms
# $error: what was wrong.
status-suggest-could-not-read = Suggest could not read the form: { $error }
status-saved-texture = Saved texture { $path }
status-loaded-texture = Loaded texture { $path }
status-depth-estimated = Depth estimated
status-cut-depth-is-thickness = Cut depth set to the material's thickness
# $path: the file written.
status-exported = Exported { $path }

## Drawing
status-shift-click-to-edit = Shift-click a shape this tool drew to edit it
status-drawing-curves = Drawing curves (S swaps)
status-drawing-lines = Drawing straight lines (S swaps)
# $typed: what has been typed so far.
status-next-point = Next point: { $typed }
status-type-a-point = Type a length, x,y, or an angle A length L
status-drawing-on-from-end = Drawing on from the end: Space or Esc finishes, Tab closes it

## Tools applied, changed and cancelled
# $count: how many were saved.
status-saved-job-sheets = { $count ->
        [one] Saved { $count } job sheet
       *[other] Saved { $count } job sheets
    }
# $tool: the tool that drew it.
status-editing-the-shape = Editing the shape in { $tool }: change the form to change it
# After a tool's name and "applied" (status-applied). $level: the level's name.
status-placed-on-level = : new component on level { $level }
status-placed-on-hidden-level = : new component on level { $level } (hidden: show the level to see it)
# $tool: the tool. $placed: where what it made went (status-placed-on-level), or nothing.
status-applied = { $tool } applied{ $placed }
# $turns: a number with two decimals.
status-strand-revolutions = Each strand makes { $turns } revolutions
# $tool: the tool whose settings changed.
status-changed = { $tool } changed
# $warnings: each warning, separated by semicolons.
status-warning = Warning: { $warnings }
status-model-imported = Model imported
# $tool: the tool whose form was cancelled.
status-cancelled = { $tool } cancelled

## Saving and sending toolpaths
status-toolpaths-not-sent = Toolpaths not sent
status-toolpaths-not-saved = Toolpaths not saved

## Selecting and snapping
# $kind: what it snapped to, such as midpoint.
status-snap-picked = Snap picked: this point is kept on the { $kind }. Press Delete to let it go.
status-stepped-out-to-group = Stepped out to the group around it
status-stepped-out-of-group = Stepped out of the group: a click picks whole groups again

## Gadgets
# $file: the gadget's file. $error: what was wrong.
status-gadget-not-used = { $file } could not be used: { $error }
status-no-gadgets-folder = Cut Craft has no settings folder on this computer to keep gadgets in
status-could-not-open-gadgets-folder = Could not open the My Gadgets folder, { $path } ({ $error })

## Jobs and files
# $material: the material's size and kind.
status-new-job = New job: { $material }
status-saved-template = Saved template { $path }
status-not-made-yet = { $tool }: not implemented yet
# $file: the file to import.
status-set-up-then-import = Set up the material, then OK makes the job and imports { $file } into it
# $path: the folder recovery copies go in.
status-no-recovery-copies = Warning: unsaved work cannot be copied for recovery, because { $path } cannot be written. Save often
status-recovery-copy-not-kept = could not keep the recovery copy: { $error }
status-new-jobs-start-so = New jobs will start with these settings
status-new-job-created = New job created

## Keys
status-end-let-go = End let go
status-preview-stopped = Preview stopped: what it had cut is kept
# $tool: the tool whose form is open again.
status-back-to = Back to { $tool }, as you left it
status-could-not-print = could not print: { $error }
status-could-not-open-folder = could not open the folder: { $error }
status-recovered-save = Recovered unsaved work from the last run: save it (File ▸ Save) or close it
status-recovered-save-as = Recovered unsaved work from the last run: save it (File ▸ Save As) or close it

## First-time setup
# $machine: the machine's name and a comma, or nothing. $size: new jobs' material size.
status-set-up-blank = Set up: { $machine }new jobs { $size }. Draw on the sheet, or Try a sample project. Change these any time in Machine ▸ Machine Configuration and Edit ▸ Options ▸ Job Defaults.
status-set-up-start = Set up: { $machine }new jobs { $size }. Press Create a new file to start, or Try a sample project. Change these any time in Machine ▸ Machine Configuration and Job Setup.
status-setup-skipped = Setup skipped. Help ▸ Run First-Time Setup Again asks the questions again.

## Options
status-title-bar-next-start = The title bar changes next time Cut Craft starts.

## The 3D view's handles
status-let-go-off-material = Let go off the material: nothing was changed
status-pointer-off-material = The pointer is off the material: bring it back, or let go to cancel
status-cancelled-nothing-done = Cancelled: nothing was changed
status-removed-piece-cut-free = Removed the piece cut free: Undo Last Preview on the Toolpaths page brings it back

## The Library
status-could-not-show = Couldn't show { $path }: { $error }
status-file-gone = Couldn't add { $path }: the file isn't there any more.

## The preview
status-preview-closed-job-changed = The job changed, so the preview's controls closed: preview again to play it
status-preview-new-material = Job Setup changed the material, so the preview starts again on the new block

## The 3D view
# $number: the button's number.
status-spacemouse-button-unused = SpaceMouse button { $number } does nothing in Cut Craft yet

## The sample project
status-sample-opened = Sample sign opened with its toolpaths made. Press Preview All Toolpaths on the Toolpaths page to watch it cut
status-sample-toolpaths-not-made = The sample's toolpaths could not be made: { $error }

## Saving toolpaths
status-still-checking = Still checking the toolpaths before cutting: save again once the checks finish
status-checked-after-work = The toolpaths are checked before cutting once the work running now is done: save again then
status-checks-cancelled = The checks before cutting were cancelled: press Check Again, then save

## The Tool Database and the Materials Library
# $error: why the library could not be read.
status-materials-not-saved = { $error }; material changes will not be saved
status-tools-not-saved = { $error }; tool changes will not be saved
# $first: the first bit joined. $more: how many more.
status-bits-joined = Bits listed once per material are now one bit each. { $first }{ $more ->
        [0] {""}
        [one] {" "}(and 1 more; see the log)
       *[other] {" "}(and { $more } more; see the log)
    }

## Importing
status-import-cancelled = Import cancelled
status-open-a-job-to-import = Open or make a job to import { $file } into

## After a crash
status-closed-with-error = Cut Craft closed with an error last time. To send crash reports, change Edit ▸ Options ▸ Bug Reports.
# $path: where its crash log is kept.
status-closed-with-error-log = Cut Craft closed with an error last time. Its crash log is kept in { $path }. To send crash reports, change Edit ▸ Options ▸ Bug Reports.
status-select-to-zoom = select something to zoom to

## The Web API
status-new-web-api-token = Made a new Web API token: scripts need the new one from now on
status-copied = Copied

## Tracing, tabs, joining ends and measuring
status-tracing-whole-picture = Tracing the whole picture
status-no-colours-traced = No colours traced: click one on the picture, or tick it in the trace window
# $colours: the colours, separated by commas.
status-colours-to-trace = Colors to trace: { $colours }
status-drop-tab-on-vector = drop a tab on a selected vector to move it there
status-tab-moved = Tab moved
status-tab-removed = Tab removed
status-tab-added = Tab added
status-click-to-add-tab = click on a selected vector to add a tab, or on a tab to remove it
status-no-open-end = No open end there: click one of the circles at the ends of open vectors
status-end-picked = End picked: now click the end to join it to
status-click-to-measure = click a vector, or drag between two points

## Drawing tools with no step of their own
# $about: what the tool does (its tooltip).
status-drag-in-the-view = { $about }: drag in the view
