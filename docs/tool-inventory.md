# Tool inventory

Date: 2026-10-24, updated 2026-11-01 (version 0.4.3). What every tool is built from, what it is reused by, and whether it works.
"Works" is checked by `every_enabled_control_works` (crates/app): it builds a small job (two
overlapping squares, a toolpath, a 3D shape) and runs every enabled menu item, tool button and
right-click menu item through the app as a click would, failing on a panic or a dead end
(92 controls today). Each command crate also
runs the shared contract tests and its own behaviour tests.

## How the tools stack

```
surfaces     menus · tool tabs · right-click menus · shortcuts · start page · view buttons
               (no logic: each one runs a tool by its id)
parent tools   Center, Center in Material, Preview All, Recalculate All, Show Only This, ...
               (built by running subordinate tools in turn: Change::chain, or a loop)
subordinate    Center Horizontally, Layer Properties, Recalculate, Preview Toolpath, ...
foundation     document + undo, the command contract, geometry, polygon booleans,
               toolpath building blocks, the model, the simulation, files, forms, views
```

A surface that reaches an existing tool another way adds no code: Edit ▸ Select All Vectors
On Current Layer is Select Vectors on Layer for the current layer; Edit ▸ Job Size and
Position and the start page's Create a new file are Job Setup.

## The foundation (used by every tool)

| Piece | Gives | Used by |
|---|---|---|
| Document, Change, History (`core/document`, `core/change`) | every edit as an undoable change; vectors, selection, layers, components, toolpaths | every command |
| Command contract, Registry (`core/command`) | params in, change out; one way to run a tool by id | menus, tabs, shortcuts, forms, tests |
| Geometry (`core/geometry`) | paths, flattening, transforms, picking, inside tests | drawing, editing, selection, every toolpath |
| Polygon (`core/polygon`) | union, difference, intersection, offset, regions with holes | Weld, Subtract, Overlap, Offset, Profile, Pocket, V-Carve |
| Toolpath blocks (`core/toolpath`) | contour moves, pass depths, cut points, zigzag, follow, tool shapes, tool database | every toolpath strategy |
| Model (`core/component`) | components, combine modes, the combined model, the placed model | Create Shape, 3D view, STL export, 3D toolpaths |
| Simulation (`core/simulation`) | material grid, cutting, drop cutter, overcut check | Preview, 3D view, 3D toolpaths' gouge check |
| Files (`core/file`, `crates/formats`, `crates/post`) | job files, import/export, post-processors | File menu, Save Toolpaths |
| Forms (`app/form`) | every settings form, generated from the tool's parameters | every tool with settings |
| Views (`app/view2d`, `app/view3d`) | zoom, pan, rulers, picking; the lit 3D mesh | 2D and 3D views |

## Parent tools and what they reuse (built, working)

| Parent | Built from |
|---|---|
| Center | Center Horizontally + Center Vertically |
| Center in Material (F9) | Center Horizontally in Material + Center Vertically in Material |
| Align Left / Right / Top / Bottom / Centre H / V | one align-selection block |
| Move, Rotate, Mirror, Set Size | one transform-selection block |
| Weld, Subtract, Overlap, Offset | polygon union / difference / intersection / offset |
| Profile, Pocket, V-Carve | polygon offsets (V-Carve: regions with holes) + contour moves + pass depths |
| 3D Roughing, 3D Finishing | placed model + machining boundary + raster or offset lines dropped along (each move checked with the simulation) + follow |
| Recalculate All Toolpaths | Recalculate, for each toolpath |
| Preview All Toolpaths | Preview Toolpath, for each toolpath, on fresh material |
| Delete Layer | removing its vectors (as Delete does) + removing the layer |
| Layers panel eye / padlock | Layer Properties |
| Zoom to Selection, Zoom Box | Zoom to Job's fit, on the selection or the box |
| Select All Vectors On Current Layer | Select Vectors on Layer |
| Job Size and Position; Create a new file | Job Setup |
| STL export, model in the 3D view | the combined model |

## Right-click menus: the shared families

Aspire's popup menus (see `re/SPEC.md`, Right-click menus) repeat a few families across kinds
of item. Each family is one parent tool; each kind needs one subordinate tool that sets a
single item's visibility, name or mode. ✅ built · ⬜ to build.

| Family | Items | On (Aspire menu) | Subordinate per kind |
|---|---|---|---|
| Show / Hide | Show This, Show Only This, Show All But This, Show All, Hide This, Hide All | layers (967), toolpaths (968, 669), components (613), levels (1046) | Layer Properties ✅ · Toolpath Properties ✅ · Component Properties ✅ · Level Properties ✅ |
| Delete | This, Visible, Invisible, Empty, All | layers (967), toolpaths (968) | Delete Layer ✅ · Delete Toolpath ✅ · Delete Component ✅ |
| Combine Mode | Add, Subtract, Merge, Low, Multiply | component, several components, active component, 3D view, levels (613, 615, 616, 619, 1046) | Component Properties ✅ · Level Properties ✅ |
| Recalculate | This, Visible, All | toolpaths (968) | Recalculate ✅ (Visible: Recalculate over each shown toolpath) |
| Rename, Properties | - | every kind | each kind's properties form |
| Activate | - | layers (967), sheets (5030) | Set Current Layer ✅ |
| Insert New | - | layers (967, 982), levels (1046, 1048), sheets (5030) | Add Layer ✅ |
| Selection (2D) | Undo, Redo, Cut, Copy, Paste, Delete, Move to Front / Back, Ungroup, Properties | 2D view (318) | Undo, Redo, Delete ✅; the rest ⬜ |
| Node editing | span to line / arc / curve, insert / delete point, cut, close, join | node edit (142, 317, 635, 636) | ⬜ (no node editing yet) |

Built, in `crates/app/src/gui.rs`, as menu data rendered by the main menu's own code. An
item either runs a tool by id or is `run:<command>:<json>` (apply it with those params) or
`form:<command>:<json>` (open its form filled in); none has logic of its own.
- **Layers panel (967):** Activate, Show ▸, Hide ▸, Lock / Unlock (the one that applies),
  Insert New Layer, Delete ▸ This / Visible / Invisible / Empty, Rename, Select Layer Vectors,
  Move Selection Here, Merge Visible (every visible layer's vectors onto this layer, the
  emptied layers removed). Arrows over the list move the active layer up or down it, and
  Show Only Layers Used on This Sheet leaves out the empty ones.
- **Toolpath list (968):** right-click selects the row. Show ▸, Hide ▸, Rename, Recalculate ▸
  This / Visible / All, Preview, Delete ▸ This / Invisible / Visible / All.
- **Component list (613):** Combine Mode ▸ (current one ticked), Duplicate (a copy in place,
  on top), Delete, Move to Level ▸
  each level (ours: Aspire moves components between levels by dragging), Rename, Show ▸,
  Hide ▸, Properties. Right-clicking an unpicked component picks it first.
- **Levels (1046), the headers of the component tree:** Combine Mode ▸, Show ▸, Hide ▸,
  Clipping ▸ Apply / Update / Remove, Mirror Mode ▸ (the eight modes and Off), Wrapping
  (rotary jobs), Insert New Level, Delete Level (with its components; the last level
  stays), Rename Level, Properties (Level Properties: name, combine mode, visible, base
  height, mirror mode, wrapping). Select Components and Export as 3D Clipart are not done.
- **2D view (318, plus ours):** Undo, Redo, Cut / Copy / Paste (greyed), Delete, Move to
  Layer ▸ each layer, Weld, Subtract, Overlap, Join, Align ▸. Object Properties opens the selection's own form (its drawing tool, Bitmap or Component
  Properties, else Set Size).

Not built from Aspire's menus yet: toolpath Show All With This Tool; component 2D preview
order, Re-Import, Copy, Move to Front / Back; the no-item menus (layers 982, components 614).

## Status, top down

What works and what does not, by menu and tab; items not built yet are greyed and
say "not implemented yet".

