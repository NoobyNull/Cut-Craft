# The words of every tool's form and of the Options, Snapping Options, Job Setup Sheet, Part
# Labels and STL windows: each setting's name and tooltip, the choices it offers, and the
# headings over groups and tabs of settings. A key is `setting-`, the text's first words and a
# check of the whole text, so a key changes when its English words do: translate the words
# after `=`, keeping any { "{" } as it is. Each text is here once, under the first form that
# has it; a later form with the same words shows the same translation.

## Edit ▸ Options
setting-3d-view-settings-f58828 = 3D View Settings
setting-what-the-3d-view-draws-de993e = What the 3D view draws when Cut Craft starts.
setting-draw-origin-5a6fbb = Draw origin
setting-draw-material-block-c31ae5 = Draw material block
setting-draw-vectors-in-3d-view-b510fa = Draw vectors in 3D view
setting-the-vectors-drawn-over-the-5eda5a = The vectors drawn over the model (the 3D view's Vectors button)
setting-use-color-shaded-view-35f7db = Use color shaded view
setting-use-shaded-background-c7d401 = Use shaded background
setting-light-follows-viewer-ab17e1 = Light follows viewer
setting-shaded-background-style-0e89a7 = Shaded background style
setting-what-fills-the-3d-view-350325 = What fills the 3D view behind the material, when the shaded background is on
setting-solid-fbec94 = Solid
setting-gradient-3c73c5 = Gradient
setting-image-0ca79a = Image
setting-background-color-fdc6f6 = Background color
setting-solid-and-the-top-of-29ba33 = Solid, and the top of a gradient
setting-gradient-background-color-38dc2c = Gradient background color
setting-the-bottom-of-a-gradient-9c6781 = The bottom of a gradient
setting-image-file-path-b64fef = Image file path
setting-the-picture-behind-an-image-8ff460 = The picture behind an Image background
setting-anti-aliasing-mode-040747 = Anti-aliasing mode
setting-smooths-the-edges-of-everything-90b990 = Smooths the edges of everything drawn (from the next start)
setting-none-4ff7fb = None
setting-4x-e362e1 = 4X
setting-simplify-the-model-while-the-861731 = Simplify the model while the view moves
setting-draw-the-model-coarser-while-cbfd75 =
    Draw the model coarser while the view turns or zooms, and at full detail once it is
    still: smoother on a slow computer, blockier while moving
setting-rotation-32777f = Rotation
setting-how-dragging-turns-the-view-5de2ce =
    How dragging turns the view: about the job's vertical with its up kept up, or any
    way without stops
setting-turntable-9aaeee = Turntable
setting-spin-about-the-job-s-1684e1 =
    Spin about the job's vertical, tilt between straight down and straight up: the job's
    up stays up, as on a machine bed
setting-free-5c36fb = Free
setting-any-way-without-stops-the-0b28ca = Any way, without stops: the job can roll and turn upside down
setting-click-reach-e67c10 = Click reach
setting-how-near-the-pointer-must-119e01 =
    How near the pointer must be to the model, in screen pixels, for a click to pick a
    point on it (drawing on the model, placing a fade's anchor). Larger is easier to hit,
    smaller is more exact
setting-perspective-angle-c57ce6 = Perspective angle
setting-how-wide-the-perspective-view-530631 =
    How wide the Perspective view sees, as a camera lens does: wider makes near things
    look larger and far ones smaller, narrower looks flatter
setting-wait-before-drawing-full-detail-f5d493 = Wait before drawing full detail
setting-how-long-the-view-must-1f861c = How long the view must be still before it is drawn at the detail when still again
setting-auto-rotate-speed-1888c5 = Auto-rotate speed
setting-how-quickly-the-3d-view-c851ce =
    How quickly the 3D view's Auto-rotate button turns the job round, degrees a second:
    slower to look closely, faster for a quick look all round
setting-toolpath-settings-6e771d = Toolpath Settings
setting-toolpath-settings-fa8b29 = Toolpath settings.
setting-auto-open-3d-view-f6563e = Auto open 3D view
setting-swap-to-the-3d-view-2468df = Swap to the 3D view after calculating a toolpath
setting-create-2d-previews-fbbd72 = Create 2D previews
setting-draw-toolpaths-in-the-2d-2a3ea4 = Draw toolpaths in the 2D view
setting-solid-2d-previews-010dd3 = Solid 2D previews
setting-draw-them-as-the-area-179b5a = Draw them as the area the tool covers
setting-2d-solid-preview-color-e83c1b = 2D solid preview color
setting-2d-toolpath-tolerance-69e66b = 2D toolpath tolerance
setting-saved-toolpaths-leave-out-cutting-2d5ca7 =
    Saved toolpaths leave out cutting points this close to the line past them, for
    shorter files; 0 keeps every point, 0.1 mm is the most. The keep-out zones are
    checked on what is saved. Profile, pocket and other 2D toolpaths
setting-3d-toolpath-tolerance-3728e8 = 3D toolpath tolerance
setting-the-same-for-3d-roughing-a348d7 = The same for 3D roughing, finishing, texture, moulding and the like
setting-v-carve-v-bit-carving-5d2ada = V-Carve (V-bit carving) toolpath tolerance
setting-the-same-for-v-carving-a437b3 = The same for V-carving, prism, inlay and chamfer toolpaths
setting-preview-multiplier-20da96 = Preview multiplier
setting-how-many-times-faster-than-91d862 =
    How many times faster than the machine a toolpath preview plays when it starts: 1
    plays each move at its tool's feed or plunge rate, rapids at the rapid rate; the
    slider over the preview changes it while it plays
setting-show-keep-out-zone-labels-3b40ed = Show keep-out zone labels
setting-write-each-keep-out-zone-3f1a9b =
    Write each keep-out zone's name on it in the 2D and 3D views ("Keep-out: Clamp 1").
    The View menu turns them on and off too
setting-clip-toolpaths-at-keep-out-8c2a85 = Clip toolpaths at keep-out zones
setting-off-safest-a-toolpath-that-e6d151 =
    Off (safest): a toolpath that would cut into a keep-out zone, such as a clamp, is not
    made. On: it is made with the part under the zone left uncut, the tool lifting over
    the zone, as if the zone were cut out of whatever is under it. Either way, moves
    across a zone go round it. Laser and drill bank toolpaths are never clipped: move the
    part or the zone for those
setting-bit-change-by-hand-120b26 = Bit change by hand
setting-how-long-changing-a-bit-8a3743 =
    How long changing a bit by hand takes, for the cutting times and Suggest's times on
    Pocket's clearing bits
setting-bit-change-by-tool-changer-9ae727 = Bit change by tool changer
setting-how-long-the-tool-changer-cb5511 = How long the tool changer takes to change a bit, the same way
setting-underscores-for-spaces-in-g-d91a56 = Underscores for spaces in G-code file names
setting-off-a-g-code-file-8537e9 =
    Off: a G-code file's name keeps its spaces, as typed. On: every space becomes an
    underscore, for controllers and USB readers that cannot open names with spaces
setting-bit-name-in-multiple-file-40f2b8 = Bit name in multiple file names
setting-when-toolpaths-are-saved-to-88ecbc =
    When toolpaths are saved to multiple files, each file's name says the bit to fit as
    well as the toolpath
setting-general-settings-95789c = General Settings
setting-general-settings-da55d6 = General settings.
setting-recent-file-list-size-ed84cd = Recent file list size
setting-how-many-files-file-recent-23b17c =
    How many files File ▸ Recent Files and the start page list. Up to 20 are always
    remembered, so a smaller number only hides the rest until it is raised again
setting-preload-icons-8d70e4 = Preload icons
setting-read-every-icon-at-startup-589808 =
    Read every icon at startup, so none shows a loading spinner the first time it is
    drawn; off reads each when first drawn, for low-end systems (from the next start).
    Dialog pictures are always read at startup
setting-file-dialog-default-01c600 = File dialog default
setting-the-folder-file-dialogs-open-a6fac4 = The folder file dialogs open in
setting-global-1280ce = Global
setting-the-desktop-s-own-choice-3084aa = The desktop's own choice, usually the last folder used
setting-job-d9307c = Job
setting-the-open-job-s-folder-c45a87 = The open job's folder
setting-window-layout-16973d = Window layout
setting-how-the-window-is-arranged-400655 =
    How the window is arranged: a ribbon of tabs over the view, a rail of tool groups
    down the left, or the Classic Design and Toolpaths pages with side tabs. Every tool
    is in every layout
setting-ribbon-7b9049 = Ribbon
setting-tabs-of-tool-groups-across-acf919 =
    Tabs of tool groups across the top; your sheets, layers and toolpaths on the left,
    the open tool's settings on the right
setting-rail-d00519 = Rail
setting-a-column-of-tool-groups-60a11a =
    A column of tool groups on the left, each opening its list of tools by name; the
    open tool's settings on the right
setting-canvas-585091 = Canvas
setting-the-whole-window-is-the-20d3da =
    The whole window is the drawing: the tools, the open tool's settings and your
    sheets, layers and toolpaths float over it
setting-classic-47b1f3 = Classic
setting-the-design-page-of-tool-73fb81 =
    The Design page of tool icons on the left, the Toolpaths page on the right, and
    tabs down the sides
setting-keep-a-recovery-copy-every-dfa2c0 = Keep a recovery copy every
setting-how-often-unsaved-work-is-c3c26b =
    How often unsaved work is copied, so it can be opened again after a crash or a power
    cut. Shorter loses less work; longer writes to the disk less often
setting-hold-to-open-a-tool-21bdd5 = Hold to open a tool group
setting-with-tool-groups-folded-how-4b9962 =
    With tool groups folded: how long to press and hold a toolbox button with a small
    arrow before its group of tools opens; a quick click runs the tool shown. Longer if
    groups open by accident
setting-fold-tool-groups-into-one-a32392 = Fold tool groups into one button
setting-show-each-group-of-tools-fcef99 =
    Show each group of tools that do one job different ways (the circles, the text
    tools, the joins…) as one button with a small arrow, to open for the rest. Off shows
    every tool in a labelled box, so none is missed
setting-ribbon-icons-6c3dab = Ribbon icons
setting-how-many-tools-each-group-178fa8 =
    How many tools each group shows as icons in the Ribbon layout. Compact shows those
    that fit under the group's name, and the name lists the rest (pin any tool to show
    it); Every Tool shows them all, the groups going on to more rows as they need
setting-compact-9ead86 = Compact
setting-the-tools-that-fit-under-d9bbf5 = The tools that fit under the group's name; the name lists the rest
setting-every-tool-ff8f76 = Every tool
setting-every-tool-in-the-group-cbcb49 = Every tool in the group
setting-canvas-tool-lists-open-9c6fcc = Canvas tool lists open
setting-when-a-group-in-the-779c12 =
    When a group in the Canvas layout's palette lists its tools beside it: On Hover as
    soon as the pointer is over the group, On Click when it is clicked
setting-on-hover-b0e222 = On hover
setting-as-soon-as-the-pointer-a3646c = As soon as the pointer is over the group
setting-on-click-9d4b08 = On click
setting-when-the-group-is-clicked-3b431a = When the group is clicked
setting-show-a-tip-when-cut-8faee5 = Show a tip when Cut Craft starts
setting-open-a-short-did-you-06c331 =
    Open a short Did You Know? tip each time Cut Craft starts, a different one each
    time. Help ▸ Did You Know? shows them whenever you like
setting-performance-517045 = Performance
setting-how-hard-cut-craft-works-41daba =
    How hard Cut Craft works the computer: how much of it calculations may use, and how much
    detail the previews and the 3D view are drawn with. Toward speed or toward detail, each.
setting-limit-calculations-so-other-programs-05ef5e = Limit calculations so other programs stay smooth
setting-while-ticked-calculations-toolpaths-previews-903129 =
    While ticked, calculations (toolpaths, previews, simulations, imports) use only the
    share of the processor set below, and give way to your other programs: a video or
    the browser stays smooth. Unticked, they use every core as fast as the computer goes,
    but other programs may stutter meanwhile. Applies from the next calculation
setting-share-of-the-computer-to-d5ef77 = Share of the computer to use
setting-how-much-of-the-processor-c23250 =
    How much of the processor calculations may use: less keeps other programs smooth,
    more finishes sooner. 80% leaves a core or more free for everything else
setting-preview-detail-90f064 = Preview detail
setting-how-finely-a-toolpath-preview-efc2f3 =
    How finely a toolpath preview is worked out to start with: toward Performance is
    quicker and uses less memory, toward Detail shows small corners, holes and thin
    walls truer (about 0.5 mm between points at 0%, 0.1 mm at 50%, 0.025 mm at 100%).
    The Preview tab has the same slider for now; Reset Preview goes back to this
setting-detail-when-still-74156e = Detail when still
setting-how-finely-the-model-is-d06960 =
    How finely the model is drawn once the view is still: higher shows more detail and
    takes longer to draw after each change
setting-low-dd9521 = Low
setting-quickest-about-250-points-across-f2f2ac = Quickest: about 250 points across the job
setting-medium-be5f96 = Medium
setting-about-500-points-across-the-19c8c9 = About 500 points across the job
setting-high-0a061d = High
setting-about-800-points-across-the-2cff16 = About 800 points across the job
setting-very-high-b45c9f = Very High
setting-finest-about-1200-points-across-a0b366 = Finest: about 1200 points across the job, or the model's own detail when that is less
setting-detail-while-moving-e8a48d = Detail while moving
setting-how-finely-the-model-is-49d0e6 =
    How finely the model is drawn while it or the view moves (turning the view, dragging
    a handle): lower keeps it smooth on a slower computer. Never finer than the detail
    when still. A playing preview keeps the detail when still
setting-one-step-lower-6cdbba = One step lower
setting-one-level-below-the-detail-25fd8e = One level below the detail when still
setting-two-steps-lower-1bafe7 = Two steps lower
setting-two-levels-below-the-detail-352806 = Two levels below the detail when still (Very High moves at Medium)
setting-adjust-detail-to-keep-the-21258b = Adjust detail to keep the view smooth
setting-while-the-view-moves-draw-876523 =
    While the view moves, draw the model as finely as this computer can keep smooth:
    coarser when frames come slowly, finer again when they come quickly, never finer
    than the detail when still
setting-smoothness-to-aim-for-5da3f7 = Smoothness to aim for
setting-how-many-pictures-a-second-14f10f =
    How many pictures a second the 3D view aims for while it moves: higher is smoother
    and coarser, lower sharper and choppier. 30 suits most; films run at 24
setting-show-detail-and-frame-rate-9e19ea = Show detail and frame rate in the status bar
setting-show-the-3d-view-s-b60ff9 =
    Show the 3D view's detail and frames a second at the foot of the window, to see what
    the setting above does
setting-language-73db9b = Language
setting-the-language-cut-craft-s-6d6e4d =
    The language Cut Craft's words are in: English, built in, or a language pack
    installed from Gadgets ▸ Add-ons.
setting-job-defaults-1d7eee = Job Defaults
setting-how-cut-craft-starts-and-3868af =
    How Cut Craft starts, and what every new job starts with: the material, where X0 Y0
    and Z zero are, the rapid heights and home (the job settings under it).
setting-classic-startup-flow-cf3436 = Classic Startup Flow
setting-start-with-the-start-page-42a7ad =
    Start with the start page (recent files, the sample project) and fill in every job
    setting before a new job opens, as Cut Craft used to. Off opens a blank job straight
    away, made with the settings below; Setup Material changes its size any time
setting-confirmation-choices-cade2c = Confirmation Choices
setting-what-happens-when-something-looks-a51fa3 = What happens when something looks wrong.
setting-toolpath-vector-intersections-check-8545dd = Toolpath vector intersections check
setting-crossing-vectors-chosen-for-a-a2645d = Crossing vectors chosen for a V-carve or pocket toolpath
setting-ask-fd61b2 = Ask
setting-ask-whether-to-go-on-20f558 = Ask whether to go on, open the Vector Validator, or stop
setting-open-vector-validator-33f356 = Open vector validator
setting-select-the-crossing-vectors-in-7f43e1 = Select the crossing vectors in the Vector Validator instead
setting-don-t-check-85d259 = Don't check
setting-go-on-without-checking-c9fcc0 = Go on without checking
setting-radiused-offset-intersections-check-62f867 = Radiused offset intersections check
setting-crossing-vectors-chosen-for-a-7298b5 = Crossing vectors chosen for a radiused (round-cornered) offset
setting-sharp-offset-intersections-check-7f9148 = Sharp offset intersections check
setting-crossing-vectors-chosen-for-a-495957 = Crossing vectors chosen for a sharp-cornered offset
setting-warn-when-material-left-is-4135b7 = Warn when material left is over
setting-the-checks-before-cutting-warn-18b450 =
    The checks before cutting warn when a tool's taper or shape will leave more material
    than this on the model. Raise it if the warnings are about bits too small to matter
setting-ask-to-recalculate-out-of-0e4e7b = Ask to recalculate out-of-date toolpaths
setting-after-an-edit-leaves-toolpaths-315346 =
    After an edit leaves toolpaths out of date, ask whether to recalculate them all. Off,
    they are still marked ⚠ in the Toolpaths list, and saving them still asks first
setting-g-code-from-an-earlier-2ac43c = G-code from an earlier save
setting-saving-toolpaths-where-an-earlier-768fcc =
    Saving toolpaths where an earlier save of the same name left G-code: ask, replace it
    (the older numbered files go too, so none is left to run by mistake), or archive it
    (moved into a dated folder under Archive beside it) and add the new files
setting-ask-each-time-replace-or-d45945 = Ask each time: Replace or Archive
setting-replace-d54f63 = Replace
setting-replace-the-earlier-files-367c03 = Replace the earlier files
setting-archive-690b65 = Archive
setting-move-the-earlier-files-into-2fc40b = Move the earlier files into a dated folder under Archive, then add the new ones
setting-importing-and-exporting-aa7bbb = Importing and Exporting
setting-importing-pictures-and-3d-models-d5c480 =
    Importing pictures and 3D models: the size pictures come in at, and what Orientate 3D
    Model offers; and the line width of exported PDF, EPS and AI files.
setting-picture-size-pixels-per-inch-d64cfb = Picture size (pixels per inch)
setting-how-big-an-imported-picture-765a2a =
    How big an imported picture comes in: this many of its pixels make an inch. 96 suits
    screenshots and web pictures; a scan or a photo for printing is usually 300. A picture
    bigger than the material is still shrunk to fit
setting-imported-model-type-7d629c = Imported model type
setting-what-an-imported-model-is-ee5cf3 =
    What an imported model is kept as: whole (its underside on the other side of a
    double-sided job, wrapped in a rotary one), its top seen from above, or the choice
    of both on the form
setting-full-3d-model-795bfe = Full 3D model
setting-flat-model-ec2f9b = Flat model
setting-both-chosen-on-the-form-4fd7bc = Both, chosen on the form
setting-model-preview-faces-760414 = Model preview faces
setting-a-model-with-more-faces-d5f2e4 =
    A model with more faces than this is shown as the surface it makes while it is
    imported, with only the faces out of reach drawn over it; drawing millions of faces
    a frame makes the view drag
setting-out-of-bounds-opacity-7b0750 = Out-of-bounds opacity (%)
setting-how-much-of-a-model-f6219b =
    How much of a model placed past the material's edges shows while it is imported:
    that part is drawn in see-through red, this many percent solid, so it is not lost
    from sight but does not hide the material either
setting-exported-pdf-eps-and-ai-8fc9b7 = Exported PDF, EPS and AI line width
setting-how-thick-the-lines-are-480fa5 =
    How thick the lines are in exported PDF, EPS and AI files, in points (a point is
    1/72 inch). Thicker shows better on paper; laser and vinyl cutter software often
    wants a hairline, so set it thin for those
setting-appearance-018b9f = Appearance
setting-how-cut-craft-looks-f82647 = How Cut Craft looks.
setting-theme-af7c92 = Theme
setting-light-dark-or-the-desktop-3f9d93 = Light, dark, or the desktop's own
setting-dark-570065 = Dark
setting-light-d2f8cf = Light
setting-light-whatever-the-desktop-s-f67391 = Light, whatever the desktop's theme
setting-system-3184bc = System
setting-the-desktop-s-own-light-657a2c = The desktop's own light or dark theme
setting-use-the-desktop-s-title-d0baca = Use the desktop's title bar
setting-the-desktop-s-own-title-47df26 = The desktop's own title bar above the menus, rather than Cut Craft's in the menu bar
setting-vector-and-toolpath-colors-8e46d4 = Vector and toolpath colors
setting-the-colors-vectors-toolpaths-and-04583b =
    The colors vectors, toolpaths and rapid moves are drawn in. Standard: as Cut Craft
    has always drawn them. Color-blind friendly: colors told apart by how light or dark
    they are, not only by their hue. My own colors: pick each one below, for each theme
setting-standard-16ab68 = Standard
setting-vectors-in-the-text-color-631895 =
    Vectors in the text color, toolpaths orange, rapid moves red: as Cut Craft has
    always drawn them
setting-color-blind-friendly-639a49 = Color-blind friendly
setting-vectors-black-white-in-the-ee67a7 =
    Vectors black (white in the dark theme), toolpaths deep orange, rapid moves pink:
    told apart by how light or dark they are as well as by their hue
setting-my-own-colors-9b5d9d = My own colors
setting-the-vector-toolpath-and-rapid-13fbf6 = The vector, toolpath and rapid move colors you pick for each theme below
setting-2d-view-in-the-light-25c8c1 = 2D view in the light theme
setting-the-2d-view-s-colors-783735 = The 2D view's colors while the light theme is on
setting-sheet-color-b4a2cf = Sheet color
setting-the-material-sheet-the-drawing-9cd929 = The material sheet the drawing sits on
setting-grid-color-95a622 = Grid color
setting-the-snap-grid-s-points-56acd7 = The snap grid's points: pick one that stands out from the sheet color
setting-vector-color-6efd09 = Vector color
setting-your-drawing-s-lines-the-b0aa55 = Your drawing's lines, the shapes toolpaths are made from
setting-toolpath-color-b9f2a7 = Toolpath color
setting-where-the-bit-cuts-pick-b00751 =
    Where the bit cuts: pick one that looks different from the vector color, lighter or
    darker as well as another hue
setting-rapid-move-color-4c95d9 = Rapid move color
setting-the-bit-s-quick-moves-69d0ba =
    The bit's quick moves above the material, and its plunges and lifts, when the
    Toolpath Drawing menu shows them
setting-2d-view-in-the-dark-162499 = 2D view in the dark theme
setting-the-2d-view-s-colors-4c80a7 = The 2D view's colors while the dark theme is on
setting-grid-point-size-39e980 = Grid point size
setting-how-big-the-snap-grid-018487 = How big the snap grid's points are drawn, in screen points
setting-text-size-041143 = Text size
setting-how-big-text-buttons-and-a5d892 =
    How big text, buttons and lines are, as a percentage: larger is easier to read and to
    click, smaller fits more on a small screen (from the next start)
setting-mouse-and-shortcuts-0e0f82 = Mouse and Shortcuts
setting-the-mouse-in-the-2d-772333 = The mouse in the 2D and 3D views, and the shortcuts set in place of Cut Craft's own.
setting-tilting-the-wheel-left-or-0d309e = Tilting the wheel left or right
setting-what-tilting-the-mouse-wheel-beb2a5 =
    What tilting the mouse wheel left or right does over the 2D and 3D views. Holding
    Shift while turning the wheel, or swiping sideways on a touchpad, does the same
setting-pans-the-view-sideways-970162 = Pans the view sideways
setting-tilting-right-moves-the-view-5d2b23 = Tilting right moves the view to show more to the right, as a web page scrolls
setting-pans-the-other-way-907d9b = Pans the other way
setting-tilting-right-moves-the-drawing-9ca5d9 = Tilting right moves the drawing to the right, for those whose wheel feels backwards
setting-does-nothing-4d67cd = Does nothing
setting-tilting-the-wheel-is-ignored-c925fc = Tilting the wheel is ignored by the views
setting-trackpad-mode-ca82fc = Trackpad mode
setting-trackpad-mode-two-fingers-dragging-f8bbd2 =
    Trackpad mode: two fingers dragging pan the 2D and 3D views, and pinching zooms (in
    place of the wheel). For laptops with no mouse
setting-invert-drag-left-right-3d-280836 = Invert drag left / right (3D view)
setting-turns-the-3d-view-the-23bf8b = Turns the 3D view the other way when dragging left or right
setting-invert-drag-up-down-3d-8087ba = Invert drag up / down (3D view)
setting-tilts-the-3d-view-the-d2dbfc =
    Tilts the 3D view the other way when dragging up or down: a drag down tilts the top
    of the job away from you instead of towards you
setting-invert-wheel-zoom-624855 = Invert wheel zoom
setting-turning-the-wheel-towards-you-02686f = Turning the wheel towards you zooms in instead of out
setting-invert-pinch-zoom-56b634 = Invert pinch zoom
setting-pinching-zooms-in-instead-of-b05807 = Pinching zooms in instead of out on a trackpad or touch screen
setting-invert-two-finger-pan-62aa58 = Invert two-finger pan
setting-in-trackpad-mode-the-view-527a0e = In Trackpad mode, the view moves against your fingers instead of with them
setting-spacemouse-4e8804 = SpaceMouse
setting-a-3dconnexion-spacemouse-turning-the-c69924 = A 3Dconnexion SpaceMouse turning the 3D view (Linux, through spacenavd).
setting-mode-4e7732 = Mode
setting-object-the-cap-moves-the-58ca46 =
    Object: the cap moves the model as if holding it. Camera: it moves the viewpoint, so
    every direction is the other way
setting-object-8e64da = Object
setting-the-cap-moves-the-model-b34d80 = The cap moves the model, as if holding it (the usual CAD way)
setting-camera-4c7b6e = Camera
setting-the-cap-moves-the-viewpoint-45e5ce = The cap moves the viewpoint round the model
setting-speed-6b8020 = Speed
setting-how-fast-the-view-follows-48b67e = How fast the view follows the cap
setting-invert-left-right-8f2f41 = Invert left / right
setting-turn-each-movement-the-other-4cbbb6 = Turn each movement the other way
setting-invert-up-down-14e455 = Invert up / down
setting-invert-zoom-push-pull-033093 = Invert zoom (push / pull)
setting-invert-tilt-8c75e4 = Invert tilt
setting-invert-spin-bcac0f = Invert spin
setting-invert-roll-free-rotation-only-2cb4ad = Invert roll (Free rotation only)
setting-updates-3075a5 = Updates
setting-help-check-for-updates-and-827231 = Help ▸ Check for Updates, and the check at startup.
setting-check-for-updates-when-cut-a5a721 = Check for updates when Cut Craft starts
setting-every-time-cut-craft-starts-933f9f =
    Every time Cut Craft starts, ask GitHub whether there is a new version,
    and say so if there is. Nothing is downloaded or installed until you choose Install Now
setting-bug-reports-1ebb86 = Bug Reports
setting-bug-reports-and-what-happens-5d7ed1 = Bug reports, and what happens after Cut Craft closes because of an error.
setting-if-cut-craft-crashes-62c84b = If Cut Craft crashes
setting-if-cut-craft-closes-because-5b412a =
    If Cut Craft closes because of an error, it keeps a crash log saying what it was
    doing. The next time it starts it can open a bug report with that log, for you to
    read and send. Your user name and home folder are left out of it, and nothing is sent
    until you press Submit.
setting-send-the-crash-log-074c31 = Send the crash log
setting-send-the-crash-log-to-c713f4 =
    Send the crash log to Cut Craft's crash-report service by itself (your user name,
    folders and file names left out). Where that can't be done, open a bug report with it ticked
setting-ask-me-each-time-55bf0e = Ask me each time
setting-open-a-bug-report-and-081306 = Open a bug report, and ask each time whether the crash log goes with it
setting-don-t-send-crash-logs-b641b9 = Don't send crash logs
setting-don-t-open-a-report-1d9346 = Don't open a report: the crash log is kept in Cut Craft's settings folder
setting-send-details-of-my-computer-877051 = Send details of my computer
setting-send-which-system-and-version-053caa =
    Send which system and version this computer runs (Windows 11 23H2, macOS 14 Sonoma,
    Ubuntu 24.04...), its processor (Intel/AMD or ARM) and graphics card with crash and
    bug reports: some bugs happen only on some computers. Never the computer's name or
    your user name. Off: only the report and its log are sent
setting-web-api-782851 = Web API
setting-options-web-api-lets-a-06e152 =
    Options ▸ Web API: lets a script, a test or Claude drive Cut Craft over HTTP, the way you
    do with the mouse. Off until switched on.
setting-turn-on-the-web-api-443912 = Turn on the Web API
setting-let-programs-on-this-computer-f0f347 =
    Let programs on this computer drive Cut Craft: select vectors, run tools, press any
    button. Calls from this computer need nothing more; other computers need the access
    token. The status bar shows WEB API Running in green while it is listening, or in red if
    it could not start and why
setting-allow-other-computers-on-my-c999f1 = Allow other computers on my network
setting-also-answer-other-computers-on-d71347 =
    Also answer other computers on your network, not only this one (listens on 0.0.0.0).
    Every call then needs the access token shown below. Calls travel unencrypted, so only
    use this on a home or workshop network you trust
setting-port-c53cc6 = Port
setting-the-number-programs-call-it-234e18 =
    The number programs call it on, the 7340 in localhost:7340. Pick another if a different
    program already uses this one
