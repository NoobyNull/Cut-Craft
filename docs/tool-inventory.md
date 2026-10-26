# Tool inventory

Date: 2026-10-24. What every tool is built from, what it is reused by, and whether it works.
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
| 3D Roughing, 3D Finishing | placed model + zigzag (each move checked with the simulation) + follow |
| Recalculate All Toolpaths | Recalculate, for each toolpath |
| Preview All Toolpaths | Preview Toolpath, for each toolpath, on fresh material |
| Delete Layer | removing its vectors (as Delete does) + removing the layer |
| Layers panel eye / padlock | Layer Properties |
| Zoom to Selection, Zoom Box | Zoom to Job's fit, on the selection or the box |
| Select All Vectors On Current Layer | Select Vectors on Layer |
| Job Size and Position; Create a new file | Job Setup |
| STL export, model in the 3D view | the combined model |

## Right-click menus: the shared families

Aspire's popup menus (`Resources_ENU.dll`, see `re/SPEC.md`) repeat a few families across kinds
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
  Move Selection Here. Merge Visible is greyed.
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
  Layer ▸ each layer, Weld, Subtract, Overlap, Join, Align ▸. Object Properties is greyed.

Not built from Aspire's menus yet: toolpath Edit, Duplicate, groups, Activate Sheet, Show All
With This Tool; component 2D preview order, Export as 3D Clipart, Re-Import, Copy,
Ungroup; Move to Front / Back; the no-item menus (layers 982, components 614).

## Status, top down

What works and what does not, by menu and tab; items not built yet are greyed and
say "not implemented yet".

**Working.**
- File: New, Open, Close, Save, Save As, Increment and Save, Recent Files, Import Vectors /
  Bitmap (bitmaps: PNG, JPEG, GIF, BMP, TIFF in grey, shown in the 2D view), Export DXF /
  SVG, Open Application Data Folder, Exit.
- Edit: Undo, Redo, Cut, Copy, Paste, Delete; Select All Vectors / Open Vectors / Duplicate
  Vectors / On Current Layer, Unselect All; the ten Align commands and the four spacing ones
  (Left To Right ... Bottom To Top); Join Vectors, Group, Ungroup, Job Size and Position.
- Selection: pick order kept; components picked in the Components tab; bitmaps picked in the
  2D view (Bitmap Properties: position and size; Delete removes them).
- Model: Add Zero Plane, Create Component from Bitmap / Visible Model / Toolpath Preview, Bake
  Components, Smooth, Reset Inside / Outside Selected Vectors, Scale Model Z Height, Selected
  Component Properties, Export as STL, Export as Grayscale Bitmap.
- Machine: Install Post-Processor (real grbl / FluidNC posts load, rotary G93 ones included).
- Job Setup: single-sided and rotary jobs.
- Save Toolpaths: through the installed post, else Cut Craft's own FluidNC posts (tool
  changer; rotary jobs wrapped onto A in inverse time).
- Toolpaths menu: Show Toolpaths Tab, Recalculate All, Toolpath Drawing (2D previews, solid
  previews, auto-open 3D view, rapid / plunge / retract / join moves; a join is a feed move above the material [inf]), Preview Simulation Quality,
  Merge Visible Toolpaths, Tool Database.
- View: Zoom to Drawing (F6), Refresh 2D View, Draw Origin / Modeling Plane / Material Block /
  Vectors in 3D View, Light Follows Viewer, Color Shaded View, Use Shaded Background,
  Auto-Wrapping View (rotary jobs, on by default: the model or preview drawn round the
  axis, closed at the seam, each face lit by its own facing; the vector, plane and origin
  overlays are left off it, and it has not been tried by hand yet).
- 3D view: left- or right-drag rotates, middle-drag / Ctrl+right-drag / left+right pans, the
  wheel zooms about the cursor, double-click or F fits; Top, Front, Right, Iso, Fit buttons.
- Help: About, Keyboard Shortcuts, Third Party Licenses.
- Tool Database: flutes and chip load; Radiused End Mill (toroidal) tools.
  Import of Vectric `.vtdb` tool databases (the vendor libraries for Aspire / VCarve 12.5):
  ball nose, end mill, radiused end mill, V-bit, engraving (as a V-bit), tapered ball nose
  (as a ball nose), drill (as an end mill); one tool per material it has cutting data for.
  Form tools and diamond drags are skipped.
