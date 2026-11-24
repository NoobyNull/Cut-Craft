# src/app/views.rs
views-2d-view-f2 = 2D View (F2)
views-3d-view-f3 = 3D View (F3)
views-a-horizontal-guide-its-position = A horizontal guide: its position is where it crosses the Y axis
views-a-locked-guide-cannot-be = A locked guide cannot be dragged
views-a-vertical-guide-its-position = A vertical guide: its position is where it crosses the X axis
views-absolute = Absolute
views-add-new-layer = Add New Layer...
views-add-new-sheet = Add New Sheet...
views-angle = Angle
views-apply = Apply
views-count = Count
views-create-new-guide = Create New Guide
views-delete-guide = Delete Guide
views-drag-between-two-points-to = Drag between two points to measure, or click a vector to inspect it.
views-guide-properties = Guide Properties
views-lock-guide = Lock Guide
views-new-guide = New guide
views-no-machine-yet-add-one = No machine yet: add one in Machine ▸ Machine Configuration
views-offset = Offset
views-parallel-to-this-guide-several = Parallel to this guide; several are the same step apart
views-position = Position
views-relative = Relative
views-swap-sides = Swap Sides
views-tool-changer = Tool Changer
views-a-logo-is-scaled-to = A logo is scaled to 165 × 52 pixels at most; landscape pictures suit it.
views-add-side-to-toolpath-name = Add side to toolpath name
views-archive = Archive
views-back-to-the-settings-new = Back to the settings new jobs start with
views-calculate-first-there-is-no = Calculate first: there is no quick engrave toolpath yet
views-cancel = Cancel
views-change-logo = Change Logo...
views-do-this-every-time = Do this every time
views-fix-the-problems-marked-must = Fix the problems marked Must fix to save
views-fix-the-problems-marked-must-2 = Fix the problems marked Must fix to send
views-immediate-output = Immediate Output
views-machine-files = Machine Files…
views-machine-and-post-processor = Machine and post-processor
views-merge-toolpaths-in-a-row = Merge toolpaths in a row that use the same tool
views-new-jobs-start-with-these = New jobs start with these settings
views-one-file-pause-to-change = One file, pause to change the tool
views-one-file-the-machine-s = One file; the machine's tool changer changes the tools
views-open-labels-to-print = Open Labels to Print
views-opens-the-labels-in-your = Opens the labels in your web browser; print them from there
views-post-processor = Post-processor
views-put-every-setting-on-this = Put every setting on this page back as Cut Craft first had it
views-remove-logo = Remove Logo
views-replace-the-earlier-g-code = Replace the earlier G-code?
views-replace = Replace
views-reset-to-my-defaults = Reset to My Defaults
views-reset-to-defaults = Reset to defaults
views-restore-default-sheet = Restore Default Sheet
views-save-stl = Save STL…
views-save-toolpath-now = Save Toolpath Now...
views-save-toolpaths = Save Toolpaths...
views-save-a-job-sheet-with = Save a job sheet with the files
views-save-nothing-and-leave-the = Save nothing and leave the earlier files as they are
views-save-the-toolpath-calculated-last = Save the toolpath calculated last through the post, keeping the form open
views-send-to-machine = Send to Machine
views-separate-file-for-each-tool = Separate file for each tool
views-show-advanced-settings = Show advanced settings
views-the-job-has-no-parts = The job has no parts yet. Draw or import closed shapes first.
views-the-safest-without-a-tool = The safest without a tool changer: change the tool and set Z zero between files
views-these-files-from-an-earlier = These files from an earlier save are where the new ones go:
views-these-files-from-an-earlier-2 = These files from an earlier send are on the machine, where the new ones go:
views-tool-changer-atc = Tool changer (ATC)
views-tool-changes = Tool changes
views-toolpaths-to-save = Toolpaths to save
views-triangulate = Triangulate
views-use-as-my-default = Use as My Default
# $count: how many parts get a label.
views-labels-count = { $count ->
    [one] { $count } part will get a label. Name parts with Edit ▸ Name Part.
   *[other] { $count } parts will get a label. Name parts with Edit ▸ Name Part.
}
# $tiles: how many tiles the job is cut in.
views-tiling-on = { $tiles ->
    [one] Tiling on: { $tiles } tile. Each tile is saved as a file of its own, the name you give with _tile1 to _tile{ $tiles } added (a tile with nothing to cut gets none). Toolpaths ▸ Toolpath Tiling turns it off
   *[other] Tiling on: { $tiles } tiles. Each tile is saved as a file of its own, the name you give with _tile1 to _tile{ $tiles } added (a tile with nothing to cut gets none). Toolpaths ▸ Toolpath Tiling turns it off
}
views-a-label-for-each-part = A label for each part, to stick on it once it is cut, so you can tell the parts apart when you put the job together.
views-delete-the-earlier-files-older = Delete the earlier files, older numbered ones too, and save the new ones: nothing old is left to run by mistake
views-how-the-light-cut-clearing = How the light-cut clearing styles cut, for this whole job: saved with it, so it cuts the same on any computer.
views-keep-this-answer-and-stop = Keep this answer and stop asking. Edit ▸ Options ▸ Confirmation Choices ▸ G-code from an earlier save changes it
views-list-settings-for-how-hard = List settings for how hard Cut Craft works your computer and how exactly it cuts. Most people never need them.
views-move-the-earlier-files-into = Move the earlier files into a dated folder under Archive beside them, then save the new ones
views-no-tool-changer-is-set = No tool changer is set up: pick or add your machine, and give it the tool changer's slots in Machine Configuration
views-on-a-label-printer-turn = On a label printer, turn off the browser's headers and footers and set the margins to None when printing.
views-save-the-files-straight-onto = Save the files straight onto the machine, in this project's own folder there
views-the-machine-stops-m0-at = The machine stops (M0) at each tool change; change the tool, set Z zero, then resume. Some controllers (GRBL among them) cannot jog while stopped, which makes setting Z zero hard: use separate files there
views-the-project-s-sheet-to = The project's sheet to print or keep by the machine: material, origin, tools, toolpaths in order and the files to run, saved beside them
views-toolpaths-next-to-each-other = Toolpaths next to each other in the list that use the same tool go in one file. The cutting order never changes
# Save Toolpaths' files. $number counts from 1; $toolpaths is their names, with their tools.
views-file-toolpaths = File { $number }: { $toolpaths }
# Save STL, after Triangulate. $error comes with its unit.
views-triangles-made = Num Tri Created: { $count }
views-maximum-error = Maximum Error: { $error }
# Under the earlier files that the new ones replace.
views-and-more = and { $count } more
# A tool's button while nothing is selected. $tool is the tool's name.
views-select-first = { $tool }: select something first
# The post-processor list: the machine's own post.
views-post-default = { $name } (default)
# The hint over a keep-out zone: a clamp of the machine's, or one of the job's own.
views-keep-out-machine-clamp = { $zone }: no toolpath cuts here. With Clip toolpaths at keep-out zones off (Edit ▸ Options) a toolpath that would cut into it is not made; with it on, the toolpath is made and lifts over it (not laser or drill bank toolpaths: move the part or the zone for those). It is a clamp of the machine's, edited in Machine Configuration; right-click for a shortcut
views-keep-out-job-zone = { $zone }: no toolpath cuts here. With Clip toolpaths at keep-out zones off (Edit ▸ Options) a toolpath that would cut into it is not made; with it on, the toolpath is made and lifts over it (not laser or drill bank toolpaths: move the part or the zone for those). It is edited in Toolpaths ▸ Keep-Out Zones…; right-click for a shortcut
# What a screen reader calls the new guide's box.
views-new-guide-offset = New guide offset
views-new-guide-position = New guide position

# src/app/split_views.rs
views-split-divider = Drag to give one view more room; drag it right to the edge to show just the other view
views-split-swap = Swap the views' places
views-picture-bar = Drag to another corner
views-picture-swap = Swap: make this view the big one