setting-advanced-f3de7b = Advanced
setting-settings-that-change-how-hard-fcc0e0 =
    Settings that change how hard Cut Craft works the computer, or how exactly it cuts: listed
    only once Show advanced settings is ticked, and put back to their defaults when the file
    was changed outside Cut Craft.
setting-use-wayland-directly-no-file-3ed07e = Use Wayland directly (no file drag and drop)
setting-on-a-wayland-desktop-open-f4501d =
    On a Wayland desktop, open the window there directly rather than through `XWayland`;
    files cannot be dragged into it then (from the next start). Listed only on Wayland
setting-dxf-curve-detail-points-per-9de5c8 = DXF curve detail (points per span)
setting-how-many-points-each-stretch-c1cd76 =
    How many points each stretch of a curved DXF spline becomes when it is imported. Fewer
    gives flat-sided curves; more gives smoother curves but bigger, slower jobs
setting-library-read-the-folders-again-fb00c7 = Library: read the folders again every
setting-how-often-the-library-reads-314f25 =
    How often the Library reads its folders again to find files added, moved or deleted.
    Sooner shows changes quicker, but keeps a network drive or a slow disk busy
setting-library-most-files-listed-1623e3 = Library: most files listed
setting-the-library-stops-listing-after-37a8c1 =
    The Library stops listing after this many files, so adding a whole drive can't stall
    it. More lists bigger collections, but uses more memory and time on every read
setting-library-keep-unused-previews-for-a92d7f = Library: keep unused previews for
setting-saved-library-previews-not-used-9bcd2b =
    Saved Library previews not used for this long are deleted when Cut Craft starts. Longer
    keeps more of them on disk, so folders open faster (from the next start)
setting-library-previews-kept-on-the-da12f0 = Library: previews kept on the graphics card
setting-how-many-library-previews-are-5db573 =
    How many Library previews are kept on the graphics card at once. More scrolls back
    faster, but uses more video memory, which can run out on built-in graphics (from the
    next start)
setting-curve-accuracy-for-toolpaths-f32c40 = Curve accuracy for toolpaths
setting-how-closely-curves-are-followed-f60769 =
    How closely curves are followed by the straight pieces offsets, pockets and profiles
    are cut from, and by the arcs written to G-code. Coarser shows as flats on curved cuts;
    finer makes toolpaths slower to work out and G-code files bigger
setting-3d-roughing-accuracy-8e81a6 = 3D roughing accuracy
setting-how-far-a-3d-roughing-530c31 =
    How far a 3D roughing move may dip below the machining allowance before it is split.
    Coarser eats into the allowance left for finishing; finer makes bigger files
setting-3d-finishing-accuracy-ea15dc = 3D finishing accuracy
setting-how-far-a-3d-finishing-60ecd6 =
    ⚠ How far a 3D finishing move may dip below the model before it is split. Raising this
    lets the tool cut into the finished part (gouging); lowering it makes much bigger files
setting-rest-machining-threshold-1a9a86 = Rest machining threshold
setting-how-much-material-must-be-439f21 =
    How much material must be left above the model for rest machining to go back for it.
    Lower finds smaller leftovers but may chase marks the last tool left; higher skips them
setting-nesting-most-sheets-added-a7a0e6 = Nesting: most sheets added
setting-the-most-sheets-nest-parts-b4cce9 =
    The most sheets Nest Parts' Add sheets as needed makes in one go. More nests bigger
    batches, but a part that can never fit makes it run much longer before it gives up
setting-nesting-most-filler-parts-dbd470 = Nesting: most filler parts
setting-the-most-filler-parts-one-1b8804 =
    The most filler parts one nesting places. More fills big sheets with small parts, but
    slows nesting down
setting-gadgets-most-work-million-steps-ee0cea = Gadgets: most work (million steps)
setting-how-much-work-a-gadget-5d3750 =
    How much work a gadget may do, in millions of Lua steps, before it is stopped as
    stuck. Higher lets big gadgets finish; a gadget caught in a loop then freezes Cut Craft
    for longer before it is stopped
setting-gadgets-most-memory-585836 = Gadgets: most memory
setting-how-much-memory-a-gadget-c8fec0 =
    How much memory a gadget may use before it is stopped. Higher lets big gadgets finish,
    but a runaway one can take that much from the computer
setting-gadgets-most-commands-in-one-13992b = Gadgets: most commands in one run
setting-how-many-commands-one-gadget-995cfb =
    How many commands one gadget run may do. Higher lets a gadget make more shapes in one
    go, but each one adds to the job's memory and to the time Undo takes

## Snapping Options
setting-snap-to-grid-19a2ce = Snap to grid
setting-grid-spacing-cf8fb8 = Grid spacing
setting-snap-to-job-center-and-e0c545 = Snap to job center and corners
setting-geometry-snapping-e43f17 = Geometry snapping
setting-snap-to-points-on-the-38aea6 = Snap to points on the vectors (the next five)
setting-object-centers-e777f0 = Object centers
setting-the-middle-of-each-shape-abce47 = The middle of each shape's outline box
setting-end-points-b2e419 = End points
setting-where-each-line-or-curve-de9e2c = Where each line or curve starts and ends (a square marker)
setting-middle-points-7aca57 = Middle points
setting-halfway-along-each-line-or-84218b = Halfway along each line or curve (a triangle marker)
setting-arc-centers-1d3579 = Arc centers
setting-the-centre-of-each-circle-7a0562 = The centre of each circle and arc (a plus marker)
setting-tangent-and-perpendicular-49410b = Tangent and perpendicular
setting-while-drawing-a-line-where-c8fe7c =
    While drawing a line, where it would just touch a circle or curve without crossing it
    (Tangent, a circle marker), or meet a line or curve at a right angle (Perpendicular,
    a right-angle marker)
setting-smart-snapping-a7d91a = Smart snapping
setting-line-the-pointer-up-with-d8ff4c = Line the pointer up with the snap points, across or up and down, when none is near
setting-snapping-radius-adaaa1 = Snapping radius
setting-how-near-in-screen-pixels-4712d5 = How near, in screen pixels, the pointer must come to a point to snap to it
setting-fixed-nudge-distance-4cd955 = Fixed nudge distance
setting-how-far-ctrl-and-an-307df3 = How far Ctrl and an arrow key move the selection
setting-rotate-snap-angle-with-shift-cd3997 = Rotate snap angle with Shift
setting-the-angle-a-rotate-drag-761f96 =
    The angle a rotate drag snaps to while Shift is held: a half of a half of a turn and
    so on down to 22.5°, or every 10° or 5° for finer work
setting-90-001ece = 90°
setting-quarter-turns-c564cb = Quarter turns
setting-45-1273f0 = 45°
setting-eighths-of-a-turn-d5f210 = Eighths of a turn
setting-22-5-cfbf2e = 22.5°
setting-sixteenths-of-a-turn-6e1e2d = Sixteenths of a turn
setting-10-c337e6 = 10°
setting-every-10-2355df = Every 10°
setting-5-9b1e12 = 5°
setting-every-5-243951 = Every 5°
setting-rotate-snap-angle-with-ctrl-006914 = Rotate snap angle with Ctrl
setting-the-finer-angle-a-rotate-2896e6 = The finer angle a rotate drag snaps to while Ctrl is held

## Job Setup Sheet
setting-the-job-setup-sheet-editor-9a4a12 = The Job Setup Sheet Editor's settings: how every job sheet looks.
setting-title-d471a9 = Title
setting-header-color-31daf7 = Header color
setting-footer-color-672bd9 = Footer color
setting-job-layout-05ecba = Job Layout
setting-job-notes-ad5a49 = Job Notes
setting-material-setup-cf031d = Material Setup
setting-toolpaths-summary-6f3ad5 = Toolpaths Summary
setting-individual-toolpath-summaries-d2b4f7 = Individual Toolpath Summaries

## Part Labels
setting-how-the-labels-look-kept-edddcd = How the labels look, kept with the options.
setting-label-size-23719e = Label size
setting-pick-the-size-of-the-17e44f = Pick the size of the labels in your printer; plain paper works on any printer
setting-plain-paper-cut-out-with-09f9d3 = Plain paper (cut out with scissors)
setting-labels-in-a-grid-on-455733 = Labels in a grid on Letter or A4 paper, with dashed lines to cut along
setting-4-6-in-shipping-label-5ffb2f = 4 × 6 in (shipping label printer)
setting-4-3-in-c8724d = 4 × 3 in
setting-4-2-in-7c9618 = 4 × 2 in
setting-3-2-in-c66095 = 3 × 2 in
setting-2-25-1-25-in-e393f7 = 2.25 × 1.25 in (Dymo, Zebra)
setting-100-50-mm-233e07 = 100 × 50 mm
setting-62-29-mm-brother-563461 = 62 × 29 mm (Brother)
setting-qr-code-6c61c3 = QR code
setting-a-square-code-a-phone-6ff2a5 = A square code a phone camera reads: it holds the same words as the label
setting-part-size-0dd0d5 = Part size
setting-sheet-name-a2871b = Sheet name
setting-which-sheet-of-material-the-4fbca1 = Which sheet of material the part is cut from
setting-job-name-6157cf = Job name
setting-the-job-s-file-name-cce354 = The job's file name on every label

## Export as STL
setting-export-model-as-stl-s-d0a814 = Export Model as STL's settings.
setting-limit-triangles-by-73c9ce = Limit triangles by
setting-how-the-detail-of-the-d18edc = How the detail of the mesh is limited.
setting-approximate-tolerance-97283a = Approximate tolerance
setting-no-point-of-the-mesh-763ef2 = No point of the mesh further than the tolerance from the model
setting-maximum-number-of-triangles-c87b42 = Maximum number of triangles
setting-no-more-than-this-many-b97198 = No more than this many triangles
setting-the-furthest-the-mesh-may-801f91 = The furthest the mesh may stray from the model
setting-back-face-of-triangulation-876227 = Back face of triangulation
setting-what-closes-the-back-of-f04f99 = What closes the back of the mesh.
setting-leave-open-no-back-face-31a14a = Leave open (no back face)
setting-none-an-open-shell-the-8d9e55 = None: an open shell, the model's face only
setting-close-with-inverted-front-a32b95 = Close with inverted front
setting-a-copy-of-the-front-d4206d = A copy of the front, mirrored onto the back of the material (a solid two-sided part)
setting-close-with-a-flat-plane-858d1b = Close with a flat plane
setting-the-material-s-flat-bottom-469dd5 = The material's flat bottom
setting-close-with-other-side-double-b85b88 = Close with other side (double-sided jobs)
setting-a-double-sided-job-s-1ee3d5 = A double-sided job's other side, turned over as the job turns

## Draw Arc (draw_arc)
setting-center-9fbea4 = Center
setting-a-position-in-millimetres-72d1ab = A position in millimetres.
setting-x-0c1e27 = X
setting-left-to-right-along-the-cae1c3 = Left to right, along the job's X axis
setting-y-0c1c94 = Y
setting-bottom-to-top-along-the-88af5d = Bottom to top, along the job's Y axis
setting-radius-8111d3 = Radius
setting-from-the-centre-to-the-9a603e = From the centre to the curve
setting-start-angle-e86330 = Start angle
setting-measured-counter-clockwise-from-x-e93a91 = Measured counter-clockwise from +X
setting-end-angle-af4685 = End angle
setting-where-the-arc-stops-measured-fd0f81 =
    Where the arc stops, measured counter-clockwise from the right (0° is to the right, 90°
    straight up)

## Draw Circle (draw_circle)
setting-center-point-475ac2 = Center Point
setting-from-the-centre-to-the-ff9a6f = From the centre to the edge: half the width across

## Draw Ellipse (draw_ellipse)
setting-width-42dfbf = Width
setting-how-wide-the-ellipse-is-83b5c0 = How wide the ellipse is from side to side
setting-height-d13562 = Height
setting-how-tall-the-ellipse-is-50d2b6 = How tall the ellipse is from bottom to top

## Draw Rectangle (draw_rectangle)
setting-where-the-left-edge-sits-32c62a = Where the left edge sits, along the job's X axis
setting-where-the-bottom-edge-sits-78c7ed = Where the bottom edge sits, along the job's Y axis
setting-width-x-938e24 = Width (X)
setting-how-wide-it-is-from-710c18 = How wide it is from side to side
setting-height-y-2e6b40 = Height (Y)
setting-how-tall-it-is-from-ab6bfe = How tall it is from bottom to top
setting-corner-type-38ffd8 = Corner Type
setting-one-of-square-rounded-or-84700b = One of square, rounded or notched corners, as the picture shows
setting-square-c98c66 = Square
setting-plain-sharp-corners-592f19 = Plain sharp corners
setting-rounded-26825a = Rounded
setting-each-corner-rounded-off-outwards-cfedd8 = Each corner rounded off outwards, by the radius
setting-notched-4ee010 = Notched
setting-each-corner-cut-inwards-in-21ee05 = Each corner cut inwards in a quarter circle, by the radius
setting-how-big-each-rounded-or-be84a7 = How big each rounded or notched corner is, from the corner's centre to its edge

## Draw Polygon (draw_polygon)
setting-centre-to-vertex-21d427 = Centre to vertex
setting-sides-d0fd77 = Sides
setting-how-many-straight-sides-it-8dc210 = How many straight sides it has: 3 for a triangle, 6 for a hexagon
setting-turns-the-shape-counter-clockwise-5a73c8 = Turns the shape counter-clockwise by this angle; at 0 a corner points straight up

## Draw Line / Polyline (draw_polyline)
setting-points-f0a4e6 = Points
setting-the-corners-the-line-passes-f8a8f8 =
    The corners the line passes through, in order: click them in the 2D view, or type them
    here
setting-closed-01cae5 = Closed
setting-join-the-last-point-back-02f8f1 = Join the last point back to the first

## Draw Star (draw_star)
setting-outer-radius-274142 = Outer radius
setting-centre-to-point-tip-186aec = Centre to point tip
setting-inner-radius-df3a93 = Inner radius
setting-centre-to-the-notch-between-fe5671 = Centre to the notch between points
setting-how-many-points-the-star-d8768c = How many points the star has
setting-turns-the-star-counter-clockwise-656ce9 = Turns the star counter-clockwise by this angle; at 0 a point faces straight up

## Mirror (mirror)
setting-flip-036886 = Flip
setting-which-way-the-selection-is-adec60 = Which way the selection is flipped
setting-horizontal-f39489 = Horizontal
setting-left-to-right-8c072f = Left to right
setting-vertical-fb2d5b = Vertical
setting-top-to-bottom-509d98 = Top to bottom
setting-both-6318c2 = Both
setting-left-to-right-and-top-377b23 = Left to right and top to bottom at once
setting-about-e096de = About
setting-what-the-selection-is-mirrored-eef649 = What the selection is mirrored about.
setting-selection-ea34b5 = Selection
setting-flipped-in-place-about-the-482d4c = Flipped in place, about the selection's own centre
setting-job-centre-a1e897 = Job centre
setting-about-the-job-s-centre-880c14 = About the job's centre lines
setting-left-773890 = Left
setting-beside-the-selection-to-the-7e1de4 = Beside the selection, to the left: about its left edge
setting-right-9e9f85 = Right
setting-beside-the-selection-to-the-0f50f2 = Beside the selection, to the right: about its right edge
setting-top-9b73dc = Top
setting-beside-the-selection-above-about-ec7338 = Beside the selection, above: about its top edge
setting-bottom-b5f34a = Bottom
setting-beside-the-selection-below-about-21d5ec = Beside the selection, below: about its bottom edge
setting-line-08f547 = Line
setting-about-the-line-between-the-088e56 = About the line between the two ends of the vector picked last (which stays put)
setting-create-mirrored-copy-da26ae = Create mirrored copy
setting-keep-the-originals-and-add-b68cd8 = Keep the originals and add the mirrored ones as copies

## Move (move)
setting-relative-9d3f3d = Relative
setting-move-by-x-y-instead-2adf8d = Move by X/Y instead of to X/Y
setting-where-to-put-the-anchor-4f5a7b =
    Where to put the anchor point along X, or with Relative ticked, how far to move left
    (negative) or right
setting-where-to-put-the-anchor-0f5cb6 =
    Where to put the anchor point along Y, or with Relative ticked, how far to move down
    (negative) or up
setting-anchor-4055d4 = Anchor
setting-point-of-the-selection-placed-8e3f46 = Point of the selection placed at X/Y; shown with Relative not ticked
setting-top-left-146935 = Top left
setting-the-top-left-corner-of-bcd2b0 = The top left corner of the selection's box
setting-the-middle-of-the-box-cc9002 = The middle of the box's top edge
setting-top-right-10974e = Top right
setting-the-top-right-corner-of-848c39 = The top right corner of the selection's box
setting-the-middle-of-the-box-5ee16c = The middle of the box's left edge
setting-the-middle-of-the-selection-0d341c = The middle of the selection's box
setting-the-middle-of-the-box-5d342d = The middle of the box's right edge
setting-bottom-left-26220b = Bottom left
setting-the-bottom-left-corner-of-2b1318 = The bottom left corner of the selection's box
setting-the-middle-of-the-box-10d306 = The middle of the box's bottom edge
setting-bottom-right-3d4c14 = Bottom right
setting-the-bottom-right-corner-of-23e5c1 = The bottom right corner of the selection's box
setting-relative-rotated-251840 = Relative rotated
setting-relative-moves-the-x-y-8438db = Relative moves: the X/Y offset turned by the angle below, so X runs along it
setting-angle-c493b8 = Angle
setting-the-angle-the-offset-is-618e18 =
    The angle the offset is turned by, anticlockwise from the X axis; shown with Relative
    and Relative rotated ticked

## Rotate (rotate)
setting-how-far-to-turn-counter-0dc3e3 = How far to turn, counter-clockwise; a negative angle turns clockwise
setting-point-of-the-selection-to-07938a = Point of the selection to rotate about

## Set Size (set_size)
setting-what-is-sized-19d220 = What is sized.
setting-scale-selection-565027 = Scale selection
setting-the-whole-selection-as-one-b54312 = The whole selection, as one group
setting-scale-items-individually-34f1cf = Scale items individually
setting-each-selected-item-on-its-75338a =
    Each selected item on its own, as if it alone were selected. A group, like a word of
    text, is one item, and so is a shape with the holes and shapes drawn inside it
setting-the-width-or-with-percent-f37937 = The width, or with Percent, the share of the present width
setting-the-height-or-with-percent-4edea0 =
    The height, or with Percent, the share of the present height. With Link XY, typing
    it works out the width to match; items one by one each take this height
setting-percent-1537fa = Percent
setting-width-and-height-are-percentages-a0885c = Width and height are percentages of the present size, not millimetres
setting-link-xy-b950d0 = Link XY
setting-type-the-width-or-the-4cb99c =
    Type the width or the height and the other is worked out, so the shape is not
    stretched. Items one by one each keep their own proportions, sized by the box typed
    last
setting-point-of-the-selection-that-ecb7bb = Point of the selection that stays put

## Job Setup (job_setup)
setting-job-type-4dab5c = Job Type
setting-cut-from-one-face-both-3cb3e9 = Cut from one face, both faces, or round a cylinder
setting-single-sided-829a3c = Single Sided
setting-cut-from-the-top-face-d4fb54 = Cut from the top face only
setting-double-sided-a72f55 = Double Sided
setting-both-faces-machined-the-material-5c196f = Both faces machined, the material turned over between them
setting-rotary-178e4c = Rotary
setting-a-cylinder-turned-by-a-7844ac = A cylinder turned by a rotary axis
setting-job-size-8eb0ad = Job Size
setting-the-size-of-the-material-d5c303 = The size of the material you are cutting
setting-the-material-s-size-across-777ceb = The material's size across, left to right
setting-the-material-s-size-up-8c6398 = The material's size up the job, front to back
setting-thickness-z-1135cc = Thickness (Z)
setting-how-thick-the-material-is-11cbfa = How thick the material is. Changing it after making toolpaths marks them out of date
setting-length-l-141806 = Length (L)
setting-rotary-jobs-the-cylinder-s-02d887 = Rotary jobs: the cylinder's length along its axis
setting-shape-80ddc6 = Shape
setting-rotary-jobs-the-material-s-3742ae =
    Rotary jobs: the material's shape across its axis. A square or a board swings out
    further than its top as it turns, by half its diagonal: the bit lifts that much
    higher before A turns, and the checks before cutting see that it clears the bed
setting-round-c753db = Round
setting-a-round-blank-a-dowel-0f6c10 = A round blank: a dowel, a turned spindle or a log. Diameter (D) is across it
setting-a-square-blank-a-face-8f73d2 = A square blank, a face flat on top at A=0. Diameter (D) is its side
setting-board-8f13cb = Board
setting-a-flat-board-lying-flat-d94832 =
    A flat board, lying flat at A=0, for carving both faces with A turning it over.
    Diameter (D) is its thickness, and Board Width (W) how wide it is
setting-diameter-d-2e76d1 = Diameter (D)
setting-rotary-jobs-the-material-s-3fa67c =
    Rotary jobs: the material's size straight up through the axis at A=0: a round
    blank's diameter, a square's side, a board's thickness
setting-side-d-e758ad = Side (D)
setting-thickness-d-c6f6ae = Thickness (D)
setting-board-width-w-a8e2e7 = Board Width (W)
setting-rotary-jobs-on-a-board-4f69bc = Rotary jobs on a board: how wide the board is, across the axis, lying flat at A=0
setting-units-d4163c = Units
setting-lengths-are-shown-in-these-c787b2 = Lengths are shown in these; the job is always stored in millimetres
setting-mm-2e1e0f = mm
setting-lengths-shown-in-millimetres-f12c0f = Lengths shown in millimetres
setting-inches-e53ee1 = inches
setting-lengths-shown-in-inches-873dde = Lengths shown in inches
setting-z-zero-position-8cbfb8 = Z Zero Position
setting-where-the-tool-is-zeroed-4e4a4a = Where the tool is zeroed in Z: on top of the material, or under it on the machine bed
setting-material-surface-d89d4b = Material Surface
setting-the-tool-is-zeroed-on-41e6f5 = The tool is zeroed on the top of the material
setting-machine-bed-5b6279 = Machine Bed
setting-the-tool-is-zeroed-under-e18095 =
    The tool is zeroed under the material: on the machine bed, or on a rotary job's
    cylinder axis
setting-rotary-jobs-where-the-tool-5efbb4 =
    Rotary jobs: where the tool is zeroed in Z, on top of the cylinder or on its axis. The
    picture shows where. Pick the one you touch the bit off on at the machine: the G-code
    heights depend on it, and a wrong pick cuts too deep or in the air
setting-material-top-3624bb = Material Top
setting-the-tool-is-zeroed-on-14eda2 =
    The tool is zeroed on the top of the material at A=0: touch the bit off on the
    highest point of a round blank, or the middle of a square's or board's top face. Cut
    depths go below zero, so a 3 mm deep cut is Z -3. Easiest when the blank is true; set
    zero again for each new blank