**Working.**
- File: New, Open, Close, Save, Save As, Increment and Save, Recent Files, Import Vectors /
  Bitmap (bitmaps: PNG, JPEG, GIF, BMP, TIFF in grey, shown in the 2D view), Export DXF /
  SVG, Open Application Data Folder, Exit.
- Edit: Undo, Redo, Cut, Copy, Paste, Delete; Select All Vectors / Open Vectors / Duplicate
  Vectors / On Current Layer, Select Vectors Inside, Unselect All; the ten Align commands and the four spacing ones
  (Left To Right ... Bottom To Top; laid out in pick order, the first picked at the named start [inf]); Join Vectors, Group, Ungroup, Job Size and Position.
- Selection: pick order kept; components picked in the Components tab; bitmaps picked in the
  2D view (Bitmap Properties: position and size; Delete removes them).
- Smart part selection (Aspire 12.5) ✅: a click inside a closed shape, away from its lines,
  picks the smallest shape round the click with everything wholly inside it (holes, and
  anything in them) as one part, without grouping; Shift-click adds or takes away the part.
  Select Vectors Inside (Edit ▸ Selection and the 2D right-click menu) adds what lies inside
  the selected closed shapes. Not yet in the 3D view.
- Model: Add Zero Plane, Create Component from Bitmap / Visible Model / Toolpath Preview, Relief
  from Photo (Cut Craft's own: Estimate Depth runs the optional Depth add-on on the chosen
  photo, or a depth map made elsewhere is used as it is; background cut by colour and by
  depth, invert, fine detail, sharpen, smoothing, lift, soft edge and base shape it into a relief
  that follows its settings live), Bake
  Components, Smooth, Reset Inside / Outside Selected Vectors, Scale Model Z Height, Selected
  Component Properties, Export as STL (its form: limit by tolerance or triangle count; back
  left open or closed by a flat plane, the inverted front or the other side; Triangulate shows
  the triangle count and largest error), Export as Grayscale Bitmap.
- Machine: Install Post-Processor (real grbl / FluidNC posts load, rotary G93 ones included).
  Install Offline Package adds the machines in a package (`machines/*.json`: Genmitsu,
  FoxAlien, Sienci, Carbide 3D, Two Trees hobby routers, on Cut Craft GRBL (mm), whose tool
  change pauses with M0), skipping names already there; Search for Machine Online opens
  `machines/` in the repository. Cut Craft's own posts: FluidNC ATC (flat and wrapped A),
  and for FluidNC ATC, GRBL (tool changes pause with M0) and Mach3 (`.tap`; T M6, G91.1
  arcs), in mm and inches: Arcs, Straight, and Wrap Y / X to A with or without G93; all
  written for Cut Craft (`scripts/make_posts.py` makes the variants from three templates).
  Package machines list GRBL Arcs and Straight in mm and inches, Arcs in their units first.
  A machine's 4th axis (A) tick adds the G93 Wrap Y / X to A twins of its Cut Craft posts;
  a rotary job saves through the machine's post wrapping its side (Y for a cylinder along
  X), and is refused on a machine without the 4th axis or without such a post. Flat jobs
  skip rotary posts. Post-Processor Management's Inspect has Edit for custom
  posts (Customize opens its copy in it): saved to the file, warning while it would not load
  or after a rename. Changelog shows a post's "+ History" comment rows, and the list shows
  its version (the date of its last change). Machine ▸ Update Post-Processor Database opens
  Post-Processor Management, says how many posts there are (and how many will not load), and
  checks for a newer Cut Craft, which is how Cut Craft's own posts are updated (custom posts
  are read from their folder each time).
- Job Setup: single-sided and rotary jobs. The form changes with the job type as Aspire's
  does: Width / Height / Thickness for a flat job; Length (L), Diameter (D) and the
  Orientation section for a rotary one (forms hide or show a field with `x-show-when` /
  `x-hide-when`). Rotary Z Zero is still labelled Material Surface / Machine Bed, and Flip
  design is not done.
