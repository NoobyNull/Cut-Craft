# The main window: its pages, the tool forms' shared rows and the windows they open.

# A page's title row.
page-pin = Pin this page open
page-unpin = Unpin: hide this page to its side tab
page-switch-layout = Switch Layout (F11 designing, F12 machining)
page-help = Help (F1)
# $hint: what the sub-tab holds, a sentence with no full stop.
subtab-in-row = { $hint }. Right-click to show it as its own section
subtab-as-section = { $hint }. Right-click to put it back in the tab row
# $tab: the tab's name.
tab-not-implemented = { $tab } tools not implemented yet

# The start page's link to the community. $site is a site's name, such as GitHub.
start-community-link = PowerWhittle on { $site }

# Rows a tool form can have.
form-next-step = Next:
form-point-at-setting = Point at a setting to see what it does.
form-reset = Reset
    .tooltip = Put every setting back to its default and make it again, as if new
form-new-shape = New Shape
    .tooltip = Leave this shape as it is, to type the numbers for another
form-save-settings = Save Settings…
    .tooltip = Save this form's settings to a file, to use again later
form-load-settings = Load Settings…
    .tooltip = Fill this form from settings you saved before
form-calculate-as-new = Calculate as New
    .tooltip = Add another toolpath with these settings, keeping the one already made

# A toolpath form's tabs across its top.
form-tab-cutting = Bit & Depth
    .tooltip = The bit, the bits that clear before it, how deep it cuts and which side of the line
form-tab-own-hint = This toolpath's own settings
# Each toolpath's own tab, named short for it.
form-tab-own-3d-finishing-toolpath = 3D Finishing
form-tab-own-3d-pencil-toolpath = 3D Pencil
form-tab-own-3d-roughing-toolpath = 3D Roughing
form-tab-own-chamfer-toolpath = Chamfer
form-tab-own-drag-knife-toolpath = Drag Knife
form-tab-own-drill-bank-toolpath = Drill Bank
form-tab-own-drilling-toolpath = Drilling
form-tab-own-fluting-toolpath = Fluting
form-tab-own-import-toolpaths = Import
form-tab-own-inlay-toolpath = Inlay
form-tab-own-keyhole-toolpath = Keyhole
form-tab-own-laser-cut-fill-toolpath = Laser
form-tab-own-laser-picture-toolpath = Laser Picture
form-tab-own-moulding-toolpath = Moulding
form-tab-own-photo-vcarve-toolpath = Photo V-Carve
form-tab-own-pocket-toolpath = Pocket
form-tab-own-prism-toolpath = Prism
form-tab-own-profile-toolpath = Profile
form-tab-own-quick-engrave-toolpath = Engrave
form-tab-own-relief-from-photo = Relief
form-tab-own-rotary-details-toolpath = Details
form-tab-own-rotary-finishing-toolpath = Finishing Pass
form-tab-own-rotary-lathe-toolpath = Lathe
form-tab-own-rotary-locked-angles-toolpath = Locked Angles
form-tab-own-rotary-machining-toolpath = Rotary
form-tab-own-rotary-roughing-toolpath = Roughing
form-tab-own-rounding-toolpath = Rounding
form-tab-own-sculpt = Sculpting
form-tab-own-sketch-carving-toolpath = Sketch
form-tab-own-surfacing-toolpath = Surfacing
form-tab-own-texture-toolpath = Texture
form-tab-own-thread-milling-toolpath = Thread
form-tab-own-vcarve-engrave-toolpath = V-Carve
form-tab-own-vcarve-inlay-toolpath = V-Carve Inlay
form-tab-where = Where
    .tooltip = What is cut and what is left alone: picked vectors, rest machining, boundaries, the 3D model
form-tab-movement = Movement
    .tooltip = How the bit gets in and moves: ramps, leads, order, start points, corners, tabs
form-tab-finishing = Finishing
    .tooltip = What is left on for a last, cleaner pass
form-tab-defaults = Defaults
    .tooltip = The toolpath's name, and this form's saved settings
form-tab-next = Next
    .tooltip = Go to the next tab. Calculate is on the last tab (Options ▸ Appearance ▸ Calculate button)
form-tab-saved-settings = Saved settings
form-tab-reset-hint = Put every setting on this form back to PowerWhittle's own, keeping the bit
form-tab-reset-note = Reset keeps the bit picked.