setting-turning-axis-0e5937 = Turning Axis
setting-the-tool-is-zeroed-on-686133 =
    The tool is zeroed on the turning axis, the centre line the rotary spins the material
    round (touch off on the axis, not on the machine bed or the rotary's base). Every Z is
    a distance from the axis, so on a 50 mm blank a 3 mm deep cut is Z 22. Zero stays put
    when you change blanks, and an out-of-round blank still cuts to the right finished
    size
setting-xy-datum-position-70d4bc = XY Datum Position
setting-which-point-of-the-material-426631 =
    Which point of the material is the machine's X0, Y0. Changing it moves only the
    coordinates: the design, toolpaths, pictures, clamp zones and guides stay where they
    are on the material
setting-position-7f342a = Position
setting-the-point-of-the-material-3b3ff7 = The point of the material at the machine's X0, Y0
setting-the-material-s-bottom-left-935074 = The material's bottom left corner
setting-the-material-s-top-left-59b826 = The material's top left corner
setting-the-material-s-top-right-273ceb = The material's top right corner
setting-the-material-s-bottom-right-cfa421 = The material's bottom right corner
setting-centre-cee8e6 = Centre
setting-the-middle-of-the-material-9cec9a = The middle of the material
setting-use-offset-10c527 = Use offset
setting-give-that-point-coordinates-other-dde2d3 = Give that point coordinates other than 0, 0
setting-the-x-the-datum-point-39b5dd = The X the datum point is given, with Use offset ticked
setting-the-y-the-datum-point-58c4f8 = The Y the datum point is given, with Use offset ticked
setting-orientation-51d9c1 = Orientation
setting-rotary-jobs-the-direction-the-9648bc = Rotary jobs: the direction the cylinder's axis runs
setting-along-x-axis-b10ccb = Along X Axis
setting-the-cylinder-lies-left-to-8f7959 = The cylinder lies left to right
setting-along-y-axis-65092a = Along Y Axis
setting-the-cylinder-lies-front-to-adac86 = The cylinder lies front to back
setting-holding-the-part-04c1e8 = Holding the Part
setting-rotary-jobs-what-of-the-ab4c52 =
    Rotary jobs: what of the material the rotary toolpaths leave to hold the part while
    it is cut
setting-keep-joined-at-44d7de = Keep Joined At
setting-where-the-part-stays-joined-3f3bf4 =
    Where the part stays joined to the material: past the model's end there the material
    is left whole, so that end of the model is a flat face to saw it free on. Neither
    cuts the material away past both ends
setting-neither-862b48 = Neither
setting-neither-end-the-material-past-f66961 = Neither end: the material past the model is cut away at both ends
setting-chuck-end-55861a = Chuck end
setting-the-chuck-end-the-job-4f0e34 = The chuck end (the job's start)
setting-tailstock-end-c4a352 = Tailstock end
setting-the-tailstock-end-the-job-d05163 = The tailstock end (the job's end)
setting-both-ends-the-material-is-0c2140 =
    Both ends: the material is left whole past the model at the chuck and the
    tailstock, and the part is sawn free at both
setting-centre-stock-diameter-7a63ef = Centre Stock Diameter
setting-leave-a-round-beam-of-500659 =
    Leave a round beam of material this far across along the turning axis, from the
    chuck to the tailstock, through the model and past it: the rotary toolpaths cut round
    it, so a long or thin part is held at both ends. 0 leaves none. Needs the material
    turning true on the axis (centred in the chuck). A last toolpath with Cut Away the
    Support ticked takes it off
setting-flip-direction-between-sides-f4868f = Flip Direction Between Sides
setting-double-sided-jobs-how-the-896c20 = Double-sided jobs: how the material is turned over between sides
setting-how-the-material-is-turned-e68463 = How the material is turned over to reach the other side
setting-turned-over-left-to-right-c5e32b = Turned over left to right
setting-turned-over-top-to-bottom-ba331c = Turned over top to bottom
setting-zero-off-same-side-fec9a7 = Zero off same side
setting-z-zero-is-on-the-03f072 = Z zero is on the same physical face whichever side is up
setting-modeling-resolution-763348 = Modeling Resolution
setting-how-finely-3d-shapes-are-5d2094 = How finely 3D shapes are worked out over the job; Standard suits most parts
setting-standard-fastest-a1f8fd = Standard (fastest)
setting-1-million-points-56cae7 = 1 million points
setting-high-3x-slower-1ce3db = High (3x slower)
setting-3-million-points-5604d5 = 3 million points
setting-highest-7x-slower-33c47d = Highest (7x slower)
setting-7-million-points-1bb111 = 7 million points
setting-material-d54f80 = Material
setting-what-the-material-is-from-fa609c =
    What the material is, from the Materials Library: toolpaths pick their bits' feeds
    and speeds for it (a layer naming its own material overrides it), and the 3D view
    shows it in the matching colour from Toolpaths ▸ Preview Colors. Left empty, each
    bit's own feeds
setting-material-settings-a6656d = Material Settings
setting-the-material-s-colour-in-3ac8c2 =
    The material's colour in the 3D view. How the preview colours what the toolpaths cut
    is Toolpaths ▸ Preview Colors, which this leaves as it is
setting-rapid-z-gaps-above-material-0298c2 = Rapid Z Gaps above Material
setting-how-high-the-tool-travels-16ad0c = How high the tool travels between cuts
setting-clearance-z1-4d4017 = Clearance (Z1)
setting-safe-height-above-the-material-6cc771 = Safe height above the material for rapid moves
setting-plunge-z2-4532b5 = Plunge (Z2)
setting-plunges-go-down-to-this-90e377 =
    Plunges go down to this gap above the material at the rapid rate, then at the plunge
    rate; the same as the clearance means no rapid plunges
setting-home-start-position-6dddba = Home / Start Position
setting-where-the-tool-starts-from-7a3c68 = Where the tool starts from and goes back to at the end
setting-where-the-tool-starts-and-51bc74 = Where the tool starts and ends, across the job
setting-where-the-tool-starts-and-1586e8 = Where the tool starts and ends, up the job
setting-z-gap-above-material-e80c69 = Z Gap above Material
setting-how-high-above-the-material-c9cf30 =
    How high above the material it waits. Left at the clearance, it follows Clearance
    (Z1) when that is changed, so the start and end moves never travel lower

## Offset Vectors (offset_vectors)
setting-distance-d96082 = Distance
setting-outwards-when-positive-inwards-when-8cb72a = Outwards when positive, inwards when negative
setting-corners-5b05b9 = Corners
setting-whether-the-offset-s-outside-0077ba = Whether the offset's outside corners are round or sharp
setting-corners-curve-round-like-the-c3ea4e = Corners curve round, like the path of a round bit
setting-sharp-858ee7 = Sharp
setting-corners-stay-pointed-the-sides-8c2e4e = Corners stay pointed, the sides carried on until they meet
setting-keep-original-873197 = Keep original
setting-keep-the-original-vectors-as-9f5c85 = Keep the original vectors as well as the offset ones

## Join Vectors (join_vectors)
setting-tolerance-a04ac8 = Tolerance
setting-ends-closer-than-this-are-d1f0e5 = Ends closer than this are joined

## Profile Toolpath (profile_toolpath)
setting-cut-depth-c9349e = Cut Depth
setting-below-the-material-surface-bc4879 = Below the material surface
setting-cutting-depths-16d99f = Cutting Depths
setting-tool-a15ae7 = Tool
setting-the-bit-that-cuts-round-6e9d73 = The bit that cuts round the shapes; Select… picks one from your tools
setting-name-e07306 = Name
setting-end-mill-1e8e98 = End mill
setting-flat-bottomed-31fddc = Flat bottomed
setting-ball-nose-01dc97 = Ball nose
setting-round-bottomed-for-3d-finishing-20bde8 = Round bottomed, for 3D finishing
setting-v-bit-c4c256 = V bit
setting-pointed-for-v-carving-0937b8 = Pointed, for V-carving
setting-radiused-end-mill-58395f = Radiused End Mill
setting-flat-bottomed-with-rounded-corners-4a6559 =
    Flat bottomed with rounded corners (a bull nose, or toroidal cutter): stronger than an
    end mill, leaves less cusp than one when 3D roughing or finishing
setting-thread-mill-55e859 = Thread Mill
setting-a-single-point-thread-mill-4591a0 =
    A single-point thread mill: a triangular tooth standing out from a narrower shank,
    for the Thread Milling toolpath
setting-form-tool-12c5e7 = Form Tool
setting-cuts-the-profile-it-was-ae6248 =
    Cuts the profile it was given: half its section, drawn as a vector (a moulding or
    router bit's shape)
setting-surfacing-bit-a58de8 = Surfacing Bit
setting-a-wide-flat-bit-for-1c4fa5 =
    A wide, flat bit for flattening a spoilboard or a slab (a spoilboard bit or fly
    cutter): cuts like an end mill, but only sideways, never plunging into the material
setting-laser-67e32c = Laser
setting-a-laser-for-the-laser-8d2bc5 =
    A laser, for the laser toolpaths: its diameter is the kerf (the width it burns), its
    feed rate the speed it moves at. It has no pass depth or flutes: Laser Cut / Fill
    sets how many passes it makes
setting-tip-radius-f2930c = Tip Radius
setting-radius-of-a-radiused-end-8f2eb2 = Radius of a radiused end mill's rounded corners
setting-included-angle-of-a-v-b1402c = Included angle of a V-bit's point
setting-diameter-4a86da = Diameter
setting-pass-depth-90da0d = Pass depth
setting-depth-cut-on-each-pass-5456fd = Depth cut on each pass
setting-stepover-dfe675 = Stepover
setting-sideways-step-between-clearing-roughing-396d90 =
    Sideways step between clearing (roughing) passes, as a fraction of the diameter:
    pockets, flat bottoms, inlay clearing and 3D roughing step over by this
setting-feed-rate-2a2f05 = Feed rate
setting-plunge-rate-a2102c = Plunge rate
setting-spindle-speed-338dbf = Spindle speed
setting-taper-angle-05ac06 = Taper Angle
setting-side-angle-of-a-tapered-64f916 =
    Side angle of a tapered ball nose or tapered flat bottom, each side of the axis; 0
    for a straight tool. The taper widens to the shank, and toolpaths keep all of it off
    the model (added with tapers).
setting-shank-diameter-565b71 = Shank diameter
setting-where-a-tapered-tool-s-cebc10 = Where a tapered tool's taper ends (added with tapers; 0 if not given)
setting-flute-length-0ba9cb = Flute length
setting-how-deep-the-tool-can-a0cc42 =
    How deep the tool can cut; toolpaths going deeper are flagged (added with tapers;
    0 if not given)
setting-number-of-flutes-a114d4 = Number of Flutes
setting-cutting-edges-for-the-chip-babe5e = Cutting edges, for the chip load (added after the first tool format)
setting-cut-e25863 = Cut
setting-outside-to-cut-a-part-52efb9 = Outside to cut a part out, Inside to cut a hole, On the Line to follow the line itself
setting-which-side-of-the-line-cb5d45 = Which Side of the Line
setting-outside-7f8b64 = Outside
setting-cut-a-part-out-the-00714f =
    Cut a part out: the tool goes round the outside of each shape, so the part is the size
    drawn
setting-inside-5d558d = Inside
setting-cut-a-hole-the-tool-086726 =
    Cut a hole: the tool goes round the inside of each shape, so the hole is the size
    drawn
setting-on-the-line-f5b32b = On the Line
setting-the-tool-s-middle-follows-1f45c9 =
    The tool's middle follows the line itself, open lines too, so the cut is as wide as the
    tool
setting-allowance-offset-923b74 = Allowance Offset
setting-positive-leaves-the-part-bigger-128788 = Positive leaves the part bigger (or the hole smaller), negative cuts into it
setting-machine-vectors-101156 = Machine Vectors…
setting-add-tabs-to-toolpath-fa593e = Add tabs to toolpath
setting-bridges-left-to-hold-the-377a08 = Bridges left to hold the part in place
setting-tabs-c4824d = Tabs
setting-no-tabs-the-part-comes-119090 = No tabs: the part comes loose when cut through
setting-2d-tabs-a75ac9 = 2D Tabs
setting-up-at-the-start-of-3aca35 = Up at the start of each tab, across, and down at its end
setting-3d-tabs-63a8e2 = 3D Tabs
setting-up-to-the-tab-s-e93560 = Up to the tab's thickness at its middle and down the other side, without stopping
setting-lead-in-out-208d8f = Lead in / out
setting-how-the-tool-comes-onto-402c40 =
    How the tool comes onto each path, from the waste side; a ramp then runs along the
    lead in, and there is no spiral
setting-leads-520074 = Leads
setting-straight-down-onto-the-path-e5fbfb = Straight down onto the path
setting-straight-0ff167 = Straight
setting-along-a-straight-line-meeting-4d82ab = Along a straight line meeting the path at the angle
setting-circular-783800 = Circular
setting-along-an-arc-that-meets-45a2ae = Along an arc that meets the path along it
setting-the-size-and-shape-of-692a41 = The size and shape of the lead in and out.
setting-a-straight-lead-s-angle-9401ac = A straight lead's angle to the path
setting-a-circular-lead-s-radius-0afca9 = A circular lead's radius; its sweep is the length over the radius, at most 90°
setting-lead-length-47da7d = Lead Length
setting-how-long-the-lead-in-892a2d = How long the lead in is
setting-lead-out-0b0fd7 = Lead out
setting-leave-the-path-the-same-84197d = Leave the path the same way at the end
setting-overcut-distance-b0d008 = Overcut Distance
setting-carry-on-past-the-start-4b39c9 = Carry on past the start of a closed path before leaving it
setting-order-ways-to-try-17d977 = Order ways to try
setting-the-ways-of-ordering-the-3287fd =
    The ways of ordering the cuts to try: the one with the shortest moves between cuts is used.
    Shapes inside others are always cut first, so a part stays held as long as it can.
setting-order-e78237 = Order
setting-vector-selection-order-4af638 = Vector selection order
setting-in-the-order-the-vectors-1b41b4 = In the order the vectors were selected
setting-from-the-parts-on-the-031cb8 = From the parts on the left across to the right
setting-bottom-to-top-109aa8 = Bottom to top
setting-from-the-parts-at-the-3f7ab7 = From the parts at the bottom up to the top
setting-grid-b366b1 = Grid
setting-square-by-square-over-a-9267a7 = Square by square over a grid the size of the biggest part
setting-shortest-path-925c66 = Shortest path
setting-on-each-time-to-the-9ac0a0 = On each time to the nearest shape not yet cut
setting-start-at-ce42d0 = Start at
setting-where-the-cut-round-each-989560 = Where the cut round each closed shape starts.
setting-start-at-7e90f0 = Start At
setting-keep-current-start-points-e39eb4 = Keep Current Start Points
setting-at-each-vector-s-own-5f9e32 = At each vector's own start point (Make Start Point moves it)
setting-optimize-start-points-d36981 = Optimize Start Points
setting-wherever-makes-the-moves-between-e5e4ba = Wherever makes the moves between cuts shortest
setting-closest-on-bounding-box-8930be = Closest on Bounding Box
setting-at-the-corner-of-the-727e9e = At the corner of the path nearest a point on each vector's bounding box
setting-start-near-219cc7 = Start near
setting-the-point-of-each-vector-7fa94b = The point of each vector's bounding box its cut starts nearest
setting-sharp-external-corners-d3bddc = Sharp External Corners
setting-at-outside-corners-the-tool-7596df =
    At outside corners the tool follows the vector's angle out to a point instead of
    rolling round the corner
setting-project-toolpath-onto-3d-model-9b4575 = Project toolpath onto 3D Model
setting-drop-the-finished-toolpath-onto-158a5f =
    Drop the finished toolpath onto the 3D model: its depth below the material surface
    becomes depth below the model's surface, going no lower than the bottom of the material
setting-what-the-toolpath-is-called-2e1d63 = What the toolpath is called in the toolpath list

## Pocket Toolpath (pocket_toolpath)
setting-the-bit-that-cuts-the-edfb33 = The bit that cuts the walls and finishes the pocket; Select… picks one from your tools
setting-clearance-tools-22af3e = Clearance Tools
setting-bigger-tools-that-clear-the-2bfff2 =
    Bigger tools that clear the pocket first, largest first: each makes a toolpath of its
    own and each tool after it cuts only what the ones before it left (rest machining)
setting-strategy-125944 = Strategy
setting-the-pattern-that-clears-the-20d0a8 = The pattern that clears the inside of the pocket
setting-clear-pocket-9c0b8c = Clear Pocket
setting-offset-95c7ea = Offset
setting-rings-stepping-in-from-the-e61b72 = Rings stepping in from the outline
setting-raster-a88b88 = Raster
setting-straight-lines-across-at-the-4d2b1e =
    Straight lines across, at the raster angle, back and forth: each line steps over to
    the next along the wall without lifting
setting-adaptive-9bafbb = Adaptive
setting-keeps-the-bit-taking-a-18e375 =
    Keeps the bit taking a thin, steady bite everywhere, never cutting full width in
    a corner: gentler on the bit and machine, so it can go a full pass deep with confidence
setting-pocket-allowance-270515 = Pocket Allowance
setting-left-on-the-walls-negative-dc4c73 = Left on the walls; negative cuts past the vectors
setting-raster-angle-e45d7b = Raster angle
setting-0-along-x-90-along-c7da79 = 0° along X, 90° along Y
setting-profile-pass-94a39b = Profile pass
setting-whether-a-pass-round-the-eac59a =
    Whether a pass round the edge cleans up the wall, and when (First is offered for
    Raster only: adaptive clearing always goes round last, if at all; offset rings end on
    the edge anyway)
setting-first-3d49e1 = First
setting-round-the-edge-before-the-6453ff =
    Round the edge before the raster lines (Raster only: adaptive clearing always goes
    round last)
setting-last-0e2439 = Last
setting-round-the-edge-after-the-d55617 = Round the edge after the clearing
setting-no-pass-round-the-edge-9b666a = No pass round the edge: it is left rough
setting-leave-for-edge-pass-afa3b5 = Leave for Edge Pass
setting-how-much-the-clearing-leaves-a08386 =
    How much the clearing leaves on the walls for the pass round the edge to take off in
    one thin, even skim, for a straighter, cleaner wall; 0 clears right up to it
setting-edge-pass-at-full-depth-541d42 = Edge Pass at Full Depth
setting-go-round-the-edge-once-d93fb8 =
    Go round the edge once, at the full depth, after all the clearing passes: one clean
    wall with no lines between the passes. Only where the bit's flutes reach that deep;
    otherwise it goes round at each pass

## Drilling Toolpath (drilling_toolpath)
setting-peck-drilling-goes-one-pass-086287 = Peck drilling goes one pass depth deeper each peck; pick a bit the size of the holes
setting-use-peck-drilling-25843c = Use Peck Drilling
setting-drill-in-pecks-of-the-7d3961 = Drill in pecks of the tool's pass depth, rising between them; off drills straight down
setting-peck-drilling-3698ab = Peck Drilling
setting-retract-a267d6 = Retract
setting-where-a-peck-drill-rises-a1e770 = Where a peck drill rises to between pecks.
setting-above-start-depth-f6eb83 = Above start depth
setting-right-out-of-the-hole-8cbc06 = Right out of the hole, the retract gap above the start depth: clears the chips
setting-above-previous-pass-b45efc = Above previous pass
setting-only-the-retract-gap-above-1d4be4 = Only the retract gap above the peck just made: quicker, but the chips may stay
setting-retract-gap-d0443e = Retract gap
setting-how-far-above-the-start-b29e0e = How far above the start depth, or the last peck, the drill rises between pecks
setting-dwell-af960b = Dwell
setting-seconds-to-wait-at-the-521940 = Seconds to wait at the bottom of each peck (0: none); the post must be able to dwell
setting-use-vector-selection-order-5db30d = Use vector selection order
setting-drill-the-holes-in-the-9cf28b = Drill the holes in the order the vectors were picked; else in the order they were drawn

## Drill Bank Toolpath (drill_bank_toolpath)
setting-the-feed-plunge-rate-spindle-c63986 =
    The feed, plunge rate, spindle speed and pass depth the bank's drills run at; the
    holes' sizes come from the drills themselves
setting-drill-in-pecks-of-the-2622e9 =
    Drill in pecks of the tool's pass depth, rising between them to clear the chips; off
    drills straight down
setting-how-far-above-the-start-e0b022 = How far above the start depth, or the last peck, the drills rise between pecks
setting-pitch-tolerance-97dc1a = Pitch Tolerance
setting-how-far-a-hole-s-d217a7 =
    How far a hole's centre may be from where a drill comes down and still be drilled by
    it: a little for holes drawn slightly off the pitch
setting-matching-holes-to-drills-03a364 = Matching Holes to Drills
setting-diameter-tolerance-9fbb91 = Diameter Tolerance
setting-how-much-a-hole-s-887071 = How much a hole's size may differ from a drill's and still be drilled by it

## Add Layer (add_layer)
setting-what-the-new-layer-is-112704 = What the new layer is called in the Layers list

## Delete Layer (delete_layer)
setting-layer-dc1b96 = Layer
setting-the-layer-to-delete-with-26a0bf = The layer to delete, with every vector on it.

## Move to Layer (move_to_layer)
setting-where-the-selected-vectors-go-b7baf4 = Where the selected vectors go.

## Select Vectors on Layer (select_layer_vectors)
setting-selects-every-vector-on-this-89e663 = Selects every vector on this layer, and nothing else.

## Layer Properties (layer_properties)
setting-what-the-layer-is-called-56376b = What the layer is called in the layer list
setting-visible-2a4941 = Visible
setting-show-the-layer-s-vectors-e4d6bc = Show the layer's vectors; hidden ones are not drawn or picked
setting-locked-6a5eb3 = Locked
setting-locked-vectors-are-drawn-but-c5b9a9 = Locked vectors are drawn but cannot be selected or edited.
setting-what-this-layer-s-vectors-29f34d =
    What this layer's vectors are cut from, from the Materials Library; a toolpath
    over several layers cuts with the hardest. Left empty, the job's material
setting-own-colour-4b02b5 = Own Colour
setting-draw-this-layer-s-vectors-02a6f1 =
    Draw this layer's vectors in a colour of its own, to tell them apart; unticked they
    are drawn in the usual vector colour. A drawing brought in from a DXF keeps the
    colours its layers had
setting-colour-cca61d = Colour
setting-the-colour-this-layer-s-2a0e97 =
    The colour this layer's vectors are drawn in (greyed unless Own Colour is ticked);
    it changes only how they look, never how they are cut

## Set Current Layer (set_current_layer)
setting-new-vectors-go-on-this-5bd62c =
    New vectors go on this layer. A shape remade from others (Weld, Trim, Join and the
    like) stays on the layer the originals were on.

## Create Shape (create_shape)
setting-what-the-new-component-is-e8a664 = What the new component is called in the Components list
setting-profile-17328e = Profile
setting-the-shape-of-the-sides-696160 = The shape of the sides as they rise from the edge
setting-rises-at-the-angle-from-ab6216 = Rises at the angle from the edge and rounds over the top
setting-angular-de7989 = Angular
setting-rises-at-the-angle-all-5ae27d = Rises at the angle all the way
setting-smooth-02d9cf = Smooth
setting-rises-in-an-s-curving-59f9bd = Rises in an S, curving smoothly out of the edge and over the top
setting-flat-170fba = Flat
setting-one-height-all-over-47704d = One height all over
setting-custom-25b78e = Custom
setting-the-open-vector-selected-with-911b44 =
    The open vector selected with the closed ones, drawn upright: its left end at the
    edge, its right end at the middle
setting-how-steeply-round-and-angular-4ecbbb = How steeply round and angular sides rise from the edge
setting-the-flat-top-s-height-b99a8e = The flat top's height; round and angular shapes stop rising here (0: no limit)
setting-smoothing-f1b7e9 = Smoothing
setting-smooth-shapes-how-pronounced-the-e1f54e = Smooth shapes: how pronounced the S is, from a straight slope (0) to steep in the middle
setting-base-bc2bf8 = Base
setting-added-everywhere-inside-the-shape-61f4fd = Added everywhere inside the shape
setting-scale-528e11 = Scale
setting-the-profile-scaled-up-or-f0bf47 =
    The profile scaled up or down to this height, its shape kept (0: as it rises).
    Greyed while Total height is set, which sizes the shape instead
setting-total-height-6a6958 = Total height
setting-the-whole-shape-base-and-bfcfe1 =
    The whole shape, base and all, scaled to this height (0: as made). Set it above the
    base: below it, the shape is a flat slab as high as the base
setting-preserve-internal-corners-c6956c = Preserve internal corners
setting-inside-corners-rise-as-straight-060119 = Inside corners rise as straight, mitred slopes to a sharp crease, not curved in
setting-blend-to-inner-vectors-7b0865 = Blend to inner vectors
setting-vectors-inside-the-outer-ones-112142 =
    Vectors inside the outer ones are the top, not holes: the profile rises from the
    outer edge to them
setting-combine-e1bd2a = Combine
setting-how-a-component-s-heights-f11663 = How a component's heights combine with the model under it.
setting-add-c3aa14 = Add
setting-raised-on-top-of-what-50a107 = Raised on top of what is under it
setting-subtract-92fd81 = Subtract
setting-sunk-into-what-is-under-f87569 = Sunk into what is under it
setting-merge-725bc7 = Merge
setting-the-higher-of-the-two-6df775 = The higher of the two
setting-the-lower-of-the-two-6f215b = The lower of the two
setting-multiply-34d965 = Multiply
setting-the-two-multiplied-a-0-c46613 = The two multiplied: a 0 to 1 component fades what is under it

## V-Carve / Engrave Toolpath (vcarve_engrave_toolpath)
setting-start-depth-7895f2 = Start depth
setting-below-the-material-surface-where-b46511 = Below the material surface where carving starts (the floor of an earlier pocket)
setting-flat-depth-5dfda7 = Flat depth
setting-deepest-the-bit-goes-below-ff9667 =
    Deepest the bit goes below the start depth; wider parts are cut flat there (0: as
    deep as it can)
setting-carve-545110 = Carve
setting-fill-carves-the-whole-shape-26af36 =
    Fill carves the whole shape, as deep as its width needs; Inside, On the Line and
    Outside carve a V groove of the depth set
setting-fill-184168 = Fill
setting-carve-the-whole-shape-the-226135 =
    Carve the whole shape: the tip runs down its middle, as deep as its width needs, so
    the sides meet in a sharp crease (the usual V-carved lettering). Its passes go at the
    bit's pass depth; Edit Passes is for the carves of a set depth
setting-a-v-groove-just-inside-dc54df =
    A V groove just inside each shape's edge, one side of the V on the vector, as deep as
    set
setting-the-tip-follows-the-vectors-ba4301 = The tip follows the vectors themselves, open ones too, as deep as set
setting-a-v-groove-just-outside-8a74ff =
    A V groove just outside each shape's edge, one side of the V on the vector, as deep as
    set; the shape stays its full size
setting-groove-depth-efb496 = Groove depth
setting-how-deep-the-groove-s-459a42 =
    How deep the groove's point goes below the start depth (Fill works its depth out
    from each shape's width instead)
setting-a-v-bit-its-stepover-dc5060 = A V-bit; its stepover sets how close together the carving passes are
setting-end-mills-that-clear-the-18f28f =
    End mills that clear the flat bottom first, largest first (needs a flat depth): each
    makes a toolpath of its own, and each tool after it, the V-bit last, cuts only what
    the ones before it left (rest machining)
setting-after-calculating-drop-the-toolpath-34bfa5 =
    After calculating, drop the toolpath onto the 3D model: its depths are then below the
    model's surface, going no lower than the bottom of the material
setting-use-vector-start-points-33af92 = Use vector start points
setting-each-pass-starts-as-near-d42596 =
    Each pass starts as near as it can to the start point of the vector it follows;
    otherwise where it starts is left to the program
setting-carve-the-shapes-one-after-fb25cd =
    Carve the shapes one after another in the order they were selected; otherwise all
    are carved together, a pass depth at a time

## 3D Finishing Toolpath (3d_finishing_toolpath)
setting-tools-0196fc = Tools
setting-in-the-order-they-run-c29e42 =
    In the order they run, a toolpath each: the first finishes all it can, each later one
    only what the tools before it left (rest machining). A ball nose follows curves best;
    its stepover spaces the raster lines. Editing a toolpath changes its own bit; to add a
    smaller one after it, make the toolpath again with both here
setting-minimum-detail-4b2204 = Minimum detail
setting-with-two-tools-or-more-251be8 =
    With two tools or more, each later tool cuts only what the tools before it left:
    areas left smaller across than this are left alone
setting-rest-machining-47cca9 = Rest Machining
setting-rest-overlap-08e438 = Rest overlap
setting-with-two-tools-or-more-ad5557 =
    With two tools or more: how far each area a later tool cuts reaches past what the
    tools before it left, so its cuts overlap theirs and leave no step
setting-machining-limit-boundary-8a4818 = Machining Limit Boundary
setting-where-the-tool-s-tip-ea8c01 = Where the tool's tip may go
setting-model-boundary-266b24 = Model Boundary
setting-round-all-the-visible-components-796acb = Round all the visible components
setting-material-boundary-ceb2c6 = Material Boundary
setting-the-whole-block-of-material-75c0b0 = The whole block of material
setting-selected-vector-s-0c5c55 = Selected Vector(s)
setting-inside-the-selected-closed-vectors-17183a =
    Inside the selected closed vectors (kept with the toolpath: recalculating uses them
    again, and changing them marks it out of date). Switched to in Edit, select the
    vectors before pressing Calculate
setting-selected-level-6f1182 = Selected level
setting-round-the-visible-components-on-bab713 = Round the visible components on the level named
setting-level-55597d = Level
setting-the-level-whose-components-make-e422b7 = The level whose components make the boundary
setting-boundary-offset-cac08a = Boundary Offset
setting-grows-the-boundary-outward-by-a353c1 = Grows the boundary outward by this much (negative shrinks it)
setting-the-pattern-the-tool-follows-cff565 = The pattern the tool follows over the model
setting-straight-lines-across-at-the-ad13e3 = Straight lines across, at the raster angle
setting-rings-stepping-in-from-the-6f38a4 = Rings stepping in from the machining boundary
setting-stepover-retract-c48eb8 = Stepover retract
setting-lift-off-the-surface-by-de66da =
    Lift off the surface by this between offset rings, which can leave fewer marks where
    the tool steps over

## 3D Pencil Toolpath (3d_pencil_toolpath)
setting-cleans-up-after-a-3d-2861df =
    Cleans up after a 3D finish: a small ball nose (the pencil bit) runs along each valley and
    inside corner of the model, where the bigger bit that finished it was too round to get
    right in, and cuts away what it left there. Use it after a 3D Finishing toolpath, with the
    finishing bit picked under "Bits that finished before": only what those bits left is cut,
    so it is quick. Extra passes either side widen the clean-up strip, a stepover apart.
setting-the-pencil-bit-a-ball-2432fe =
    The pencil bit: a ball nose smaller than the bit that finished before, so it fits into
    the corners that bit could not. Its finishing stepover spaces the extra passes
setting-bits-that-finished-before-c3f92b = Bits that finished before
setting-the-bits-that-finished-the-ba9f9c =
    The bits that finished the model before this one (usually the 3D Finishing bit): the
    pencil bit cuts only where they left material. With none, it runs along every valley
    sharper than the angle below
setting-valleys-sharper-than-5a25fb = Valleys sharper than
setting-how-sharp-a-valley-must-6b9170 =
    How sharp a valley must be to get a pencil pass: how much the surface bends where its
    two sides meet. 90° is a wall meeting the floor; lower picks up gentler valleys too
setting-extra-passes-each-side-2801f6 = Extra passes each side
setting-passes-beside-each-valley-line-a10ac8 =
    Passes beside each valley line, on each side, a stepover apart (the pencil bit's
    finishing stepover), to clean a wider strip. 0 cuts the valley line alone. Passes stop
    where the bits that finished before already cut
setting-material-the-bits-before-left-07f262 = Material the bits before left in strips narrower than this is left alone
setting-what-the-bits-before-left-5aa698 = What the bits before left
setting-overlap-4dcca8 = Overlap
setting-how-far-the-cut-reaches-d0fab8 = How far the cut reaches past what the bits before left, so it blends in with no step

## 3D Roughing Toolpath (3d_roughing_toolpath)
setting-allowance-c5f5b5 = Allowance
setting-left-on-the-model-for-981902 = Left on the model for the finishing toolpath to take off
setting-machining-allowance-548ac7 = Machining Allowance
setting-its-pass-depth-spaces-the-c40a82 = Its pass depth spaces the levels, its stepover the lines
setting-roughing-strategy-4b2f1f = Roughing Strategy
setting-how-the-bulk-of-the-03ad43 = How the bulk of the material is cleared away
setting-z-level-665847 = Z Level
setting-flat-at-each-level-round-0e3738 = Flat at each level, round the model
setting-3d-raster-8230bf = 3D Raster
setting-over-the-model-never-deeper-0f0635 = Over the model, never deeper than each level: a more even skin for finishing
setting-which-order-the-levels-and-7fbb0c = Which order the levels and areas are cut in
setting-level-by-level-9b3a84 = Level by level
setting-all-of-each-level-before-d72017 = All of each level before the next one down
setting-depth-first-6a645c = Depth first
setting-each-separate-area-all-the-820b1b = Each separate area all the way down before moving to the next
setting-a-pass-round-the-edge-604b15 = A pass round the edge of each level, before or after its raster lines
setting-round-the-edge-before-the-2139ff = Round the edge before the raster lines
setting-round-the-edge-after-them-001743 = Round the edge after them
setting-raster-lines-only-459053 = Raster lines only
setting-reverse-step-direction-fd1ba0 = Reverse step direction
setting-step-from-line-to-line-3be130 = Step from line to line the other way: top to bottom along X, left to right along Y
setting-avoid-machined-areas-4e7d43 = Avoid machined areas
setting-leave-out-what-an-earlier-0a3ab1 = Leave out what an earlier pass already cut down to the model
setting-ramp-plunge-moves-004b12 = Ramp plunge moves
setting-zig-zag-down-from-the-68f5be =
    Zig zag down from the level above over the ramp distance instead of plunging. Z Level
    only: 3D Raster does not ramp
setting-ramp-plunge-moves-c282d2 = Ramp Plunge Moves
setting-ramp-distance-648384 = Ramp distance
setting-how-far-along-the-cut-c5526d = How far along the cut each zig zag ramp runs

## Toolpath Properties (toolpath_properties)
setting-hidden-toolpaths-are-not-drawn-2b9bbc = Hidden toolpaths are not drawn or previewed

## Component Properties (component_properties)
setting-what-the-component-is-called-a0dc97 = What the component is called in the component tree
setting-combine-mode-d69be5 = Combine Mode
setting-hidden-components-are-left-out-871b15 = Hidden components are left out of the model
setting-shape-height-0a7bc1 = Shape Height
setting-height-as-a-percentage-of-733b72 = Height as a percentage of the component's own
setting-base-height-ea2933 = Base Height
setting-raises-the-component-on-a-1a113c = Raises the component on a flat plane this thick
setting-fade-15cc33 = Fade
setting-fades-the-heights-down-from-9c9a16 = Fades the heights down from the first point toward the second.
setting-fade-the-heights-down-between-58c2e0 = Fade the heights down between the two points
setting-from-743595 = From
setting-keeps-its-height-ec7102 = Keeps its height
setting-to-f59644 = To
setting-faded-down-by-the-percentage-924c11 = Faded down by the percentage
setting-how-much-lower-the-second-152857 = How much lower the second point ends up, as a percentage of the height
setting-tilt-e11726 = Tilt
setting-tilts-the-component-up-about-9a3504 = Tilts the component up about the first point toward the second.
setting-tilt-the-component-up-between-38f355 = Tilt the component up between the two points
setting-pivot-9e22b7 = Pivot
setting-does-not-move-be353a = Does not move
setting-toward-3bfcae = Toward
setting-raised-by-the-angle-a2bb46 = Raised by the angle
setting-how-steeply-it-tilts-up-35bda9 = How steeply it tilts up, in degrees
setting-bake-817ba0 = Bake
setting-make-the-shape-and-base-e58484 =
    Make the shape and base height, fade and tilt permanent, then reset them: after
    Apply the form goes back to 100 % with this unticked, so it bakes only once
setting-how-the-3d-view-colours-5e1b78 = How the 3D view colours it
setting-same-as-parent-afc803 = Same as Parent
setting-takes-the-colour-of-the-125e62 = Takes the colour of the group or level it is in
setting-solid-color-930303 = Solid Color
setting-shown-in-its-own-colour-b899a8 = Shown in its own colour, picked below
setting-its-colour-with-solid-color-eca446 = Its colour, with Solid Color (greyed with Same as Parent)

## Delete Toolpath (delete_toolpath)
setting-toolpath-036110 = Toolpath
setting-the-toolpath-to-delete-5a9f99 = The toolpath to delete

## Delete Component (delete_component)
setting-component-d28780 = Component
setting-the-component-to-delete-f74593 = The component to delete

## Show / Hide (show_hide)
setting-items-36382f = Items
setting-the-kinds-of-item-the-e4dc32 = The kinds of item the right-click families (Show / Hide, Delete) act on.
setting-layers-7fe97f = Layers
setting-the-layers-in-the-layer-65442d = The layers in the layer list
setting-toolpaths-524ed9 = Toolpaths
setting-the-toolpaths-in-the-toolpath-447335 = The toolpaths in the toolpath list
setting-components-6c0389 = Components
setting-the-3d-components-in-the-5c49d0 = The 3D components in the component list
setting-levels-5b310a = Levels
setting-the-levels-the-3d-components-80078a = The levels the 3D components sit on
setting-item-648006 = Item
setting-the-item-right-clicked-a-4d70c6 = The item right-clicked (a layer, toolpath or component id)
setting-action-77a91f = Action
setting-what-to-show-or-hide-ef974a = What to show or hide
setting-show-this-8e47f0 = Show This
setting-shows-the-one-you-right-b28ba3 = Shows the one you right-clicked
setting-show-only-this-33ff7a = Show Only This
setting-shows-the-one-you-right-f31982 = Shows the one you right-clicked and hides all the others
setting-show-all-but-this-96fa38 = Show All But This
setting-shows-all-the-others-and-5e4c6e = Shows all the others and hides the one you right-clicked
setting-show-all-3c130f = Show All
setting-shows-every-one-of-them-667eff = Shows every one of them
setting-hide-this-20a917 = Hide This
setting-hides-the-one-you-right-686868 = Hides the one you right-clicked
setting-hide-all-061392 = Hide All
setting-hides-every-one-of-them-b06820 = Hides every one of them

## Delete Items (delete_items)
setting-the-item-right-clicked-for-da59e1 = The item right-clicked, for Delete This
setting-which-bc1282 = Which
setting-which-of-them-to-delete-b72b93 = Which of them to delete
setting-delete-this-8807e6 = Delete This
setting-only-the-one-you-right-08b5cd = Only the one you right-clicked
setting-delete-visible-5ee850 = Delete Visible
setting-every-one-that-is-shown-56adde = Every one that is shown
setting-delete-invisible-fc434d = Delete Invisible
setting-every-one-that-is-hidden-5656f1 = Every one that is hidden
setting-delete-empty-b28333 = Delete Empty
setting-layers-with-no-vectors-on-2fec6d = Layers with no vectors on them
setting-delete-all-e2162d = Delete All
setting-every-one-of-them-85da91 = Every one of them

## Fluting Toolpath (fluting_toolpath)
setting-below-the-material-surface-where-68e7f9 = Below the material surface where the flute begins (the floor of an earlier pocket)
setting-flute-depth-113a0c = Flute depth
setting-below-the-start-depth-at-35a3da = Below the start depth at the deepest point; cut in passes of the bit's pass depth
setting-flute-type-59c8fb = Flute type
setting-where-along-each-vector-the-3bfbc3 = Where along each vector the flute goes down and comes up
setting-flute-type-86151b = Flute Type
setting-ramp-over-complete-length-ff4d44 = Ramp over complete length
setting-from-the-start-depth-at-b8140b = From the start depth at the start of each vector down to full depth at its end
setting-ramp-at-start-ddcaa2 = Ramp at start
setting-down-to-full-depth-over-2ee740 = Down to full depth over the ramp, then along at full depth
setting-ramp-at-start-and-end-919842 = Ramp at start and end
setting-down-at-the-start-and-ee9cef = Down at the start and back up at the end
setting-ramp-type-e58613 = Ramp type
setting-the-shape-of-the-slope-0c7551 = The shape of the slope down into the flute
setting-ramp-c71109 = Ramp
setting-linear-cc59e0 = Linear
setting-a-straight-slope-f4b489 = A straight slope
setting-a-curve-that-levels-out-0087dc = A curve that levels out into the full depth
setting-use-ramp-percent-d0e5ad = Use ramp percent
setting-size-the-ramps-as-a-a70a64 = Size the ramps as a percentage of the longest they can be, instead of a length
setting-ramp-length-745b03 = Ramp length
setting-length-of-each-ramp-along-c1fc91 =
    Length of each ramp along the vector (capped at the longest it can be), with Use ramp
    percent unticked
setting-ramp-percent-00dc98 = Ramp percent
setting-percentage-of-the-longest-ramp-e83f9d =
    Percentage of the longest ramp: the whole vector at the start only, half of it at both
    ends, with Use ramp percent ticked
setting-the-bit-that-cuts-the-6fea4e = The bit that cuts the flutes, usually a ball nose or V-bit

## V-Carve Inlay Toolpath (vcarve_inlay_toolpath)
setting-pocket-depth-12368a = Pocket depth
setting-deepest-the-pocket-goes-narrow-2da094 = Deepest the pocket goes (narrow parts stop short of it)
setting-glue-gap-993878 = Glue gap
setting-between-the-top-of-the-e43e65 = Between the top of the plug and the bottom of the pocket once fitted
setting-inlay-39ccc4 = Inlay
setting-surface-clearance-1d99a2 = Surface clearance
setting-between-the-plug-s-material-3755bf = Between the plug's material and the pocket's surface once fitted
setting-plug-boundary-3a77fd = Plug boundary
setting-how-far-round-the-shapes-882c9d = How far round the shapes the plug's surface is cleared, on the Inlay plug sheet
setting-boundary-d42cc7 = Boundary
setting-vector-offset-7f07c1 = Vector offset
setting-only-round-the-shapes-the-50122c = Only round the shapes, the surface clearance plus the boundary offset out from them
setting-sheet-limits-28b348 = Sheet limits
setting-from-the-shapes-out-to-c980c2 = From the shapes out to the edges of the plug's board (the Inlay plug sheet)
setting-boundary-offset-dfcdaa = Boundary offset
setting-extra-material-cut-away-round-069f45 = Extra material cut away round the plug
setting-a-v-bit-the-same-18b798 = A V-bit, the same for both halves

## Prism Carving Toolpath (prism_toolpath)
setting-below-the-material-surface-where-a31295 = Below the material surface where the prism's top is (the floor of an earlier pocket)
setting-prism-depth-c49f1f = Prism depth
setting-of-the-prism-s-base-af4257 = Of the prism's base below the start depth; too shallow and the top is cut flat
setting-depth-for-full-prism-b4fcd7 = Depth for full prism
setting-instead-use-the-least-depth-aea099 = Instead, use the least depth that brings the widest shape to a point
setting-cut-direction-5b05fe = Cut direction
setting-which-way-round-the-shapes-19fe22 = Which way round the shapes the tool travels
setting-cut-direction-cdb05e = Cut Direction
setting-climb-2c7b2a = Climb
setting-clockwise-round-the-shapes-993e00 = Clockwise round the shapes
setting-conventional-0ef65f = Conventional
setting-anticlockwise-round-the-shapes-05d3a8 = Anticlockwise round the shapes
setting-passes-4b5a7c = Passes
setting-every-pass-the-same-way-e656ab = Every pass the same way, or each back the way the last came
setting-single-direction-d52f44 = Single direction
setting-every-pass-the-cut-direction-3c0ed2 = Every pass the cut direction's way
setting-both-directions-d46738 = Both directions
setting-the-first-pass-the-cut-746ec2 =
    The first pass the cut direction's way, the next back the other way (climb then
    conventional, or the other way about), and so on
setting-the-v-bit-that-carves-6ba9c5 = The V-bit that carves the prism

## Texture Toolpath (texture_toolpath)
setting-use-selected-vectors-as-pattern-57a3a5 = Use selected vectors as pattern
setting-cut-along-the-selected-vectors-efcf92 =
    Cut along the selected vectors instead of a random pattern inside them. Off, the
    pattern fills the selected closed vectors, or the whole material when none are selected
setting-texture-4c5594 = Texture
setting-for-a-pattern-dip-in-86c3c1 = For a pattern: dip in and out along each vector instead of cutting it at one depth
setting-max-cut-depth-4e7c14 = Max cut depth
setting-deepest-any-scallop-goes-74ebd1 = Deepest any scallop goes
setting-min-depth-percent-b57067 = Min depth percent
setting-shallowest-a-scallop-goes-as-19e8b1 = Shallowest a scallop goes, as a percentage of the deepest
setting-max-cut-length-ac7763 = Max cut length
setting-longest-any-scallop-is-7e97ff = Longest any scallop is
setting-min-length-percent-96fdf4 = Min length percent
setting-shortest-a-scallop-is-as-5f4d85 = Shortest a scallop is, as a percentage of the longest
setting-max-overlap-percent-c9bfab = Max overlap percent
setting-how-far-a-scallop-may-b7a32b = How far a scallop may run back over the one before, as a percentage of the longest
setting-overlap-variation-percent-9e6c5c = Overlap variation percent
setting-how-much-the-overlap-varies-ea4bf3 = How much the overlap varies, as a percentage of the most it may be
setting-between-rows-of-scallops-f7f7ec = Between rows of scallops
setting-direction-of-the-rows-anticlockwise-38c519 = Direction of the rows, anticlockwise from the X axis
setting-keeps-the-texture-this-far-6c73a3 = Keeps the texture this far in from the boundary (up to the tool radius in a recess)
setting-seed-7208cc = Seed
setting-another-number-gives-another-random-071b6d = Another number gives another random pattern
setting-the-bit-that-cuts-the-efcb75 = The bit that cuts the texture, usually a ball nose

## Group Toolpaths (group_toolpaths)
setting-the-toolpaths-to-put-in-f4d3fc = The toolpaths to put in the group; none makes an empty group
setting-group-74902c = Group
setting-the-group-s-name-empty-b53a19 = The group's name; empty for no group
setting-the-group-a-new-group-3fa2c0 = The group a new group goes inside; empty for the top of the list

## Rename Toolpath Group (rename_toolpath_group)
setting-its-new-name-1a7a0a = Its new name

## Ungroup Toolpaths (ungroup_toolpaths)
setting-the-toolpath-group-to-take-e03e90 = The toolpath group to take apart
setting-delete-toolpaths-cb2714 = Delete toolpaths
setting-delete-the-group-s-toolpaths-a095f8 = Delete the group's toolpaths and sub-groups with it, not only the group

## Show or Hide Toolpath Group (show_toolpath_group)
setting-the-toolpath-group-to-show-c2b45b = The toolpath group to show or hide
setting-show-its-toolpaths-ticked-or-00515c = Show its toolpaths (ticked), or hide them

## Quick Engrave (quick_engrave_toolpath)
setting-below-the-material-surface-or-2a41bb =
    Below the material surface (or below the nose cone); cut in passes of the bit's pass
    depth
setting-follow-the-lines-or-fill-65e1f1 = Follow the lines, or fill the shapes
setting-engrave-8d8ef3 = Engrave
setting-outline-85e05d = Outline
setting-the-tip-runs-along-the-2d4cb4 = The tip runs along the lines
setting-a-pattern-marks-the-inside-f52d04 = A pattern marks the inside of closed shapes
setting-fill-pattern-4adbb2 = Fill pattern
setting-the-pattern-that-fills-the-4ae4d5 = The pattern that fills the shapes, with Fill
setting-hatch-a1e521 = Hatch
setting-lines-along-x-f817d9 = Lines along X
setting-cross-hatch-1b70bd = Cross hatch
setting-lines-along-x-and-along-f3f276 = Lines along X and along Y
setting-between-fill-lines-set-here-a2394c = Between fill lines (set here, not from the tool)
setting-passes-at-least-fe0c44 = Passes at Least
setting-kept-from-a-job-saved-1b90ac =
    Kept from a job saved when Passes ran the pattern again at the same depth: at least this
    many passes are cut, each a step deeper, even when Edit Passes sets fewer. Set it to 1
    to cut the bit's passes (Edit Passes) only; it is then hidden
setting-the-engraving-bit-or-drag-6b726a = The engraving bit or drag point that marks the lines

## Keyhole Toolpath (keyhole_toolpath)
setting-direction-fefc0a = Direction
setting-which-way-the-slot-runs-f516af = Which way the slot runs from the entry hole
setting-keyhole-2c9b1e = Keyhole
setting-from-the-entry-hole-up-b99cd1 = From the entry hole up the job
setting-from-the-entry-hole-down-4ac264 = From the entry hole down the job
setting-from-the-entry-hole-to-85dfa0 = From the entry hole to the right
setting-right-to-left-b0285b = Right to left
setting-from-the-entry-hole-to-09a4af = From the entry hole to the left
setting-entry-bfed33 = Entry
setting-one-entry-hole-or-a-d6c79f = One entry hole, or a hole at both ends of the slot
setting-single-776509 = Single
setting-goes-in-and-out-through-12b9f5 = Goes in and out through one hole; the screw head slides to the closed end
setting-double-464c28 = Double
setting-goes-in-at-one-end-fc40a0 = Goes in at one end and out at the other, leaving a hole at both ends
setting-slot-depth-c87b6c = Slot depth
setting-deep-enough-that-the-cutter-9aa707 = Deep enough that the cutter's wide part is below the surface
setting-slot-length-c576ab = Slot length
setting-how-far-the-slot-runs-6ffc86 = How far the slot runs from the entry hole
setting-create-preview-vectors-ab6ec9 = Create preview vectors
setting-draw-how-the-slots-will-3e0bb5 = Draw how the slots will look on the surface
setting-entry-hole-diameter-b3df8a = Entry hole diameter
setting-of-the-hole-the-cutter-3393af = Of the hole the cutter makes going in, for the preview
setting-slot-diameter-e9ac9c = Slot diameter
setting-of-the-slot-on-the-ba1262 = Of the slot on the surface, for the preview
setting-an-end-mill-with-the-aff7fb = An end mill with the keyhole cutter's feed rates

## Surfacing Toolpath (surfacing_toolpath)
setting-flatten-9f1b69 = Flatten
setting-the-whole-material-your-spoilboard-707953 = The whole material, your spoilboard, or the inside of shapes you selected
setting-what-to-flatten-7c44f2 = What to Flatten
setting-whole-material-0c0435 = Whole material
setting-the-material-set-in-job-d718cf = The material set in Job Setup, all of it
setting-spoilboard-96f6e6 = Spoilboard
setting-the-spoilboard-on-your-machine-d7c650 =
    The spoilboard on your machine's bed, from its front-left corner (X0 Y0): this cuts
    the spoilboard itself, with nothing on it
setting-selected-vectors-28afc8 = Selected vectors
setting-the-inside-of-the-closed-c359e3 = The inside of the closed shapes selected, such as a slab's outline
setting-spoilboard-width-7806ce = Spoilboard width
setting-across-the-spoilboard-left-to-024148 = Across the spoilboard, left to right; filled in from your machine's bed size
setting-spoilboard-height-466041 = Spoilboard height
setting-of-the-spoilboard-front-to-4e6b13 = Of the spoilboard, front to back; filled in from your machine's bed size
setting-depth-to-take-off-00f83b = Depth to Take Off
setting-how-much-comes-off-the-4ee62d =
    How much comes off the top: a light skim is enough to flatten. Edit Passes… takes it
    off in more than one pass
setting-a-surfacing-bit-spoilboard-bit-f0bdff =
    A surfacing bit (spoilboard bit or fly cutter) from your tools; its stepover and
    feeds are used
setting-cut-in-34c7fa = Cut in
setting-rows-along-x-or-y-082777 = Rows along X or Y, or a spiral from the outside in
setting-pattern-9120c9 = Pattern
setting-rows-along-x-a519b5 = Rows along X
setting-rows-running-left-and-right-35097d = Rows running left and right (along X), stepping towards the back
setting-rows-along-y-a51822 = Rows along Y
setting-rows-running-front-to-back-c20c3d = Rows running front to back (along Y), stepping to the right
setting-spiral-in-3cc1e7 = Spiral in
setting-rings-from-the-outside-in-0ac05a = Rings from the outside in, each a stepover inside the last
setting-overhang-348813 = Overhang
setting-how-far-the-middle-of-eca199 =
    How far the middle of the bit runs past every edge, as a share of the bit's width,
    so the edges are cut cleanly

## Drag Knife Toolpath (drag_knife_toolpath)
setting-blade-offset-841d30 = Blade offset
setting-from-the-knife-s-pivot-081bed = From the knife's pivot to its tip (from the maker)
setting-knife-7e797a = Knife
setting-tolerance-angle-83343b = Tolerance angle
setting-corners-turning-less-than-this-a4eab8 = Corners turning less than this are cut straight through, without a swivel
setting-spring-travel-c34598 = Spring travel
setting-how-far-the-knife-holder-eef787 =
    How far the knife holder's spring lets the blade push back into it (from the
    holder's maker). Zero Z with the blade tip just touching the material: the knife then
    presses down by a share of this
setting-pressure-204cc2 = Pressure
setting-how-hard-the-blade-presses-a6ab99 =
    How hard the blade presses while cutting, as a share of the spring travel: more for
    thicker or tougher material. The knife goes down this share of the spring travel
    below the top
setting-swivel-pressure-8d0efc = Swivel pressure
setting-how-hard-it-presses-while-870ebf =
    How hard it presses while it turns at a corner, as a share of the spring travel: less
    than the cutting pressure, so the blade can swivel
setting-sharp-corner-angle-680863 = Sharp Corner Angle
setting-corners-turning-more-than-this-9bc98d =
    Corners turning more than this are sharp: the knife cuts on through the corner by
    the overshoot, lifts out and starts again along the next edge. Gentler corners turn
    the blade without lifting it out
setting-overshoot-fa4efa = Overshoot
setting-how-far-the-cut-runs-490e06 =
    How far the cut runs on past a sharp corner (and past the start of a closed shape),
    so the cuts meet cleanly; never into another vector
setting-equal-passes-down-to-the-14a73b =
    Equal passes down to the cut depth; each pass swivels at its own share of the swivel
    depth, so it never presses deeper turning a corner than it cuts
setting-feed-and-plunge-rates-switch-d0fde4 = Feed and plunge rates (switch the spindle off)
setting-cut-on-an-uneven-surface-eb8ee6 = Cut on an uneven surface anyway
setting-a-knife-needs-a-flat-c1f86c =
    A knife needs a flat surface, so it must be the first toolpath or come straight after
    a Surfacing toolpath. Tick this only if you know the surface under it is flat anyway
    (for example flattened in another job)
setting-drop-the-knife-s-cut-14bb94 =
    Drop the knife's cut onto the 3D model, so it cuts its depth below the model's
    surface: for domes and other curved work

## Chamfer Toolpath (chamfer_toolpath)
setting-below-the-material-surface-where-e1476e = Below the material surface where the top of the chamfer is
setting-across-the-chamfer-with-a-1d6aa1 =
    Across the chamfer. With a V-bit the chamfer must stay within its V: start depth plus
    chamfer depth (from the top surface) no deeper than the V is tall
setting-chamfer-6c9275 = Chamfer
setting-slope-from-vertical-for-a-8afdf6 =
    Slope from vertical for a ball nose. Hidden with a V-bit, whose slope is half its
    angle (a 90° V-bit cuts a 45° chamfer)
setting-overcut-cad055 = Overcut
setting-moves-a-v-bit-s-1fc105 =
    Moves a V-bit's point off the chamfer so its side cuts (makes it cut deeper); shown
    only with a V-bit
setting-side-61137e = Side
setting-which-side-of-the-vectors-e71d72 = Which side of the vectors the chamfer goes
setting-the-chamfer-lies-outside-the-880e85 = The chamfer lies outside the shapes
setting-the-chamfer-lies-inside-the-cdc292 = The chamfer lies inside the shapes
setting-slope-c4fca8 = Slope
setting-which-way-the-chamfer-slopes-c2dad6 = Which way the chamfer slopes from the vector
setting-down-995035 = Down
setting-going-down-away-from-the-485195 = Going down away from the vector
setting-up-f35940 = Up
setting-going-up-away-from-the-ce28ea = Going up away from the vector
setting-a-v-bit-or-a-911c0e = A V-bit or a ball nose

## Thread Milling Toolpath (thread_milling_toolpath)
setting-below-the-material-surface-where-09d0b4 = Below the material surface where the thread starts
setting-max-depth-c2ff64 = Max depth
setting-deepest-the-bottom-of-the-2c639e = Deepest the bottom of the tool goes below the start depth
setting-thread-size-9f1222 = Thread size
setting-a-standard-thread-size-which-bb815f =
    A standard thread size, which sets both the diameter and the pitch; Custom to type
    them below
setting-thread-904991 = Thread
setting-the-diameter-and-pitch-typed-915a69 = The diameter and pitch typed below
setting-iso-m2-0-4-f7f32b = ISO M2 × 0.4
setting-metric-2-mm-bolt-thread-edd1f9 = Metric 2 mm bolt thread, 0.4 mm from ridge to ridge
setting-iso-m3-0-5-526a25 = ISO M3 × 0.5
setting-metric-3-mm-bolt-thread-2d2ea1 = Metric 3 mm bolt thread, 0.5 mm from ridge to ridge
setting-iso-m4-0-7-cdaea0 = ISO M4 × 0.7
setting-metric-4-mm-bolt-thread-2f9940 = Metric 4 mm bolt thread, 0.7 mm from ridge to ridge
setting-iso-m5-0-8-857d6c = ISO M5 × 0.8
setting-metric-5-mm-bolt-thread-ef9926 = Metric 5 mm bolt thread, 0.8 mm from ridge to ridge
setting-iso-m6-1-0-4089dc = ISO M6 × 1.0
setting-metric-6-mm-bolt-thread-4f7688 = Metric 6 mm bolt thread, 1.0 mm from ridge to ridge
setting-iso-m8-1-25-b64553 = ISO M8 × 1.25
setting-metric-8-mm-bolt-thread-d3b937 = Metric 8 mm bolt thread, 1.25 mm from ridge to ridge
setting-iso-m10-1-5-817ea8 = ISO M10 × 1.5
setting-metric-10-mm-bolt-thread-771c3e = Metric 10 mm bolt thread, 1.5 mm from ridge to ridge
setting-iso-m12-1-75-594b3f = ISO M12 × 1.75
setting-metric-12-mm-bolt-thread-3268fd = Metric 12 mm bolt thread, 1.75 mm from ridge to ridge
setting-iso-m14-2-0-938622 = ISO M14 × 2.0
setting-metric-14-mm-bolt-thread-648258 = Metric 14 mm bolt thread, 2.0 mm from ridge to ridge
setting-iso-m16-2-0-223b04 = ISO M16 × 2.0
setting-metric-16-mm-bolt-thread-533a9a = Metric 16 mm bolt thread, 2.0 mm from ridge to ridge
setting-iso-m20-2-5-21417c = ISO M20 × 2.5
setting-metric-20-mm-bolt-thread-38602c = Metric 20 mm bolt thread, 2.5 mm from ridge to ridge
setting-iso-m24-3-0-fd1798 = ISO M24 × 3.0
setting-metric-24-mm-bolt-thread-b51298 = Metric 24 mm bolt thread, 3.0 mm from ridge to ridge
setting-4-40-unc-42aa45 = #4-40 UNC
setting-number-4-machine-screw-thread-db2418 = Number 4 machine screw thread, coarse, 40 threads per inch
setting-6-32-unc-916986 = #6-32 UNC
setting-number-6-machine-screw-thread-4fa42d = Number 6 machine screw thread, coarse, 32 threads per inch
setting-8-32-unc-430894 = #8-32 UNC
setting-number-8-machine-screw-thread-53e0eb = Number 8 machine screw thread, coarse, 32 threads per inch
setting-10-24-unc-f5ecac = #10-24 UNC
setting-number-10-machine-screw-thread-4d947f = Number 10 machine screw thread, coarse, 24 threads per inch
setting-10-32-unf-bc7a38 = #10-32 UNF
setting-number-10-machine-screw-thread-379671 = Number 10 machine screw thread, fine, 32 threads per inch
setting-1-4-20-unc-1161cc = 1/4-20 UNC
setting-1-4-inch-bolt-thread-0715e7 = 1/4 inch bolt thread, coarse, 20 threads per inch
setting-1-4-28-unf-d6fbdb = 1/4-28 UNF
setting-1-4-inch-bolt-thread-8f6904 = 1/4 inch bolt thread, fine, 28 threads per inch
setting-5-16-18-unc-f8cbe8 = 5/16-18 UNC
setting-5-16-inch-bolt-thread-a36703 = 5/16 inch bolt thread, coarse, 18 threads per inch
setting-3-8-16-unc-424ae3 = 3/8-16 UNC
setting-3-8-inch-bolt-thread-d1695e = 3/8 inch bolt thread, coarse, 16 threads per inch
setting-7-16-14-unc-974e2a = 7/16-14 UNC
setting-7-16-inch-bolt-thread-5bc899 = 7/16 inch bolt thread, coarse, 14 threads per inch
setting-1-2-13-unc-5f1754 = 1/2-13 UNC
setting-1-2-inch-bolt-thread-370b7b = 1/2 inch bolt thread, coarse, 13 threads per inch
setting-9-16-12-unc-10eb42 = 9/16-12 UNC
setting-9-16-inch-bolt-thread-0a2de5 = 9/16 inch bolt thread, coarse, 12 threads per inch
setting-5-8-11-unc-40887c = 5/8-11 UNC
setting-5-8-inch-bolt-thread-73a367 = 5/8 inch bolt thread, coarse, 11 threads per inch
setting-3-4-10-unc-b21629 = 3/4-10 UNC
setting-3-4-inch-bolt-thread-c2aa94 = 3/4 inch bolt thread, coarse, 10 threads per inch
setting-1-8-unc-483375 = 1-8 UNC
setting-1-inch-bolt-thread-coarse-74333e = 1 inch bolt thread, coarse, 8 threads per inch
setting-pitch-5a8151 = Pitch
setting-between-the-ridges-of-the-4d9cb2 = Between the ridges of the thread
setting-the-thread-s-major-largest-0ad891 =
    The thread's major (largest) diameter, about the size of the bolt; it should be
    close to the size of the circles selected, or the form warns in red
setting-fit-tolerance-7dbb9b = Fit tolerance
setting-cuts-the-thread-this-much-701c97 = Cuts the thread this much deeper so it turns freely
setting-thread-type-65a1db = Thread type
setting-a-thread-inside-a-hole-4cecd5 = A thread inside a hole, or round a post
setting-internal-3eb420 = Internal
setting-inside-a-hole-for-a-426d22 = Inside a hole, for a bolt to screw into
setting-external-951daa = External
setting-round-a-post-as-on-f2cb57 = Round a post, as on a bolt
setting-thread-direction-00934c = Thread direction
setting-which-way-the-thread-turns-27814c = Which way the thread turns to tighten
setting-right-handed-e0a437 = Right handed
setting-tightens-turning-clockwise-as-most-7c1ca1 = Tightens turning clockwise, as most threads do
setting-left-handed-4b1438 = Left handed
setting-tightens-turning-anticlockwise-5de546 = Tightens turning anticlockwise
setting-which-way-the-tool-spirals-030adc = Which way the tool spirals along the thread
setting-spiralling-down-from-the-top-67f376 = Spiralling down from the top of the thread
setting-spiralling-up-from-the-bottom-4c232f = Spiralling up from the bottom
setting-create-circles-for-internal-threads-77db3d = Create Circles for Internal Threads
setting-internal-threads-also-draws-a-abaffd =
    Internal threads: also draws a circle of the thread's minor diameter at each centre,
    on the current layer, for a Pocket toolpath to clear the hole first
setting-use-vector-selection-order-1da86d = Use Vector Selection Order
setting-thread-the-vectors-in-the-862c32 =
    Thread the vectors in the order they were selected, else in the order that cuts
    quickest
setting-tooth-offset-6293f4 = Tooth offset
setting-from-the-bottom-of-the-346603 =
    From the bottom of the thread mill up to the tip of its tooth, for a tool that is not
    a thread mill (a thread mill gives its own)
setting-a-single-point-thread-mill-af41ec = A single-point thread mill; its diameter is tip to tip

## Moulding Toolpath (moulding_toolpath)
setting-create-sharp-corners-7833c7 = Create sharp corners
setting-mitred-corners-where-the-rails-c6511f = Mitred corners where the rails have sharp corners, instead of rolling round them
setting-moulding-a44808 = Moulding
setting-carries-the-passes-this-far-20f20f = Carries the passes this far past both ends of the profile
setting-usually-a-ball-nose-its-03fc69 = Usually a ball nose; its stepover spaces the passes
setting-which-way-the-passes-go-154a7b =
    Which way the passes go round the rails: climb (clockwise) or conventional
    (anticlockwise), for any bit, a custom form tool too
setting-climb-clockwise-d1633f = Climb (clockwise)
setting-clockwise-round-a-closed-rail-3a091d = Clockwise round a closed rail: usually the cleaner finish, the usual choice on a CNC
setting-conventional-anticlockwise-0fb3ca = Conventional (anticlockwise)
setting-anticlockwise-round-a-closed-rail-032aed =
    Anticlockwise round a closed rail, as you would push a hand router: gentler on a
    light machine or a large form tool, sometimes rougher in the finish
setting-every-pass-the-cut-direction-4f6f7e =
    Every pass the cut direction's way, or each pass back the way the last came (quicker
    along an open rail, but half the passes cut the other way)
setting-vary-stepover-21b647 = Vary Stepover
setting-space-the-passes-by-the-dace7a =
    Space the passes by the stepover along the profile's surface, so they come closer
    together where it is steep
setting-larger-area-clearance-tool-1b48cd = Larger Area Clearance Tool
setting-use-larger-area-clearance-tool-731f17 =
    Use Larger Area Clearance Tool: the first of these roughs the moulding first, in
    levels of its pass depth, as a toolpath of its own named [Clear]
setting-machining-allowance-e31d67 = Machining allowance
setting-material-the-clearance-tool-leaves-981863 = Material the clearance tool leaves on the profile for the finishing tool
setting-machine-flat-regions-b85ed4 = Machine flat regions
setting-the-clearance-tool-cuts-the-7bd7f8 = The clearance tool cuts the profile's flat parts it fits in to their finished height
setting-skip-flat-regions-4b8ac8 = Skip flat regions
setting-the-finishing-tool-leaves-out-1c561c = The finishing tool leaves out the flat parts the clearance tool finished

## Rounding Toolpath (rounding_toolpath)
setting-blank-e40961 = Blank
setting-the-shape-of-the-stock-d9cdb4 = The shape of the stock before it is rounded
setting-square-stock-its-size-is-5c9252 = Square stock; its size is the side
setting-round-stock-its-size-is-6a2bc3 = Round stock; its size is the diameter
setting-blank-size-d25c12 = Blank size
setting-side-of-a-square-blank-52a695 = Side of a square blank or diameter of a round one
setting-method-a2c750 = Method
setting-the-pattern-the-tool-follows-055a7a = The pattern the tool follows round the cylinder
setting-rounding-2bdf3d = Rounding
setting-radial-9f548e = Radial
setting-a-full-turn-round-the-ca5bb1 = A full turn round the cylinder, then a stepover along it, and back
setting-along-the-cylinder-then-a-494d64 = Along the cylinder, then a stepover round it, and back
setting-optimized-raster-59d053 = Optimized raster
setting-along-the-cylinder-only-where-640f75 = Along the cylinder only where a square blank still has material
setting-left-on-the-finished-cylinder-b37992 = Left on the finished cylinder
setting-its-stepover-pass-depth-and-3cab30 = Its stepover, pass depth and feed rates are used

## Photo V-Carve Toolpath (photo_vcarve_toolpath)
setting-below-the-material-surface-where-efe2e1 = Below the material surface where the carving is (the floor of an earlier pocket)
setting-max-carving-depth-84ce3c = Max carving depth
setting-where-the-picture-is-black-03b590 = Where the picture is black
setting-lift-above-the-start-depth-05ecd6 = Lift above the start depth to move between lines
setting-the-pattern-of-lines-the-9ef727 = The pattern of lines the picture is carved in
setting-one-set-of-parallel-lines-b72638 = One set of parallel lines
setting-two-sets-of-lines-at-38089f = Two sets of lines at right angles: denser, twice as long to cut
setting-along-the-selected-vectors-ff4196 = Along the selected vectors
setting-line-spacing-percent-cab76b = Line spacing percent
setting-between-lines-as-a-percentage-d44660 = Between lines, as a percentage of the groove width at the max depth (100: grooves meet)
setting-line-angle-a6a3f8 = Line angle
setting-of-the-lines-from-horizontal-21b5f4 = Of the lines, from horizontal
setting-the-v-bit-that-carves-33b502 = The V-bit that carves the picture

## Sketch Carving Toolpath (sketch_carving_toolpath)
setting-source-e2a3f8 = Source
setting-where-the-sketch-lines-come-64de26 = Where the sketch lines come from
setting-sketch-4fa2a9 = Sketch
setting-bitmap-6a9046 = Bitmap
setting-a-picture-the-one-picked-44611d = A picture: the one picked last, else the newest
setting-model-85e8fa = Model
setting-the-3d-model-its-hollows-5ebe8c = The 3D model: its hollows are the lines
setting-below-the-material-surface-where-3ea60b = Below the material surface where carving starts
setting-cut-depth-84a63e = Cut depth
setting-below-the-start-depth-cut-106d2c = Below the start depth, cut in passes no deeper than the tool's pass depth
setting-how-far-the-sketch-lines-2fb9f9 = How far the sketch lines reach
setting-bitmap-boundary-39cf98 = Bitmap boundary
setting-the-edges-of-the-picture-c17f23 = The edges of the picture or the job
setting-inside-the-selected-closed-vectors-b7aec5 = Inside the selected closed vectors
setting-moves-the-boundary-out-by-0e715c = Moves the boundary out by this much
setting-line-thickness-96b6f5 = Line thickness
setting-heavier-lines-but-less-detail-549573 = Heavier lines but less detail as it goes up
setting-its-stepover-spaces-the-passes-9c1609 = Its stepover spaces the passes over the lines; its pass depth is the most each cuts

## Laser Cut / Fill (laser_cut_fill_toolpath)
setting-power-49a5e6 = Power
setting-of-the-laser-s-maximum-2109d6 =
    Of the laser's maximum. The laser burns only along the cuts: it is off while it
    comes down to the surface, so no dot is burned where a line starts
setting-times-the-toolpath-is-run-d832f6 = Times the toolpath is run, for thicker material
setting-cut-along-the-shapes-or-53c289 = Cut along the shapes, or fill them with lines (each shows only its own settings)
setting-cut-outside-ac5b68 = Cut outside
setting-outside-the-shapes-by-half-0ba1f4 = Outside the shapes by half the kerf, so the part is the drawn size
setting-cut-inside-ea6f79 = Cut inside
setting-inside-the-shapes-by-half-a4e424 = Inside the shapes by half the kerf, so the hole is the drawn size
setting-cut-on-43a024 = Cut on
setting-the-beam-s-centre-on-7a80a4 = The beam's centre on the line
setting-hatch-fill-9263a2 = Hatch fill
setting-stripes-across-the-shapes-a42739 = Stripes across the shapes
setting-extra-offset-for-cutting-outside-c26852 = Extra offset for cutting outside or inside, to ease or tighten a fit
setting-between-hatch-lines-ed07b6 = Between hatch lines
setting-hatch-angle-01d9d2 = Hatch angle
setting-the-angle-of-the-hatch-5604ae = The angle of the hatch lines: 0° along X, 90° along Y
setting-a-second-set-of-hatch-a27309 = A second set of hatch lines at right angles
setting-round-the-shapes-as-well-d2d2e4 = Round the shapes as well, so a hatch has crisp edges
setting-the-laser-a-laser-in-50a03f =
    The laser, a Laser in the Tool Database (Select… picks another): its diameter is the
    kerf, its feed rate the move speed

## Laser Picture (laser_picture_toolpath)
setting-of-the-laser-s-maximum-10009b =
    Of the laser's maximum, where the picture is black (white burns nothing: a picture
    white where the lines run is made with a warning that nothing will burn)
setting-times-the-picture-is-burned-860eac = Times the picture is burned, for a darker mark
setting-the-pattern-of-lines-the-45e811 =
    The pattern of lines the picture is burned in (along the selected vectors, the line
    spacing and angle are theirs, so those settings are hidden)
setting-picture-186b29 = Picture
setting-two-sets-at-right-angles-f5ee57 = Two sets at right angles
setting-selected-233294 = Selected
setting-along-the-selected-vectors-where-ab04e4 = Along the selected vectors, where they cross the picture
setting-dither-image-ce2904 = Dither image
setting-full-power-or-off-spread-529a58 = Full power or off, spread to give the tones, for materials that only burn or don't
setting-line-spacing-f3d3de = Line spacing
setting-between-lines-in-kerf-widths-60c1b7 = Between lines, in kerf widths (1 densest to 30 sparsest)
setting-the-angle-of-the-lines-77e56c = The angle of the lines: 0° along X, 90° along Y

## Array Copy Toolpath (array_copy_toolpath)
setting-what-the-toolpath-is-called-071ba9 =
    What the toolpath is called in the toolpath list. It copies the toolpaths ticked
    (shown) in the toolpath list when Apply is pressed: tick them there first
    (it is marked out of date when one of them changes: recalculate it then, and the
    hidden ones out of date are recalculated first). Those
    toolpaths are then hidden, so saving the shown toolpaths does not cut them twice
setting-rows-6592b8 = Rows
setting-how-many-copies-up-the-2a0848 =
    How many copies up the job, counting the first. Copies that would go past the
    material's edge are still made, with a warning saying how many rows and columns fit
setting-columns-156fdc = Columns
setting-how-many-copies-across-the-ae53ee = How many copies across the job, counting the first
setting-spacing-77939e = Spacing
setting-how-the-x-and-y-75a19e = How the X and Y distances are measured
setting-x-and-y-are-from-aee39c = X and Y are from each copy's start to the next one's
setting-gap-a1b96f = Gap
setting-x-and-y-are-the-156819 = X and Y are the gaps between the copies' extents
setting-the-distance-between-copies-across-ff7e75 = The distance between copies across the job
setting-the-distance-between-copies-up-25e799 = The distance between copies up the job
setting-only-cc6917 = Only
setting-set-on-each-array-toolpath-278a82 = Set on each array toolpath made: the one source it copies

## Merged Toolpath (merged_toolpath)
setting-what-the-toolpath-is-called-114e98 =
    What the toolpath is called in the toolpath list. It merges the toolpaths ticked
    (shown) in the toolpath list when Apply is pressed: tick them there first
    (it is marked out of date when one of them changes: recalculate it then, and the
    hidden ones out of date are recalculated first). Those
    toolpaths are then hidden, so saving the shown toolpaths does not cut them twice. They
    must use one tool, and all is cut with the first one's feeds and speed: a toolpath
    whose copy of that tool is set differently is warned of
setting-left-to-right-c1e1cf = Left to Right
setting-cuts-in-order-across-the-527674 =
    Cuts in order across the job; of the orders ticked, the quickest is kept. Whatever
    the order, the cuts inside a closed shape always come before the shape itself
setting-ordering-718d01 = Ordering
setting-bottom-to-top-860308 = Bottom to Top
setting-cuts-in-order-up-the-000cf0 = Cuts in order up the job
setting-cuts-in-rows-up-the-921bed = Cuts in rows up the job, across each row
setting-grid-rows-fb65ae = Grid rows
setting-how-many-rows-the-grid-29fb00 =
    How many rows the Grid order cuts the job into, bottom to top. More rows suit many
    small parts; fewer suit a few big ones
setting-shortest-path-eba746 = Shortest Path
setting-each-cut-the-nearest-to-ef1542 = Each cut the nearest to where the last ended
setting-merge-by-part-8d7cd9 = Merge By Part
setting-each-part-s-cuts-together-69258c =
    Each part's cuts together: a part is an outermost closed cut with the cuts inside
    it, cut in the toolpath list's order within the part (its outline always last)

## Ungroup Merged Toolpath (ungroup_merged_toolpath)
setting-the-merged-toolpath-to-take-ca3cc3 = The merged toolpath to take apart

## Inlay Toolpath (inlay_toolpath)
setting-inlay-type-ceab24 = Inlay type
setting-which-part-of-the-inlay-dee5e2 = Which part of the inlay this toolpath cuts
setting-inlay-type-a25f04 = Inlay Type
setting-male-straight-600116 = Male straight
setting-the-insert-cut-out-with-9717bd = The insert, cut out with straight sides
setting-male-stepped-e438c7 = Male stepped
setting-the-insert-with-a-shelf-a83585 = The insert with a shelf round its base (push-through letters)
setting-female-pocket-d42241 = Female pocket
setting-a-pocket-for-the-insert-5cf503 = A pocket for the insert
setting-female-hole-00d86d = Female hole
setting-a-hole-for-the-insert-50de53 = A hole for the insert to go through
setting-below-the-material-surface-where-1284bf = Below the material surface where cutting starts
setting-below-the-start-depth-right-8d69f2 =
    Below the start depth. Right through the material, an insert or hole's waste comes
    loose (inlays have no tabs): leave a thin skin, or hold the piece down
setting-step-depth-7a7c62 = Step depth
setting-stepped-insert-how-far-down-fff8d9 = Stepped insert: how far down from the start depth the step is
setting-steps-6b8bd4 = Steps
setting-step-width-e30967 = Step width
setting-stepped-insert-how-far-out-68e388 = Stepped insert: how far out from the shape the shelf reaches
setting-makes-an-insert-this-much-e4bb8d = Makes an insert this much smaller, or a cavity this much bigger, so they fit
setting-the-same-tool-for-both-59610b = The same tool for both halves

## Space Horizontally (space_horizontally)
setting-which-object-goes-where-along-5b28e2 = Which object goes where along the span.
setting-as-placed-17e2c6 = As placed
setting-keep-the-order-they-are-e2ebb8 = Keep the order they are in
setting-the-first-picked-on-the-cffac9 = The first picked on the left
setting-the-first-picked-on-the-132a6a = The first picked on the right
setting-inside-last-vector-62c262 = Inside last vector
setting-spread-them-inside-the-last-dc6692 = Spread them inside the last vector picked, which stays put

## Space Vertically (space_vertically)
setting-the-first-picked-at-the-04cc57 = The first picked at the bottom
setting-the-first-picked-at-the-76775f = The first picked at the top

## Scale Model Z Height (scale_model_z_height)
setting-the-model-s-new-height-ceb999 = The model's new height, from its lowest to its highest point

## Create Component from Bitmap (create_component_from_bitmap)
setting-what-the-new-component-is-903e0d = What the new component is called in the component tree
setting-max-height-b48b14 = Max height
setting-height-added-by-the-lightest-86f4b8 = Height added by the lightest (or darkest) parts
setting-base-height-d51c13 = Base height
setting-height-under-the-whole-picture-2403ea = Height under the whole picture
setting-white-high-329c4a = White high
setting-white-is-high-otherwise-black-af4a02 = White is high; otherwise black is

## Relief from Photo (relief_from_photo)
setting-depth-from-dbdf5a = Depth from
setting-heights-from-the-estimated-depth-4e1983 = Heights from the estimated depth, or from the picture itself when it is a depth map
setting-estimated-14d749 = Estimated
setting-the-depth-estimate-depth-worked-ea67ec = The depth Estimate Depth worked out for the photo
setting-the-picture-is-already-a-063e81 = The picture is already a depth map (white nearest), made elsewhere
setting-background-cut-off-ede194 = Background cut off
setting-background-by-colour-how-different-a35823 =
    Background by colour: how different from the photo's border a part must be to count
    as the subject. Raise it if bits of background are kept; 0 turns it off
setting-depth-cut-off-1003af = Depth cut off
setting-background-by-distance-everything-farther-2e2ec3 =
    Background by distance: everything farther back than this share of the way from
    the farthest point to the nearest is dropped. Raise it for a busy background; 0
    turns it off
setting-invert-fa82c1 = Invert
setting-near-and-far-swapped-for-e2e6db = Near and far swapped, for a depth map drawn with black nearest
setting-fine-detail-26cc16 = Fine detail
setting-fine-texture-hair-fur-scales-171b7c = Fine texture (hair, fur, scales) taken from the photo's light and shade. 0 for none
setting-sharpen-b59cd8 = Sharpen
setting-makes-the-relief-s-own-997ba9 = Makes the relief's own small details stand out more. 0 for none
setting-smoothness-111550 = Smoothness
setting-evens-out-speckle-while-keeping-c880af = Evens out speckle while keeping edges crisp. 0 for none
setting-how-high-the-nearest-point-17ecc1 = How high the nearest point stands
setting-lift-f5399c = Lift
setting-height-of-the-subject-s-cff831 = Height of the subject's lowest point, so it stands clear of the background
setting-soft-edge-005fca = Soft edge
setting-how-far-the-subject-s-e0bc62 =
    How far the subject's edge slopes down to the background instead of dropping
    straight down (easier to carve and to print). 0 for a straight drop
setting-a-flat-slab-under-the-a61125 = A flat slab under the whole picture, for a plaque or a printed base. 0 for none
setting-only-what-the-bit-can-bffb82 = Only what the bit can carve
setting-shows-the-relief-as-your-3ae69a =
    Shows the relief as your bit will really carve it: pits and grooves too narrow for
    the bit fill in, and the foot of a steep wall slopes at the bit's taper
setting-carving-bit-5a2f7c = Carving bit
setting-the-bit-you-will-carve-123dd2 =
    The bit you will carve it with, picked with Select… in the Tool Database: the same
    bit the 3D Finishing toolpath uses, and that Calculate checks can reach everything

## Generate Border (generate_border)
setting-what-the-border-is-called-4b1881 = What the border is called in the Library (a number is added if the name is taken)
setting-style-67ec76 = Style
setting-the-look-of-the-moulding-55375c = The look of the moulding
setting-classical-baroque-ae5fc9 = Classical Baroque
setting-ornate-acanthus-leaves-scrolls-and-9ce02a = Ornate acanthus leaves, scrolls and shell motifs, deeply carved
setting-modern-minimalist-34ad8f = Modern Minimalist
setting-clean-stepped-profiles-and-flat-f4adce = Clean stepped profiles and flat bands, with no ornament
setting-art-deco-0be93b = Art Deco
setting-geometric-fans-chevrons-and-stepped-e87a20 = Geometric fans, chevrons and stepped zig-zags
setting-rustic-vintage-4e1301 = Rustic Vintage
setting-weathered-wood-grain-knots-and-284e34 = Weathered wood grain, knots and simple beading
setting-victorian-carved-ac81ff = Victorian Carved
setting-rich-floral-garlands-ribbons-and-3e7632 = Rich floral garlands, ribbons and beaded edges
setting-contemporary-metallic-fbe56f = Contemporary Metallic
setting-sleek-brushed-ridges-and-fine-969ffd = Sleek brushed ridges and fine parallel grooves
setting-rococo-3febba = Rococo
setting-flowing-asymmetric-c-and-s-b02ed4 = Flowing asymmetric C- and S-scrolls with shells and flowers
setting-mid-century-modern-82489f = Mid-Century Modern
setting-simple-repeating-ovals-dots-and-2c6bd3 = Simple repeating ovals, dots and tapering lines
setting-gothic-pewter-eb80f2 = Gothic Pewter
setting-pointed-arches-tracery-and-quatrefoils-1d5364 = Pointed arches, tracery and quatrefoils
setting-your-own-description-fc0f54 = Your Own Description
setting-nothing-but-what-you-type-bc3666 = Nothing but what you type under Details
setting-details-c24245 = Details
setting-anything-to-add-to-the-623dbc =
    Anything to add to the style in your own words, such as "laurel leaves" or "a
    twisted rope". With Your Own Description, the whole description. Sent to Google
setting-the-tile-s-shape-how-89378a = The tile's shape: how tall it is for its length
setting-narrow-21-9-fade1d = Narrow (21:9)
setting-21-long-to-9-high-88e629 = 21 long to 9 high: a narrow moulding
setting-medium-16-9-6a3323 = Medium (16:9)
setting-16-long-to-9-high-df9e56 = 16 long to 9 high
setting-wide-4-3-4f5a78 = Wide (4:3)
setting-4-long-to-3-high-243c58 = 4 long to 3 high: a wide moulding
setting-length-5e1935 = Length
setting-one-repeat-end-to-end-234fc6 = One repeat, end to end. The border's height follows from its shape
setting-depth-70218a = Depth
setting-how-far-its-highest-point-55dbc4 = How far its highest point stands above its lowest
setting-where-the-depth-comes-from-7a69c1 = Where the depth comes from
setting-google-b85fa6 = Google
setting-google-turns-the-picture-it-3d40c9 = Google turns the picture it made into a depth map: it knows what it drew
setting-depth-add-on-e53579 = Depth add-on
setting-the-depth-add-on-works-576753 = The Depth add-on works it out on your own computer: free, and nothing more is sent
setting-depth-curve-1f32f1 = Depth curve
setting-below-1-the-middle-heights-240270 =
    Below 1 the middle heights rise for a fuller carving; above 1 they fall for crisper
    tops. 1 leaves them as they came
setting-smooths-the-steps-between-greys-a438d8 =
    Smooths the steps between greys so the carving isn't terraced, as a share of the
    border's height. 0 for none
setting-seam-blend-0743b4 = Seam blend
setting-how-far-in-from-each-a401d1 =
    How far in from each end the two ends are blended so repeats meet without a step, as
    a share of the length. 0 for none

## Smooth Selected Components (smooth_selected_components)
setting-passes-of-smoothing-more-is-35235d = Passes of smoothing: more is smoother

## Bitmap Properties (bitmap_properties)
setting-what-the-picture-is-called-8234df = What the picture is called
setting-where-the-picture-s-left-829b87 = Where the picture's left edge sits, along X
setting-where-the-picture-s-bottom-8b2b62 = Where the picture's bottom edge sits, along Y
setting-how-wide-the-picture-is-a5a50c = How wide the picture is on the job
setting-how-tall-the-picture-is-eb9ca4 = How tall the picture is on the job

## Draw Text (draw_text)
setting-plain-text-at-a-set-ee2b69 = Plain text at a set height, or text that grows or shrinks to fill a box
setting-text-142d5e = Text
setting-text-of-a-set-height-0aad52 = Text of a set height at the anchor point
setting-text-in-a-box-1acdab = Text in a box
setting-text-scaled-to-fit-a-faa404 = Text scaled to fit a box whose lower-left corner is the anchor point
setting-each-line-of-text-is-e857e4 = Each line of text is a line of the block
setting-font-7a5eb0 = Font
setting-single-line-fonts-vectric-font-a0021e =
    Single-line fonts (Vectric font `.vfont` files in Cut Craft's fonts folder) come first,
    then the system's TrueType fonts
setting-bold-768d96 = Bold
setting-truetype-fonts-only-f22427 = TrueType fonts only
setting-italic-b0c08d = Italic
setting-text-alignment-0ac0af = Text alignment
setting-how-the-lines-of-a-18b2a5 = How the lines of a block line up with each other and with the anchor.
setting-lines-line-up-on-their-7690f5 = Lines line up on their left ends
setting-each-line-centred-on-the-ae6422 = Each line centred on the others
setting-lines-line-up-on-their-94c2e4 = Lines line up on their right ends
setting-text-height-13a239 = Text height
setting-height-of-a-capital-letter-61de7c = Height of a capital letter
setting-anchor-point-10f6d2 = Anchor point
setting-where-the-text-sits-its-aec1c9 = Where the text sits: its lower-left corner. Click in the 2D view to place it
setting-bounding-box-dimensions-d46695 = Bounding box dimensions
setting-the-box-the-text-fills-cfe710 = The box the text fills: its width (X) and height (Y)
setting-margin-size-867ab0 = Margin size
setting-text-in-a-box-the-998588 = Text in a Box: the border left and right
setting-fill-the-box-s-width-12b314 = Fill the box's width or height
setting-normal-de2772 = Normal
setting-10-of-the-width-each-0e1df3 = 10 % of the width each side
setting-wide-501a0a = Wide
setting-20-each-side-d8f548 = 20 % each side
setting-vertical-stretch-a2fa76 = Vertical stretch
setting-text-in-a-box-how-c084a9 = Text in a Box: how text that fits the box's width fills the room above and below it.
setting-no-vertical-stretch-a99d79 = No vertical stretch
setting-the-text-keeps-its-shape-63321d = The text keeps its shape; the room above and below stays empty
setting-stretch-line-space-to-fit-bc0770 = Stretch line space to fit
setting-the-lines-move-apart-to-572672 = The lines move apart to fill the height
setting-stretch-characters-to-fit-96f3c4 = Stretch characters to fit
setting-the-letters-grow-taller-to-5ba448 = The letters grow taller to fill the height
setting-horizontal-stretch-a53508 = Horizontal stretch
setting-text-in-a-box-how-8af7d5 = Text in a Box: how text that fits the box's height fills the room at its sides.
setting-no-horizontal-stretch-e121e3 = No horizontal stretch
setting-the-text-keeps-its-shape-ba43e0 = The text keeps its shape; the room at the sides stays empty
setting-stretch-spaces-between-words-fd4ff8 = Stretch spaces between words
setting-the-spaces-between-words-grow-912d75 = The spaces between words grow
setting-stretch-kerning-9f3ad2 = Stretch kerning
setting-the-room-between-letters-grows-3da056 = The room between letters grows
setting-stretch-character-size-d1744a = Stretch character size
setting-the-letters-grow-wider-b5ff33 = The letters grow wider
setting-arc-angle-7abd0c = Arc angle
setting-text-the-line-bent-into-a3e712 =
    Text: the line bent into an arc, the angle the arc turns through; above zero it bends
    up, below zero down. Drag the green handles above and below the text to set it
setting-arc-rotation-29a463 = Arc rotation
setting-text-on-an-arc-the-40ea0f = Text on an arc: the arc turned about its centre, anticlockwise

## Break Block into Lines (break_text_into_lines)
setting-the-text-group-9a6f10 = The text group

## Fit Curves to Vectors (fit_curves_to_vectors)
setting-fitting-type-e0f6e8 = Fitting type
setting-what-the-new-vectors-are-2bd4e2 = What the new vectors are made of
setting-circular-arcs-4a7519 = Circular Arcs
setting-arcs-of-circles-which-machines-3655f6 = Arcs of circles, which machines cut smoothly
setting-bezier-curves-7ed7c2 = Bezier Curves
setting-free-flowing-curves-best-for-fea01c = Free-flowing curves, best for organic shapes
setting-straight-lines-e95fdc = Straight Lines
setting-short-straight-lines-only-be3088 = Short straight lines only
setting-the-new-vectors-stay-within-6dba70 = The new vectors stay within this distance of the old
setting-keep-sharp-corners-ace80e = Keep sharp corners
setting-corners-turning-more-than-the-7df9a3 = Corners turning more than the max angle stay sharp
setting-max-angle-28fbe6 = Max. angle
setting-with-keep-sharp-corners-ticked-ebaac4 = With Keep Sharp Corners ticked: corners turning more than this stay sharp
setting-replace-selected-vectors-5ac1fc = Replace selected vectors
setting-delete-the-originals-otherwise-the-79f8c9 = Delete the originals; otherwise the fitted vectors are added beside them

## Trace Bitmap (trace_bitmap)
setting-trace-type-70568a = Trace type
setting-round-each-shape-s-outline-6fb758 = Round each shape's outline, or one line down the middle of each stroke
setting-closed-shapes-round-each-dark-cb6b51 = Closed shapes round each dark area, for cutting out, pocketing or carving
setting-centerline-31d296 = Centerline
setting-one-line-down-the-middle-8aa8a1 =
    One line down the middle of each stroke, for engraving or V-carving a drawing,
    signature or lettering along its lines
setting-how-the-picture-is-read-9d3ea2 = How the picture is read (black and white, or by colour).
setting-black-and-white-222c90 = Black and white
setting-the-picture-is-read-as-656767 = The picture is read as dark and light only
setting-colors-bb4ed1 = Colors
setting-a-colour-picture-cut-down-40c527 = A colour picture cut down to a few colours
setting-color-similarity-287a5f = Color similarity
setting-colours-closer-than-this-count-8e660f =
    Colours closer than this count as one: 0 keeps every colour apart, higher joins more
    of them
setting-threshold-897598 = Threshold
setting-grey-levels-darker-than-this-3dad28 = Grey levels darker than this count as black (0 black … 255 white)
setting-trace-color-ef81bd = Trace color
setting-which-pixels-are-traced-in-5ac79c = Which pixels are traced (in black and white mode).
setting-black-043744 = Black
setting-trace-the-dark-parts-of-51259b = Trace the dark parts of the picture
setting-white-eb2d06 = White
setting-trace-the-light-parts-of-461899 = Trace the light parts of the picture
setting-corner-fit-7b16ff = Corner fit
setting-loose-1-rounds-more-corners-df20a5 = Loose (1) rounds more corners off; tight (10) keeps more of them sharp
setting-noise-filter-5c1ecd = Noise filter
setting-regions-smaller-than-this-many-53847f = Regions smaller than this many pixels across are left out
setting-group-vectors-b484d0 = Group vectors
setting-make-the-traced-vectors-one-f1ccb4 = Make the traced vectors one group

## Replace Below (replace_below)
setting-replace-value-below-b3a04f = Replace value below
setting-the-component-s-own-heights-8aae71 = The component's own heights lower than this are replaced
setting-replace-value-with-5ca87e = Replace value with
setting-what-happens-to-the-parts-03d0e2 = What happens to the parts lower than that height
setting-transparent-69cdd1 = Transparent
setting-remove-those-parts-e3bf32 = Remove those parts
setting-plane-c582f5 = Plane
setting-flatten-them-to-the-height-f3dd7c = Flatten them to the height
setting-lower-component-by-value-f68937 = Lower component by value
setting-drop-what-is-left-by-83fc80 = Drop what is left by the height, so it starts at the zero plane

## Offset Model (offset_model)
setting-offset-distance-9df765 = Offset distance
setting-clip-z-values-to-zero-a686f2 = Clip Z values to zero plane
setting-nothing-ends-up-below-the-7d0e09 = Nothing ends up below the zero plane

## Slice Model (slice)
setting-model-slicing-a2308f = Model slicing
setting-how-the-model-s-thickness-a031dc = How the model's thickness is shared out between the slices
setting-slice-thickness-4411db = Slice thickness
setting-slices-of-the-given-thickness-ed8248 = Slices of the given thickness; the last takes what is left
setting-number-of-slices-27be78 = Number of slices
setting-the-model-s-thickness-shared-7273b0 = The model's thickness shared between this many slices
setting-custom-slice-list-829438 = Custom slice list
setting-each-slice-its-own-thickness-fea3ac = Each slice its own thickness, from the list; what is left over is one more slice
setting-how-thick-each-slice-is-08b88d = How thick each slice is, with Slice Thickness
setting-from-top-66b1ae = From top
setting-slice-from-the-top-down-db6ad7 = Slice from the top down, so the thinner last slice is at the bottom
setting-how-many-equal-slices-with-3340c8 = How many equal slices, with Number of Slices
setting-create-boundary-vectors-f319a1 = Create boundary vectors
setting-a-closed-vector-round-each-41c0a2 = A closed vector round each slice
setting-slice-thicknesses-dc4f4b = Slice thicknesses
setting-custom-slice-list-each-slice-df61e6 =
    Custom Slice List: each slice's thickness, in order (from the top when slicing from
    the top)

## Emboss Component (emboss)
setting-scale-height-f6586c = Scale height
setting-the-broad-shape-s-new-c6a6d2 = The broad shape's new height, as a share of its height now
setting-detail-smoothness-f80171 = Detail smoothness
setting-passes-of-smoothing-over-the-6b423f = Passes of smoothing over the result: more for a larger scale

## Add Draft to Model (add_draft)
setting-draft-angle-96139f = Draft angle
setting-the-least-angle-a-wall-5d746c = The least angle a wall leans from vertical

## Create Cross Section Vector (cross_section)
setting-where-the-section-line-starts-7018a5 = Where the section line starts; drag across the model to set both ends

## Notes (notes)
setting-notes-kept-with-the-job-e5eee2 =
    Notes kept with the job: rich text with headings, lists, check boxes and links, and live
    values from the job, the machine and the toolpaths, shown on the job sheet and, if you
    like, at the top of the G-code.
setting-notes-f01b6a = Notes
setting-the-notes-written-like-markdown-1a582f =
    The notes, written like Markdown: `# heading`, `- bullet`, `1. numbered`,
    `- [ ] check box`, `**bold**`, `*italic*`, `[words](https://…)` and `{"{"}{"{"}variable{"}"}{"}"}`.
    To have notes open when the file is loaded, start them with a "."
setting-put-these-notes-at-the-d54ca2 = Put these notes at the top of the G-code
setting-put-the-notes-variables-filled-94a5ee = Put the notes, variables filled in, at the top of the G-code as comments

## Create Texture Area (texture_area)
setting-spacing-x-f09646 = Spacing x
setting-gap-between-tiles-across-as-aa9c3d = Gap between tiles across, as a share of the tile's width; negative overlaps them
setting-spacing-y-f097d9 = Spacing y
setting-gap-between-rows-as-a-09b3b3 = Gap between rows, as a share of the tile's height
setting-shift-x-5bbf07 = Shift x
setting-every-other-row-moves-across-c59745 = Every other row moves across by this share of the tile's width
setting-shift-y-5bbd74 = Shift y
setting-every-other-column-moves-up-b8d66a = Every other column moves up by this share of the tile's height
setting-reflection-1574c4 = Reflection
setting-lower-left-lower-right-upper-994d9f = Lower left, lower right, upper left, upper right tile of each group of four
setting-the-tile-as-it-is-31310b = The tile as it is
setting-the-tile-flipped-left-to-8a7070 = The tile flipped left to right
setting-the-tile-flipped-top-to-1ac79b = The tile flipped top to bottom
setting-the-tile-flipped-both-ways-f54aa5 = The tile flipped both ways

## Create Decorative Relief (decorative_relief)
setting-the-two-fills-composed-of-3521ee = The two fills composed of whole scrolls, which can also be carved as a relief.
setting-baroque-98ad88 = Baroque
setting-broad-c-and-s-scrolls-39c042 =
    Broad C and S scrolls wrapped in lobed acanthus leaves, as Dutch and French carving,
    mirrored left to right
setting-western-leaf-c3e6cb = Western leaf
setting-western-sheridan-style-leaf-curling-132c64 = Western (Sheridan-style) leaf: curling bands, broad wavy leaves and layered flowers
setting-how-high-the-carving-stands-52c588 = How high the carving stands at its highest point, above the material it sits on
setting-pattern-size-830b2a = Pattern size
setting-how-big-a-scroll-is-93bca5 = How big a scroll is at its smallest: bigger makes fewer, bolder scrolls
setting-finishing-bit-c3af95 = Finishing bit
setting-the-ball-nose-bit-you-b95b00 =
    The ball-nose bit you will finish with. Nothing in the carving is finer than it, so it
    carves what you see: 1/8 inch is 3.175 mm
setting-echo-lines-1492c7 = Echo lines
setting-carved-lines-inside-each-leaf-d29339 =
    Carved lines inside each leaf and scroll that follow its edge, evenly spaced to its
    middle, as carvers echo a leaf's outline; left out where a piece is too narrow
setting-variation-cc7ade = Variation
setting-another-seed-gives-another-carving-5ee5a0 = Another seed gives another carving with the same settings
setting-the-name-the-carving-goes-db233d = The name the carving goes by in the list of 3D components

## Array Copy (array_copy)
setting-copies-up-the-job-the-041fb3 = Copies up the job, the original's row included
setting-copies-across-the-job-the-279037 = Copies across the job, the original's column included
setting-whether-x-and-y-below-251918 = Whether X and Y below are the gaps between copies or the distance between their starts
setting-x-and-y-are-the-64c980 = X and Y are the space between neighbouring copies
setting-x-and-y-are-from-e74621 = X and Y are from one copy's start to the next one's
setting-across-the-distance-between-copies-f6cb2c = Across: the distance between copies side by side
setting-up-the-distance-between-rows-e20ff7 = Up: the distance between rows of copies
setting-symmetry-2f8433 = Symmetry
setting-how-the-copies-in-each-e42de2 =
    How the copies in each 2 × 2 block of the grid are turned: the original's corner as it
    is, the others mirrored or turned about their own centres.
setting-every-copy-as-the-original-ceeb8b = Every copy as the original
setting-mirror-columns-e3e3bd = Mirror columns
setting-every-other-column-mirrored-left-d04252 = Every other column mirrored left to right
setting-mirror-rows-fcd96b = Mirror rows
setting-every-other-row-mirrored-top-2b6f67 = Every other row mirrored top to bottom
setting-mirror-both-704e6d = Mirror both
setting-both-a-four-way-mirrored-0e374a = Both: a four-way mirrored block
setting-rotate-0a0a2a = Rotate
setting-every-other-copy-chequered-turned-60f80a = Every other copy, chequered, turned half round
setting-row-displacement-79cd0a = Row displacement
setting-every-other-row-moved-along-b2c43f = Every other row moved along X by this
setting-column-displacement-ed3e36 = Column displacement
setting-every-other-column-moved-along-4ebe0a = Every other column moved along Y by this
setting-group-copies-064565 = Group copies
setting-the-original-and-its-copies-4836f1 =
    The original and its copies made one group. Off, each copy of a group (a word of
    text, say) is a group of its own

## Circular Copy (circular_copy)
setting-how-many-in-all-the-a42aea = How many in all, the original included
setting-around-249cac = Around
setting-the-rotation-centre-6ed80d = The rotation centre
setting-selection-centre-cf156c = Selection centre
setting-the-middle-of-the-selection-a61a12 = The middle of the selection: move the centre to spread the copies out
setting-point-a8ef31 = Point
setting-the-point-x-y-01ffd5 = The point X, Y
setting-of-the-centre-when-copying-a820d3 = Of the centre, when copying around a point
setting-of-the-centre-when-copying-7b52c4 = Of the centre, when copying around a point: up the job
setting-angle-mode-3443db = Angle mode
setting-whether-the-angle-below-is-ec9a72 = Whether the angle below is shared out among the copies or is the step between two
setting-total-066d9d = Total
setting-the-angle-is-shared-out-16d30e = The angle is shared out evenly between the items
setting-step-71f92f = Step
setting-the-angle-from-one-item-1d930f = The angle from one item to the next
setting-positive-places-the-copies-clockwise-b443b8 = Positive places the copies clockwise, negative counter-clockwise
setting-rotate-copies-6be033 = Rotate copies
setting-turn-each-copy-to-match-cd67a7 =
    Turn each copy to match its place on the circle; off, copies keep the original's
    orientation

## Copy Along Vectors (copy_along_vectors)
setting-copy-8f3664 = Copy
setting-the-first-object-picked-or-aef6fb = The first object picked, or circles
setting-copy-object-26f4bb = Copy Object
setting-the-first-object-picked-with-ed53f2 = The first object picked, with its group
setting-copy-circles-fc80e9 = Copy Circles
setting-circles-of-a-diameter-every-189b41 = Circles of a diameter: every selected vector is one to copy them along
setting-circle-diameter-5b44c4 = Circle diameter
setting-copy-circles-their-diameter-e8c010 = Copy Circles: their diameter
setting-space-the-copies-by-how-65e5e1 = Space the copies by how many there are, or by the distance between them
setting-count-e7b894 = Count
setting-a-number-of-copies-spread-09fb09 = A number of copies, spread evenly from end to end (all round, on a closed vector)
setting-distance-along-the-vector-from-2b47a4 = Distance along the vector from one copy to the next, starting at its start
setting-copies-c22130 = Copies
setting-how-many-copies-on-each-74f713 = How many copies on each vector, when spacing by count
setting-between-copies-when-spacing-by-95d045 = Between copies, when spacing by distance
setting-force-even-spacing-8b05f9 = Force even spacing
setting-spacing-by-distance-stretch-or-059af8 =
    Spacing by distance: stretch or shrink the distance a little so copies land on both
    ends
setting-align-to-vector-fb8f9a = Align to vector
setting-turn-each-copy-to-follow-1cd995 = Turn each copy to follow the vector, as if the object travelled along it facing +X
setting-reverse-direction-301ed8 = Reverse direction
setting-run-along-each-vector-from-95f2a3 = Run along each vector from its end instead of its start
setting-create-copies-on-new-layer-8bea00 = Create copies on new layer
setting-put-the-copies-on-a-4d6678 = Put the copies on a new layer of their own

## Vector Validator (vector_validator)
setting-overlapping-4f8a80 = Overlapping
setting-vectors-lying-on-top-of-e6175e = Vectors lying on top of another one
setting-intersections-6ca801 = Intersections
setting-vectors-crossing-themselves-or-each-204ad7 = Vectors crossing themselves or each other
setting-zero-length-spans-3e11ac = Zero length spans
setting-spans-or-whole-vectors-shorter-cd84b8 = Spans, or whole vectors, shorter than the tolerance
setting-open-8b10e9 = Open
setting-open-vectors-they-cannot-be-797eb5 = Open vectors: they cannot be pocketed or modelled
setting-fix-zero-length-spans-4b693f = Fix zero length spans
setting-instead-of-selecting-problems-remove-c7e6c5 = Instead of selecting problems, remove the zero-length spans from the vectors checked
setting-v-carving-mode-f30687 = V-Carving Mode
setting-checks-as-v-carving-needs-4cea52 =
    Checks as V-carving needs them: crossings and overlaps within one text are let be (a
    font's letters often overlap, and V-carving cuts them fine), and open vectors are
    found, since V-carving needs closed ones
setting-how-close-counts-as-the-417ae6 = How close counts as the same point

## Close / Extend Vector (close_extend_vector)
setting-how-each-vector-is-closed-9e2ff5 = How each vector is closed
setting-a-straight-line-from-the-458f3d = A straight line from the end back to the start
setting-extend-8c0169 = Extend
setting-lengthen-the-first-and-last-4c5783 =
    Lengthen the first and last lines until they meet; when they never do, close with a
    straight line
setting-a-smooth-curve-leaving-the-b3c021 = A smooth curve leaving the end and reaching the start along their own directions
setting-common-point-dc1312 = Common point
setting-move-the-start-and-the-ed25fe = Move the start and the end to the point halfway between them
setting-max-gap-9a516d = Max gap
setting-only-close-vectors-whose-ends-512db4 = Only close vectors whose ends are at most this far apart; 0 closes any

## Create Fillets (create_fillets)
setting-kind-f50c63 = Kind
setting-what-is-cut-into-each-5d9523 = What is cut into each corner
setting-a-rounded-corner-an-arc-a219c6 = A rounded corner: an arc tangent to both sides
setting-dog-bone-7d1c59 = Dog bone
setting-a-cutter-sized-circle-into-6c1870 = A cutter-sized circle into the corner, along the line that halves it
setting-t-bone-cd092b = T bone
setting-a-cutter-sized-notch-into-8dc623 = A cutter-sized notch into the longer side of the corner
setting-plasma-drag-knife-8dad96 = Plasma drag knife
setting-the-sides-run-on-past-71b670 =
    The sides run on past the corner into a loop outside it, so a plasma torch or drag
    knife turns there instead of on the corner
setting-of-the-fillet-or-of-b60539 = Of the fillet, or of the cutter for dog-bones and T-bones
setting-open-vectors-have-no-inside-6e9920 = Open vectors have no inside, so they get every corner
setting-all-afe364 = All
setting-every-sharp-corner-f3a915 = Every sharp corner
setting-outer-7a8214 = Outer
setting-corners-pointing-out-of-a-b7b616 = Corners pointing out of a closed shape: those a cutter inside a pocket cannot reach
setting-inner-9b6ec7 = Inner
setting-corners-pointing-into-a-closed-4bc58f = Corners pointing into a closed shape: those a cutter outside the shape cannot reach

## Dimensions (dimensions)
setting-what-to-measure-a-length-903816 = What to measure: a length, a horizontal or vertical distance, an angle or an arc
setting-length-dimension-ec84a9 = Length Dimension
setting-the-straight-distance-in-any-63dc11 = The straight distance, in any direction
setting-vertical-dimension-3beaa7 = Vertical Dimension
setting-the-up-and-down-distance-4fb87b = The up-and-down distance only, however far apart the points are sideways
setting-horizontal-dimension-8df9ad = Horizontal Dimension
setting-the-side-to-side-distance-69a105 = The side-to-side distance only, however far apart the points are up and down
setting-angle-dimension-6c9814 = Angle Dimension
setting-the-angle-at-a-corner-e9913d = The angle at a corner: drag from the corner to where the arc and its text go
setting-arc-dimension-039407 = Arc Dimension
setting-an-arc-s-or-circle-60fe2a = An arc's or circle's radius, or its diameter: drag from the arc to where the text goes
setting-drag-from-one-point-to-5b4b95 = Drag from one point to the other to set both
setting-the-second-point-measured-9149ae = The second point measured
setting-the-lettering-the-measurement-is-d99cfd = The lettering the measurement is written in
setting-how-tall-the-measurement-s-32de4e = How tall the measurement's letters are
setting-decimal-places-abafc4 = Decimal places
setting-how-many-digits-after-the-830873 = How many digits after the decimal point the measurement shows
setting-how-far-the-dimension-line-e544a6 = How far the dimension line sits from the measured points, to their left
setting-show-diameter-24e137 = Show diameter
setting-arc-dimensions-the-diameter-instead-da90aa = Arc dimensions: the diameter instead of the radius
setting-use-custom-text-467e4e = Use custom text
setting-shown-instead-of-the-measurement-bdd352 = Shown instead of the measurement when not empty

## Turn and Spin (turn_and_spin)
setting-type-of-shape-91817b = Type of shape
setting-how-the-profile-is-swept-8823c3 = How the profile is swept round to make the shape
setting-turn-f2879a = Turn
setting-turn-rotate-round-the-line-ee5f59 = Turn (rotate) round the line from the start point to the end point
setting-spin-bb8d39 = Spin
setting-spin-round-the-left-end-bc4308 = Spin round the left end point
setting-scale-to-exact-height-2cfb58 = Scale to exact height
setting-scale-the-shape-so-its-ff9589 = Scale the shape so its highest point is the height
setting-the-highest-point-of-the-1230b2 = The highest point of the shape, with Scale to exact height
setting-combine-with-other-components-f53cb0 = Combine with other components

## Extrude and Weave (extrude_and_weave)
setting-drive-rails-04366e = Drive rails
setting-how-many-of-the-vectors-efead6 = How many of the vectors selected first are drive rails
setting-sweep-ba0d83 = Sweep
setting-what-is-swept-along-the-5531a7 = What is swept along the rails.
setting-use-vector-cross-sections-cd08dd = Use vector cross sections
setting-the-open-vectors-selected-after-e9e251 = The open vectors selected after the rails
setting-use-a-component-cbf300 = Use a component
setting-the-selected-component-its-x-439c1a = The selected component, its X along the rail
setting-how-a-component-follows-a-c21d7a = How a component follows a rail.
setting-stretch-to-drive-rail-length-0efa77 = Stretch to drive rail length
setting-one-copy-stretched-along-the-6b50ff = One copy stretched along the whole rail
setting-fill-with-multiple-copies-dad9bd = Fill with multiple copies
setting-copies-at-its-own-size-fc55a1 = Copies at its own size, one after another
setting-each-copy-overlaps-the-one-67080e = Each copy overlaps the one before by this share of its length
setting-flip-alternate-copies-d42b9d = Flip alternate copies
setting-every-other-copy-mirrored-across-cec89d = Every other copy mirrored across the rail
setting-corners-in-the-rails-make-cc0bda = Corners in the rails make sharp corners in the shape, not rounded ones
setting-sweep-between-spans-24e312 = Sweep between spans
setting-between-cross-sections-with-as-495924 =
    Between cross-sections with as many nodes as each other, each node flows to the same
    node of the next
setting-weave-under-over-at-crossings-bf7b8a = Weave under/over at crossings
setting-where-rails-cross-one-goes-3ee890 = Where rails cross, one goes over and the other under, alternately along each rail
setting-weave-by-605a12 = Weave by
setting-how-a-weave-raises-and-a27c9c = How a weave raises and lowers the shape where rails cross.
setting-scale-shape-1a930c = Scale shape
setting-the-shape-s-height-scaled-6f2cdb = The shape's height scaled by the percentages
setting-add-base-df9d89 = Add base
setting-the-shape-kept-lifted-on-bb3241 = The shape kept, lifted on a base of the percentages of its height
setting-z-under-7e27e3 = Z Under
setting-where-rails-cross-the-height-4a33a3 = Where rails cross, the height of the one going under, as a percentage of its own
setting-z-over-edbe8d = Z Over
setting-where-rails-cross-the-height-cc5159 = Where rails cross, the height of the one going over, as a percentage of its own

## Two Rail Sweep (two_rail_sweep)
setting-reverse-rail-1-direction-01b235 = Reverse rail 1 direction
setting-run-along-the-first-rail-f61fff = Run along the first rail from its other end
setting-reverse-rail-2-direction-98f008 = Reverse rail 2 direction
setting-run-along-the-second-rail-322955 = Run along the second rail from its other end
setting-scale-cross-section-with-width-243836 = Scale cross section with width
setting-the-cross-section-s-height-74f981 = The cross-section's height grows and shrinks with the gap between the rails
setting-swap-rail-order-7c2232 = Swap rail order
setting-the-second-rail-taken-first-44b487 = The second rail taken first, so the cross-sections hang the other way
setting-add-to-all-rail-nodes-2dbd0b = Add to all rail nodes
setting-the-rails-nodes-joined-in-c78eee =
    The rails' nodes joined in pairs, first to first and so on, so corners meet corners
    (the rails need as many nodes as each other)
setting-fill-center-of-inner-closed-1a3804 = Fill center of inner closed vector rails
setting-closed-rails-the-inside-of-c9b3d5 =
    Closed rails: the inside of the inner one filled to the height the cross-sections
    reach at it

## Nest Parts (nest_parts)
setting-tool-diameter-c4563a = Tool diameter
setting-the-cut-out-tool-the-424d46 = The cut-out tool: the least gap left between parts
setting-clearance-d438a5 = Clearance
setting-added-to-the-tool-diameter-14a703 = Added to the tool diameter between parts, and to the border
setting-border-gap-2d43df = Border gap
setting-room-left-clear-round-the-1ad13d = Room left clear round the edge of the material
setting-rotate-parts-to-find-best-04fbcd = Rotate parts to find best fit
setting-turn-parts-a-quarter-turn-66c6b4 = Turn parts a quarter turn where that fits better
setting-remove-original-parts-0d5e16 = Remove original parts
setting-move-the-parts-otherwise-copies-2ec979 = Move the parts; otherwise copies are nested and the originals stay
setting-nest-from-4565e9 = Nest from
setting-the-corner-of-the-material-60f12d = The corner of the material the parts are packed into first
setting-start-in-the-bottom-left-580716 = Start in the bottom left corner
setting-start-in-the-bottom-right-4ea9db = Start in the bottom right corner
setting-start-in-the-top-left-ef3104 = Start in the top left corner
setting-start-in-the-top-right-c338b1 = Start in the top right corner
setting-nest-direction-521006 = Nest direction
setting-which-way-the-rows-of-f7d6a7 = Which way the rows of parts run from that corner
setting-along-x-5c38b2 = Along x
setting-rows-run-across-the-material-212c69 = Rows run across the material, side to side
setting-along-y-5c3a45 = Along y
setting-rows-run-up-the-material-43e9ed = Rows run up the material, bottom to top
setting-true-shape-544c88 = True shape
setting-nest-by-the-parts-own-2ce271 =
    Nest by the parts' own shapes, not their boxes: parts fit round each other and into
    each other's holes
setting-rotation-step-854697 = Rotation step
setting-true-shape-the-steps-parts-6f9630 = True shape: the steps parts are turned in to fit (0: quarter turns when rotating)
setting-copies-of-each-part-eb64df = Copies of each part
setting-true-shape-how-many-of-cb5ec7 = True shape: how many of each part to nest
setting-mirror-parts-to-find-best-10e1d5 = Mirror parts to find best fit
setting-true-shape-also-try-each-f4c8a4 =
    True shape: also try each part flipped over, where that fits better; only when the
    direction the parts are cut in does not matter
setting-allow-parts-inside-other-parts-96e51c = Allow parts inside other parts
setting-true-shape-smaller-parts-go-37ce94 = True shape: smaller parts go into the holes of bigger ones
setting-nest-boundary-layer-be1542 = Nest boundary layer
setting-true-shape-the-layer-whose-dd93e2 =
    True shape: the layer whose closed vectors are the area to nest inside (holes in
    them are kept clear); empty nests inside the job
setting-filler-part-layer-4e6e41 = Filler part layer
setting-true-shape-the-layer-whose-7e5a6b =
    True shape: the layer whose parts fill what space is left, as many as fit; empty for
    none
setting-add-sheets-as-needed-3a33c9 = Add sheets as needed
setting-true-shape-parts-that-do-2581bb = True shape: parts that do not fit go onto new sheets the size of this one
setting-nest-two-sided-parts-a9d293 = Nest two-sided parts
setting-true-shape-double-sided-jobs-80ca1b = True shape, double-sided jobs: what is on the other side under a part goes with it

## Text on a Curve (text_on_a_curve)
setting-keep-the-letters-their-own-2cb010 = Keep the letters their own size, or grow them to run the whole curve
setting-maintain-text-size-52ac10 = Maintain text size
setting-the-letters-stay-the-size-b17836 = The letters stay the size they are
setting-scale-text-to-fill-curve-4a0e75 = Scale text to fill curve
setting-the-letters-grow-or-shrink-b61f07 = The letters grow or shrink so the text runs the curve's whole length
setting-text-spacing-f8cac7 = Text spacing
setting-spacing-between-letters-as-a-ce3260 =
    Spacing between letters, as a share of their normal spacing: under 100% closes them
    up, over 100% spreads them out. 50% is the closest they go
setting-text-position-841245 = Text position
setting-which-side-of-the-curve-e8c845 = Which side of the curve the letters stand on
setting-above-curve-37d411 = Above curve
setting-the-letters-stand-on-top-41c56e = The letters stand on top of the curve
setting-on-curve-4e0d2b = On curve
setting-the-curve-runs-through-the-1082e9 = The curve runs through the middle of the letters
setting-below-curve-3524f9 = Below curve
setting-the-letters-hang-below-the-c2dea2 = The letters hang below the curve
setting-text-on-other-side-2614c0 = Text on other side
setting-put-the-text-on-the-2465e8 = Put the text on the curve's other side
setting-gap-between-the-curve-and-50c3fc = Gap between the curve and the text
setting-where-along-the-curve-the-a8783e = Where along the curve the text starts
setting-the-text-starts-at-the-127703 = The text starts at the curve's start
setting-middle-92c238 = Middle
setting-the-text-is-centred-along-7df0f5 = The text is centred along the curve
setting-the-text-ends-at-the-31000b = The text ends at the curve's end
setting-letters-7c290a = Letters
setting-whether-each-letter-leans-to-8951ac = Whether each letter leans to follow the curve or stays upright
setting-align-to-curve-87716c = Align to curve
setting-each-letter-leans-with-the-66e363 = Each letter leans with the curve
setting-keep-vertical-c2cd02 = Keep vertical
setting-every-letter-stays-upright-as-6e2172 = Every letter stays upright, as on the page

## Move Node (move_node)
setting-the-node-to-move-02e982 = The node to move
setting-vector-50ad22 = Vector
setting-the-vector-being-edited-the-b11c0a = The vector being edited: the one you click on
setting-at-d563a8 = At
setting-reach-b8c218 = Reach
setting-how-close-the-click-must-3cb5e1 = How close the click must be to the node or span
setting-where-it-goes-56ac09 = Where it goes

## Move Handle (move_handle)
setting-move-handle-s-settings-which-acfdf5 = Move Handle's settings: which handle, and where it goes.
setting-the-handle-to-move-the-f01c52 = The handle to move: the one of `vector` nearest `at`, within `reach`

## Insert Point (insert_point)
setting-where-a-node-editing-command-023736 =
    Where a node-editing command acts: the node or span of `vector` nearest `at`, within
    `reach` (the 2D view's pick distance).

## Interactive Vector Trim (interactive_vector_trim)
setting-how-near-the-click-must-54924d = How near the click must be to a vector
setting-rejoin-trimmed-sections-f4cc6e = Rejoin trimmed sections
setting-join-the-pieces-left-by-02091d = Join the pieces left by trimming where they meet end to end

## Create Vector Texture (vector_texture)
setting-what-fills-the-shapes-4bd1a6 = What fills the shapes.
setting-wavy-lines-9d5cab = Wavy lines
setting-rows-of-wavy-lines-at-f6b95e = Rows of wavy lines at an angle
setting-floral-e3087b = Floral
setting-curling-vines-with-leaves-and-bd7a3d = Curling vines with leaves and flowers
setting-western-sheridan-style-leaf-curling-e54613 =
    Western (Sheridan-style) leaf: curling bands, broad wavy leaves with veins and layered
    flowers, as tooled leather
setting-frost-ce5e6d = Frost
setting-ice-ferns-growing-in-from-e7dd82 = Ice ferns growing in from the edge, as frost on a window
setting-snowflakes-03311c = Snowflakes
setting-six-armed-snowflakes-of-different-4a34a4 = Six-armed snowflakes of different sizes
setting-fractal-601eda = Fractal
setting-branching-lines-that-fork-again-b976bc = Branching lines that fork again and again, as fractal (Lichtenberg) burning
setting-stones-fb6ccb = Stones
setting-rounded-stones-with-a-gap-51a271 = Rounded stones with a gap between, as cobbles
setting-ripples-fb160c = Ripples
setting-lines-following-the-edge-each-dd73dc = Lines following the edge, each a step further in
setting-hammered-e05a66 = Hammered
setting-round-dimples-of-a-few-434923 = Round dimples of a few sizes, as hammered metal
setting-bandanna-61b3c4 = Bandanna
setting-paisley-teardrops-with-flowers-and-267cae = Paisley teardrops with flowers and dots between, as on a bandanna
setting-the-lines-angle-from-the-e1a287 = The lines' angle from the X axis
setting-max-spacing-e8545c = Max. spacing
setting-the-widest-gap-between-neighbouring-ef7802 = The widest gap between neighbouring lines
setting-spacing-variation-2ea2eb = Spacing variation
setting-0-keeps-the-lines-evenly-17b9bc = 0 keeps the lines evenly spaced; 100 lets the gaps run from nothing to the maximum
setting-wavelength-f36718 = Wavelength
setting-how-far-each-wave-runs-8f3ada = How far each wave runs before it repeats: longer gives gentler waves
setting-amplitude-035342 = Amplitude
setting-how-far-each-wave-swings-20c207 = How far each wave swings to either side: 0 gives straight lines
setting-noise-2ecf71 = Noise
setting-randomness-in-the-waves-a04129 = Randomness in the waves
setting-how-big-the-pattern-s-2127b9 = How big the pattern's parts are: a scroll's curl, a snowflake, a stone
setting-smallest-gap-425130 = Smallest gap
setting-the-least-room-between-two-0b14f5 =
    The least room between two lines, and between a line and the edge. Keep it wider than
    your bit cuts, or neighbouring cuts run together: a 60° V-bit 1 mm deep cuts about
    1.2 mm wide
setting-another-seed-gives-another-pattern-f2c886 = Another seed gives another pattern with the same settings
setting-place-vectors-on-layer-124024 = Place vectors on layer
setting-put-the-lines-on-the-a7f39b = Put the lines on the layer named below (made if the job has none of that name)
setting-the-layer-the-lines-are-9662d7 = The layer the lines are put on

## Embellish (embellish)
setting-the-one-look-it-has-288777 =
    The one look it has. A closed shape is filled with it; an open line has it run
    along the line
setting-folk-mix-2940c5 = Folk mix
setting-a-band-and-bead-row-207717 =
    A band and bead row round the edge, and inside it a vine with scrolls, flowers, leaf
    fans and small curls together (along a line, the vine and its scrolls)
setting-scrolls-58222d = Scrolls
setting-a-vine-down-the-middle-78fcd3 = A vine down the middle with scrolls curling off it, and small S-curls
setting-flowers-8013cf = Flowers
setting-rosettes-where-there-is-most-9f7112 = Rosettes where there is most room, and smaller flowers round them
setting-band-and-beads-9983de = Band and beads
setting-a-rounded-band-with-a-450852 = A rounded band with a row of beads beside it
setting-rope-70ee4d = Rope
setting-slanted-strands-laid-side-by-12629a = Slanted strands laid side by side
setting-chain-ec1176 = Chain
setting-oval-links-seen-face-on-211da1 = Oval links seen face on, joined by links seen edge on
setting-twist-8c01a0 = Twist
setting-two-strands-twisting-round-each-039024 = Two strands twisting round each other
setting-braid-232451 = Braid
setting-three-strands-plaited-over-and-2bc55e = Three strands plaited over and under
setting-bead-and-reel-39dfd0 = Bead and reel
setting-oval-beads-with-a-pair-1b9f2a = Oval beads with a pair of thin discs between each two
setting-laurel-leaves-69948c = Laurel leaves
setting-pairs-of-laurel-leaves-along-9ff40c = Pairs of laurel leaves along a stem, as in a wreath
setting-feathers-321361 = Feathers
setting-feathers-one-after-another-each-4e99fa = Feathers one after another, each with a quill and barbs
setting-chevrons-eba049 = Chevrons
setting-v-shapes-one-after-another-fea111 = V shapes one after another, all pointing the same way
setting-how-wide-one-row-of-62f84a =
    How wide one row of the pattern is. A closed shape is filled row after row inward; on
    a shape narrower than this it is made narrower to fit, one row down the middle
setting-repeat-length-e1dbba = Repeat length
setting-how-long-each-repeat-of-dd18a4 =
    How long each repeat of the pattern is, against its width; the count is rounded so
    the pattern meets itself
setting-reverse-fadaa5 = Reverse
setting-run-the-pattern-the-other-e35bf3 =
    Run the pattern the other way along its line: chevrons, feathers, leaves and scrolls
    point back, and the rope and plaits twist the other way
setting-twist-angle-fe92b3 = Twist angle
setting-how-steeply-the-rope-s-177093 = How steeply the rope's strands lie
setting-gap-from-the-edge-3552af = Gap from the edge
setting-how-far-in-from-the-5e71e0 = How far in from the shape's edge the pattern starts
setting-sequence-cec188 = Sequence
setting-how-each-repeat-is-drawn-b2e6a0 =
    How each repeat is drawn, in turn: 0 a space, 1 as designed, 2 flipped UD, 3
    flipped LR, 4 flipped LR and UD. /n turns it by 180° ÷ n (1/4 is 45°), R turns it the
    other way, S gives it a random size and T a random lean; S or T before brackets
    reaches all inside, as ST(1,2). Split entries with commas. It starts again when it
    runs out, nothing added between. Empty is 1 over and over; 12 makes the vine wave.
    The buttons under it add each sample
setting-edge-lines-1680cf = Edge lines
setting-thin-raised-lines-along-both-6ffa2e = Thin raised lines along both sides of the pattern, framing it
setting-vine-62ed5d = Vine
setting-the-wavy-stem-down-the-0a0d10 = The wavy stem down the middle that the other pieces grow from
setting-include-025607 = Include
setting-scrolls-curling-off-the-vine-ce91da = Scrolls curling off the vine, one side then the other
setting-flowers-the-main-pieces-for-0c96ef = Flowers: the main pieces for Flowers, small ones beside the scrolls for Folk mix
setting-leaves-4bd3e9 = Leaves
setting-pairs-of-leaves-in-the-35b0b3 = Pairs of leaves in the vine's bends (for Folk mix, where a flower does not fit)
setting-dots-9266c9 = Dots
setting-a-dot-where-the-vine-4c560c = A dot where the vine crosses the middle, top then bottom
setting-detail-4473b4 = Detail
setting-low-makes-plain-pieces-high-10aa32 =
    Low makes plain pieces; high adds eyes and bead rows to the scrolls and makes the
    flowers rosettes
setting-ornament-904975 = Ornament
setting-piece-size-b4a436 = Piece size
setting-how-much-of-its-place-bdb340 = How much of its place each scroll, flower or leaf fills: low leaves more open ground
setting-turn-with-the-line-f3cf1d = Turn with the line
setting-each-piece-turns-to-follow-d29355 =
    Each piece turns to follow the line it sits on; off, they all stay upright and only
    move along it
setting-random-size-906b07 = Random size
setting-each-repeat-a-different-size-d995cc =
    Each repeat a different size, from the Layout number (off: every repeat the same
    size)
setting-random-rotation-9afdec = Random rotation
setting-each-repeat-leaning-a-different-3d1c77 =
    Each repeat leaning a different way, up to 45° either side, from the Layout number
    (off: every repeat turned only as the Sequence says)
setting-tightness-81a518 = Tightness
setting-how-close-the-pieces-sit-621ee7 =
    How close the pieces sit: the smallest gap between them is 3 mm at 1 and 0.75 mm at
    10. Finish with a ball-nose bit no wider than the gap
setting-fit-to-your-bit-47d9b7 = Fit to your bit
setting-make-6bedcf = Make
setting-outlines-to-cut-with-2d-13815a = Outlines to cut with 2D toolpaths, a rounded 3D carving, or both
setting-vectors-00d683 = Vectors
setting-the-pieces-outlines-to-carve-92fde1 = The pieces' outlines, to carve with 2D toolpaths
setting-3d-carving-f25cc6 = 3D carving
setting-a-3d-component-each-piece-0aaf9d = A 3D component, each piece rounded over, for 3D roughing and finishing
setting-the-outlines-and-the-3d-a446ff = The outlines and the 3D component together
setting-layout-88421f = Layout
setting-picks-the-shape-of-the-4122a1 =
    Picks the shape of the pieces: how tight the scrolls curl, the flowers' petals, how
    wide the leaves fan, the same in every repeat; with Random size or Random rotation,
    also how the repeats differ. 1 is the pattern as designed
setting-names-the-new-layer-the-88c0c4 = Names the new layer the ornament's vectors go on, and its 3D component
setting-whether-the-3d-carving-s-b0290b = Whether the 3D carving's pieces stand up, are hollowed in, or both
setting-raised-8400e5 = Raised
setting-every-piece-stands-up-from-e2466c = Every piece stands up from the material
setting-sunk-in-16f1e5 = Sunk in
setting-every-piece-is-hollowed-into-8a241c = Every piece is hollowed into the material, as an incised carving
setting-mixed-a8ef98 = Mixed
setting-the-border-and-the-bigger-0467d5 = The border and the bigger pieces stand up; the small fillers are hollowed in
setting-raise-height-4244ce = Raise height
setting-how-far-the-carving-s-27e157 =
    How far the carving's raised pieces stand above the material (not used when every
    piece is sunk in)
setting-sink-depth-41b98f = Sink depth
setting-how-deep-the-carving-s-02ac83 = How deep the carving's hollowed pieces go into the material
setting-stippled-ground-d833e5 = Stippled ground
setting-small-dots-punched-into-the-19b1dc = Small dots punched into the open ground of the 3D carving
setting-how-the-3d-carving-joins-29af4c = How the 3D carving joins any 3D already in the job

## Draw Freehand (draw_freehand)
setting-how-far-the-smoothed-curve-afd87b = How far the smoothed curve may stray from the drawn path
setting-stabilize-9afe7e = Stabilize
setting-small-movements-this-close-to-219e49 =
    Small movements this close to the path's end do not move it: the pointer has to
    reach the edge of the circle drawn there to pull the path on
setting-end-vector-at-cursor-885c6c = End vector at cursor
setting-stabilized-the-path-ends-where-c334c7 = Stabilized, the path ends where the pointer was let go, not at the circle's centre

## Draw Curve (draw_curve)
setting-the-points-the-curve-passes-0e72ae =
    The points the curve passes through, in order: click them in the 2D view, or type them
    here
setting-join-the-last-point-back-f40a00 = Join the last point back to the first, smoothly

## Vector Selector (vector_selector)
setting-open-vectors-741771 = Open vectors
setting-pick-vectors-whose-ends-do-624109 = Pick vectors whose ends do not meet
setting-closed-vectors-164bfd = Closed vectors
setting-pick-shapes-whose-ends-meet-f4eb47 = Pick shapes whose ends meet
setting-closed-kind-2ecc15 = Closed kind
setting-every-closed-shape-or-circles-91704d = Every closed shape, or circles only
setting-all-closed-vectors-76eac0 = All closed vectors
setting-any-shape-whose-ends-meet-8a5331 = Any shape whose ends meet
setting-only-circles-261cbe = Only circles
setting-circles-only-no-other-shapes-76a709 = Circles only, no other shapes
setting-circles-d4892e = Circles
setting-every-circle-or-only-circles-0ce316 = Every circle, or only circles of one size
setting-select-all-circles-08bef7 = Select all circles
setting-every-circle-whatever-its-size-a908f5 = Every circle, whatever its size
setting-select-circles-matching-33cd2b = Select circles matching
setting-circles-within-the-tolerance-of-4801f8 = Circles within the tolerance of the diameter
setting-the-size-of-circle-to-63a436 = The size of circle to pick, for holes of one size
setting-circle-tolerance-ed99e2 = Circle tolerance
setting-how-far-a-circle-may-2231d5 = How far a circle may be from that size and still be picked
setting-select-vectors-on-510eca = Select vectors on
setting-which-layers-to-pick-vectors-e08aee = Which layers to pick vectors from
setting-all-visible-layers-6cbebc = All visible layers
setting-every-layer-that-is-shown-43a08f = Every layer that is shown
setting-current-layer-only-e1bbf3 = Current layer only
setting-only-the-layer-you-are-240d74 = Only the layer you are working on
setting-selected-layers-d4275e = Selected layers
setting-the-layers-named-below-shown-ebee1d =
    The layers named below, shown or not. Vector Selector itself leaves out the vectors
    on hidden or locked layers, so they cannot be deleted by mistake
setting-layer-names-b7ff4e = Layer names
setting-selected-layers-their-names-9cb2c8 = Selected layers: their names

## Orientate 3D Model (orientate_3d_model)
setting-file-183663 = File
setting-the-stl-file-binary-or-c90195 = The STL file (binary or ASCII)
setting-initial-orientation-cfcd85 = Initial Orientation
setting-which-face-is-up-then-a757d0 = Which face is up, then a turn about Z.
setting-face-up-d9e539 = Face up
setting-the-face-of-the-model-5873fe = The face of the model as drawn that becomes its top
setting-the-top-as-drawn-stays-796787 = The top as drawn stays on top
setting-turned-over-so-its-underside-43b02d = Turned over, so its underside faces up
setting-front-e89878 = Front
setting-the-side-facing-you-as-b5ae61 = The side facing you as drawn turns up
setting-back-954bc2 = Back
setting-the-side-facing-away-as-07e9d2 = The side facing away as drawn turns up
setting-the-left-side-as-drawn-b22909 = The left side as drawn turns up
setting-the-right-side-as-drawn-2d34e4 = The right side as drawn turns up
setting-rotation-about-z-axis-a61fc9 = Rotation about Z Axis
setting-then-a-turn-about-the-83749e = Then a turn about the upright axis, in degrees
setting-model-size-0bf94f = Model Size
setting-the-model-s-size-the-b7cca2 = The model's size, the ratio lock and the file's units.
setting-lock-xyz-ratio-50762e = Lock XYZ ratio
setting-keep-the-model-s-proportions-af8904 = Keep the model's proportions: the size changed sets the others
setting-the-model-s-length-once-65c596 =
    The model's length once placed. Typing one scales the model once; turning it after
    keeps that scale
setting-the-model-s-width-once-b8958c =
    The model's width once placed. Typing one scales the model once; turning it after
    keeps that scale
setting-z-0c214d = Z
setting-the-model-s-height-once-bb0157 =
    The model's height once placed. Typing one scales the model once; turning it after
    keeps that scale
setting-the-numbers-in-the-file-48f6b9 = The numbers in the file are in these units
setting-mm-dfff2f = Mm
setting-the-file-was-drawn-in-fa983f = The file was drawn in millimetres
setting-inches-732201 = Inches
setting-the-file-was-drawn-in-824f2e = The file was drawn in inches
setting-change-size-4c405e = Change size
setting-unlocks-the-size-x-y-4e768f =
    Unlocks the size: X, Y, Z and Scale to Fit Material can change it. Off, the model
    keeps the size it has, so nothing resizes it by accident
setting-position-relative-to-modeling-plane-5a6c04 = Position relative to Modeling Plane
setting-where-the-model-sits-relative-eec65c = Where the model sits relative to the modelling plane.
setting-where-the-model-s-lower-6f4dd4 = Where the model's lower-left corner goes when not centred
setting-depth-below-top-ae1ab8 = Depth Below Top
setting-how-far-below-the-model-091f7a =
    How far below the model's top the zero plane (the material's surface) cuts it: what
    is above the plane becomes the component; 0 keeps the whole model, sitting on the
    plane. More than the model's height lifts it off the plane, up to the material's top
setting-create-both-sides-f5c7f4 = Create both sides
setting-in-a-double-sided-job-63c8f0 = In a double-sided job, the model seen from below goes to the other side too
setting-discard-data-below-zero-plane-39cf52 = Discard data below zero plane
setting-throw-away-the-part-of-2de4c2 = Throw away the part of the model below the zero plane, keeping only what is above
setting-highlight-undercuts-ba7c38 = Highlight undercuts
setting-colour-the-faces-a-cutter-9f001e =
    Colour the faces a cutter from above cannot reach (turned down, or under an
    overhang) in the 3D view while the form is open
setting-a-full-3d-model-is-c9c37b =
    A full 3D model is kept whole: in a double-sided job its underside goes to the other
    side, and in a rotary job it is wrapped round the axis, its distance out at each
    angle becoming the height. A flat model is its top seen from above, placed like
    clipart. Shown when Options ▸ Importing and Exporting offers both
setting-the-whole-model-both-sides-5e8c0a = The whole model: both sides of a double-sided job, all the way round a rotary one
setting-its-top-seen-from-above-28595a = Its top seen from above, like clipart
setting-fit-material-a7290d = Fit material
setting-scaled-to-fit-the-material-e3ae8c =
    Scaled to fit the material again on every change, keeping its proportions: as big as
    its length, width and thickness allow (a rotary job's length and diameter). Not in the
    form, which scales to fit once instead (its Scale to Fit Material button, kept in
    `size.scale`); kept for settings saved before, and for scripts
setting-offered-d643c4 = Offered
setting-what-options-importing-and-exporting-2d04c3 =
    What Options ▸ Importing and Exporting offers; never shown. Set by the app when the form
    opens; one kind alone also sets `model_type`
setting-full-a5ff5c = Full
setting-only-full-3d-models-are-cb17b2 = Only full 3D models are offered
setting-only-flat-models-are-offered-340bf9 = Only flat models are offered
setting-both-kinds-are-offered-to-1fd34a = Both kinds are offered, to pick from
setting-interactive-rotation-eb5d93 = Interactive Rotation
setting-fine-turns-about-the-job-207217 =
    Fine turns about the job's axes after the initial
    orientation, which a drag in the 3D view adds to while the drag turns the model.
setting-drag-turns-ab84ff = Drag turns
setting-what-a-left-drag-in-d3be95 =
    What a left drag in the 3D view turns: the view, or the model about all three axes
    or about one (the view then looks along it)
setting-xyz-view-7b4771 = XYZ view
setting-the-view-as-always-1904c2 = The view, as always
setting-xyz-model-2076cd = XYZ model
setting-the-model-about-all-three-2c0407 = The model about all three axes: sideways about Z, up and down about X
setting-the-model-about-the-x-fa6ad3 = The model about the X axis only (tips it forward and back)
setting-the-model-about-the-y-e4fbf6 = The model about the Y axis only (rolls it side to side)
setting-the-model-about-the-z-d1bf28 = The model about the Z axis only (spins it flat)
setting-how-far-the-model-is-382841 = How far the model is turned about the X axis, in degrees
setting-how-far-the-model-is-059da8 = How far the model is turned about the Y axis, in degrees
setting-how-far-the-model-is-562a6b = How far the model is turned about the Z axis, in degrees
setting-rotation-axis-movement-d64f69 = Rotation Axis Movement
setting-shown-in-a-rotary-job-9ad908 = Shown in a rotary job
setting-drag-moves-the-axis-d55197 = Drag moves the axis
setting-a-pan-drag-in-the-cd14e5 = A pan drag in the 3D view moves the model off the axis instead of moving the view
setting-across-2e8644 = Across
setting-how-far-the-model-s-0f31d7 = How far the model's middle sits across from the axis
setting-how-far-the-model-s-269e5d = How far the model's middle sits above the axis
setting-model-position-in-material-cf9951 = Model Position in Material
setting-where-the-model-sits-in-090897 =
    Where the model sits in the material's thickness when it is cut: its top flush with
    the material's top or a gap under it (Gap Above Model), or its bottom on the
    material's bottom or a gap over it (Gap Below Model). The gap above and the gap below
    add up to the material's thickness less the model's. The same setting as Material
    Setup's, for the whole job
setting-which-face-of-the-material-ecd857 =
    Which face of the material the gap is measured from: down from its top to the
    model's top, or up from its bottom to the model's bottom
setting-gap-above-model-4e43f3 = Gap Above Model
setting-between-the-model-s-top-f9fc57 = Between the model's top and the material surface (Gap Outside Model on rotary jobs)
setting-gap-below-model-3851fb = Gap Below Model
setting-between-the-model-s-bottom-288848 =
    Between the model's bottom and the material's bottom (Gap Inside Model, from the
    axis, on rotary jobs)
setting-how-far-the-model-sits-7079da = How far the model sits from the material's face
setting-the-job-is-rotary-set-d97089 = The job is rotary; set by the app when the form opens, never shown

## Sculpting (sculpt)
setting-what-the-brush-does-keys-528c0f = What the brush does (keys 1 to 5).
setting-averages-the-highs-and-lows-d48350 = Averages the highs and lows under the brush
setting-smudge-f12984 = Smudge
setting-drags-the-material-along-the-609c27 = Drags the material along the stroke, like clay under a thumb
setting-deposit-f76a55 = Deposit
setting-adds-height-where-you-drag-0e8962 = Adds height where you drag
setting-remove-a5901d = Remove
setting-takes-height-away-c9d59c = Takes height away
setting-undo-brush-3c4c8b = Undo Brush
setting-brings-back-the-shape-from-dfacbf = Brings back the shape from before sculpting
setting-how-wide-the-brush-is-79edd5 = How wide the brush is
setting-strength-4d03d0 = Strength
setting-how-much-each-stroke-changes-a131dc = How much each stroke changes
setting-how-gently-the-brush-s-919ab1 = How gently the brush's effect fades toward its edge
setting-preserve-transparency-983095 = Preserve Transparency
setting-sculpt-only-inside-the-component-3221f8 = Sculpt only inside the component's shape, not into the plane round it
setting-which-heights-the-brush-may-3a9f2e = Which heights the brush may change.
setting-up-or-down-as-the-54ff12 = Up or down as the tool goes
setting-raise-b55ec3 = Raise
setting-only-up-fills-holes-2207df = Only up: fills holes
setting-lower-8d545a = Lower
setting-only-down-takes-off-spikes-c611fb = Only down: takes off spikes
setting-erase-3c6075 = Erase
setting-with-the-undo-brush-remove-264d7c = With the Undo Brush: remove the shape under the brush altogether (hold Shift)
setting-custom-brush-05238a = Custom brush
setting-deposit-and-remove-a-component-678926 =
    Deposit and Remove: a component saved as a sculpting brush (a `.ccclip` file, Load
    Brush… below) pressed into the surface, its larger side the diameter; empty for the
    round brush
setting-the-custom-brush-s-turn-1e450b = The custom brush's turn, degrees anticlockwise
setting-dynamic-rotation-86e0d8 = Dynamic Rotation
setting-the-custom-brush-turns-to-a26296 = The custom brush turns to follow the stroke
setting-a-custom-brush-s-stroke-0e73d0 =
    A custom brush's stroke merges with the strokes before it (the higher wins) instead of
    being added onto them
setting-dab-spacing-b2992d = Dab spacing
setting-how-far-apart-the-brush-4dd8a6 =
    How far apart the brush presses along a stroke, as a share of its diameter. Closer
    gives a smoother stroke, but each one takes longer and works the surface harder
setting-brush-feel-115b01 = Brush Feel
setting-build-up-rate-f433c7 = Build-up rate
setting-how-much-deposit-and-remove-44a985 =
    How much Deposit and Remove change the middle of each press at full strength, as a
    share of the diameter. Higher builds up or takes away faster
setting-custom-brush-depth-ed6237 = Custom brush depth
setting-how-deep-a-custom-brush-bc96c3 = How deep a custom brush presses at full strength, as a share of its diameter

## Distort Object (distort_object)
setting-which-envelope-the-objects-are-467736 = Which envelope the objects are bent into.
setting-bounding-box-distortion-6e9237 = Bounding Box Distortion
setting-their-bounding-box-its-corners-25872b = Their bounding box, its corners moved to the points below
setting-along-a-single-curve-4cf58c = Along a Single Curve
setting-bent-to-follow-the-open-4f535d = Bent to follow the open vector picked last; they keep their height above it
setting-between-two-curves-9e9f63 = Between Two Curves
setting-stretched-between-the-two-open-df2702 = Stretched between the two open vectors picked last
setting-create-envelope-3ea1bb = Create Envelope
setting-adds-a-rectangle-round-the-f5f72c = Adds a rectangle round the objects, picked last, to reshape with Node Edit (N)
setting-distort-to-envelope-bc719b = Distort to Envelope
setting-bent-into-the-closed-envelope-68637b =
    Bent into the closed envelope picked last (its four corners and the sides between
    them, as Node Edit left them); the envelope goes
setting-bounding-box-distortion-only-2ac607 = Bounding Box Distortion only

## Level Properties (level_properties)
setting-what-the-level-is-called-1beaba = What the level is called in the component list
setting-hidden-levels-are-left-out-88f322 = Hidden levels are left out of the model
setting-raises-the-whole-level-on-4e45c6 = Raises the whole level on a flat plane this thick
setting-mirror-mode-f9938d = Mirror Mode
setting-a-level-s-mirror-mode-12824e = A level's mirror mode: which part of the job is drawn, the rest being its reflection.
setting-off-c6b4aa = Off
setting-no-mirroring-the-level-is-0a060c = No mirroring: the level is as modelled
setting-the-left-half-is-copied-b658bc = The left half is copied, flipped, onto the right half
setting-right-to-left-de077b = Right to Left
setting-the-right-half-is-copied-5269fc = The right half is copied, flipped, onto the left half
setting-top-to-bottom-fe1278 = Top to Bottom
setting-the-top-half-is-copied-ec75c7 = The top half is copied, flipped, onto the bottom half
setting-the-bottom-half-is-copied-df1241 = The bottom half is copied, flipped, onto the top half
setting-top-left-quadrant-b6663f = Top Left Quadrant
setting-the-top-left-quarter-is-2920b7 = The top left quarter is copied, flipped, into the other three
setting-top-right-quadrant-4c944a = Top Right Quadrant
setting-the-top-right-quarter-is-c75acc = The top right quarter is copied, flipped, into the other three
setting-bottom-left-quadrant-f2b051 = Bottom Left Quadrant
setting-the-bottom-left-quarter-is-ff0b2f = The bottom left quarter is copied, flipped, into the other three
setting-bottom-right-quadrant-7768d4 = Bottom Right Quadrant
setting-the-bottom-right-quarter-is-d8eda4 = The bottom right quarter is copied, flipped, into the other three
setting-wrapping-20c389 = Wrapping
setting-rotary-jobs-what-runs-off-4c4d72 = Rotary jobs: what runs off one side comes back on the other

## Insert New Level (insert_level)
setting-what-the-new-level-is-778730 = What the new level is called in the component tree

## Level Clipping (level_clipping)
setting-what-happens-to-the-level-0c2832 = What happens to the level's clipping
setting-apply-77bbdb = Apply
setting-clip-to-the-selected-closed-f8f504 = Clip to the selected closed vectors
setting-update-230e94 = Update
setting-clip-again-to-the-selected-031dbc = Clip again to the selected vectors, else the ones used last
setting-the-whole-level-shows-again-0c015b = The whole level shows again, unclipped

## Vector Unwrapper (vector_unwrapper)
setting-center-of-rotation-9c1539 = Center of Rotation
setting-where-the-rotation-axis-crosses-b94a99 = Where the rotation axis crosses the cross-section.
setting-use-center-of-contour-9ec834 = Use center of contour
setting-the-middle-of-the-cross-b5982e = The middle of the cross-section's bounding box
setting-use-center-of-workspace-de6077 = Use center of workspace
setting-the-middle-of-the-job-f50a4a = The middle of the job
setting-use-custom-point-6494a1 = Use custom point
setting-the-point-below-d96b73 = The point below
setting-use-custom-point-only-8980cd = Use custom point only
setting-refine-unwrapped-vectors-92142c = Refine unwrapped vectors
setting-fit-curves-through-the-profile-7b5b89 = Fit curves through the profile so it has far fewer nodes

## Move Component (reorder_component)
setting-the-component-to-move-in-7db48d = The component to move in the tree
setting-which-way-it-moves-in-6f0b31 = Which way it moves in the component tree
setting-toward-the-top-of-the-72b28b = Toward the top of the tree: combined later
setting-toward-the-bottom-combined-sooner-b16463 = Toward the bottom: combined sooner

## Duplicate Component (duplicate_component)
setting-the-component-to-make-a-b0ceab = The component to make a copy of

## Segment 3D Model (segment_3d_model)
setting-segmenting-plane-square-to-8690bc = Segmenting plane square to
setting-the-axis-the-segmenting-plane-d91729 = The axis the segmenting plane is square to.
setting-across-the-model-s-length-bf736b = Across the model's length: a left and a right segment
setting-across-the-model-s-width-3f2a6e = Across the model's width: a front and a back segment
setting-across-the-model-s-height-01f2dd = Across the model's height: a top and a bottom segment
setting-number-of-cuts-f5b886 = Number of cuts
setting-how-many-planes-cut-the-cfd048 = How many planes cut the model: one where Position says, or several in equal slices
setting-where-the-one-plane-cuts-b91854 =
    Where the one plane cuts, as a share of the model's size along the axis (shown only
    with one cut: several cuts slice it equally)
setting-turn-the-second-segment-over-35cf10 = Turn the second segment over
setting-turn-the-segment-beyond-the-bdb8e4 = Turn the segment beyond the plane over, so its far side faces up to be machined
setting-space-between-the-laid-out-a19021 = Space between the laid-out segments
setting-cut-segments-again-6e06cb = Cut segments again
setting-the-segment-tree-a-segment-5400a4 =
    The segment tree: a segment cut again by a plane of its own gives two segments
    named after it (cutting segment 2 gives 2.1 beyond the plane and 2.2 before it),
    which can be cut again in turn
setting-segment-ab2f86 = Segment
setting-the-segment-s-name-as-3986a7 = The segment's name, as its component is named: 2, or 2.1 for a part of segment 2
setting-plane-square-to-548ec1 = Plane square to
setting-where-the-plane-cuts-as-78aa47 = Where the plane cuts, as a share of the segment's size along the axis
setting-segment-orientations-0d2d45 = Segment orientations
setting-which-face-of-a-segment-eed238 =
    Which face of a segment is turned up in the layout; segments not listed follow
    Turn the second segment over
setting-the-segment-s-name-as-2d43c4 = The segment's name, as its component is named
setting-the-side-of-a-segment-fee0bd = The side of a segment that faces up when it is laid out.
setting-as-in-the-model-46c5fa = As in the model
setting-turned-over-its-underside-up-3e67d3 = Turned over, its underside up
setting-the-side-facing-y-turned-dc2ad0 = The side facing -Y turned up
setting-the-side-facing-y-turned-0f20f6 = The side facing +Y turned up
setting-the-side-facing-x-turned-89f7f7 = The side facing -X turned up
setting-the-side-facing-x-turned-e26c35 = The side facing +X turned up

## Material Setup (material_setup)
setting-material-thickness-4eab52 = Material Thickness
setting-the-material-s-thickness-changing-5b7f15 =
    The material's thickness. Changing it, or the model's gap, after making toolpaths
    marks them out of date. A rotary job has none here: its size is the cylinder's
    diameter, set in Job Setup
setting-which-point-of-the-material-f3e2de = Which point of the material is the machine's X0, Y0
setting-material-setup-s-model-position-6c05cd = Material Setup's Model Position in Material.
setting-model-thickness-d5bc28 = Model Thickness
setting-the-3d-model-s-height-38cd8e =
    The 3D model's height from its lowest to its highest point; a new value scales the
    model to it (Set). 0 when there is no model, or to leave it alone

## Group Components (group_components)
setting-what-the-new-group-is-be0fa9 = What the new group is called in the component tree

## Group Properties (group_properties)
setting-what-the-group-is-called-dc3163 = What the group is called in the component tree

## Add New Sheet (add_sheet)
setting-what-the-new-sheet-is-a111ec = What the new sheet is called in the Sheets list

## Delete Sheet (delete_sheet)
setting-sheet-01fa48 = Sheet
setting-the-sheet-s-place-in-75c68c = The sheet's place in the Sheets list, from 0.

## Sheet Properties (sheet_properties)
setting-what-the-sheet-is-called-ba135e = What the sheet is called in the Sheets list

## Duplicate Toolpath (duplicate_toolpath)
setting-the-toolpath-to-make-a-2321ed = The toolpath to make a copy of

## Edit Picture (edit_picture)
setting-the-picture-s-tones-b9cfa5 = The picture's tones.
setting-contrast-56b993 = Contrast
setting-above-0-the-light-and-4c6784 = Above 0 the light and dark parts move apart; below, toward middle grey
setting-brightness-84324a = Brightness
setting-above-0-lighter-below-darker-fd8429 = Above 0 lighter, below darker
setting-gamma-e618aa = Gamma
setting-above-1-lighter-and-below-08e064 = Above 1 lighter and below darker, keeping the detail
setting-white-becomes-black-and-black-45a9fc = White becomes black and black white
setting-border-ad6f57 = Border
setting-a-border-fading-the-edges-d66f0c = A border fading the edges to white.
setting-add-border-9c88f2 = Add Border
setting-fade-the-picture-s-edges-45dcde = Fade the picture's edges to white
setting-the-shape-the-fade-follows-7fa456 = The shape the fade follows
setting-rectangular-border-e5904b = Rectangular Border
setting-fades-in-from-the-four-e3dd3e = Fades in from the four sides
setting-oval-border-918f29 = Oval Border
setting-fades-in-round-an-oval-e96ff4 = Fades in round an oval, like an old portrait
setting-how-far-in-the-fade-b63d4c = How far in the fade reaches, as a percentage of the picture's shorter side

## Document Variables (document_variables)
setting-named-values-kept-with-the-e58da6 =
    Named values kept with the job. Make them here: add a row, give it a name and a value,
    then Apply. Use one in any number field by typing its name in braces, alone or in a
    sum: {"{"}depth{"}"}, {"{"}depth{"}"}/2 or {"{"}width{"}"} + 10. The sum is worked out when it is entered, so
    changing a variable here changes the fields filled in after, not those already done.
    A name starts with a letter, then letters, digits or _.
setting-variables-96f062 = Variables
setting-every-variable-one-taken-off-f98cc9 = Every variable; one taken off the list is deleted
setting-what-you-type-in-braces-c8fa6c = What you type in braces to use it, such as depth for {"{"}depth{"}"}
setting-value-47f96a = Value
setting-the-number-it-stands-for-e752e4 = The number it stands for wherever it is used

## Preview Colors (materials)
setting-what-the-3d-view-shows-dce2dc =
    What the 3D view shows the job as. Only its look: feeds and speeds follow Job Setup's
    Material, which picks the matching colour here
setting-a-solid-colour-of-one-5ac43c = A solid colour of one's own
setting-pine-b7ed83 = Pine
setting-a-pale-softwood-with-a-dc77a4 = A pale softwood with a strong grain
setting-maple-d92a46 = Maple
setting-a-pale-fine-grained-hardwood-21f311 = A pale, fine-grained hardwood
setting-birch-plywood-e0183b = Birch Plywood
setting-layered-birch-sheet-pale-with-35ed6a = Layered birch sheet, pale with a light grain
setting-oak-d7f912 = Oak
setting-a-mid-brown-hardwood-with-bd52b1 = A mid-brown hardwood with an open grain
setting-cedar-52a8dc = Cedar
setting-a-reddish-softwood-often-used-e41f66 = A reddish softwood, often used outdoors
setting-cherry-08cb98 = Cherry
setting-a-reddish-brown-fine-grained-f766ef = A reddish-brown fine-grained hardwood
setting-mahogany-3337af = Mahogany
setting-a-deep-red-brown-hardwood-03e6a4 = A deep red-brown hardwood
setting-walnut-7c770e = Walnut
setting-a-dark-brown-hardwood-1186c4 = A dark brown hardwood
setting-mdf-605ad6 = MDF
setting-medium-density-fibreboard-even-with-9403cd = Medium-density fibreboard: even, with no grain
setting-hdu-foam-fc9581 = HDU Foam
setting-high-density-urethane-sign-foam-75d842 = High-density urethane sign foam
setting-hdpe-white-629842 = HDPE (White)
setting-white-high-density-polyethylene-plastic-14ab57 = White high-density polyethylene plastic
setting-acrylic-black-df472e = Acrylic (Black)
setting-black-acrylic-perspex-sheet-8d7d78 = Black acrylic (Perspex) sheet
setting-aluminium-b64966 = Aluminium
setting-aluminium-plate-shown-in-plain-1cf38d = Aluminium plate, shown in plain grey
setting-brass-14348a = Brass
setting-brass-plate-shown-in-gold-9dd15c = Brass plate, shown in gold
setting-solid-color-244663 = Solid color
setting-custom-s-solid-colour-d1cdc5 = Custom's solid colour
setting-machined-area-color-bf26c0 = Machined Area Color
setting-how-the-preview-colours-what-e8a434 =
    How the preview colours what the toolpaths cut; each toolpath's colour is the swatch
    beside it in the toolpath list
setting-material-color-d9e0f7 = Material color
setting-the-material-s-own-colour-624d04 = The material's own colour, as the uncut surface
setting-global-fill-color-999b28 = Global fill color
setting-every-machined-area-in-the-3c1fdf = Every machined area in the one fill colour
setting-each-toolpath-s-areas-in-3b17a3 = Each toolpath's areas in its own colour (none: the material's)
setting-lithophane-09589b = Lithophane
setting-lit-from-behind-white-where-c3fa12 =
    Lit from behind: white where nothing is left, the material's colour at full
    thickness, between by what is left (added with the lithophane preview)