- Every toolpath strategy in `re/SPEC.md` (backlog item 19): Profile, Pocket, Drilling,
  V-Carve (with Start Depth), V-Carve Inlay, Inlay (pocket / plug), Quick Engrave (Ctrl+Q),
  Chamfer, Fluting, Prism, Thread Milling, Keyhole, Drag Knife, Moulding, Texture, Photo V-Carve,
  Sketch Carving, 3D Roughing, 3D Finishing (with rest machining), Rounding (rotary), Laser
  Cut / Fill and Laser Picture (need a post with LASER_SUPPORT), Array Copy, Merged. What
  each leaves out is in its `re/SPEC.md` section.
- Drawing tab:
  - Select, Job Setup, Pan, Zoom Box, Zoom to Job, Zoom to Selection, Switch Layout.
  - Polyline, Rectangle, Circle, Ellipse, Polygon, Star, Arc.
  - Text: system TrueType fonts and Vectric single-line `.vfont` fonts (from
    `~/.config/cut-craft/fonts`), bold, italic, alignment, capital height, click to anchor.
    The text is a group that remembers its settings: Draw Text on selected text edits it;
    Ungroup turns it into curves.
  - Draw Curve: a smooth Bézier curve (Catmull-Rom) through a list of points, open or
    closed; points are entered in the form (Aspire places them with clicks).
  - Draw Freehand: the whole drag path, smoothed into Béziers within a tolerance.
  - Create Vector Texture: wavy lines at an angle over the selected closed vectors or the
    job, max spacing with variation, wavelength, amplitude, noise and a seed (vectors go on
    the current layer; no new-layer option yet).
  - Interactive Vector Trim: click a piece of a vector and the stretch between the nearest
    crossings either side goes; what is left comes back as polylines (curves are not kept
    exactly), and drag-trimming and rejoining are not done.
  - Measure / Inspect: drag between two points for distance, angle and X / Y distances;
    click a vector for the span's type and length and the vector's perimeter and area, on
    the status bar (Aspire shows them in a form).
  - Node Edit (N): handles on the selected vectors' nodes; drag a node to move it (arcs
    keep their bulge, Bézier handles follow); right-click a node or span for Insert Point,
    Delete Point, Convert Span to Line / Bezier / Arc, Cut Vector, Make Start Point, Reverse Direction,
    Close With a Line, Exit Node Editing.
    Dragging control points or spans, several nodes at once, smooth
    points and Keep Bezier Tangency are not done.
  - Text in a Box (Draw Text's other mode): the text scaled to fit a box at the anchor,
    inside a None / Normal / Wide margin, centred (dialog 279; its stretch options are not
    done).
  - Text on a Curve: one line of text laid along a selected curve, kept or scaled to fill
    it, with spacing, above / on / below, other side, offset, alignment, turned or upright
    (dialog 426). The result is plain curves (Aspire keeps it editable).
  - Fit Curves to Vectors (Edit ▸ Curve Fit Vectors): arcs, Béziers or lines within a
    tolerance, sharp corners kept past a max angle, replacing or beside the originals.
  - Array Copy (rows and columns by gap or offset, alternate row / column displacement,
    group copies; dialog 2516) and Circular Copy (round a centre by total or step angle,
    rotated or upright, group copies). Array Copy's symmetry buttons are not done.
  - Nest Parts (Edit ▸ Nest Selected Objects too): parts (grouped or overlapping vectors)
    packed by their boxes in rows from a chosen corner, along X or Y, a tool diameter plus
    clearance apart and a border in, quarter-turned where that fits better, moved or
    copied. True-shape nesting, step angles, mirroring, parts inside parts, copies per
    part, fillers and bounds layers are not done.
  - Copy Along Vectors: the first selected object (or circles) along the other selected
    vectors, by distance (optionally evened to land on the ends) or count, upright or
    turned with the curve, reversed, optionally on a new layer (dialog 458).
  - Vector Validator (crossings, self-crossings and zero-length spans in the selection or
    all shown vectors; the faulty vectors are selected, zero-length spans fixed) and Extend
    Vectors (two selected open vectors' nearest ends run straight to where they meet;
    Aspire picks the ends by hovering and clicking).
  - Dimensions: length, vertical and horizontal, dragged between two points, with offset,
    text height, font, decimal places and custom text, as one group (dialog 963). Angle and
    arc dimensions, separate text placement and editing are not done.
  - Create Fillets: click a corner where two straight spans meet for a normal or dog-bone
    fillet of the radius (dialog with T-Bone and Plasma / Drag Knife types, and removing
    fillets, not done yet).
  - Trace Bitmap: black / white tracing of the chosen bitmap with threshold, trace colour,
    corner fit, noise filter and grouping (dialog 347).
- Model ▸ Create Vector Boundary Around Selected Components: the silhouette as closed
  vectors (grouped when several), from the tracing core.
  - Join, Weld, Subtract, Overlap, Move, Rotate, Set Size, Mirror, Offset.
- Toolpaths ▸ Toolpath Summary: each toolpath's estimated time and the visible ones' total
  (moves at their feed, plunge or rapid rate, times a scale factor; dialog 231). Rapid rate
  and scale last for the session; no summary notes.
- File ▸ New from Template and Save As Template: `.ccjobt` job templates, kept in
  `~/.config/cut-craft/templates`; a template opens as a new, unsaved job.
- Toolpaths ▸ Templates: save the selected or all visible toolpaths' settings as a
  `.cctpt` template, and load one as empty toolpaths to edit and calculate. The Missing
  Layers dialog is not done.
- File ▸ Print and Print Preview: the drawing as a PDF opened in the system viewer, which
  prints it (Aspire's own print window with scale and tiling is not done).
- Edit ▸ Select ▸ Vector Selector: selects shown vectors by open / closed, circles, circles
  of a diameter within a tolerance, on all visible layers or the current one (dialog 949;
  Associate with toolpath and Z values are not done).
- Edit ▸ Snap Options (F4): drawing and node drags snap to span ends and middles, arc and
  object centres, the job's centre and corners, and a grid, within a snapping radius in
  pixels; Shift holds snapping off. Guides, intersections, smart snapping, nudging and
  cursor text are not done; the options last for the session.
- Toolpaths ▸ Create Job Sheet: an HTML page with the job layout, notes, material setup,
  toolpaths summary and each toolpath's tool, feeds and estimated time (the sections of
  Aspire's job setup sheet; no logo or colour settings).
- Edit ▸ Notes: text kept with the job, undoable, printed by posts as `[FILE_NOTES]`; notes
  starting with "." open with the job (dialog 606).
- File ▸ Export ▸ PDF, EPS and AI (a PDF saved as `.ai`): the vectors on a page the size of
  the material, arcs as Béziers.
- File ▸ Import ▸ Import Vectors: Vectric `.crv3d` / `.crv` jobs, vectors only (lines, arcs,
  Béziers onto layers of the job's names; text objects, the model and toolpaths are not
  read). Layout from samples saved by Aspire 12 under Wine (`re/SPEC.md`, Job files).
- File ▸ Import ▸ Import Component / 3D Model (and the Design tab button): binary and ASCII
  STL through Aspire's Orientate 3D Model form (dialogs 524 and 4104): initial
  orientation (which face is up), rotation about Z, file units, model size with locked
  ratio, centre or position, depth below top, discard below the zero plane. No OBJ; no
  undercut highlighting, perspective or segmenting.
  In a rotary job (help: Orientate 3D Model Rotary) the Imported model type picks Full 3D
  model (the default there) or Flat model (placed like clipart). A full model's axis runs
  along the job's through the middle of its box; each cell of the unrolled job holds how
  far out from the axis the model reaches there, so heights are measured from the axis.
  Scale model to fit material (on by default) fits it to the material's diameter and
  length, else Model size X is its length. Angle 0 is straight up [inf]. Not done: the
  rotation handles, moving the rotation axis, Resize material block, and Material Setup's
  gap (the model's top is machined at the material surface, so a full model narrower than
  the material should be fitted to it).
- Modeling tab: Create Shape, Zero Plane, from Bitmap, Smooth, Scale Z, Bake.
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
    and tilt anchors carried along. Aspire's Transform handles and Set Size's Z scaling
    are not done.
  - Replace Below (transparent or plane, optional lowering; several selected are baked
    first), Offset Model (a ball offset of the visible model into a new component, the
    originals hidden, optional clip to the zero plane), Slice Model (by thickness, from the
    top or bottom, or by count; optional boundary vectors). Slice heights cannot be edited
    one by one yet.
  - Split Component (by a closed vector: "- A" inside, "- B" outside), Emboss Component
    (broad shape scaled 1–50 %, detail kept, then smoothed), Add Draft to Model (a new
    component with no wall steeper than the draft angle), Create Cross Section Vector (drag
    a line; the profile is drawn from it, height 1:1).
  - Extrude and Weave: one open cross-section swept along a drive rail (picked first),
    scaled to height if asked. Several cross-sections placed along the rail, rail reversing
    and start points, and weaving components are not done.
  - Two Rail Sweep: one open cross-section stretched between two rails (picked first),
    either rail reversible, optionally scaled with the width and to an exact height (the
    Two Rail Sweep form). Several sections, blending, capping and Sweep Between Spans are
    not done.
  - Turn and Spin: an open profile turned round the line between its ends, or spun round
    its left end, optionally scaled to an exact height, with name and combine mode.
  - Create Texture Area: the chosen component tiled over the selected closed vectors or
    the job, with spacing, alternate shifts and a reflection per tile of each four; the
    tile is hidden. The result is an ordinary component (Aspire's stays re-editable).
  - Sculpting (Model ▸ Sculpt Visible Model; help: Sculpting): drag over the selected
    component in the 2D view (several are baked into one first) with Smooth, Smudge,
    Deposit, Remove or the Undo Brush (back toward the heights from before the first
    stroke; Erase removes the shape, Aspire's Shift); diameter, strength, smoothness,
    Normal / Raise / Lower mode, Preserve Transparency. Each drag is one undoable stroke.
    Dab spacing and the deposit rate are our own [inf]. Not done: sculpting in the 3D
    view, custom brushes, dynamic rotation, merge, presets, the bitmap overlay, the
    keyboard shortcuts, Keep / Discard (undo does that), and growing the grid past the
    component's own.
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
  - Distort Object (Drawing tab, and Model ▸ Distort Selected Component; help: Distort
    Object): the selected vectors, or the selected components (not both), bent into an
    envelope: their bounding box with its four corners moved (typed in the form), the band
    above the open vector picked last (they keep their height above it), or the band
    between the two open vectors picked last. Vectors are refitted as Béziers within
    0.01 mm; components are resampled, heights kept. The curves run left to right [inf].
    Not done: editing the envelope with node editing (the distortion is applied at once,
    not kept live, so there is no Bake Distortion), Use Rotated Bounds, the 3D view.
- Toolpaths tab: every strategy above; the list's Recalculate, Recalculate All, Delete,
  Preview, Preview All, Reset Preview.

**Not built yet.**
- File: Import Toolpaths.
- Edit: Swap Sides, Document Variables, Options.
- Model: Export as 3D Clipart.
- Machine: Machine Configuration, Add Machine, Offline Package, Search Online,
  Post-Processor Management, Update Post Database.
- Toolpaths: Materials.
- View: Guide Lines, Keep-Out Zones, Shadow Shading, Save Shaded Image,
  window tiling and cascade.
- Gadgets; Help Contents.
- Drawing tab:
  - Transform, Trim Objects.
  - Auto Layout Text.
- 3D view: ViewCube, perspective, tilting past side-on; overlays are drawn over the surface,
  not hidden behind it.
- Feeds and speeds: cutting data per machine and per material, rate units chosen per tool
  (mm/sec … feet/min), Lua-scripted posts, arc output (toolpaths are straight moves).
- Job Setup: double-sided jobs.
- 3D Finishing's several-tools form: one toolpath per tool made in one go (today each
  rest-machining toolpath is made on its own, naming the tools that ran before it).

**Known problems and stand-ins.**
- Split Component takes closed vectors only (Aspire also extends open ones to the edges).
  Emboss's split into shape and detail is our own [inf].
- Trace Bitmap has no colour mode (bitmaps are kept in grey) and no traced-area selection;
  Corner Fit's scale is our own.
- TrueType text has no kerning; line spacing is a guess (`re/SPEC.md`, Fonts).
- Components show in the 2D view as greyscale depth maps (black lowest, white highest, per
  component) and a click on one selects it, as in Aspire; they are not picked in the 3D view.
- Left To Right spaces the same as Right To Left (the selection now keeps pick order, but the
  spacing commands do not yet use it).
- Groups hold vectors only and do not nest.
- Display settings (Toolpath Drawing, quality, View toggles) last only for the session.
- Thread mills and lasers are not tool types in the database: their tooth offset and power
  are on the toolpath forms. Cut Craft's own FluidNC posts are not laser posts.
- Guessed behaviour, marked [inf] / [unk] in `re/SPEC.md`: Fluting's Smooth curve (no code
  reference to the fluting strings was found in Aspire.exe), prism and photo V-carve
  geometry, texture scallop shape, sketch-line contrast rule.
- Not done inside strategies: Project onto 3D Model, Fill Regionally, Merge By Part, the
  extra clearance tools, laser passes lowering the focus.
- The strategies and menu items added on 2026-10-24 have tests but have not been tried by
  hand in the running app.

## Differences from Aspire 12 seen on screen

- Aspire's rotary Job Setup calls its Z zero under the material "Cylinder Axis"; ours reuses
  "Machine Bed" (posts print "Cylinder Axis").