# A shape drawn more than one way: the choice at the top of its form.
form-way-caption = Draw from:
form-way-corner = Corner
form-way-center = Center
form-way-edge = Edge
form-way-corner-hint = Drag from one corner to the opposite corner
form-way-center-rectangle-hint = Drag from the middle out to a corner, the same distance each way
form-way-center-circle-hint = Drag from the middle out to the edge
form-way-edge-hint = Drag from one side straight across to the other
# A shape's button, while its other ways are picked in its form. $ways: their names, such as Corner / Center.
form-way-tooltip = Draw from { $ways }: pick which under Draw from, at the top of its form.

# Toolpaths left out of date by an edit. $message is what the edit said in the status bar.
recalculate-out-of-date = { $count ->
    [one] { $message }. 1 toolpath is now out of date (⚠ in the Toolpaths list): recalculate it, and save its G-code again if you already have
   *[other] { $message }. { $count } toolpaths are now out of date (⚠ in the Toolpaths list): recalculate them, and save their G-code again if you already have
}
recalculate-ask-title = Toolpaths out of date
recalculate-ask = { $count ->
    [one] 1 toolpath is now out of date: the vectors it cuts have changed. Recalculate all toolpaths now, so the G-code you save matches the drawing?
   *[other] { $count } toolpaths are now out of date: the vectors they cut have changed. Recalculate all toolpaths now, so the G-code you save matches the drawing?
}
recalculate-dont-ask = Don't ask again
    .tooltip = Stop this question (Options ▸ Confirmation Choices turns it back on). Out-of-date toolpaths are still marked ⚠, and saving them still asks first
recalculate-all = Recalculate All
    .tooltip = Make every out-of-date toolpath again from its vectors now
recalculate-not-now = Not Now
    .tooltip = Leave them marked ⚠ in the Toolpaths list; Recalculate All is there when you want it

# A new toolpath order left toolpaths that cut only what is above them out of date. $names lists them.
reorder-ask-title = Out of date after the new order
reorder-ask = { $count ->
    [one] { $names } cuts only what the toolpaths above it leave, and they are different now: it is out of date (⚠) until it is recalculated. Put the old order back?
   *[other] { $names } cut only what the toolpaths above them leave, and those are different now: they are out of date (⚠) until they are recalculated. Put the old order back?
}
reorder-put-back = Put it back
    .tooltip = Undo the new order: everything goes back where it was, up to date as before
reorder-keep = Keep the new order
    .tooltip = Keep it, and recalculate the toolpaths marked ⚠ when you are ready
reorder-put-back-done = The old order is back