setting-fill-color-21dfef = Fill color
setting-the-one-colour-all-cut-82c639 = The one colour all cut areas are shown in
setting-lithophane-light-3628cb = Lithophane light
setting-how-a-lithophane-is-lit-4649db =
    How a lithophane is lit: to the left high contrast, to the right lighter, as with no
    light behind it

## Keep-Out Zones (keep_out_zones)
setting-what-happens-to-the-keep-6e9e1b = What happens to the keep-out zones
setting-create-from-selection-3fc119 = Create from Selection
setting-the-zones-become-the-selected-0fae80 =
    The zones become the selected closed vectors, grown by the clearance, in place of
    any there were. The zones removed are counted in red below. Once there are zones the
    form opens on Add Selection instead
setting-add-selection-954e12 = Add Selection
setting-the-selected-closed-vectors-grown-9932a7 = The selected closed vectors, grown by the clearance, are added to the zones there are
setting-clear-zones-dae141 = Clear Zones
setting-all-the-keep-out-zones-783f80 = All the keep-out zones are removed
setting-what-the-zones-are-called-4d97b8 =
    What the zones are called on their labels: "Clamp" names one zone "Clamp" and
    several "Clamp 1", "Clamp 2" and so on
setting-how-far-beyond-the-vectors-b491a7 = How far beyond the vectors the zones reach
setting-clamp-height-8d0467 = Clamp Height
setting-how-tall-the-clamps-stand-24a5b9 =
    How tall the clamps stand above the top of the material. Toolpaths already keep the
    bit out of the zones; this lets the checks see the wider collet nut and spindle
    coming down over a clamp beside a deep cut. Measure from the material's top to the
    clamp's highest point (a screw head counts)

