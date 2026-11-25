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
start-community-link = Cut Craft on { $site }

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
spelling-no-dictionary = Spelling: no dictionary found (put a Hunspell .dic and .aff in Cut Craft's dictionaries folder)
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
    .disabled = Cut Craft has no settings folder to put it in

# Generate Border: a repeating border made with the Borders add-on and the person's own Google key.
border-google-title = Your Google key
border-sent-to-google = Generate sends the style and your Details to Google, which draws the border and bills it to your own Google account. Nothing else from your job is sent.
border-key = Google key
    .tooltip = Your own Google AI Studio API key. It is kept in this computer's password store, never in your jobs or settings
border-save-key = Save Key
    .tooltip = Keep this key in this computer's password store for next time
border-key-saved = Key saved in this computer's password store.
border-key-saved-file = This computer has no password store, so the key is saved in a private file in Cut Craft's settings folder.
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
credits-intro = Cut Craft is built with free libraries, pictures and icons made by other people. Thank you to all of them. Each keeps its own licence, shown here in full: click a line to read it.
credits-search = Search by library or licence name
credits-showing = Showing { $shown } of { $total }
credits-none = Nothing matches "{ $search }". Try part of a library's name, such as serde, or a licence, such as MIT.
credits-more = { $names } and { $count } more

# Help ▸ About.
about-name = Cut Craft
about-version = Version { $version }
about-what = 2D and 3D design and CNC toolpaths.
about-license = License
about-read-license = Read the full license
about-acknowledgements = Acknowledgements
about-credits = Credits…
    .tooltip = The libraries, pictures and icons Cut Craft is built with, and their licences
about-contributors = Contributors on GitHub
about-commits = { $count ->
    [one] { $count } commit
   *[other] { $count } commits
}
lathe-a-speed = Nearest the axis ({ $radius } out) A turns at { $rate }°/min, to keep the bit's feed of { $feed }/min there.
lathe-a-speed-capped = Nearest the axis ({ $radius } out) A would need { $rate }°/min to keep the bit's feed of { $feed }/min, but this machine turns A at most { $most }°/min: within { $slow } of the axis the bit cuts slower than its feed, and its chips get thinner. A slower feed or a bigger holding stub keeps them.
