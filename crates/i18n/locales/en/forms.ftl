# A tool form's buttons, along its foot.

# The commit button: named for what pressing it does. Its tooltip says what that is.
commit-create = Create
    .tooltip = Make a new one with these settings
    .tooltip-drawing = Make the shape from these numbers. You can also drag on the canvas to draw it
commit-calculate = Calculate
    .tooltip = Work out the toolpath with these settings
    .tooltip-again = Work out this toolpath again with these settings
commit-apply = Apply
    .tooltip = Make these changes
commit-generate = Generate
    .tooltip = Ask for a new one with these settings, and save it in your Library

# Where a form with no commit button would have one: why it has none.
commit-none-canvas = Use it on the canvas: each change shows as you make it
commit-none-drawing = Changes show on the selected shape as you make them
commit-none = Changes show as you make them

form-close = Close
    .tooltip = Close, keeping what is there
form-cancel = Cancel
    .tooltip = Undo what this tool did since it opened, and close it. A change made in the toolpath list meanwhile is kept
    .disabled = Nothing done yet to take back

# A new job's setup form.
job-setup-ok = OK
    .tooltip = Accept these settings and start the job
job-setup-cancel = Cancel
    .tooltip = Leave the job as it was
job-setup-set-as-default = Set as Default
    .tooltip = Start every new job with these settings (your Clearing Settings default is kept)

# Setup Material and Job Defaults (open straight to a blank job).
setup-material-more-tip = Show every Job Setup setting here: Z zero, X0 Y0, the rapid heights, home and the material, for this job only
setup-material-defaults-tip = Edit ▸ Options ▸ Job Defaults: what every new job starts with
job-defaults-note = Every new job starts with these. Changing them leaves the job that is open as it is: its own are in Setup Material ▸ More settings for this job…

# A toolpath form's bit: picked from the Tool Database.
tool-select = Select…
    .tooltip = Pick the bit this toolpath cuts with in the Tool Database: it brings its own feeds, speeds and pass depth
    .tooltip-laser = Pick the laser this toolpath burns with in the Tool Database (its Lasers): it brings its own kerf and speed
    .tooltip-another = Pick another bit for this in the Tool Database: it brings its own feeds, speeds and pass depth
    .tooltip-another-bit = Pick another bit for this in the Tool Database
tool-edit = Edit…
    .tooltip = Change this tool in the Tool Database
    .tooltip-bit = Change this bit in the Tool Database
tool-list-add = Add a tool…
    .tooltip = Pick one more in the Tool Database
tool-list-remove = Remove

# Pocket's clearing bits.
clearing-suggest = Suggest clearing bits
    .tooltip = Try your own bigger end mills, one or two at a time, and pick the quickest way to finish this pocket (bits in the tool changer first; Options sets how long a bit change takes)
clearing-add-smaller = Add a smaller clearing bit…
    .tooltip = Another bit, between the ones above and the finishing bit, for corners too tight for them
clearing-remove-bit = Remove this bit

# Rest Machining: cutting only what earlier tools left.
rest-title = Rest Machining
    .tooltip = Cut only what the tools that ran before this one could not reach
rest-off = Off: add the bigger tools that cut before this one to cut only what they left
rest-minimum-detail = Minimum detail
    .tooltip = Leave alone unmachined areas narrower than this
rest-overlap = Overlap
    .tooltip = Grow each unmachined area by this

# A toolpath's Vector Selector: which vectors it takes each time it is calculated.
selector-associate = Associate a Vector Selector with this toolpath
    .tooltip = Take the vectors that pass it each time the toolpath is calculated
selector-open-vectors = Open vectors
selector-closed-vectors = Closed vectors
selector-all-closed = All closed vectors
selector-only-circles = Only circles
    .tooltip = For drilling: only the closed vectors that are circles
selector-all-circles = All circles
selector-circles-of-diameter = Circles of this diameter
selector-diameter = Diameter
selector-tolerance = Tolerance
    .tooltip = How far a circle's diameter may be from it and still pass

# Toolpaths ▸ Toolpath Summary.
summary-total = Total machining time: { $time }
    .tooltip = Every move: cutting, rapids, getting to each toolpath, tool changes and the trip back home
summary-write-notes = Click to write its notes
summary-each-time = Each time counts every move at the tools' feed rates and the machine's speeds, getting to it from the toolpath before and its tool change included
summary-acceleration = Machine acceleration
summary-notes-for = Notes for { $toolpath }
summary-save-notes = Save Notes
    .tooltip = Keep these notes with the toolpath