## Plate Production (plate_production)
setting-how-the-plates-are-laid-67a79c = How the plates are laid out on the material.
setting-top-margin-7172ea = Top margin
setting-space-left-clear-along-the-103d04 = Space left clear along the material's top edge
setting-bottom-margin-39af48 = Bottom margin
setting-space-left-clear-along-the-7699ec = Space left clear along the material's bottom edge
setting-left-margin-e563de = Left margin
setting-space-left-clear-along-the-015352 = Space left clear along the material's left edge
setting-right-margin-ad18c9 = Right margin
setting-space-left-clear-along-the-bac92b = Space left clear along the material's right edge
setting-between-plates-side-by-side-f38dd3 = Between plates side by side
setting-between-plates-one-above-another-847746 = Between plates one above another
setting-data-5279c5 = Data
setting-the-data-merged-into-the-df7625 = The data merged into the template.
setting-a-text-file-of-rows-c00f4e = A text file of rows; empty to number plates with the counter alone
setting-separator-d8725c = Separator
setting-what-splits-one-field-of-0876da = What splits one field of a row from the next
setting-comma-b5f718 = Comma
setting-fields-split-at-commas-a-00dad1 = Fields split at commas (a .csv file)
setting-tab-8358ec = Tab
setting-fields-split-at-tabs-as-783d29 = Fields split at tabs (as copied from a spreadsheet)
setting-semicolon-5670be = Semicolon
setting-fields-split-at-semicolons-a5a915 = Fields split at semicolons
setting-space-c43225 = Space
setting-fields-split-at-spaces-one-29b92c = Fields split at spaces, one record per line
setting-first-row-is-column-names-1dff57 = First row is column names
setting-the-file-s-first-row-399803 = The file's first row holds the column names, not a plate
setting-plates-0e6970 = Plates
setting-how-many-plates-when-there-66a524 = How many plates when there is no data file
setting-which-column-fills-each-name-e752cf = Which column fills each !!name!! variable in the template text
setting-variable-f59325 = Variable
setting-the-variable-s-name-as-26022f = The variable's name, as written between !! marks in the template text
setting-column-c12a47 = Column
setting-the-data-file-s-column-591fc2 = The data file's column, from 1; 0 for the counter
setting-number-format-897823 = Number Format
setting-the-counter-variables-set-to-132b9f = The counter variables set to column 0 take.
setting-start-e8097f = Start
setting-the-number-the-first-plate-387b0d = The number the first plate gets
setting-how-much-the-number-goes-af9edc = How much the number goes up from one plate to the next
setting-digits-c22f5f = Digits
setting-padded-with-zeros-to-at-2c3dd4 = Padded with zeros to at least this many digits
setting-create-toolpaths-8a0f3b = Create Toolpaths
setting-the-template-s-toolpaths-made-9aea47 =
    The template's toolpaths made again for each sheet of plates, each named "S1 - …"
    after its sheet and the template's toolpath