# Draw Text's spelling check.
spelling-title = Spelling
spelling-no-dictionary = Spelling: no dictionary found (put a Hunspell .dic and .aff in PowerWhittle's dictionaries folder)
spelling-none-wrong = No spelling mistakes
spelling-add-word = Add Word
spelling-remove-word = Remove Word
spelling-a-word-added = a word added…

# Import 3D Model's buttons and placing.
import-model-cancel = Cancel
    .tooltip = Close without importing the model (Esc does the same)
import-model-import = Import
    .tooltip = Keep the model as placed
    .waiting = Nothing is placed yet: change the settings above until the model shows, or press Cancel
align-in-material = Align in Material
align-height = Height
undercuts-update = Update Undercuts
    .tooltip = Judge again what a cutter from above cannot reach
    .disabled = Tick Highlight undercuts first
scale-to-fit = Scale to Fit Material
    .tooltip = Make the model as big as the material allows, turned as it is now. Done once: turning the model after keeps its size
    .disabled = Tick Change size under Model Size first

# Sculpting's brushes.
sculpt-load-brush = Load Brush…
    .tooltip = A component saved as a sculpting brush, for Deposit and Remove
sculpt-round-brush = Round Brush

# Texture Toolpath's files.
texture-load = Load Texture...
    .tooltip = A texture saved before, from the textures folder or anywhere
texture-save = Save Texture...
    .tooltip = Keep these texture settings to use again

# A photo's depth, for turning it into a relief.
depth-title = Depth
depth-from-picture = The picture is used as the depth map itself (white nearest), so no estimate is needed. Tick Invert if its black is nearest.
depth-needs-addon = Turning a photo into a relief needs the free Depth add-on (about 100 MB, downloaded once). If the picture is already a depth map, set Depth from to Picture instead.
depth-estimating = Estimating depth… (up to a minute on a large photo)
depth-has-depth = This photo has its depth. Estimate again after editing the photo.
depth-estimate-first = First estimate how far each part of the photo is from the camera.
depth-estimate = Estimate Depth
    .tooltip = Works out the photo's depth on this computer; nothing is uploaded
    .disabled = Import a photo first
depth-downloading-high-detail = Downloading the High detail model (about 670 MB)...
depth-high-detail = High detail
depth-high-detail-note = Slower, and for personal, non-commercial use only.
depth-want-finer = Want finer detail? The free High detail model (about 670 MB, downloaded once) sees more, but is slower and for personal, non-commercial use only.
depth-get-high-detail = Get High Detail Model
depth-addon-finding = Looking for the newest Depth add-on...
depth-addon-downloading = Downloading the Depth add-on...
depth-addon-asking = The Add-ons window is asking whether to install it.
depth-addon-install = Install Depth Add-on
    .tooltip = Downloads it from GitHub once; Gadgets ▸ Add-ons updates it
    .disabled = PowerWhittle has no settings folder to put it in

# Generate Border: a repeating border made with the Borders add-on and the person's own Google key.
border-google-title = Your Google key
border-sent-to-google = Generate sends the style and your Details to Google, which draws the border and bills it to your own Google account. Nothing else from your job is sent.
border-key = Google key
    .tooltip = Your own Google AI Studio API key. It is kept in this computer's password store, never in your jobs or settings
border-save-key = Save Key
    .tooltip = Keep this key in this computer's password store for next time
border-key-saved = Key saved in this computer's password store.
border-key-saved-file = This computer has no password store, so the key is saved in a private file in PowerWhittle's settings folder.
border-key-kept = Your Google key is saved on this computer.
border-forget-key = Forget Key
    .tooltip = Remove the key from this computer. You can type it in again at any time
border-get-key = Get a free Google key
border-needs-key = Type your Google key above and press Save Key first. Get one free from Google AI Studio.
border-needs-addon = Generate Border needs the free Borders add-on (about 5 MB, downloaded once).
border-needs-depth-addon = Depth from is set to the Depth add-on, which is not installed. Install it from Gadgets ▸ Add-ons, or set Depth from to Google.
border-no-documents = This computer has no Documents folder to save the border in.
border-addon-install = Install Borders Add-on
    .tooltip = Downloads it from GitHub once; Gadgets ▸ Add-ons updates it
border-addon-installing = Installing the Borders add-on...
border-asking-picture = Asking Google to draw the border... (about half a minute)
border-asking-depth = Working out its depth...
border-saving = Saving it in your Library...
border-saved = Saved "{ $name }" in your Library under Generated ▸ Borders. Double-click it there to place it in your drawing.

# Help ▸ Keyboard Shortcuts.
shortcuts-change = Change Shortcuts…
    .tooltip = Set your own keys, or the mouse's thumb buttons and wheel click, in Options ▸ Mouse and Shortcuts

# Help ▸ Credits.
credits-intro = PowerWhittle is built with free libraries, pictures and icons made by other people. Thank you to all of them. Each keeps its own licence, shown here in full: click a line to read it.
credits-search = Search by library or licence name
credits-showing = Showing { $shown } of { $total }
credits-none = Nothing matches "{ $search }". Try part of a library's name, such as serde, or a licence, such as MIT.
credits-more = { $names } and { $count } more

# Help ▸ About.
about-name = PowerWhittle
about-version = Version { $version }
about-what = 2D and 3D design and CNC toolpaths.
about-license = License
about-read-license = Read the full license
about-acknowledgements = Acknowledgements
about-credits = Credits…
    .tooltip = The libraries, pictures and icons PowerWhittle is built with, and their licences
about-contributors = Contributors on GitHub
about-commits = { $count ->
    [one] { $count } commit
   *[other] { $count } commits
}
lathe-a-speed = Nearest the axis ({ $radius } out) A turns at { $rate }°/min, to keep the bit's feed of { $feed }/min there.
lathe-a-speed-capped = Nearest the axis ({ $radius } out) A would need { $rate }°/min to keep the bit's feed of { $feed }/min, but this machine turns A at most { $most }°/min: within { $slow } of the axis the bit cuts slower than its feed, and its chips get thinner. A slower feed or a bigger holding stub keeps them.