- Save Toolpaths (a form, as Aspire's): the machine and its posts suiting the job (the
  machine's default first, else Cut Craft's own FluidNC posts: tool changer, rotary jobs
  wrapped onto A in inverse time); the selected toolpath, the shown ones to one file, or each
  shown one to a file of its own (`name_N_Toolpath`), Group Where Possible putting toolpaths
  in a row with one tool together (labelled Merge Toolpaths in a Row That Use the Same Tool;
  the order never changes); Add Side to Toolpath Name on double-sided jobs. Tool Changes:
  Separate File for Each Tool (`name_N_Tool`; the default without a tool changer, and the
  only way with a post that cannot change tools), Tool Changer (ATC; the default with one,
  greyed without), or One File, Pause to Change the Tool (M0, the post's own tool change if
  it already stops, else Cut Craft's). The checks before cutting are listed in the form;
  Save is greyed while any must be fixed.
- Checks before saving (Cut Craft's own, `core/preflight`): the toolpaths are simulated in
  cutting order. Errors stop the save: a rapid move into material still there (crash), a
  toolpath with no moves, no feed or plunge rate, toolpaths wider than the machine's bed or
  deeper than its Z travel. Warnings ask first (or go to the status bar, per Options): air
  cutting (30% and 100 mm or more of a toolpath's cutting takes nothing off), out-of-date
  toolpaths, no spindle speed, speeds outside the machine's spindle range or over its max
  rates, a tool not loaded in the tool changer, cuts over 1 mm into the spoilboard, plus
  the flute-length and taper checks.
- Toolpaths menu: Show Toolpaths Tab, Recalculate All, Toolpath Drawing (2D previews, solid
  previews, auto-open 3D view, rapid / plunge / retract / join moves; a join is a feed move above the material [inf]), Preview Simulation Quality,
  Merge Visible Toolpaths, Tool Database.
- View: Zoom to Drawing (F6), Refresh 2D View, Draw Origin / Modeling Plane / Material Block /
  Vectors in 3D View, Light Follows Viewer, Color Shaded View, Use Shaded Background,
  Auto-Wrapping View (rotary jobs, on by default: the model or preview drawn round the
  axis, closed at the seam, each face lit by its own facing; the vector, plane and origin
  overlays are left off it, and it has not been tried by hand yet).
- 3D view: left- or right-drag rotates, middle-drag / Ctrl+right-drag / left+right pans, the
  wheel zooms about the cursor, F (or a double-click off the model) fits; Top, Front, Right, Iso, Fit buttons.
  A right-click picks the component under it and opens the 2D/3D view menu headed by its
  name (Cut, Copy, Paste, Duplicate, Delete, Group, Ungroup, Select ▸, Activate Layer of
  Selection, Text ▸, Convert To Vectors, Join / Close Vectors..., Node Edit Mode, Appearance,
  Component ▸ with Create Vector Boundary, Copy To ▸, Move To ▸; the vector items greyed for a
  component; the components under the click are listed first, the selected ticked, a click
  picking one). In Transform mode (a second double-click on a selected component, or the
  Transform icon) Aspire 12's component editing icons sit along the near edge of the
  selected component's box: Total Height, Shape Height and Base Height (drag up or down; a
  quarter of the view doubles, the base height moves the material's thickness over it,
  the component following the drag, one undo in all) and Additional Settings, a popup with
  the combine modes (Aspire's icons, also in the tree's Combine Mode menus and the
  Component Properties form), Fade and Tilt (Set, then two clicks on the model in the 3D
  view: the point that keeps its height or stays put, then the point faded toward or tilted
  up, as the help says the 3D view does), Appearance (Same as Parent or Solid Color) and its
  colour, all applied through Component Properties. A component's Solid Color
  shows on the GPU-drawn surface [inf: the CPU path of the headless tests leaves it out].
  The 3D Transform Handles sit over the selection's box (vectors or components), level
  with the highest surface inside it: a cross in
  the middle moves, white corners and black edges scale, and a rotation arrow on a stem
  above the far edge turns it, with the 2D view's keys; the drag shows the box as it will
  be and is applied when let go (one undo). Escape cancels a drag; let go off the material
  (over the horizon in perspective) it changes nothing. [inf: the manual's edit box for an exact value and
  moving the rotation centre are not offered; Move, Scale and Rotate take exact values.]
- Drawing in the 3D view (Aspire 12 [inf: "the drawing tools are now available in the 3D
  view", What's New in V12.0]): with a drawing tool's form open, the left button draws in the
  3D view as it does in the 2D view (drags, clicks, Shift-click to edit, the polyline's
  clicks, text, trim, dimensions, cross section), at the point where the pointer meets the
  model (off it, the material's top plane). The shape being drawn shows on the model; the
  right button still turns the view, the middle pans, the wheel zooms; snapping works as in
  2D and the status bar adds "right-drag turns the view". Vectors are still flat 2D vectors.
  Not offered while the job is shown wrapped round a rotary axis, while a 3D model is being
  imported, or with a view tool on; nor before anything is in 3D (the view is empty then).
- Import 3D Model is one page (Aspire 12 has two; Matthew asked for one): Initial
  Orientation, Model Size, the model type and Scale to fit, Position relative to Modeling
  Plane (Position, Depth Below Top, Create both sides, Discard data below zero plane), Align
  in Material (sides, centre, and Bottom / Middle / Top in the material), Highlight undercuts
  with Update Undercuts, Import and Cancel. Center model is shown as the position it gives.
  Depth Below Top reads the model's height until a depth is typed; past the height it lifts
  the model off the plane, up to the material's top. Every dragged value moves the model as
  it is dragged. [inf: Aspire's slider
  beside the picture is not drawn; Overcut Distance is not offered.]
- Help: About, Keyboard Shortcuts, Third Party Licenses, Check for Updates (crate
  `crates/updater`). Asks GitHub's releases (never a mirror) for a newer stable version,
  at startup about once a day (Options ▸ Updates turns that off) and from the menu. Update
  Now downloads the release's file for this install (Windows .zip, Linux .tar.gz or
  AppImage), refuses it unless it carries a minisign signature from the release key built
  into the app (`crates/updater/release-key.pub`, its trusted comment naming the file) and
  matches GitHub's SHA-256 checksum, swaps the program in
  (the old one kept as `.old` until the next start), then offers Restart Now, which asks to
  save first. Flatpak, macOS, other processors and builds from the source code are told how
  to update by hand instead. Skip This Version quiets the startup check for that version.
- Side tabs: Sheets (the Sheets List with each sheet's X, Y, Z; Add New Sheet; Activate,
  Show, Hide, Edit Sheet, Rename, Delete; objects are not kept per sheet yet), Components
  (headed with the sheet; level menu Select Components and Export as 3D Clipart), Library
  (Aspire's Clipart tab, renamed: folders of your own files, kept between runs, as a tree
  of the folders in them however deep, each with how many files are in it; hidden folders
  and links to folders are passed over. A folder is listed once: adding one inside a folder
  already added opens it in the tree instead, adding one around folders already added
  takes their place, and duplicates kept from before are tidied as the library loads. Click a folder to see what is in it and the folders
  in it, or All folders for everything; the one looked in is remembered. Right-click a
  folder: Open Folder, and Remove Folder for one added. Folders are read in the background
  and again every 5 s; only the rows of tiles on screen are drawn, so thousands of files
  open at once (at most 100 000 listed). A missing folder is marked "not found" and says
  how to fix it). Double-click a tile to add it to the project
  centred on the job, or drag it onto the 2D or 3D view to add it centred where it is
  dropped; a model goes through Orientate 3D Model already placed and levelled. Click picks
  a tile, Ctrl+click adds or removes one, Shift+click picks a range. Right-click (on what is
  picked): Add to Project (several are added one after another, each model's form in
  turn), Add to Level (one of the job's levels, or a New Level each, named after the
  file), Locate (shown picked in the file manager: Explorer, Finder, or a Linux one
  answering `org.freedesktop.FileManager1`, else its folder opened), Add Tag / Remove Tag
  (kept in the config folder's `library-tags.json`, never in the files), Delete from Disk
  (asks first, naming the files; a model's picture goes with it; not to the bin). A Show
  filter picks Everything, 3D models, 2D drawings, Pictures or Depth maps (greyscale
  pictures, by their header, or tagged "depth map"); Search finds names and tags. A 3D
  model's tile shows a preview: `mydesign.png` beside `mydesign.stl` if there is one (and
  that picture gets no tile of its own), else the model rendered from its largest side
  (from above, or face on for one saved standing up), once, on background threads for the
  tiles on screen, and kept in the cache folder until the file changes; previews unused for
  60 days are cleared. Vectric's locked `.3dclip` / `.v3m` files are counted and explained.
  Not done: subfolders, thumbnail sizes.
- Top bar over the views: 2D over 3D or beside it; sheet, layer and level drop-downs;
  geometry, smart and grid snapping toggles. Smart snapping lines the pointer up with any
  snap point across or up and down, and puts it on a span's tangent at the span's ends or
  square to it (Aspire's woken nodes and connecting lines are not done).
- Menus and drop-downs draw each category in a border.
- Number fields take sums (Calculation Edit Boxes): + - * / ^, brackets, W H T, P, I, M, '.
- Sheets keep their own vectors, components, pictures and toolpaths; activating one swaps them.
  A sheet's right-click menu copies or moves the selected vectors onto it and inserts a new
  sheet of its size; a toolpath's Copy to Sheet copies it there with the vectors it was made from.
- Edit ▸ Options (3D view, toolpath and general settings, in options.json) and Edit ▸
  Document Variables (used in sums as {name}). Options also has the 3D view's background
  (solid, gradient or picture, with its colours) and anti-aliasing (4x, from the next start);
  the 2D, 3D and V-carve toolpath tolerances (saved toolpaths drop cutting points within them,
  Douglas–Peucker); the 2D solid preview colour; the file dialogs' folder (the desktop's or the
  job's); and Confirmation Choices: crossing vectors for pocket and V-carve toolpaths and for
  radiused and sharp offsets (ask, open the Vector Validator, or don't check), and whether the
  checks before cutting ask before saving.
- Keep-Out Zones (Toolpaths menu; named, with labels in both views; a toolpath that would cut
  into one is not made unless Options allow clipping it there), Transform mode
  (handles to move, scale, rotate), guide lines from the rulers (View ▸ Guide Lines).
- View ▸ Save Shaded Image: the 3D view (the model, or the preview) saved as a PNG, cut
  from the window's picture once the 3D view is shown.
- Double-sided jobs (Job Setup's Double Sided, Flip Direction Between Sides): each side keeps
  its own objects; Edit ▸ Swap Sides or the top bar's side button turns over; the other side
  is drawn faint and mirrored; the 2D right-click Copy to Other Side mirrors copies across.
  Preview All (and Preview Visible) cuts both sides: the 3D view draws the other side's
  cuts on the block's underside, turned over as Job Setup's flip says, cut through where
  the two sides meet; Swap Sides turns the preview over with the job.
- Plate Production (Offset and Layout): a template's !!variables!! filled from a data file's
  columns or a counter, plates laid out inside margins, a layer per sheet of material when
  they need more. The data file is read as spreadsheets save CSV: quoted fields (a comma in
  "Smith, John"), UTF-8 or Excel's Windows encoding. Create Toolpaths makes the template's
  toolpaths per sheet; one cutting the !!variable!! text is recalculated straight away for
  each plate's own text. Rows whose text runs past the plate are named in a warning that
  points to Text in a Box (which shrinks long names to fit). Not done: one job / toolpath
  file and proof picture per row with names from a column, a review of every plate before
  saving, XLSX.
- Toolpath Tiling (Toolpaths menu): individual tiles or feed-through in X or Y, tile size,
  overlap, smallest tile first; tiles drawn in the 2D view; Save Toolpaths writes one file
  per tile (`name_tileN`), cuts clipped to the tile and its overlap and moved to the job's
  corner. Active Tile (or a double-click on a tile while the form is open, shaded in the
  2D view) makes the preview cut only that tile's moves, at the machine's corner or, with
  Draw Toolpaths in Original Position, where the tile sits in the finished piece.
- Gadgets ▸ Add-ons (Cut Craft's own): installs and updates add-ons from a GitHub
  repository's newest release, official ones (Depth) listed with their repositories built in,
  signed ones straight away and unsigned ones after a warning.
- Gadgets ▸ Wrapped Fluting Layout and Wrapped Spiral Layout (rotary jobs): flutes or
  rope-twist strands, with coves, laid out as vectors for Profile / Fluting toolpaths.
- Gadgets ▸ Celtic Weave Creator: a knot grid's loops as smooth closed vectors (crossings
  cut vertically or horizontally from a list; Aspire picks them on a preview image) and a
  half-round cross-section, for Extrude and Weave.
- Animated toolpath preview (Toolpaths ▸ Toolpath Drawing ▸ Animate Preview, on by default):
  A double-click on the previewed material in the 3D view takes away the piece cut free
  there (waste or part).
  Run / Pause, Single Step, Run to Retract, Stop (keeps what is cut), a slider to scrub
  along the path and a speed slider over the 3D view; the tool drawn as an upright
  cylinder. Not done: the tool's real shape, Preview All Sides.
- Rest machining for every toolpath (a Rest Machining section in each toolpath form; 3D
  Finishing keeps its own): what the toolpath would leave is simulated, each earlier tool's
  best try at it shows what that tool left, and only the cuts over areas every earlier tool
  left are kept (minimum detail, overlap; a cut counts where the tool's edge is over what was
  left), each leftover spot finished before the nearest next. Kept with the toolpath, so recalculating does it
  again. The rest grid is a quarter of the tool across [inf].
- Toolpath menu: Duplicate. Edit Objects: Edit Picture (contrast, brightness, gamma,
  invert, a rectangular or oval border fading to white) and Crop Bitmap.
- Tool Database: flutes and chip load; Radiused End Mill (toroidal) tools; taper angle, shank
  diameter and flute length (tapered ball noses and flat bottoms are cut with their whole
  taper, read from .vtdb and Fusion .tools files). The editor draws the tool to scale from
  its sizes, each typed on its dimension line, with neck and shank lengths and a
  Manufacturer (`{Manufacturer}` in name formats).
  Import of Vectric `.vtdb` tool databases (the vendor libraries for Aspire / VCarve 12.5):
  ball nose, end mill, radiused end mill, V-bit, engraving (as a V-bit), tapered ball nose
  (its tip, taper and shank), drill (as an end mill); one tool per material it has cutting data for.
  Form tools and diamond drags are skipped.
- Checks before cutting (Cut Craft's own; Aspire has none): a toolpath deeper than its tool's
  flute length (or with none set), a 3D finish whose taper cannot reach part of the model, and
  an STL's undercuts; shown when made, and Save Toolpaths asks before writing them.
- Every toolpath strategy in `re/SPEC.md` (backlog item 19): Profile, Pocket, Drilling,
  V-Carve (with Start Depth), V-Carve Inlay, Inlay (pocket / plug), Quick Engrave (Ctrl+Q),
  Chamfer, Fluting, Prism, Thread Milling, Keyhole, Drag Knife, Moulding, Texture, Photo V-Carve,
  Sketch Carving, 3D Roughing, 3D Finishing (with rest machining), Rounding (rotary), Laser
  Cut / Fill and Laser Picture (need a post with LASER_SUPPORT), Array Copy, Merged. What
  each leaves out is in its `re/SPEC.md` section.
- Drill Bank (Aspire 12.5's Advanced Machining Module): Machine Configuration ▸ Drill Bank
  sets up a head of drills on an X and a Y arm (count, pitch, where the arms cross, each
  drill's controller number and diameter, the reference drill), with a picture of it. The
  Drill Bank Toolpath (Toolpath Operations, beside Drilling) takes the active machine's bank,
  matches the selected circles to drills by size and position (pitch and diameter
  tolerances), and drops as many drills per plunge as line up; holes no drill fits are named
  with the sizes there are. Peck, retract and dwell as Drilling. Posts with
  `SETUP_DRILL_BANK` / `FINAL_DRILL_BANK_ACTION` and either `DRILL_BANK_PATTERN` or
  `DRILL_BANK_EXTEND_DRILL` + `DRILL_BANK_RETRACT_DRILL` write it (`[DB]`, `[DBX]`, `[DBY]`,
  `[DL]`, `[CD]`); others refuse it, saying what is missing. Array Copy and Plate
  Production move its plunges with it; Merged refuses it. Not done: a Cut Craft post with
  drill bank blocks (none of the hobby controllers has one), a toolpath icon of its own (it
  borrows Drilling's), the simulation and time estimate count every hole as its own plunge,
  vector selection templates, projection onto the 3D model, and the bank in the Tool
  Database (`.vtdb` `drill_bank_*` tables are not read).
- Toolpath options: Profile ramps (smooth, zig zag, spiral; by distance or angle), leads
  (straight or circular, lead out, overcut) and tabs (2D or 3D, by number or distance); Pocket
  Offset or Raster (angle, profile pass first / last / none) with ramped plunges; 3D Roughing
  Z Level (level by level or depth first) or 3D Raster, and 3D Finishing Raster (any angle) or
  Offset, both inside a machining boundary (model, material, selected vectors or a level,
  plus offset). Tabs are sized from the tool (pass depth thick, twice that long) and spaced about every
  150 mm by default (by distance, kept between a minimum and maximum number); they can
  start at the start point, keep off corners, or be placed by clicking the vectors (a click
  on a tab takes it off, a drag moves it along); climb or conventional and an allowance on Profile and Pocket; Z Level
  profile pass, reverse step and (3D Raster) avoid machined areas; neighbouring 3D lines are
  joined without lifting, offset finishing rings lifted by a stepover retract.
- Profile: nested shapes cut on the right side, inner before outer; cut order (selection
  order, left to right, bottom to top, grid, shortest path; the shortest of those ticked wins);
  start points (keep, optimise, nearest a point of the bounding box); sharp external corners.
- V-Carve: Use Vector Start Points (each pass begins nearest its vector's start) and Use
  Vector Selection Order (each shape, with its holes, carved before the next selected).
- Moulding: Vary Stepover (passes a stepover apart along the profile's surface); a Larger
  Area Clearance Tool roughing first in levels as its own [Clear] toolpath, with Machining
  Allowance, Machine Flat Regions and the finish's Skip Flat Regions.
- Tool picking in toolpath forms: on a machine with a tool changer, the magazine row comes
  first: 0 is a bit changed by hand (picked from the Tool Database, written as T0), then each
  holder with its tool. Every tool in a list (clearance tools, finishing tool sets, rest
  machining) has the same picker, Edit... and summary.
- Snap grid: while Snap to Grid is on, the 2D view draws its points over the job (thinned
  when zoomed out so they never crowd); Options ▸ Appearance sets the grid colour and point size.
- Project toolpath onto 3D Model: Profile, Pocket, Fluting, Texture, Photo V-Carve, V-Carve.
- Clearing bits on Pocket and V-Carve ("Clear with bigger bits first" / "Clear the flat
  bottom with end mills first", each bit picked with Select… in the Tool Database, kept
  largest first; Pocket checks each is bigger than the finishing bit and none the same size;
  the separate Rest Machining list shows on these forms only for an older job that has one):
  each makes a toolpath of its own, named `[1]`, `[2]`... in
  cutting order. Pocket's later tools (its own last) cut only what the ones before left
  (rest machining, kept on each so it recalculates alone; a raster only for the first).
  V-Carve's end mills pocket the flat bottom (needs a flat depth); the V-bit then carves the
  slopes and only the smallest clearance radius into the flat bottom, so the corners a round
  end mill misses are still cut.
- 3D Finishing takes a list of tools and makes a toolpath for each (rest machining).
- Posts: G2/G3 arcs (and helices) fitted to curves when the post has arc blocks, Cut Craft's
  own FluidNC posts included; Plunge (Z2) and the Home / Start Position in Job Setup.
- Tool Database form tools: Form Tool is a tool type whose cutting edge is a profile;
  Use Selected Vector takes it from the job's selected open vector (half the section,
  drawn upright, the axis at its left and the tip at its lowest point; the diameter is
  twice its width). The preview and the toolpaths cut with that shape. Form tools in
  `.vtdb` files are still skipped, and no picture is drawn for the type.
- Tool Database name formats (help: Tool Database - Name Format): a tool's Name Format
  (`{Tool Type} {Diameter|F}`, the variables from a list, modifiers U, L, 0.00 and F for
  inch fractions) names it; Set as Default makes it the format new tools of that type are
  named by, Restore puts it back; a toolpath's copy is named for the active material.
  Not done: custom variables, Update Existing Tools, machine variables.
- Tool Database: cutting data per material (imported from `.vtdb` and Fusion presets), an
  active material, rate units per tool, thread mills as a tool type. Tools with no data for
  the active material are greyed; the tree's right-click Hide Unset Tools leaves them out.
  A search box over the tree finds tools by name (indexed by word: each word typed matches
  the start of a word in the name, in any order and case).
- 3D view: ViewCube (faces, edges, corners), perspective, tilting underneath, overlays hidden
  behind the model.
- Drawing tab:
  - Select, Job Setup, Pan, Zoom Box, Zoom to Job, Zoom to Selection, Switch Layout.
  - Polyline, Rectangle, Circle, Ellipse, Polygon, Star, Arc; Cut Craft's own Rectangle
    from Center and Circle from Edge (the drag is the diameter).
  - Toolbox tool groups (Cut Craft's own; Aspire shows every tool as a loose icon): tools
    that do one job different ways sit together in a box captioned with what they are for,
    every tool its own button (`crates/app/src/flyout.rs`). Options ▸ General ▸ Fold tool
    groups into one button (or a toolbox button's right-click menu) folds each box into a
    flyout button with an arrow in its corner, showing the one picked last; click the arrow,
    hold the button, or press Down on it for the rest. Folded: Drawing tab 78 buttons to 44,
    Modeling 23 to 20.
  - Design sub-tabs (Cut Craft's own; Aspire runs every group down one page): File
    Operations and 2D View Control stay at the top; Create, Edit, Arrange and Model across
    the page each show a few groups (`DESIGN_SUBTABS` in `crates/app/src/gui.rs`). A tool
    run from a menu, a shortcut or Find Tool opens its sub-tab. Edit splits Aspire's Edit
    Objects into Select and Edit, and Join, Combine and Trim.
  - Next-step guidance (Cut Craft's own): a box in the room under a page's icons names the
    hovered tool, what it does and what to do next ("Select the closed vectors to pocket",
    "Drag from the centre out to the radius"); with nothing hovered, the tool in hand's. The
    status bar carries the same line, each tooltip ends with it, and a form opens with it
    at the top while its tool has nothing selected. The step is the tool's own: run on an
    empty job, a command says what to select (`crates/app/src/app/guide.rs`).
  - Edit ▸ Edit Object (E) opens the tool that drew the one selected vector on it, and a
    Shift-click in a drawing tool's form edits another shape that tool drew; a change to
    the form then changes that shape. Rectangles (square, rounded or notched corners, not
    turned) and circles are read back; ellipses, polygons and stars not yet.
  - Draw Line / Polyline in the 2D view as Aspire's: a click adds a point, a click and drag
    a smooth span (the next one smooth too), S swaps curve mode and straight lines (a click
    on the last point does too), Backspace takes a point back, Tab closes, Space finishes
    and starts another, Esc or a right-click finishes, a click on the first point closes;
    typed values then Enter place the next point (a length along the pointer, `x,y`, or
    `angle A length L`). Node editing's right-click on an open vector's end node has Edit
    Polyline, drawing on from it into the same vector. Not done: Ctrl-drag from an end,
    tangent lines (T).
  - Text: system TrueType fonts and Vectric single-line `.vfont` fonts (from
    `~/.config/cut-craft/fonts`), bold, italic, alignment, capital height, click to anchor.
    The text is a group that remembers its settings: Draw Text on selected text edits it;
    Ungroup turns it into curves. With the form open on selected text, two green handles
    above and below it bend the line into an arc (the middle follows the pointer; drag back
    to flatten); Arc angle and Arc rotation are typed too. Each letter stays whole. Not
    done: the red / blue / white handles (rotate, radius, move) and kerning clicks on arched
    text.
  - Draw Curve: a smooth Bézier curve (Catmull-Rom) through a list of points, open or
    closed; points are entered in the form (Aspire places them with clicks).
  - Draw Freehand: the whole drag path, smoothed into Béziers within a tolerance.
  - Create Vector Texture: wavy lines at an angle over the selected closed vectors or the
    job, max spacing with variation, wavelength, amplitude, noise and a seed; on the
    current layer, or (Place Vectors on Layer) on the layer named, made if needed.
  - Embellish: folk-swirl ornament designed for the selected closed vectors rather than cut
    out of a pattern, read from each shape's distance to its edge: a border band and bead
    row follow the outline; a shape with n-fold mirror symmetry (a regular polygon, a star,
    hanging-chain sides) is designed one wedge at a time and mirrored into the rest (corner
    stems and leaf fans, side sunbursts, a centre rosette, the biggest scrolls that fit);
    any other shape gets a vine down its middle with curls at its ends and scrolls
    branching off it, leaf fans in its points and flowers where it is roomiest. Where (Fill
    / Border / Both), Refinement, Number of objects, Scroll size and Tightness (smallest gap
    3 to 0.75 mm) are sliders; it makes grouped vectors, a 3D component of Height, or both,
    with stippled ground if asked, and follows its settings live (one undo).
    The border can instead be a pattern run round the edge with a whole number of
    repeats so it closes on itself: Rope (twist angle), Chain (face and edge links), Twist
    (two plaited strands), Braid (three) or Bead and reel, with Border width, Repeat size,
    Inset from the edge and optional rails; the ornament inside keeps clear of it.
  - Trim Objects (Edit Objects): the selected vectors are cut where they cross the closed
    vector picked last (with its group) and the pieces outside it, or inside it, cleared;
    the rest are refitted as curves if asked. The boundary stays.
  - Interactive Vector Trim: click a piece of a vector and the stretch between the nearest
    crossings either side goes, or drag across pieces and each one the drag crosses goes;
    Rejoin Trimmed Sections joins what is left where pieces meet end to end. Pieces come
    back as polylines (curves are not kept exactly).
  - Measure / Inspect: drag between two points for distance, angle and X / Y distances;
    click a vector for the span's type and length and the vector's perimeter and area, on
    the status bar and in the Measure / Inspect form (also the two points, the span's ends,
    an arc's centre and radius, and the vector's span count).
  - Node Edit (N): handles on the selected vectors' nodes; drag a node to move it (arcs
    keep their bulge, Bézier handles follow); right-click a node or span for Insert Point,
    Delete Point, Convert Span to Line / Bezier / Arc, Cut Vector, Make Start Point, Reverse Direction,
    Close With a Line, Smooth Point (the spans either side become Béziers with handles in
    line through the node), Node Position... (Move Node's form, typed X / Y), Exit Node
    Editing. A box round nodes of the selected vectors picks them (drawn filled; Shift adds)
    and a drag of one moves them all (Move Nodes). Dragging control points or spans and
    Keep Bezier Tangency are not done.
  - Text in a Box (Draw Text's other mode): the text scaled to fit a box at the anchor,
    inside a None / Normal / Wide margin, centred (dialog 279). Vertical Stretch (line space
    or characters to the box height) when the text fits the width, Horizontal Stretch (word
    spaces, kerning or character size to the box width) when it fits the height.
  - Text on a Curve: one line of text laid along a selected curve, kept or scaled to fill
    it, with spacing, above / on / below, other side, offset, alignment, turned or upright
    (dialog 426). The text stays editable: selected alone, the form opens with its settings
    and lays it out anew on the curve it keeps.
  - Fit Curves to Vectors (Edit ▸ Curve Fit Vectors): arcs, Béziers or lines within a
    tolerance, sharp corners kept past a max angle, replacing or beside the originals.
  - Array Copy (rows × columns by gap or offset; symmetry block of mirrors or half turns,
    row / column displacement, Group Copies) and Circular Copy (round the selection's
    middle, the job centre or a point; total or step angle, clockwise positive; Group Copies; turning the
    copies or not). Copies stay on their originals' layers; copies of grouped vectors are
    not grouped.
  - Nest Parts (Edit ▸ Nest Selected Objects too): parts (grouped or overlapping vectors)
    packed by their boxes in rows from a chosen corner, along X or Y, a tool diameter plus
    clearance apart and a border in, quarter-turned where that fits better, moved or
    copied. True Shape nests each part where its own shape (grown by half the gap) first
    fits on a grid of the job, inside the boxes of parts already placed first so smaller
    parts fill holes, turned in the Rotation Step (or quarter turns), with Copies of Each
    Part. Mirroring, the sheets list, boundary layers, two-sided nesting and filler parts
    are not done.
  - Copy Along Vectors (the first object picked is copied along the other vectors; by count
    or distance with force even spacing; align, reverse, copies on a new layer).
  - Close / Extend Vector (a closing line, a smooth curve, both ends moved to a common
    point, or the end lines run on until they meet; max gap).
  - Vector Validator (overlaps, intersections within and between vectors, zero-length spans,
    and optionally open vectors; selects them or fixes the spans).
  - Dimensions: length, vertical and horizontal, dragged between two points, with offset,
    text height, font, decimal places and custom text, as one group (dialog 963). Angle and
    arc dimensions, separate text placement and editing are not done.
  - Create Fillets (Normal, Dog-Bone, T-Bone, Plasma / Drag Knife; all, outer or inner
    corners of the selection at once, in turn as if clicked; an oversize radius gets the
    largest that fits).
  - Not built from these tools' Aspire forms: see each one's section in `re/SPEC.md`.
  - Trace Bitmap: black / white tracing of the chosen bitmap with threshold, trace colour,
    corner fit, noise filter and grouping (dialog 347); or in Colors mode, a colour picture
    (imported pictures keep their colours) cut down to 2 to 16 colours, numbered from the
    darkest, the ones listed (or all but the lightest) traced each onto a layer named after
    it, the palette given in the status. With the form open, a box dragged over the picture
    is the part traced (drawn dashed) and a click on the picture lets it go; in Colors mode
    a click picks the colour under it (again to let go). No colour swatches in the form.
- Model ▸ Create Vector Boundary Around Selected Components: the silhouette as closed
  vectors (grouped when several), from the tracing core.
  - Join, Weld, Subtract, Overlap, Move, Rotate, Set Size, Mirror, Offset. Mirror flips about
    the selection's centre or the job's, or beside the selection (about its left, right, top
    or bottom edge), or about the line between the ends of the vector picked last; Create
    Mirrored Copy keeps the originals and adds the flipped copies. Move goes to a point
    (an anchor of the selection's box there) or by an offset, which Relative Rotated turns
    by an angle. Set Size scales the
    selection as one, or each item on its own (Scale Items Individually), to a width and
    height in millimetres or as percentages, Link XY keeping proportions (Auto Scale Z for
    components and rotated bounds are not done).
- Toolpaths ▸ Toolpath Summary: each toolpath's estimated time and the visible ones' total
  (moves at their feed, plunge or rapid rate, times a scale factor; dialog 231). The rapid
  rate is shown in the units picked (mm/sec to feet/min); rate, units and scale last for
  the session. A toolpath clicked there has Notes to write and Apply, kept with it (and
  through recalculation).
- File ▸ New from Template and Save As Template: `.ccjobt` job templates, kept in
  `~/.config/cut-craft/templates`; a template opens as a new, unsaved job.
- Toolpaths ▸ Templates: save the selected or all visible toolpaths' settings as a
  `.cctpt` template, and load one as empty toolpaths to edit and calculate. The Missing
  Layers dialog is not done.
- File ▸ Print and Print Preview: the drawing as a PDF opened in the system viewer, which
  prints it (Aspire's own print window with scale and tiling is not done).
- Edit ▸ Select ▸ Vector Selector: selects vectors by open / closed, circles, circles of a
  diameter within a tolerance, on all visible layers, the current one or the layers named
  (dialog 949; Z values are not done). Every toolpath form has Associate a Vector Selector
  with this toolpath: it is then made from the vectors that pass, and takes them again each
  time it is recalculated, so a template's toolpaths find new vectors on their layers.
- Edit ▸ Snap Options (F4): drawing and node drags snap to span ends and middles, arc and
  object centres, the job's centre and corners, and a grid, within a snapping radius in
  pixels; Shift holds snapping off. Guides and smart snapping (see the top bar) snap too.
  While drawing, a line's far end (and a polyline's next point) also snaps Tangent to a
  circle, arc or curve and Perpendicular onto a line or curve, from where it starts
  (Tangent and Perpendicular check box, on by default). Polyline clicks snap; the view marks
  each snap with its own shape and name (square End, triangle Middle, circle Tangent, right
  angle Perpendicular, plus Center) and the status bar says what it means. Drawing tools
  show a crosshair pointer. Fixed Nudge Distance moves the selection with Ctrl and an arrow key (the arrows alone,
  with Shift, or with Ctrl and Shift move by the grid, half or a tenth). Intersections and
  cursor text are not done; the options last for the session.
- Snaps that stay (Cut Craft's own, not in Aspire): a point placed on an End, Middle,
  Center, Shape center, material corner, Tangent or Perpendicular snap is kept there, saved
  with the job. When what it snapped to moves or changes, it follows; a Draw Line/Polyline
  point (or a node dragged onto a snap) stretches its line, any other drawn shape moves
  whole. Moving the snapped shape on its own puts the point back, with a status-bar note.
  Kept points show a small blue snap mark; click one and press Delete to let it go.
  Deleting either shape, or adding or deleting its nodes, lets go of its snaps. Not done:
  snaps of Edit Polyline's extensions, an "on the line anywhere" (Nearest) snap,
  Intersection snaps, and a menu to show or hide the marks.
- Toolpaths ▸ Create Job Sheet: an HTML page with the job layout, notes, material setup,
  toolpaths summary and each toolpath's tool, feeds and estimated time (the sections of
  Aspire's job setup sheet), with the home position and, double-sided, the side and flip.
  It also names the job's file, the active machine, its post and the time it was made,
  totals the run time and tool changes, and lists each tool with its tool changer slot
  ("By hand" for one not loaded) or, without a changer, the T number the post writes, with
  the toolpaths that use it and their run time. Run times use the Toolpath Summary's rapid
  rate (and the machine's accelerations when it allows for them). Toolpaths ▸ Print Job
  Sheet opens it in the web browser to print.
  A job never saved (or whose file has gone) and toolpaths not yet saved are printed as
  NO LOCAL FILE in red; the unsaved job is copied to the auto-recovery file first, and the
  sheet gives that path.
  A job of several sheets gets a page for each sheet with toolpaths (`name_Sheet.html`); a
  double-sided job's page ends `_Top` or `_Bottom`. Gadgets ▸ Job Setup Sheet Editor sets
  the title, a logo (PNG, JPEG or GIF), the header and footer colours and which of the
  five sections show, kept with the options.
- Cut lists and part labels (the Advanced Machining Module's cabinet tools, made plain; see
  `re/SPEC.md`, Cut lists and part labels). A part is a group or a closed vector with what
  lies inside its box (holes, pockets, engraved text); open lines alone and hidden layers
  are left out. Parts are numbered sheet by sheet, top row first, left to right, and that
  number is the same on the cut list, the labels and the engraved names (`fmt-cut-list`).
  - File ▸ Import ▸ Import Cut List (CSV): rows of name, length, width, quantity and
    material (headings in any order, or none; commas, semicolons or tabs; decimal commas;
    UTF-8 or Excel's Windows CSV) become named rectangles, length along X, a material's on a
    layer of its name, laid out in rows and selected; Nest Parts then opens on them, set to
    move them. A mistake in the file opens a message naming the row and how to fix it.
  - Edit ▸ Name Part (Design tab, Offset and Layout): the selected parts' name, carried by
    each outline's group (an ungrouped outline gets a group of its own); empty clears it.
  - Edit ▸ Engrave Part Names: number and name (or either) as text in the middle of each
    part, smaller where the part is small, on a Part Names layer, editable with Draw Text;
    selected parts or the whole sheet; running it again replaces the names.
  - Toolpaths ▸ Save Cut List (File ▸ Export too): a CSV of the parts, identical ones on one
    row with the quantity and their sheets, sizes in the job's units.
  - Toolpaths ▸ Print Part Labels: a label for each part (name, "3 of 12", size, sheet, job
    name, and a QR code of the same words) opened in the web browser to print; plain paper
    with cut lines, or one label a page in seven label-printer sizes. The style is kept with
    the options. Barcodes and Data Matrix codes, the cabinet programs' own export formats and
    grain direction are not done.
- Edit ▸ Notes: text kept with the job, undoable, printed by posts as `[FILE_NOTES]`; notes
  starting with "." open with the job (dialog 606).
- File ▸ Export ▸ PDF, EPS and AI (a PDF saved as `.ai`): the vectors on a page the size of
  the material, arcs as Béziers.
- File ▸ Import ▸ Import Vectors: Vectric `.crv3d` / `.crv` jobs, vectors only (lines, arcs,
  Béziers onto layers of the job's names; text objects, the model and toolpaths are not
  read). Layout from samples saved by Aspire 12 under Wine (`re/SPEC.md`, Job files).
- File ▸ Import ▸ Import Component / 3D Model (and the Design tab button): binary and ASCII
  STL, and Wavefront OBJ (polygon faces fanned into triangles), through Aspire's Orientate 3D Model form (dialogs 524 and 4104), in its sections and
  pictures on one page (Aspire's has two): initial orientation (a radio per face round the
  model's picture), rotation about Z (-180 … +180 in quarter turns), file units, model size with locked
  ratio, centre or position, depth below top, discard below the zero plane. Show
  Undercuts colours the faces a cutter from above cannot reach while the form is open; in
  a double-sided job a full model's underside goes to the other side (Aspire's Create
  Both Sides). No perspective, segmenting or on-screen rotation handles.
  In a rotary job (help: Orientate 3D Model Rotary) the Imported model type picks Full 3D
  model (the default there) or Flat model (placed like clipart). A full model's axis runs
  along the job's through the middle of its box; each cell of the unrolled job holds how
  far out from the axis the model reaches there, so heights are measured from the axis.
  Scale model to fit material (on by default) fits it to the material's diameter and
  length, else Model size X is its length. Angle 0 is straight up [inf]. For a full model
  narrower than the material, set Material Setup's Gap Inside Model to 0 so it is machined
  from the axis out. Not done: the rotation handles, moving the rotation axis, Resize
  material block.
- Model ▸ Export as 3D Clipart: the selected components (baked into one, shaping and all)
  saved as Cut Craft's own `.ccclip` (JSON); Import Component / 3D Model reads them back,
  centred on the job, on the first level. Aspire's `.3dclip` is encrypted (`re/SPEC.md`),
  so it is neither read nor written; the Clipart tab lists folders of your own files instead
  (and says how many locked Vectric files a folder has, and why they aren't shown).
- Model ▸ Segment 3D Model (help: Segment 3D Model): an STL picked first, split by planes
  square to X, Y or Z: one at a percentage of its size, or (Number of Cuts) several in
  equal slices; the first segment and the ones after it (turned over, if asked, so their
  far sides are machined) are laid out side by side across the job, a gap apart, tops at
  the surface, as components, and a segment that still has undercuts says so. Faces lying
  in a plane go to neither. Cut Segments Again is the segment tree: a named segment cut by
  a plane of its own (2 gives 2.1 beyond the plane and 2.2), cut again in turn; Segment
  Orientations turns a chosen face (top, bottom, front, back, left, right) up per segment.
  Not done: the plane dragged with 3D view handles, merging segments by drag and drop,
  undercuts painted pink, and the Import Options (backface caps, resize material,
  boundary vectors).
- Modeling tab: Create Shape, Zero Plane, from Bitmap, Smooth, Scale Z, Bake.
  - Create Shape: round, angular, smooth, flat, or Custom (the profile of an open vector
    selected with the closed ones, drawn upright, edge at its left) over closed vectors; a
    ring between nested vectors rises from both edges; limit, base, Scale (the profile to a
    height) and Total Height (profile and base together), Preserve Internal Corners (inside
    corners mitred to a crease), Blend to Inner Vectors (vectors inside the outer ones are
    the top). The shape keeps its settings and outlines: E on it opens the form and a
    change remakes it in place. The 3D view's height handles work on it; it has no live
    handles of its own.
  - Component Properties' dynamic properties (help: Component Properties): Shape Height
    (%), Base Height, Fade (kept at one point, lowered by a percentage toward another) and
    Tilt (pivot and raised point, angle), kept apart from the heights and applied as the
    model is built; Bake makes them permanent. Tools that change a component's heights
    bake them first. The anchors are typed in the form (Aspire sets them with two clicks);
    tilt is a plane through the pivot line, so it also lowers behind it [inf]. Several
    components at once and levels' base heights are not done.
  - Move, Rotate, Mirror, Set Size and the Align / Center tools also act on the selected
    components (picked in the 2D view or the Components tab): each is resampled onto cells
    of its own size where it lands (bilinear inside the shape), heights unchanged, fade
    and tilt anchors carried along; the 3D view's Transform handles too. Set Size's Z
    scaling is not done.
  - Replace Below (transparent or plane, optional lowering; several selected are baked
    first), Offset Model (a ball offset of the visible model into a new component, the
    originals hidden, optional clip to the zero plane), Slice Model (by thickness, from the
    top or bottom, by count, or a Custom Slice List of each slice's thickness; optional
    boundary vectors).
  - Split Component (by a closed vector: "- A" inside, "- B" outside), Emboss Component
    (broad shape scaled 1–50 %, detail kept, then smoothed), Add Draft to Model (a new
    component with no wall steeper than the draft angle), Create Cross Section Vector (drag
    a line; the profile is drawn from it, height 1:1).
  - Extrude and Weave: shapes swept along drive rails (the form's count of vectors picked
    first). Open cross-sections after them are hung across the rail nearest each, where
    drawn (first end on the rail's left), and blended along it (node to node with Sweep
    Between Spans); one section serves every rail. Or the selected component is stretched
    along each rail or repeated at its size with overlap and alternate copies flipped.
    Create Sharp Corners mitres corners and squares off open ends. Weave Under/Over at
    Crossings alternates over and under along each rail, by Scale Shape or Add Base with Z
    Under / Z Over. Not done: attaching and dragging sections on the rail with the mouse,
    per-section Smooth, rail reversing and start points in the form (use the vectors'
    own Reverse Direction / Make Start Point).
  - Two Rail Sweep: open cross-sections stretched between two rails (picked first), read
    upright (X across, Y up from the lowest point; one on its side, as Vector Unwrapper lays them, is read sideways). One section is used all the way;
    several are each placed where drawn along the first rail and blended between, node to
    node with Sweep Between Spans. Add To All Rail Nodes pairs the rails' nodes (corners
    meet corners); Fill Center fills the inner closed rail to the sections' height there;
    Swap Rail Order hangs them the other way; either rail reversible; scaled with the
    width and to an exact height. Not done: attaching and dragging sections on the rails
    with the mouse, the per-section Smooth switch.
  - Turn and Spin: an open profile turned round the line between its ends, or spun round
    its left end, optionally scaled to an exact height, with name and combine mode.
  - Create Texture Area: the chosen component tiled over the selected closed vectors or
    the job, with spacing, alternate shifts and a reflection per tile of each four; the
    tile is hidden. The result is an ordinary component (Aspire's stays re-editable).
  - Sculpting (Model ▸ Sculpt Visible Model; help: Sculpting): drag over the selected
    component in the 2D view, or over the model in the 3D view (there the right button
    turns the view and the middle pans), several baked into one first, with Smooth, Smudge,
    Deposit, Remove or the Undo Brush (back toward the heights from before the first
    stroke; Erase removes the shape, Aspire's Shift); diameter, strength, smoothness,
    Normal / Raise / Lower mode, Preserve Transparency. Each drag is one undoable stroke.
    Keys 1 to 5 pick the tool; over a view the wheel sizes the brush (finer with Ctrl),
    Shift and the wheel set the strength, and Alt and the wheel zoom. Dab spacing and the
    deposit rate are our own [inf]. Not done: custom brushes, dynamic rotation, merge,
    presets, the bitmap overlay, the V / H constraints, Keep / Discard (undo does that),
    and growing the grid past the component's own.
  - Vector Unwrapper (rotary jobs; help: Vector Unwrapper, Modelling 3D Rotary Projects):
    a closed cross-section becomes the open profile Two Rail Sweep takes: the distance
    out to it round the centre of the contour, of the job or a typed point, laid along
    the circumference with legs to the baseline at each end, refined to Béziers if asked.
    Two drive rails along the job's sides go on the "Unwrapped Vectors Drive Rails" layer
    and the rails and profile are left selected, so Two Rail Sweep makes the turned shape
    at once. Angle 0 is up the cross-section, turning clockwise [inf]; the profile sits
    across the middle of the job [inf]. The axis handle and the fit circle are not drawn.
  - Levels (help: Component Tree, Level Clipping, 3D Design and Management): every
    component is on a level; a level's components combine with each other first, then
    the shown levels combine from the bottom up with their combine mode and base height.
    Clipping keeps a level only inside closed vectors (Update re-reads them); the mirror
    modes reflect one half or quarter of the job about its middle; Wrapping (rotary jobs)
    brings what runs off one side of the unrolled job round the other, keeping the
    higher part where it meets itself. Older jobs get one level, so their models are
    unchanged. New components go on the first level. Move Up / Move Down over the tree
    (Aspire's blue arrows) move the picked component within its level, and the level
    menu's Move Level Up / Down move a level with its components (Move Component, Move
    Level); dragging in the tree is not done.
  - Component groups (help: Component Tree): Group Components makes the selected ones (two
    or more, one level) a group; its members combine with each other first and the group
    then combines, with its own combine mode, where its top member is. The tree shows a
    group under its level with its members; its menu has Combine Mode, Ungroup, Rename and
    Properties (Group Properties); the component menu has Group and Ungroup. Groups do not
    nest; Color from Children and a group's own base height are not done.
  - Distort Object (Drawing tab, and Model ▸ Distort Selected Component; help: Distort
    Object): the selected vectors, or the selected components (not both), bent into an
    envelope: their bounding box with its four corners moved (typed in the form), the band
    above the open vector picked last (they keep their height above it), or the band
    between the two open vectors picked last. Vectors are refitted as Béziers within
    0.01 mm; components are resampled, heights kept. The curves run left to right [inf].
    Create Envelope adds a rectangle round the objects (picked last) to reshape with Node
    Edit: move corners, insert nodes, turn sides into curves; Distort to Envelope then bends
    the objects into it as a Coons patch of its four sides (corners: the nodes nearest its
    bounding box's corners) and removes it. Not done: a live envelope (the distortion is
    applied at once, so there is no Bake Distortion), Use Rotated Bounds, the 3D view.
- Toolpaths tab ▸ Material Setup (help: Material Setup - Flat / Rotary): the material's
  thickness (flat jobs), Z zero, XY datum and offset, Rapid Z Gaps (Z1 / Z2) and Home, set as
  Job Setup sets them; Model Position in Material, as the gap above the model's top (Gap
  Outside Model on rotary jobs) or below its bottom (Gap Inside Model, from the axis); and
  the model's thickness, which a new value scales the model to (Aspire's Set). 3D roughing
  and finishing and the wrapped 3D view place the model by it, refusing a model it pushes
  out of the material. Toolpath forms do not show Aspire's summary of these settings.
- Toolpaths ▸ Materials (also beside Material Setup): a library of woods, sheet goods,
  plastics and metals that sets the material's colour in the 3D view (Job Setup's colour);
  Custom keeps Job Setup's. Woods show grain on the 3D view's top (lines along X that drift,
  bunch, and move across as the surface goes down, so carvings show rings); the wrapped
  rotary view and the block's sides are plain. Custom is a solid colour of one's own.
  As Aspire's Material Appearance it also sets Machined Area Color: Material Color (cuts
  show the material), Global Fill Color (every cut area in one colour) or Toolpath Color
  (each toolpath's areas in its own colour: a swatch beside it in the toolpath list, a
  palette colour until set; right-click ▸ Preview Color: No Fill leaves its areas the
  material's). Previews mark which toolpath cut each cell last, so changing the colours
  repaints the preview without previewing again.
- Toolpaths tab: every strategy above; the list's Recalculate, Recalculate All, Delete,
  Preview, Preview Visible (the shown toolpaths), Preview All (every toolpath, shown or not),
  Undo Last Preview (back to before the last preview, once), Reset Preview. Each row has a tick to show or hide it and opens its
  form on a double-click; the up and down arrows move the selected toolpath through the cut
  and save order (Move Toolpath), Merged and Array Copy toolpaths keeping their sources.

**Not built yet.** (Probe 2026-11-01: every menu item run, those that say "not
implemented yet" listed.)
- File ▸ Import ▸ Import Toolpaths of Vectric's PhotoVCarve / Machinist / Cut3D files (it
  reads G-code now).
- Gadgets beyond the rotary layouts and Celtic Weave.
- Feeds and speeds: cutting data per machine (it is per material now), Lua-scripted posts;
  toolpath forms still show feeds in mm/min or in/min, not the tool's own units.
- Double-sided jobs: Zero off same side is kept but not yet used by the posts; an animated
  preview plays this side only (the other side is cut at once).
- Sketch Carving's Selected Level boundary.

**Known problems and stand-ins.**
- Split Component takes closed vectors only (Aspire also extends open ones to the edges).
  Emboss's split into shape and detail is our own [inf].
- Trace Bitmap's Corner Fit scale is our own.
- TrueType text has no kerning; line spacing is a guess (`re/SPEC.md`, Fonts).
- Components show in the 2D view as greyscale depth maps (black lowest, white highest, per
  component) and a click on one selects it, as in Aspire; in the 3D view a double-click picks
  the topmost shown component under it (Shift adds or takes away; off the model it fits).
- Vector groups do not nest. Grouping leaves each member on its own layer; the right-click
  Ungroup ▸ puts them back as they were (onto Original Object Layers) or all onto the
  current layer (Aspire's onto Group's Layer, as Cut Craft's groups have no layer).
- The Toolpath Drawing toggles (rapids, plunges, retracts, joins) last only for the session;
  the 3D view's toggles are kept with Options.
- The 3D view draws the model solid down to the modeling plane on the GPU only; the CPU
  fallback (and the headless tests) draw its top alone. The modeling plane is always drawn
  while a model is imported.
- Imported models: a cell over only a near-vertical side takes the height of the top beside
  it (up to 4 cells in), so steep sides leave no spikes; a wider steep band keeps its slope.
- Lasers are not a tool type in the database (laser features are skipped by decision).
- Guessed behaviour, marked [inf] / [unk] in `re/SPEC.md`: Fluting's Smooth curve (not described in the help), prism and photo V-carve
  geometry, texture scallop shape, sketch-line contrast rule, and the 2026-11-01 batch
  (`docs/ROADMAP.md`, section 6).
- Not tried by hand: the 2026-11-01 batch (`docs/ROADMAP.md`, section 5) and earlier work
  listed in `docs/hand-test-3d.md`.

## Differences from Aspire 12 seen on screen

- Aspire's rotary Job Setup calls its Z zero under the material "Cylinder Axis"; ours reuses
  "Machine Bed" (posts print "Cylinder Axis").