## Toolpath Tiling (toolpath_tiling)
setting-tile-toolpaths-6d03bf = Tile toolpaths
setting-save-the-toolpaths-as-tiles-933fc9 =
    Save the toolpaths as tiles, one file each (ticked while the job is tiled). While it
    is on, Save Toolpaths says "Tiling on" with how many tiles, and writes one file for
    each tile, numbered after the name given (tile1, tile2…), in place of that name.
    Unticked, the tile settings are kept for when it is ticked again
setting-tiling-options-7fa2a0 = Tiling Options
setting-how-the-job-is-split-a24f3e = How the job is split into tiles
setting-separate-tiles-or-one-strip-5e973a = Separate tiles, or one strip fed through the machine
setting-individual-tiles-c90fff = Individual Tiles
setting-separate-pieces-split-in-x-962192 = Separate pieces, split in X and Y
setting-feed-through-in-x-510948 = Feed-through in X
setting-one-strip-fed-through-the-758d8c = One strip fed through the machine along X
setting-feed-through-in-y-510adb = Feed-through in Y
setting-one-strip-fed-through-the-758f1f = One strip fed through the machine along Y
setting-tile-width-b5abed = Tile width
setting-tile-width-along-x-db5a79 = Tile width (along X)
setting-tile-height-fca444 = Tile height
setting-tile-height-along-y-232665 = Tile height (along Y)
setting-how-far-each-tile-s-e51d5a =
    How far each tile's cuts run past its edges (at least the tool's radius for V-carving).
    Each tile file's zero is at the start of its overlap, so a file covers the tile and
    the overlap on both sides. A tile at the material's edge also holds every cut past
    that edge (an outside profile's centreline, a lead-in): one running further out than
    the overlap goes below zero (or past the far side) by the difference, and the checks
    before saving measure each file with it. Laser toolpaths have none: their tiles meet
    edge to edge, so no seam burns twice
setting-machine-smallest-tile-first-e3b7fb = Machine smallest tile first
setting-put-the-part-size-tile-34ef65 = Put the part-size tile first rather than last
setting-active-tile-029c81 = Active tile
setting-the-tile-previewed-alone-counting-81acdf =
    The tile previewed alone, counting from the bottom left along X (double-click one in
    the 2D view while this form is open); 0 previews the whole job
setting-draw-toolpaths-in-original-position-173e3e = Draw toolpaths in original position
setting-preview-the-active-tile-where-dc2d7a =
    Preview the active tile where it sits in the finished piece, not moved to the
    machine's corner as it is cut

## Wrapped Fluting Layout (wrapped_fluting_layout)
setting-number-of-flutes-cb6eb4 = Number of flutes
setting-how-many-flutes-run-along-7d44b0 = How many flutes run along the piece, spaced evenly round it
setting-start-offset-b2f682 = Start offset
setting-from-the-start-of-the-fb1327 = From the start of the work piece to where the flutes start
setting-end-offset-55f049 = End offset
setting-from-the-end-of-the-a1b550 = From the end of the work piece back to where the flutes end
setting-cove-at-start-5611cb = Cove at start
setting-a-groove-round-the-cylinder-53dafc = A groove round the cylinder where the flutes start
setting-cove-at-end-27538e = Cove at end
setting-a-groove-round-the-cylinder-5752e5 = A groove round the cylinder where the flutes end

## Wrapped Spiral Layout (wrapped_spiral_layout)
setting-number-of-strands-912ce8 = Number of strands
setting-how-many-strands-twist-round-406c43 = How many strands twist round the piece
setting-from-the-start-of-the-388a1d = From the start of the work piece to where the strands start
setting-from-the-end-of-the-0091e6 = From the end of the work piece back to where the strands end
setting-whether-the-distance-below-is-9bc770 = Whether the distance below is the pitch of one strand or the gap between strands
setting-spiral-pitch-e26b4e = Spiral Pitch
setting-how-far-along-the-axis-f0c820 = How far along the axis a strand goes in one turn
setting-spacing-between-strands-079ee3 = Spacing between strands
setting-how-far-apart-neighbouring-strands-6b5b8f = How far apart neighbouring strands are along the axis
setting-the-pitch-or-the-spacing-22efcf = The pitch, or the spacing between strands, as chosen above
setting-which-way-the-strands-spiral-6a2c3a = Which way the strands spiral, like the thread of a screw
setting-right-hand-726fb6 = Right hand
setting-turns-like-an-ordinary-screw-3a2f3b = Turns like an ordinary screw thread
setting-left-hand-f713a1 = Left hand
setting-turns-the-other-way-like-0bb274 = Turns the other way, like a left-hand thread
setting-a-groove-round-the-piece-f99302 = A groove round the piece where the strands start
setting-a-groove-round-the-piece-4fbb47 = A groove round the piece where the strands end

## Celtic Weave Creator (celtic_weave)
setting-the-knot-grid-s-size-c47675 = The knot grid's size in cells
setting-how-many-cells-tall-the-5d2bab = How many cells tall the knot grid is
setting-cell-size-618b06 = Cell size
setting-the-size-of-each-square-e574f2 = The size of each square cell of the grid: bigger cells make a bigger knot
setting-cuts-50a130 = Cuts
setting-crossings-the-strands-turn-at-190459 = Crossings the strands turn at rather than cross
setting-which-crossing-across-counted-in-57ef71 =
    Which crossing across, counted in half cells from the left; column and row must add
    up to an odd number, as at column 1, row 2
setting-row-98b51b = Row
setting-which-crossing-up-counted-in-1b7cb7 =
    Which crossing up, counted in half cells from the bottom; column and row must add up
    to an odd number, as at column 1, row 2
setting-which-way-the-strands-turn-5a8e1a = Which way the strands turn back at this crossing
setting-cut-vertical-057f5f = Cut Vertical
setting-a-wall-up-and-down-835e9c = A wall up and down: strands turn back to the side they came from
setting-cut-horizontal-3d8735 = Cut Horizontal
setting-a-wall-across-strands-turn-765b24 = A wall across: strands turn back up or down
setting-create-cross-section-a7fb4f = Create cross-section
setting-a-half-round-profile-beside-f8f3a1 = A half-round profile beside the knot, for Extrude and Weave

## Join / Close Vectors (join_close_vectors)
setting-how-the-gap-between-the-65f771 = How the gap between the ends is filled
setting-straight-line-a59b27 = Straight line
setting-a-straight-line-across-the-39ee12 = A straight line across the gap
setting-smooth-curve-81321c = Smooth curve
setting-a-curve-that-carries-on-00de00 = A curve that carries on smoothly from both ends
setting-move-end-points-daaa48 = Move end points
setting-the-two-ends-are-moved-2cec8c = The two ends are moved to meet halfway, with no new line

## Join Open Ends (join_open_ends)
setting-joins-open-vectors-where-you-e7727c =
    Joins open vectors where you click. Each open end has a circle on it: click one end, then
    the end to join it to. Click both ends of one vector to close it into a shape that can be
    pocketed or carved.
setting-how-the-gap-between-the-04b6a6 = How the gap between the two ends is bridged
setting-straight-line-d37a47 = Straight Line
setting-a-straight-line-from-one-e68ec3 = A straight line from one end to the other
setting-extend-to-meet-464029 = Extend to Meet
setting-both-ends-run-on-the-ab5ec4 = Both ends run on the way they are heading until they meet, making a sharp corner
setting-arc-fb557b = Arc
setting-arcs-that-carry-on-smoothly-66ae82 = Arcs that carry on smoothly from each end, with no corner where they meet

## Vector Boundary (vector_boundary)
setting-offset-boundary-6ffed4 = Offset Boundary
setting-grow-the-boundary-outwards-by-bda44d = Grow the boundary outwards by the distance
setting-how-far-out-the-boundary-425c32 = How far out the boundary is grown
setting-whether-the-grown-boundary-s-271c53 = Whether the grown boundary's corners are round or sharp
setting-rubber-band-boundary-55488c = Rubber Band Boundary
setting-the-boundary-a-rubber-band-fe73fa = The boundary a rubber band stretched round the selection would make

## Auto Layout Text (auto_layout_text)
setting-the-words-they-are-wrapped-0f58a7 = The words; they are wrapped onto as many lines as make them largest
setting-baseline-to-baseline-as-a-4e4fd6 = Baseline to baseline, as a percentage of the font's own
setting-left-clear-inside-the-vector-3bc4ce = Left clear inside the vector on every side, as a percentage of its width and height

## Move Toolpath (reorder_toolpath)
setting-the-toolpath-to-move-in-a414bf = The toolpath to move in the list
setting-which-way-it-moves-in-9d0b37 = Which way it moves in the toolpath list
setting-toward-the-top-of-the-94eb46 = Toward the top of the list: cut sooner
setting-toward-the-bottom-cut-later-7708d4 = Toward the bottom: cut later

## Arrange Toolpaths (arrange_toolpaths)
setting-every-toolpath-in-the-order-0d309c = Every toolpath, in the order they are to cut
setting-groups-7f458d = Groups
setting-the-group-each-toolpath-named-3276a0 = The group each toolpath named here goes in (empty: none); the others keep theirs
setting-parents-d9572a = Parents
setting-the-group-each-toolpath-group-b1ec8c =
    The group each toolpath group named here goes inside (empty: the top of the list);
    the others stay where they are

## Move Nodes (move_nodes)
setting-nodes-bd4cfa = Nodes
setting-the-nodes-to-move-each-d9cda8 = The nodes to move, each on its vector
setting-by-ddadb4 = By
setting-how-far-they-all-move-e6b71e = How far they all move

## Trim Objects (trim_objects)
setting-clear-a57fc2 = Clear
setting-which-side-of-the-boundary-206931 = Which side of the boundary is trimmed away
setting-outside-boundary-919622 = Outside boundary
setting-what-lies-outside-the-boundary-30276a = What lies outside the boundary goes
setting-inside-boundary-2e99a9 = Inside boundary
setting-what-lies-inside-the-boundary-ce6729 = What lies inside the boundary goes
setting-refit-curves-after-clipping-659f77 = Refit curves after clipping
setting-turn-the-trimmed-pieces-back-9517a0 = Turn the trimmed pieces back into curves rather than many short lines

## Move Layer (reorder_layer)
setting-the-layer-to-move-in-0f7ab3 = The layer to move in the list
setting-which-way-it-moves-in-c83b8e = Which way it moves in the Layers list
setting-toward-the-top-of-the-c1bb4c = Toward the top of the list
setting-toward-the-bottom-e1ebe4 = Toward the bottom

## Merge Visible Layers (merge_visible_layers)
setting-the-layer-the-visible-layers-1b2628 = The layer the visible layers merge into

## Copy / Move to Sheet (send_to_sheet)
setting-move-2c2a34 = Move
setting-take-the-vectors-off-this-bbd898 = Take the vectors off this sheet (else copies are sent)

## Copy Toolpath to Sheet (copy_toolpath_to_sheet)
setting-the-toolpath-to-copy-onto-906244 = The toolpath to copy onto the other sheets

## Name Part (name_part)
setting-part-name-78af77 = Part name
setting-what-the-cut-list-and-ba4743 =
    What the cut list and the part labels call the selected parts, such as "Left side"
    or "Shelf". Leave it empty to take the name away.

## Engrave Part Names (engrave_part_names)
setting-what-each-part-is-marked-10d76f = What each part is marked with
setting-number-and-name-7a59e0 = Number and name
setting-3-side-the-number-matches-32ff42 = "3 Side": the number matches the part's label
setting-name-only-070c94 = Name only
setting-just-the-part-s-name-d65df8 = Just the part's name, such as "Side"
setting-number-only-5f06b2 = Number only
setting-just-the-part-s-number-7d7fc2 = Just the part's number, such as "3"
setting-single-line-fonts-vectric-font-4746ed =
    Single-line fonts (Vectric font `.vfont` files in Cut Craft's fonts folder) engrave
    fastest; TrueType fonts work too
setting-the-height-of-a-capital-b863c3 =
    The height of a capital letter; smaller parts get smaller text. Text under 2 mm
    (0.08 in) cannot be engraved legibly, so a smaller height is made that tall

## Import Toolpaths (import_toolpaths)
setting-the-toolpath-s-name-with-d87788 =
    The toolpath's name; with more than one tool in the file, each is named after it and
    its tool number
setting-the-g-code-file-nc-80af1b =
    The G-code file (.nc, .tap, .gcode, .ngc, .cnc, .txt). Straight moves and arcs are
    read, arcs in the XY plane and the upright XZ and YZ ones too (G17, G18, G19); a
    drilling cycle is not. Rotary-axis turns (A, B, C) are left out, with a note in red,
    so a rotary file shows and saves flat
setting-the-tool-the-file-cuts-6c4b5d =
    The tool the file cuts with: its shape and size draw the preview; the file's own
    feed rate (from its first cutting move), plunge rate (from its first straight-down
    plunge), spindle speed and tool number are used where it gives them. A toolpath keeps
    one of each, so a file that cuts at a second feed rate, plunges at a second one or
    changes the spindle speed with one tool is noted in red, as saving it again writes
    every move at those. With more than
    one tool in the file each part's tool is named after its number (`6 mm End Mill (T3
    in the file)`), so saving it again still changes tools between them

## Quick Trace (quick_trace)
setting-quick-trace-has-no-settings-aa0d3a =
    Quick Trace has no settings: it looks at the chosen picture and picks them itself, so a
    colour picture comes out in its colours and a black and white or grey one as dark shapes.
    Open Trace Bitmap to see a preview and change any of them.

## 4-Axis Rotary Machining (rotary_machining_toolpath)
setting-the-bit-a-ball-nose-40e65e = The bit: a ball nose follows curves best. Its finishing stepover spaces the rings
setting-how-the-bit-goes-round-a6e7cf = How the bit goes round the part
setting-the-bit-stays-straight-over-36fe1f =
    The bit stays straight over the turning axis while A turns and it moves along: like
    unrolled 3D finishing, but kept true to the round part
setting-off-centre-07cf25 = Off centre
setting-the-bit-is-held-a-dc984d =
    The bit is held a set distance to one side of the turning axis while A turns: the side
    of the ball cuts, not its slow-turning tip, for a cleaner finish, and walls that face
    sideways are reached
setting-indexed-43cc0a = Indexed
setting-a-turns-to-a-few-ec60ad =
    A turns to a few angles in turn and stays still at each, while the bit cuts that side
    back and forth along the part like a flat 3D finish: for flats, faces and machines
    whose A is better at holding still than moving smoothly
setting-sides-then-details-a95ad0 = Sides then details
setting-bulk-to-fine-detail-a-9bb38a =
    Bulk to fine detail: A turns to each side in turn and holds still while the bulk is
    cleared from it, a level at a time, then each side is finished, like a two-sided job
    the machine turns over itself; last, A locks at a few more angles, picked where they
    reach the most of what is left, for the details. The bit never cuts air, and the holder
    above it never touches the material, the chuck or the tailstock
setting-how-far-to-one-side-3d44a4 =
    How far to one side of the turning axis the bit is held. Positive is the way the part
    comes up towards the bit as A turns back
setting-faces-9c251d = Faces
setting-how-many-sides-the-part-276288 =
    How many sides the part is cut from, evenly round it, starting at A=0: A turns to
    each and holds still while that side is cut
setting-roughing-allowance-6ec284 = Roughing allowance
setting-material-the-bulk-clearing-leaves-632bb0 = Material the bulk clearing leaves on the model for each side's finish to take off
setting-tilts-57bacf = Tilts
setting-the-most-angles-a-locks-b7ec04 =
    The most angles A locks at after the sides, for the details: each picked where it
    reaches the most of what the sides and the angles before it left. 0 for none
setting-tilt-angles-c34f16 = Tilt angles
setting-the-angles-a-locks-at-e92e6d =
    The angles A locks at for the details, in degrees, in order (such as 15, 165, 45).
    Left empty, they are picked by themselves
setting-where-along-the-axis-the-386a30 = Where along the axis the rings start, from the job's start (the chuck end)
setting-along-the-part-80c0b2 = Along the part
setting-end-93764a = End
setting-where-along-the-axis-the-f38825 = Where along the axis the rings end, from the job's start; 0 runs to the job's end
setting-material-left-on-the-part-de02f0 = Material left on the part for a later pass, all the way round
setting-registration-mark-ad2169 = Registration mark
setting-a-small-notch-at-a-74552c =
    A small notch at A=0, just past the last ring, so the part can be put back exactly as
    it was if a job has to be started again
setting-cut-support-efe14c = Cut support
setting-cut-away-the-centre-stock-3b5b5d =
    Cut away the centre stock Job Setup leaves along the axis (Holding the Part), down to
    the model: for the last toolpath, once everything else is cut. The ends the part is
    kept joined at stay as they are

## Rotary Lathe (rotary_lathe_toolpath)
setting-the-bit-a-square-end-a1b221 =
    The bit: a square end mill leaves a flat face. Its stepover spaces the turns along
    the part and its pass depth the steps down
setting-operation-8e02a6 = Operation
setting-what-the-lathe-does-6cf93f = What the lathe does
setting-lathe-e94e05 = Lathe
setting-face-122cfc = Face
setting-takes-everything-between-start-and-aa7d02 =
    Takes everything between Start and End down to the holding stub: to square off an
    end of the material, or clear room beside the part
setting-takes-everything-between-start-and-38438e =
    Takes everything between Start and End down to the model's outline, round: the
    quickest way to rough a round part, before the carving
setting-part-off-864ea1 = Part off
setting-cuts-a-groove-one-bit-39bfe9 =
    Cuts a groove one bit wide at Start down to the holding stub, so the finished part
    can be snapped or sawn free
setting-where-along-the-axis-it-44e8c0 =
    Where along the axis it starts, from the job's start (the chuck end); for Part off,
    where the groove is
setting-where-along-the-axis-it-0fada5 = Where along the axis it ends, from the job's start; 0 runs to the job's end
setting-holding-stub-aa4acc = Holding stub
setting-across-the-stub-left-on-ac7faa =
    Across the stub left on the turning axis, joining the part to the rest of the
    material until it is cut free (for Turn, where there is no model)
setting-material-left-on-the-model-0c768e = Material left on the model's outline for the carving after
