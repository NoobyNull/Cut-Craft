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
| Show / Hide | Show This, Show Only This, Show All But This, Show All, Hide This, Hide All | layers (967), toolpaths (968, 669), components (613), levels (1046) | Layer Properties ✅ · Toolpath Properties ✅ · Component Properties ✅ |
| Delete | This, Visible, Invisible, Empty, All | layers (967), toolpaths (968) | Delete Layer ✅ · Delete Toolpath ✅ · Delete Component ✅ |
| Combine Mode | Add, Subtract, Merge, Low, Multiply | component, several components, active component, 3D view, levels (613, 615, 616, 619, 1046) | Component Properties ✅ |
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
- **Component list (613):** Combine Mode ▸ (current one ticked), Delete, Rename, Show ▸,
  Hide ▸, Properties.
- **2D view (318, plus ours):** Undo, Redo, Cut / Copy / Paste (greyed), Delete, Move to
  Layer ▸ each layer, Weld, Subtract, Overlap, Join, Align ▸. Object Properties is greyed.

Not built from Aspire's menus yet: toolpath Edit, Duplicate, groups, Activate Sheet, Show All
With This Tool; component 2D preview order, Export as 3D Clipart, Re-Import, Copy, Duplicate,
Ungroup; Move to Front / Back; the no-item menus (layers 982, components 614).

## Status, top down

What works and what does not, by menu and tab; items not built yet are greyed and
say "not implemented yet".

**Working.**
- File: New, Open, Close, Save, Save As, Recent Files, Import Vectors / Bitmap, Export DXF / SVG, Exit.
- Edit: Undo, Redo, Delete, Select All Vectors, Select All Vectors On Current Layer, Unselect
  All, the ten Align commands, Join Vectors, Job Size and Position.
- Model: Export as STL.
- Machine: Install Post-Processor (real grbl / FluidNC posts load, rotary G93 ones included).
- Job Setup: single-sided and rotary jobs.
- Save Toolpaths: through the installed post, else Cut Craft's own FluidNC posts (tool
  changer; rotary jobs wrapped onto A in inverse time).
- Tool Database: flutes and chip load; Radiused End Mill (toroidal) tools.
- 3D Finishing: rest machining after earlier tools (Minimum Detail, Boundary Offset).
- Fluting: ramps at start, both ends or the whole length, by length or %, linear or smooth.
- V-Carve: Start Depth. V-Carve Inlay: pocket plus mirrored plug (glue gap, surface clearance,
  vector-offset or sheet-limit boundary) as two V-carvings.
- Prism Carving: start and prism depth, depth for a full prism, climb / conventional.
- Texture: random scallops (depth, length, overlap, stepover, angle, boundary offset, seed) or
  selected vectors as the pattern, ramped or flat.
- Quick Engrave (Ctrl+Q): outline or fill (offset, hatch, cross hatch), number of passes.
- Keyhole (slot direction, depth, length, preview outlines) and Drag Knife (blade offset,
  tolerance angle, swivel depth, passes) toolpaths.
- Chamfer: inside / outside, down / up, width, overcut, climb / conventional; V-bit or ball nose.
- Thread Milling: internal / external, pitch, diameter, fit tolerance, hand, up / down
  (single-point tool; tooth offset on the form).
- Moulding: an open profile swept along drive rails (outside closed rails, right of open ones),
  sharp corners, boundary offset.
- Toolpaths: Show Toolpaths Tab, Recalculate All, Tool Database.
- View: Zoom to Drawing (F6).
- Drawing tab:
  - Select, Job Setup, Pan, Zoom Box, Zoom to Job, Zoom to Selection, Switch Layout.
  - Polyline, Rectangle, Circle, Ellipse, Polygon, Star, Arc.
  - Join, Weld, Subtract, Overlap, Move, Rotate, Set Size, Mirror, Offset.
- Modeling tab: Create Shape.
- Toolpaths tab: Profile, Pocket, Drilling, V-Carve, 3D Roughing, 3D Finishing, Save Toolpaths,
  Tool Database; the list's Recalculate, Recalculate All, Delete, Preview, Preview All, Reset Preview.

**Not built yet.**
- File: New from Template, Save As Template, Increment and Save, Print, Print Preview,
  Import Component / 3D Model, Import Toolpaths, Export EPS / AI / PDF, Open Application Data
  Folder.
- Edit:
  - Cut, Copy, Paste.
  - Select All Open Vectors, Select All Duplicate Vectors, Vector Selector.
  - Align Left / Right To Left / Right and Top / Bottom To Bottom / Top.
  - Curve Fit, Nest, Swap Sides, Notes, Document Variables, Snap Options, Options.
- Model: Two Rail Sweep, Extrude and Weave, Turn and Spin, Textured Area, Zero Plane, the
  "Create Component from" set, from Bitmap, Import, Export as Grayscale Bitmap / 3D Clipart,
  Sculpt, Smooth, Reset Inside / Outside, Split, Vector Boundary, Cross Section, Emboss, Scale Z,
  Distort, Replace Below, Component Properties.
- Machine: Machine Configuration, Add Machine, Offline Package, Search Online,
  Post-Processor Management, Update Post Database.
- Toolpaths: Toolpath Drawing toggles, Preview Simulation Quality, Templates, Merge Visible,
  Job Sheet, Materials.
- View: Refresh, Guide Lines, Draw Origin / Modeling Plane / Material Block / Vectors / Keep-Out
  Zones, shading options, Save Shaded Image, window tiling.
- Help: Help Contents, Keyboard Shortcuts, Third Party Licenses, About.
- Drawing tab:
  - Import Component.
  - Node Edit, Transform, Group, Trim, Close / Extend, Fit Curves, Validator, Measure.
  - Curve, Freehand, Text, Text on a Curve, Auto Layout Text, Dimensions, Fillets,
    Vector Texture, Trace Bitmap.
  - Nest, Copy Along Vectors, Array Copy, Circular Copy, Distort.
- Modeling tab: Extrude and Weave, Two Rail Sweep, Turn and Spin, Texture Area, Zero Plane,
  from Bitmap; Sculpt, Smooth, Emboss, Distort, Offset Model, Slice, Draft, Scale Z, Replace
  Below, Split, Bake.
- Toolpaths tab: Inlay (pocket/plug, not V-carve),
  Sketch, Rounding, Laser Cut / Fill, Laser Picture, Toolpath Summary.

- Feeds and speeds: cutting data per machine and per material, rate units chosen per tool
  (mm/sec … feet/min), Lua-scripted posts, arc output (toolpaths are straight moves).
- Job Setup: double-sided jobs.
- 3D Finishing's several-tools form: one toolpath per tool made in one go (today each
  rest-machining toolpath is made on its own, naming the tools that ran before it).

## Differences from Aspire 12 seen on screen

- Aspire's rotary Job Setup calls its Z zero under the material "Cylinder Axis"; ours reuses
  "Machine Bed" (posts print "Cylinder Axis").
