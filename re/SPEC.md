# Aspire 12 — clean-room behaviour spec

What Vectric Aspire does, written down so Cut Craft can reimplement it. This file describes **behaviour** only.
Don't copy Vectric code, icons, `.vfont` fonts, help text or `.pp` files into this repo. Write our own
implementations from this description.

Confidence tags: **[obs]** seen in hands-on use of the Aspire trial (or in a file the trial saved) · **[inf]** inferred
from Vectric's public documentation (help, guides, tutorial videos) · **[check]** a detail still to be confirmed in the
trial or the docs before relying on it · **[unk]** not established. A section marked `(help: …)` names the help topic
it comes from. Older sections tagged [obs] whose only source is the help should be read as [inf] until checked in
the trial.

## Sources

Only two kinds of source are allowed:

| Source | What it gives |
|---|---|
| Vectric's public documentation: the help (online, and as installed with the trial; `re/help2txt.py` turns it into text), the getting-started and reference guides, the post-processor editing guide, and Vectric's tutorial videos | features, forms, options, limits and error messages |
| Hands-on use of the legitimate Aspire trial | menus, right-click menus, form layouts, keyboard shortcuts, and job files the trial saved (read with `re/crv3d.py`) |

Never decompile, disassemble or otherwise analyse Aspire's program files or resource DLLs, and never extract
artwork or strings from them. Vendor tool databases (`.vtdb`) published by tool makers are read as files.

```bash
python3 -I re/help2txt.py "<Aspire install>/Language/ENU/Help" re/extracted/help
```

## Scripting

- Aspire's gadgets are Lua scripts (help: Gadgets).
- **Deliberate difference:** CutCraft gadgets are Lua 5.4 too, but call CutCraft's own commands by id (`cutcraft.run("draw_circle", {...})`) instead of Vectric's gadget API, run sandboxed (no files, programs or network), and each run is one Undo step. Aspire gadgets do not run unchanged.
- Gadgets built into Cut Craft are signed with the app. A gadget from the user's My Gadgets folder is unsigned: before it first runs each session, Cut Craft warns that it is not responsible for what the gadget does, and the user picks Run Gadget or Cancel. Only gadgets from Cut Craft's own builds count as signed; any other gadget, from the folder or downloaded, is allowed after the same warning.

Our equivalents so far: wgpu for the 3D view, winit for windows.

---

## P2 — UI framework

### Window layout [obs]
- **Menu bar:** File, Edit, Model, Machine, Toolpaths, View, Gadgets, Help. Gadgets is built at runtime from installed Lua gadgets.
- **Left Design panel:** tabbed tool pages (Drawing, Modeling, Sheets, Layers, Clipart, plus the component tree).
- **Right Toolpaths panel:** the top half holds the toolpath strategy icons, the bottom half the toolpath tree (groups, reorder arrows, sheet filter).
- **Centre:** the **2D View** and **3D View** share one area as tabs. F2/F3 switch between them. PgUp/PgDn tile them horizontally/vertically, and maximising either view goes back to tabs.
- **Quick drop-downs** for the current Layer, Sheet and Component Level.
- **Auto-hide panels:** each panel has a pin. An unpinned panel collapses to a side tab, shows on hover and hides again once a tool is picked.
- **Two preset layouts:** *Design* (Drawing/Modeling pinned) and *Machining* (Toolpaths pinned). F11/F12 toggle them, as does a "switch layout" button on each panel.
- **3D view overlay** (`x64/Assets/View_Overlay/*.png`): an ortho/perspective toggle, on/off toggles (vectors, 2D toolpaths, components, bitmap, material block, modeling plane, origin, plan view, keep-out zones, auto-wrap), zoom buttons (to drawing limits, to selection, 2D view zoom), rotary CW/CCW buttons, and a help button.
- **ViewCube** in the 3D view. Drag it to orbit, click a face to animate to that view, double-click a face to go to the opposite side.
- **Help prompts:** an on-screen panel that follows the active tool, with a `?` on every form linking to its help topic.
- **[unk]** Status bar contents: no evidence found yet.

### Startup sequence [obs] (help: how-to-get-started, create-a-new-file, Job Setup Single; trial screenshots)
- **Start page, no job open:** the left panel holds *Startup Tasks*: Create a new file, New file from template, Open an existing file. Below them is *Recently opened files*, a rolling list of the last 4 jobs, empty on a first run. The rest of the panel is vendor links (tutorial videos, online resources, social media, clipart shop), which are out of scope. The centre shows the product splash. There are no Drawing or Toolpaths panels, and no 2D/3D view.
- **No-job menu:** File (New, New from template, Open, Open Application Data Folder, Recent files, Close, Exit), Edit (Options only), Machine (as with a job), Toolpaths (Materials, Tool Database), Gadgets, Help. Model and View are absent.
- **Create a new file → Job Setup:** the form fills the left panel, with OK and Cancel at the bottom. The 2D view (rulers, zoom buttons) appears beside it, and the full menu bar is back. Sections, top down: Job Type, Job Size + Units, Z Zero Position, XY Datum Position (+ Use Offset X/Y), Modeling Resolution, Material Settings.
- **OK** creates the new empty job: a grey rectangle in the 2D view, with dotted lines through X0 and Y0. **[inf]** Cancel abandons the new job and goes back to the start page; File ▸ Close closes the job and goes back to the start page too.
- **Open an existing file** takes native jobs and importable 2D vector files (DXF, EPS, AI, PDF). Opening a 2D file starts a job at the drawing's size (see Import).

### Right-click menus [check] (to re-confirm item by item in the trial)
- **Families that repeat across kinds of item:**
  - **Show / Hide:** Show This, Show Only This, Show All But This, Show All; Hide This, Hide All. On layers, toolpaths ( 669; toolpaths also have Show All With This Tool), components and levels.
  - **Delete:** This, Visible, Invisible, Empty, All. On layers and toolpaths.
  - **Combine Mode:** Add, Subtract, Merge, Low, Multiply. On a component, several components, the active component, a component in the 3D view and levels.
  - **Rename, Properties:** on every kind.
- **Layers:** Activate, Show / Hide, Lock, Unlock, Insert New Layer, Delete (This, Visible, Invisible, Empty), Rename, Merge Visible, Select Layer Vectors. With no layer under the pointer: New Layer.
- **Toolpaths:** Show / Hide, Activate Sheet, Edit, Rename, Duplicate, Recalculate (This, Visible, All), Create an Empty Group, Group Visible, Ungroup, Delete (This, All Invisible, All Visible, All).
- **Components:**
  - On a component: Combine Mode, 2D preview order, Export as 3D Clipart, Save as Sculpting Brush, Re-Import, Copy, Duplicate, Copy / Move to other side, Ungroup, Delete, Rename, Show / Hide, Properties.
  - With no component under the pointer: Paste, Import, Show All, Hide All.
- **Levels:** Combine Mode, Show / Hide, Select Components, Clipping, Mirror Mode, Wrapping, Insert New Level, Delete Level, Rename Level, Export as 3D Clipart.
- **2D selection:** Undo, Redo, Cut, Copy, Paste, Delete, Move to Front / Back, Ungroup Component, Object Properties.
  - **[inf]** Cut Craft's Object Properties opens the form that edits what is selected: a shape's or text's own tool, Bitmap Properties for a picture, Component Properties for components, else Set Size for the selected vectors' position and size.
- **Node editing:** span to Line / Bezier / Arc, insert / delete / smooth point, cut vector, make start point, mirror, close or join with a line / curve / moved end points, reverse direction, exit node editing.
- **Also:** sheets, post-processor list (make default), clipart library.

### Menus [check] (job open; the no-job menu is under Startup sequence; to re-confirm in the trial)
- **File:** New (Ctrl+N), New from template, Open (Ctrl+O), Close, Save (Ctrl+S), Save As, Save As Template, Save To Version ▸ 11.0, Increment and Save (Ctrl+Alt+S), Print / Preview / Setup.
  - Import ▸ Vectors/Bitmap (Ctrl+I), Component/3D Model, PhotoVCarve/Machinist/Cut3D toolpaths.
  - Export ▸ EPS, DXF, AI, SVG, PDF.
  - Also: Open Application Data Folder, Recent files, Exit.
- **Edit:** Undo (Ctrl+Z) / Redo (Ctrl+Y), Cut/Copy/Paste, Delete.
  - Selection ▸ All, All Open, All Duplicates, All on Current Layer, Unselect, Vector Selector.
  - Align ▸ centre in material (F9), centre H/V in material, centre, left/right/top/bottom, centre H/V, distribute L→R, R→L, T→B, B→T.
  - Also: Join, Curve Fit, Nest, Job Size and Position, Swap Sides, Notes, Document Variables, Snap Options (F4), Options.
- **Model:**
  - Create Component ▸ Two Rail Sweep, Extrude and Weave, Turn and Spin, Textured Area, Zero Plane.
  - Create Component from ▸ Bake, Visible Model, with Draft, Offset Model, Slice, Toolpath Preview.
  - From Bitmap, Import Component, Export STL / Grayscale Bitmap / 3D Clipart, Sculpt, Smooth, Reset Inside/Outside Vectors, Split, Vector Boundary around Components, Cross Section Vector, Emboss, Scale Z Height, Distort, Replace Below, Component Properties.
- **Machine:** Machine Configuration, Add Machine, Install Offline Package, Search Online, Post-Processor Management / Install / Update DB.
- **Toolpaths:** Show tab, Recalculate All.
  - Drawing ▸ 2D previews, solid previews, auto-open 3D, draw rapid/plunge/retract/join moves.
  - Preview quality ▸ Standard ×1 / High ×2 / Very High ×4 / Extremely High ×8 / Maximum ×25.
  - Templates ▸ save selected / all visible, load.
  - Also: Merge Visible, Job Sheet, Materials, Tool Database.
- **View:** Toolbar, Zoom to Drawing (F6), Refresh 2D (F5), Guides ▸ visible / delete all / lock / unlock.
  - Draw Origin / Modeling Plane / Material Block / Vectors in 3D / Keep-Out Zones.
  - Auto-Wrapping, Colour Shaded, Shaded Background, Light Follows Viewer, Shadow Shading, Save Shaded Image, Tile H/V, Cascade.
  - **[inf]** Cut Craft: Use Shadow Shading (off at first) traces a ray to the light from each cell of a flat job's surface, drawing what a wall or raised part hides from the light at the ambient level only (rotary views are not shadowed). Draw Keep-Out Zones in 3D View (on at first) shows or hides their red outlines. Cut Craft has one window holding the 2D and 3D views, so Tile Windows Horizontal (PgDn) puts them one above the other, Tile Windows Vertical (PgUp) side by side (the top bar's split buttons), and Cascade Windows puts them back in tabs.
- **Help:** contents, shortcuts, tutorials, licence, updates, about. Licensing and vendor web links are out of scope.
  - **Check for Updates [inf]** (Aspire checks Vectric's own servers and has the user download and run a new installer). Cut Craft's differs on purpose, for a novice who will not go looking for a download page: it checks GitHub's releases by itself at startup (about once a day, off in Options ▸ Updates), only says something when there is a newer version, and installs in place after one click, checking the download's minisign signature (the release key is built in; releases are signed in CI by `scripts/sign-release`) and GitHub's SHA-256 checksum first. Nothing downloads or restarts without the user pressing Update Now and Restart Now; restarting goes through the usual "Save changes?" question.

---

## P3 — File I/O

### Native job files [obs] / [unk]
| Ext | Meaning |
|---|---|
| `.crv3d` | Aspire job: 2D vectors + 3D components + calculated toolpaths |
| `.crv` | VCarve job (2D + toolpaths) |
| `.crvt` / `.crvt3d` | job templates |
| `.3dclip` | 3D clipart, keeps its component structure |
| `.v3m` | Vectric 3D clipart model (Vector Art 3D) |
| `.pvc` / `.v3d` | PhotoVCarve / Cut3D toolpath files (import only) |

- **[obs]** The format is versioned. The loader rejects "written by an unsupported later version", can save down to 11.0, and checks trial-version and machine binding (its error messages).
- **[unk]** The full byte layout. What is known comes from jobs the trial saved (see Job files below).
- **Recommendation:** use our own serde/JSON job format (serde_json is already a dependency) and treat `.crv3d` import as a much later stretch goal.

### Import [obs]
- **2D vectors:** DXF/DWG, EPS, AI, PDF. Vectors keep their original size and scale. Opening (not importing) a DXF/EPS/AI opens Job Setup pre-filled with the design extents, and EPS/AI are placed with their lower-left corner at X0,Y0. Each page of a multi-page PDF becomes its own layer. When the result is empty, the importer offers to check the file and retry.
- **Bitmaps:** BMP, JPG, TIF, GIF, PNG. Used to trace over, auto-trace into vectors, or turn into a 3D component (greyscale → height).
- **3D:** CRV3D, 3DCLIP, V3M (placed at their saved size and position), plus STL, 3D-mesh DXF, 3DS, 3DM (render meshes only), SKP. Third-party meshes open an **Orientate 3D Model** step first (orientation, scaling, then conversion to a component).
- **[unk]** SVG import: SVG appears under Export, but the import help page doesn't list it.
- **Toolpaths [inf]:** Aspire's File ▸ Import ▸ Import Toolpaths reads PhotoVCarve, Machinist and Cut3D toolpath files. Cut Craft's reads G-code instead (any post's `.nc`, `.tap`, `.gcode`, `.ngc`, `.cnc`): G0 to G3 (XY arcs by I J or R), G20 / G21, G90 / G91, F, S and T with M6, a toolpath for each tool, z put back from the job's Z zero onto the material surface. The form asks which tool cut it, for the preview; the file's feed, spindle speed and tool number are used where it gives them. Recalculate reads the file again. Drilling cycles (G81 to G89) are refused with a message rather than guessed at. Deliberate difference: G-code is what a hobbyist has from every program, and it lets a file be previewed and checked before it runs. The Vectric formats are still to do.

### Export [obs]
- **Vectors:** EPS, DXF, AI, SVG, PDF.
- **Model:** STL by default, also OBJ (+ `.mtl` + `.jpg` texture, carrying component colours) and POV-Ray `.pov`.
- **Model as greyscale heightmap:** BMP/JPG/GIF.
- **Machining:** toolpaths through a post-processor, as one file per tool or one file for all tools if the machine has a tool changer.

### Cheapest order for us [inf]
1. Binary STL export (about 30 lines, no new crate).
2. SVG export.
3. DXF import/export.
4. Bitmap import (the `image` crate).
5. PDF/EPS/AI and SKP/3DM last; they need heavy libraries.

---

## P4 — Modeling tools

### Job model [obs]
- **Job type:** Single-sided, Double-sided (both sides in one file, `Sd` subsystem) or Rotary (4th axis, wrapped view).
- **Material block:** width X, height Y, thickness Z, and units (mm or inch).
- **Z-zero:** on the material surface or on the machine bed.
- **XY datum:** any corner or the centre, with an optional offset. A red square marks it while Job Setup is open.
- **Design scaling:** when resizing an existing job, choose whether drawings scale with it.
- **Modeling resolution:** Standard / High (3× slower) / Highest (7× slower), applied over the whole job area.
- **Material setup:** the model sits inside the block via *gap above* or *gap below*, the modeling-plane Z is derived from those gaps, there are rapid **Clearance Z1** and **Plunge Z2** heights, and a home/start position.
- **[inf]** The 3D composite model is a **Z-heightmap over the job area**, not a solid mesh. Evidence: the resolution setting covers the whole job area, the model exports as a greyscale bitmap, "Reset Inside/Outside Vectors" exists, and true 3D imports must first be "orientated" into a component. This is the single most important design decision for the clone.

### Vectors (Drawing tab) [obs]
- **Create:** Rectangle, Circle, Ellipse, Polygon, Star, Arc, Line/Polyline, Freehand, Text, Text on a Curve, Dimensions, Fillets, Vector Texture.
- **Edit:** node editing, Join (J), Weld, Subtract, Trim, Overlap, Offset, Extend, Fit Curves, Mirror, Align, Distort, Array Copy, Circular Copy, Copy Along Vectors, Nest Parts, Group/Ungroup, Vector Validator, Vector Unwrapper, Trace Bitmap, Measure (Ctrl+M).
- **Layers** (`Cm`): name, colour, visible, active, ordering arrows, select-all-on-layer, move objects to a layer, Ctrl+L toggles the dialog.
- **Sheets:** several material sheets per job. Double-click a sheet in the 2D view to make it active.
- **Shape-tool interaction**, using Draw Rectangle as the template; every shape tool works the same way:
  1. Drag in the 2D view to create the shape. Alt-drag draws from the centre, Ctrl-drag constrains (square/circle).
  2. **Quick Keys:** type values while still holding the mouse button, then Enter. `a,b`↵ sets width and height, `a`↵ makes a square, and suffix letters pick fields (`X`, `Y`, `W`, `H`, `R` = corner radius), e.g. `0.1R1X`.
  3. A form offers the same values as exact typed input. Rectangle corners can be square, external-radius or internal-radius, with draggable green radius handles.
  4. To edit an existing shape, select it with the form open, change values and Apply. Shift+click moves on to the next shape.
  5. Every tool works in both the 2D and 3D views.
- **Toolbox flyouts** (Cut Craft's own choice, not Aspire's) [inf]: Aspire shows each of these
  tools as an icon of its own; Cut Craft groups tools that do one job different ways behind
  one button so a beginner faces fewer, clearer buttons (Drawing tab 78 buttons to 44). The
  button shows the tool picked from it last (kept in the options); a press held for 0.4 s, a
  click on the arrow in its bottom-right corner, or Down with the button focused opens the
  others, each by icon and name. Find Tool, the Rail layout and Favourites still name every
  tool on its own. The groups are `FLYOUTS` in `crates/app/src/flyout.rs`. Two variants are
  Cut Craft's own: Draw Rectangle from Center (the drag read as Alt-drag) and Draw Circle from
  Edge (the drag is the diameter).
- **Design sub-tabs** (Cut Craft's own choice, not Aspire's) [inf]: Aspire's Drawing and
  Modeling tabs each run every group down one long page. Cut Craft's Design tab keeps File
  Operations and 2D View Control at the top, then four sub-tabs across the page: **Create**
  (Create Vectors), **Edit** (Select and Edit; Join, Combine and Trim; Transform Objects), **Arrange** (Align Objects,
  Offset and Layout, Array Objects) and **Model** (Shape Creation, Shape Editing), so a page
  shows a couple of dozen buttons instead of sixty-odd and a beginner looks under a word that
  says what the tools are for. Each sub-tab says so on hover. A tool run from a menu, a
  shortcut or Find Tool opens its sub-tab, so it shows highlighted where it lives. Favourites,
  the Rail and Ribbon layouts and Find Tool are as before. The sub-tabs are `DESIGN_SUBTABS`
  in `crates/app/src/gui.rs`.
- **Next-step guidance** (Cut Craft's own) [inf]: Aspire's status bar shows a tool's tooltip.
  Cut Craft adds a box in the room left under a page's icons: the hovered tool's name, what
  it does and its next step (a flyout's names the tools it holds too), else the tool in
  hand's; the status bar carries the same line, each tooltip ends with "Next: …", and a
  tool's form opens with the step at its top while its tool has nothing to work on ("Select
  the closed vectors to pocket"; "Select the object to copy first, then the vectors to copy
  it along"; a drawing tool's "Drag from the centre out to the radius"). The step is the
  command's own complaint when run on an empty job, so every tool that works on a selection
  says what to select without a list to keep (`crates/app/src/app/guide.rs`). Once something
  is selected the form shows its settings alone; a wrong selection is still reported by
  Apply.
- **Icon style** [inf]: every icon is drawn by `scripts/make_icons.py` (the seven in
  `scripts/draw_icons.py` match it; `scripts/make_3d_icons.py` keeps only Interactive
  Rotation's shaded pictures) in one clean line style: one steel-blue ink that reads on the
  light and the dark theme, even 2.5-unit strokes of 48 with round caps, open shapes with no
  fills, and one small warm accent only on the part a tool makes or moves. The 3D tools are
  small shaded renders of wood solids (`scripts/make_3d_icons.py`), toolpaths green on a
  wood slab, so the Modeling and Toolpaths families tell apart from the drawing tools at a
  glance. Cut Craft 0.1.0 to 0.2.1 used icons traced
  from Aspire's (see ACKNOWLEDGEMENTS.md); this set keeps their quiet, single-colour feel
  with drawings of Cut Craft's own.

### Create Fillets [obs] (help: Create Fillets, V12.5; Aspire 12.5 trial, 2026-10-25)
- Fillets corners where two **straight** spans meet; corners touching arcs or curves cannot be filleted. The form: **Fillet / Tool Radius** and a type: **Normal** (a rounded corner of the radius), **Dog-Bone** (a relief circle the radius whose edge touches the sharp corner: for slots clearly wider than the cutter), **T-Bone** (the relief circle pushed sideways into one side: for slots about as wide as the cutter; the trial put it in the longer side when the corner itself was clicked) and **Plasma / Drag Knife** (the two sides run on past the corner and join in a loop outside it, so the cutter turns there).
- Trial: a fillet may use a **whole side** (0.8 on a 2 × 1 rectangle's corner worked); a radius larger than the side (5) did nothing in the 2D view. The help says the 3D view gives the largest fillet that fits, with a warning icon.
- Interactive: hover a corner and click; clicking an existing fillet arc removes it (seen in the trial), rebuilding a sharp corner from straight lines.
- Ours fillets every corner of the selection at once, in path order as if clicked in turn (each corner gets what its earlier neighbours left of the sides); optionally only the **Outer** or **Inner** corners of closed shapes [inf]; an oversize radius gets the largest that fits (the help's 3D behaviour). Built (course video dLD0Li8DMYw): with the form open, a click on a sharp corner of any vector fillets it, the fillet previewed under the pointer, a T-bone notching the side the click is nearer, an oversize radius cut to the largest that fits with a warning; a click on a fillet takes it out, its straight sides meeting sharp again. Not built: the floating radius field by the pointer, and dragging the radius.

### Array Copy and Circular Copy of vectors [obs] (help: Array Copy, Circular Copy, V12.5; Aspire 12.5 trial)
- **Array Copy:** form: Selected Objects Size, **Rows (Y)**, **Columns (X)** (default 2 × 2), Spacing **Gap** (between neighbouring copies' edges) or **Offset** (start to start), X and Y; Symmetry buttons; Row / Column Displacement X, Y; Group Copies. Trial: the original stays bottom-left and the grid grows **right and up**; rows × columns objects in all. Not built: Symmetry, Displacement, Group Copies.
- **Circular Copy:** form: Selected Objects Size, **Rotation Center** X, Y (defaults to the middle of the selection), Rotate Copies (on), **Number of items** (default 12), **Total Angle** (default 360) or **Step Angle**, Group Copies. Trial (4 items, total 90°): 4 objects in all, so **the original counts as an item**; the total is **divided by the number of items** (the copies spanned 67.5°, not 90°); a positive total goes **clockwise**. The help: a positive step goes clockwise too. Ours also offers the job centre as the rotation centre. Not built: Group Copies.

### Copy Along Vectors [obs] (help: Copy Along Vectors, V12.5)
- The **first** object selected (with its group) is copied; the other selected vectors are the curves it is copied along.
- Spacing by **Distance between copies** (from the start; **Force even spacing** adjusts it so copies land on the ends) or by **Number of copies** spread along the whole curve. **Align Objects to curve** turns each copy with the curve (ours: as if the object faced +X); **Reverse Direction** runs from the other end; **Create Copies on new layer**.
- Not built: **Copy Circles** (circles of a typed diameter instead of an object). On a closed curve no copy lands on top of the first [inf].

### Close / Extend Vector [obs] (Aspire 12.5 trial tooltips; help: Join / Close with a Smooth Curve, Extend Vectors)
- Aspire's Edit Objects tools here: **Join open vectors** (J), **Join / Close with a straight line**, **Join / Close with a smooth curve**, **Join / Close by moving end points to a common point**, and **Extend vectors to a common point of intersection** (interactive: pick an open end, see its dashed extension, click where it meets another span). The straight-line one has no form; it acts on the selection when clicked.
- Ours is one tool closing each selected open vector by **Line**, **Smooth** curve, **Common Point** (start and end both moved halfway) or **Extend** (the end lines run on until they meet; a line when they never do), within a **max gap** [inf]. Joining two different vectors is Join Vectors. Not built: the smooth join between two vectors, interactive extend to another vector.

### Vector Validator [obs] (help: Vector Validator, V12.5)
- Finds **overlapping** contours, **intersections** and **zero-length spans**; a **Fix zero length Spans** button removes the spans. With a selection only those vectors are checked, else every visible vector on the current sheet and side.
- Aspire marks each problem on the vectors; ours selects the vectors with problems (or fixes the spans). Extra: an **Open vectors** check, off by default. Not built: V-Carving Mode (ignores intersections in text).

### 3D components [obs]
- **Component tree:** the composite model is built from the **bottom of the tree upwards**. Combine modes are **Add / Subtract / Merge / Multiply**. Each component belongs to one **Level**: components inside a level combine first, then levels combine with each other. Per-component settings include combine mode, Fade, Tilt and Appearance (colour or material). Blue arrows reorder components, and double-clicking a component opens Component Properties.
- **Create Shape** (from closed vectors): Round, Angled, Flat, Custom and Smooth profiles. The other creation tools are in the Model menu: Two Rail Sweep, Extrude & Weave, Turn & Spin, Texture Area, Zero Plane and From Bitmap.
- **Modify:** Sculpt, Smooth, Emboss, Distort, Offset Model, Slice, Add Draft, Scale Z Height, Replace Below, Split, Bake, Segment 3D Model.
- **Relief from Photo** (Cut Craft only, not in Aspire): Aspire's From Bitmap reads grey as height, so it works only on a depth map made elsewhere. Cut Craft estimates a photo's depth with an optional add-on (Depth Anything V2 Small, run on the CPU, Apache 2.0 weights; the larger models are non-commercial and are not offered). It then cuts away the backdrop, which it finds from the photo's border so light and dark backdrops both work, and lifts the subject clear of it. It adds the photo's fine shading back as texture and smooths speckle while keeping edges (Perona-Malik). Settings: Depth from (the estimate, or the picture itself when it is already a depth map), Background cut-off (by colour against the photo's border), Depth cut-off (drops everything farther back than a share of the depth range, depth2stl's floor), Invert, Fine detail, Sharpen, Smoothness, Max height, Lift, Soft edge (the outline slopes down instead of dropping, depth2stl's fade), Base. The relief follows its settings live once applied, and shaping runs at the relief's own cell size. This is a deliberate addition for novices: one tool goes from photo to relief, and STL comes from Export as STL.

### Clipart tab (Cut Craft: Library) [inf] (help: Clipart Tab)
- **[inf]** Aspire: the Clipart tab browses clipart libraries by folder, each piece shown as a thumbnail; a double-click imports one, and a thumbnail dragged onto the 2D or 3D view drops it there. Vectric's own `.3dclip` / `.v3m` files carry their previews.
- **[inf]** Cut Craft: a library of the user's own folders, shown as a tree of the folders in them (however deep; hidden folders and links to folders passed over), each with how many files are in it. Clicking a folder shows what is in it and every folder in it; All folders shows everything. Pictures (PNG, JPEG, BMP) show as themselves, shrunk once and kept in the preview cache like a model's preview, so a folder of big photos doesn't fill the memory; GIF and SVG are drawn as they are. A 3D model (STL, OBJ, `.ccclip`) shows a preview: a picture beside it with the same name (`mydesign.png` or `.jpg` / `.jpeg` / `.bmp` for `mydesign.stl`, any letter case) if there is one, else the model seen from the side with the largest outline (from above, unless it was saved standing up: then face on, as a statue is from the front; of that side and its opposite, the front (from above, or from -Y) unless the other has far more relief, so a relief's flat back isn't shown), lit from the top left and with shallow carvings deepened so their detail shows. Previews are made on background threads, only for tiles on screen (newest first), and kept as PNGs in the cache folder (`%LOCALAPPDATA%\cut-craft\clipart-previews` on Windows, `~/Library/Caches/cut-craft/clipart-previews` on macOS, else `$XDG_CACHE_HOME` or `~/.cache`), named from the file's path, size and time changed, so a model is rendered once and again only when it changes. A model that can't be read keeps a "3D" tile, and its tooltip says why.
- **Deliberate differences, for novices:** a model's picture beside it goes with it and is not a tile of its own (one tile per design). Vectric's `.3dclip` / `.v3m` are encrypted, so they are not shown, but the tab counts them and says why, and how to bring them across (export as STL from Aspire). The tab is called Library (it holds the user's own files, not bought clipart). Tiles are picked like files in a file manager (click, Ctrl+click, Shift+click) and a right-click menu acts on all picked: Add to Project, Add to Level, Locate, Add / Remove Tag, Delete from Disk (confirmed; for good). Tags live in the config folder, never in the files. A Show filter (Everything, 3D models, 2D drawings, Pictures, Depth maps) and a Search box (names and tags) narrow the folder. Folders are read in the background, again every 5 s, so files that vanish drop out; a missing folder says so. Only the rows of tiles on screen are drawn, for folders of thousands. Adding and removing folders stays in the tab (not Settings), where a novice looks for them. No folder is listed twice: one added inside another opens there, one added around others replaces them, and old duplicates are tidied on load. Not built: Aspire's thumbnail sizes, a preview for vector files (DXF, SVG drawings show their type).

### Toolpaths (beyond P4, listed for completeness) [obs]
- **Strategies:** Profile, Pocket, Drilling, V-Carve, Quick Engrave (Ctrl+Q), Inlay / V-Carve Inlay, Fluting, Prism, Chamfer, Keyhole, Thread Milling, Drag Knife, Moulding/Extruded, Texture, Rough and Finish Machining (3D), Rounding, PhotoVCarve, Sketch Carving, Laser Cut/Fill and Laser Picture, Array Copy, Merged.
- **Support:** Tool Database (with remote databases), Materials, Templates, Tabs, Keep-Out Zones, Toolpath Tiling, Preview (simulation), Job Sheet, Summary.

### Progress of slow jobs [inf] (help: Calculate buttons; trial use)
- **[inf]** Aspire calculates a toolpath, previews and recalculates on the spot: the window waits, with a progress bar along the bottom for the long ones.
- **Deliberate differences, for novices:** Cut Craft runs a job that takes more than about a quarter of a second (calculating or editing a toolpath, Relief from Photo and the other non-drawing forms, Recalculate, Preview without animation, opening a job, importing a drawing or picture) off the window's thread and shows a window over the app saying what it is doing (the toolpath's name, the 3D level it is on, "Placing the model"), a bar with the percentage where the job can tell, the time so far and about how long is left, and Cancel, which stops it with nothing in the job changed. Quicker jobs finish in place, so the window never flashes. While it is up, keys and clicks do nothing behind it; if the job changes anyway, what the slow job made is thrown away and the status bar says to try again. Not yet: saving a job and saving toolpaths still run in place, and a 3D model's Orientate 3D Model form reads its file in place.

### Tool database: feeds and speeds [obs] (help: Tool Database)
- **Tool types:** V-Bit, Engraving, Tapered Ball Nose, Ball Nose, End Mill, Radiused End Mill (a toroidal or bull-nose cutter: flat bottom, rounded corners of a tip radius), Form Cutters (profile imported from vectors), Diamond Drag, Drills.
- **Geometry vs cutting data:** a tool's geometry (diameter, tip radius, angle…) is one record; its cutting data (pass depth, stepover, spindle speed, feed rate, plunge rate) is kept **per machine and per material**, and the active machine / material picks which set a toolpath gets. The tool number is per machine only. Copying a tool copies its geometry without cutting data.
- **Fields:** Number of Flutes; Pass Depth (sets the number of Z passes, e.g. 25.4 deep at 6.35 per pass = 4 passes); Stepover (stepovers over 50 % add "tail" moves in corners of offset strategies; V-bits have Final Pass and Clearance Pass stepovers instead); Spindle Speed in rpm; Feed Rate and Plunge Rate (plunge also used for ramping); Maximum Burn Rate (lasers, simulation only); Notes.
- **Cut Craft, a deliberate difference:** every tool gets two stepovers, not only V-bits: a **Clearing Stepover** (pockets, flat bottoms, inlay clearing, 3D roughing, moulding clearance) and a **Finishing Stepover** (3D finishing, V-carved surfaces, chamfers, roundings, moulding profiles). A beginner's ball nose then roughs at 40 % and finishes at 10 % without retyping the stepover between toolpaths. Tools saved with one stepover finish with it too (an unset finishing stepover follows the clearing one). Both are kept per material.
- **Cut Craft, a deliberate difference:** loading a tool into a tool changer slot (Tool Database's Tool Changer section, or Machine ▸ Tool Changer…) makes the slot its tool number, and the number field is locked while it is loaded, so the number shown always matches the holder. A tool taken out keeps its last number until another tool is loaded in that slot; then it has none (0), so no two tools share a number. The number is one per tool, not per machine (Aspire keeps it per machine): with two tool changers, the active machine's slots win.
- **Rate units** are chosen per tool: distance per second or per minute. **[check]** The labels offered are mm/sec, mm/min, m/min, inches/sec, inches/min and feet/min.
- **Chip Load** is shown, calculated from flutes, spindle speed and feed rate (feed per tooth: feed ÷ (rpm × flutes)), to compare with the maker's recommendation.
- **Radiused End Mill [obs]** has a Tip Radius (its name template offers `{Tip Radius}` and `{Flat Diameter}`). **[inf]** The shape is a torus: flat across `diameter − 2 × tip radius`, then a quarter circle of the tip radius up to the full diameter, so its edge rises `tip − √(tip² − d²)` at `d` past the flat.

### Profile, Pocket, 3D Roughing and 3D Finishing options [obs] (help: uiProfileMachineForm, uiPocketMachineForm, Toolpath Tabs, Rough / Finish Machining Toolpath)
- **Profile tabs** (Add tabs to toolpath, Length, Thickness, Create 3D tabs, Edit Tabs): **2D tabs** stop at each tab's start, lift by the thickness, cross and plunge down the other side; **3D tabs** are triangular in section, ramping up to the thickness and down without stopping. Thickness is measured up from the bottom of the cut. The Tabs dialog places them automatically at a **Constant number** or **Constant distance**, optionally the first at the start point or **avoiding corners and curved regions**, or by clicking (add, delete, drag).
- **Profile ramps** (Add ramps, Smooth / Zig Zag / Spiral, Distance / Angle, Ramp on Lead In): applied at the start of every pass, at the plunge rate. Smooth ramps over a distance or at an angle; Zig Zag goes back and forth by the distance (Angle: down at the angle, zig length no more than the distance — settings key `rampingRampingAngleMaxDistance`); Spiral (closed paths without leads only) goes down a pass depth per loop round the whole path. With a lead in, Ramp on Lead In keeps the ramp on the lead.
- **Profile leads** (Add leads, Straight Line Lead with Angle, Circular Lead with Radius, Lead Length, Do lead out, Overcut Distance): a straight lead meets the edge at the angle; a circular one curves on tangent to it (angle 0.1–90°, the radius and angle give the length). Lead out leaves the same way; the overcut runs past the start first.
- **Pocket:** **Offset** (rings, climb or conventional; tails in corners over 50 % stepover) or **Raster** (Raster Angle 0–90°, a **Profile Pass** First / Last / None, Pocket Allowance), and **Ramp Plunge Moves** over a distance.
- **3D Roughing:** **Machining Limit Boundary** Model (all components' outline) / Material / Selected Vector(s) / Selected Level, plus a **Boundary Offset** (the tool tip, not its side, stays inside); Machining Allowance; **Z Level** (pocket-like levels round the model, Profile First / Last / None, Order **Level by level** or **Depth-first**: each separate region all the way down before the next) or **3D Raster** (over the whole model; more even skin, slower when deep); Raster direction along X or Y, Reverse step direction, Avoid machined areas; Ramp Plunge Moves.
- **3D Finishing:** the same Machining Limit Boundary and offset; **Offset** or **Raster** (Raster Angle 0–90°) area strategy; Stepover Retract (offset only).
- **[unk]** Defaults (tab size and count, ramp distance and angle, lead length, angle and radius) are not given in the help. Cut Craft uses a 10 mm ramp at 10° and 5 mm leads at 45° or radius 5. Tabs default to a rule of thumb from the user: as thick as the tool's pass depth (at most half the cut depth) and twice as long as thick (a 1/4" bit in 1/4" passes: 1/2" × 1/4"), spaced automatically about every 150 mm, at least three a path and no more than a third of the path in tabs.
- **[inf]** Cut Craft: tab length is the material left, so the toolpath lifts over the length plus the tool's diameter on the centre line. Tabs go evenly from half a spacing in, from the start point, or moved up to half a spacing to a stretch turning under 5° (avoid corners); with Placed spacing, a click on a selected vector in the 2D view adds a tab there and a click on one removes it (the points are kept in the toolpath's settings, each tab on the path nearest it). Leads come from the waste side (outside an outside profile, inside an inside one, left of an open or On path) and ramps then run down the lead; Pocket's ramps and 3D Roughing's are zig zags from the level above. Climb runs clockwise round a part and anticlockwise round a hole or pocket (a spindle turning clockwise). Z Level's profile pass follows the traced edge of each level's area; Avoid Machined Areas leaves out points above the level before (keeping the move up to them); Reverse step direction reverses the order of the lines. Neighbouring lines and rings (on one level, for roughing) are joined at depth when the move between them stays out of the model, lifted by the Stepover Retract between offset finishing rings. Selected Level takes a level by name. Not yet: tabs dragged along a vector; Profile's order, start points and sharp corners are below (2026-10-27).

### 3D Finishing: several tools (rest machining) [obs] (help: Finish Machining Toolpath)
- Any number of tools can be chosen, in the order they will run. The first removes as much as it can; each later one removes only material the earlier ones left ("previously unmachined areas"). An allowance may be left for the last tool.
- **Minimum Detail** (value and slider): unmachined areas smaller than this are ignored by that tool — a trade between detail and machining time.
- **Boundary Offset**: every unmachined area is grown by this before it is machined; small values give many small areas, large values fewer, larger ones.
- Finishing is always one pass at full depth (it ignores pass depth); the usual tool is a ball nose with an 8–12 % stepover.

### Fluting [obs] (help: Fluting Toolpath)
- Cuts along each selected vector from its start point. **Start Depth** is where it begins (0 at the surface, or a pocket floor); **Flute Depth** is added below it.
- **Flute Type:** ramp over the complete length (start depth at the start → full depth at the end); ramp at start (then full depth to the end); ramp at start and end.
- **Ramp Length** is capped at the longest possible ramp (the vector's length, or half of it for start and end). **Ramp %** is a percentage of that longest ramp; **[check]** it must be greater than 0 and less than 100 ("Ramp % is invalid").
- **Ramp Type:** Linear, or Smooth, which curves and transitions smoothly into full depth. **[unk]** the curve itself: no code reference to the fluting strings was found (no LEA or pointer to them); Cut Craft uses a quarter sine, level at full depth.
- **[unk]** whether it cuts in several passes; Cut Craft cuts one pass. Project onto 3D Model is not yet done.

### V-Carve Inlay [obs] (help: VCarve Inlay Toolpath, -Pocket, -Plug)
- Makes two toolpaths with one V-bit: a Pocket carving the shapes, and a Plug carving mirrored copies; after calculation they are separate (editing one does not change the other). Plug vectors are added to the job.
- **Pocket Depth** is the most the pocket goes down. **Glue Gap**: between the plug's top and the pocket floor once fitted; more than 0 and less than the pocket depth. **Surface Clearance**: between the plug's material and the pocket's surface; more than 0 and less than the material thickness.
- **Plug Outer Boundary:** Sheet Limits (cut to the sheet edge) or Vector Offset (only round the shapes), widened by a Boundary Offset (negative ignored). **Plug Destination Sheet:** the same sheet places the plug mirrored; another sheet mirrors and centres it there.
- The pocket form later allows a Start Depth (into an existing pocket). Optional clearance tools clear the flat areas, leaving an allowance for the V-bit.
- **[inf]** Geometry Cut Craft uses: pocket = V-carve to flat depth D; plug = V-carve of (boundary − mirrored shapes) starting at D − glue gap, going a further surface clearance down. Turned over, its walls then lie on the pocket's. Cut Craft has one sheet, so it mirrors across the job's centre line; clearance tools are not done yet.

### Prism Carving [obs] (help: Prism Carving Toolpath)
- An angled tool profiles round the selected shapes at the Start Depth plus Prism Depth, leaving a raised bevel ("hand carved" look). Usually paired with a profile or pocket to cut round the shapes.
- If the prism depth is too shallow the top is flat (truncated). **Set Depth for Full Prism** sets the least depth for a point, from the widest point W and the tool angle A: **[inf]** (W/2) / tan(A/2).
- Cut Direction: conventional or climb. **[inf]** Cut Craft runs on the line itself (the cone makes both slopes) and refuses a full prism wider than the V-bit.

### Texture [obs] (help: Texture Toolpath)
- By default a random pattern of scallops in parallel rows, clipped by any selected vectors. Rows are Stepover apart at an Angle from the X axis; Boundary Vector Offset keeps it in from the boundary (up to the tool radius in a recess).
- Scallop depth is random between Min. Depth (% of Max) and Max. Cut Depth; length random between Min Length (% of Max) and Max. Cut Length; each may overlap the one before by up to Max. Overlap % of the max length, the overlap varying by Variation % (1 % ≈ constant pattern).
- **Use selected vectors as pattern:** the vectors themselves are cut at random depths; **Ramp at start and end** cuts each as a fluting move, otherwise a profile move.
- **[inf]** Cut Craft: each scallop is a half sine; a Seed makes the random pattern repeatable so it recalculates the same. Nothing selected textures the whole material.
- **Load… / Save…** [obs: the shipped `VectorTextures/*.vtexture` files]: a texture's settings as a text file: the line `Vectric Vector Texture Definition`, then `Version: 1`, `Type: 1`, `Units: 1`, `ToolRadius: r`, and `Length`, `Depth`, `Stepover`, `Overlap`, `Angle`, each a value and a second number (Length and Depth: the max and the min %; Overlap: the max as a fraction of the max length and the variation %). **[unk]** what Type, Units and the second numbers of Stepover and Angle mean; both shipped files hold millimetres. Cut Craft reads and writes the same file (lengths in millimetres) and keeps them in its textures folder.

### Quick Engrave [obs] (help: Quick Engrave)
- Ctrl+Q. Depth to engrave (a diamond drag tool uses a Pressure instead, its preview depth from its angle and line width). Stepover is set on the form, not from the tool database.
- Strategy: **Outline** (the tip runs on the lines) or **Fill**, with the patterns **Offset, Hatch, Cross Hatch**. **Use Nose Cone** (the cutter sticks out of a sprung cone by the depth; the preview uses the depth as given). **Number of Passes** runs the pattern several times. Immediate Output saves or sends the toolpath from the form; quick engrave toolpaths cannot be output with other strategies. **[inf]** Cut Craft's Immediate Output: a post-processor, Add side to toolpath name (two-sided jobs) and Save Toolpath Now…, which saves the quick engrave toolpath calculated last (the one being edited) through Save Toolpaths' checks and file dialog. Not built: Output Direct to Machine, and the rule against saving quick engraving with other strategies.
- **[inf]** Cut Craft: hatch lines run along X, cross hatch adds Y; a fill also runs the outline. Diamond drag tools, nose cone and immediate output are not done.

### Keyhole and Drag Knife [obs] (help; VCarve Pro 12.5 gadgets `Keyhole_Toolpath`, `Dragknife_Toolpath`, read for behaviour only)
- **Keyhole:** select vectors (usually circles); each one's centre is a plunge point. The tool plunges to the Slot Depth, runs the Slot Length in one of four directions (bottom→top, top→bottom, left→right, right→left), retraces and lifts out at the plunge point. Optional preview vectors outline the surface: an entry hole of its diameter and a round-ended slot of the slot diameter, on a named layer. Any end mill supplies the feed rates. Length and depth must be more than 0.
- **Drag Knife:** a centre-line cut along the selected vectors, spindle off. Inputs: Blade Offset (pivot to tip), Tolerance Angle, Cut Depth (default: the material thickness), Swivel Depth (less than the cut depth), Passes (equal steps), Use Vector Start Points. The blade is taken to start facing +X.
  - At a corner turning more than the tolerance: cut on past the corner by the blade offset, lift to the swivel depth, arc (radius = blade offset, centred on the corner) to the blade offset along the next span, plunge back to the cut depth, carry on.
  - First cut on a contour not along the blade's direction: an arc at swivel depth, centred the blade offset behind the start, ending at the start; then plunge.
  - **[inf]** Cut Craft always uses the vector start points.

### Chamfer [obs] (help: Chamfer Toolpath)
- An angled edge along the selected vectors. **Start Depth** is the top of the chamfer. **Angle (A)** from vertical: a V-bit fixes it at half its angle; a round-nosed tool takes it from the form and approximates the slope with a series of fine cuts. **Width (W)** and **Cut Depth (C)** are linked by the angle (C = W / tan A); Max Cut Depth (read only) shows how deep the tool actually goes.
- **Overcut** (V-bit): moves the tool's point off the part so its side cuts, avoiding a witness mark; it cuts deeper.
- **Chamfer Type:** inside or outside the vector, sloping down or up away from it (outside-down usually with a clearance pocket outside, outside-up with one inside). Cut direction climb / conventional; optional vector selection order.
- **[inf]** Cut Craft: a V-bit's point runs W + overcut out (down) or the overcut back (up), at start + C + overcut / tan A; a ball nose touches the slope at points a stepover apart. End mills are refused.

### Thread Milling [obs] (help: Thread Milling Toolpath)
- **Pitch Preset**: standard pitches, ISO Metric for metric units and the Unified Thread Standard for imperial, pre-fill the Pitch; for an external thread the preset also fills a default Fit Tolerance for that pitch. **Create Circles for Internal Threads** makes a circle inside the thread for a Pocket toolpath to clear. **Use Vector Selection Order**, else the order is optimised. **[inf]** Cut Craft: the preset drop-down (ISO coarse M2–M24, UNC/UNF #4 to 1″) stands in for the pitch field while chosen; the circle is the ISO minor diameter (major − 1.0825 × pitch) drawn with the toolpath on the current layer; the optimised order is nearest first. Not built: the preset's default fit tolerance.
- A helix round the centre of each selected vector, for internal (nut) or external (bolt) threads. **Start Depth**; **Max Depth (M)** below it; **Thread Length (L)** is derived from M, the tool's tooth offset and the pitch, and is less than M.
- Tools: single point (a triangular tooth; Size S, Height H, Diameter D tip to tip, Angle A, Offset O from the tool bottom to the tooth tip, at least S/2) cuts one thread per turn as a helix; multi point also has a threaded length and cuts every thread in one turn at its own pitch.
- **Pitch** (presets: ISO metric / Unified), **Diameter** (major), **Fit Tolerance** (positive cuts deeper; presets fill one for external threads), **Create Circles for Internal Threads** (a circle to pocket out the middle). Thread direction right / left handed; cut direction spiralling down or up. Threads follow the ISO profile (60° tools give flat crests).
- **[inf]** Cut Craft: L = M − O; the tooth reaches the major diameter (internal) or the ISO external minor diameter d − 1.22687 P (external), widened by the tolerance; a right-hand thread climbs anticlockwise. Single point only, the tooth offset on the toolpath form; no presets or circles yet.

### Moulding [obs] (help: Moulding Toolpath)
- Extrudes an open cross-section profile along one or more drive rails (2.5D, no 3D model). Select the rails, then the profile last. The profile's start point hangs on the rail's start; on a closed rail the profile is always outside (the rail is the inside edge of a frame). Reverse Rail / Reverse Profile change direction and start.
- The profile's height sets the toolpath's Z; one taller than the material must be fixed by the user. Passes follow the rail a stepover apart; at the end of an open rail it lifts at least a stepover, steps over and comes back the other way.
- Options: Vary Stepover (closer passes on steep parts), Use Larger Area Clearance Tool (a "[Clear]" Z-level roughing toolpath first, with Machining Allowance, Machine Flat Regions, Ramp Plunge Moves; the finish can Skip Flat Regions), Create Sharp Corners (mitred rather than rolled), Boundary Offset (carry the tool past the profile's ends; or automatic).
- **[inf]** Cut Craft: the profile is the newest selected open vector (the selection has no order); passes lie on the right of an open rail; each pass is the tool as low as it can go without cutting the profile (flat beyond its ends), limited to the material bottom; it lifts to the safe height between passes. No clearance tool or vary stepover yet.

### Rounding [obs] (help: Create Rounding Toolpath; a gadget)
- For rotary jobs: machines a square or round blank (its side or diameter) down to the finished diameter, leaving an optional allowance. Z zero is best at the cylinder's centre for wrapped output.
- Methods: **Radial** (a full turn round, step along by the stepover, turn back), **Raster** (along the cylinder, index round by the stepover, back), **Optimized Raster** (square blanks only: only where there is material). The tool database tool sets stepover, step down and feeds.
- **[inf]** Cut Craft: levels from the blank's outer radius (half the diagonal of a square) down in equal steps of the pass depth; the stepover round the cylinder is measured on the finished surface.

### Photo V-Carve [obs] (help: Photo V-Carve Toolpath)
- A V-bit etches the selected bitmap (prepared with Crop Bitmap / Edit Picture). **Start Depth**; **Max Carving Depth** (also limits how many lines fit); **Stepover Retract** (lift between lines: enough to move safely, as little as possible).
- Strategy: **Raster** (one set of lines), **Hatch** (two sets at 90°, denser, twice the time), **Selected Vectors** (along them, e.g. from Vector Texture). **Line Spacing** is a percentage of the tool's width at the max depth: 100 % gives knife-edge ridges in the darkest parts; more leaves flats on the surface. **Line Angle**: 0 horizontal, 90 vertical. Optional projection onto the 3D model.
- **[inf]** Cut Craft: depth = start + max depth × darkness; lines run back and forth; the newest bitmap is carved (bitmaps cannot be selected yet).

### Sketch Carving [obs] (help: Sketch Carving)
- Carves a sketch-style outline from the contrast in a bitmap or a 3D model. **Start Depth**, **Cut Depth**, a tool. **Machine Limit Boundary:** the bitmap's (or model's) edge, a selected vector (Shift-click after the source), or a component level; plus a **Boundary Offset**. **Line Thickness** 0–100: heavier lines with less detail as it rises. The form previews the area to be cut in green.
- **[inf]** Cut Craft: a cell is on a line where it is darker (or, for the model, lower) than the mean of its surroundings by 4 %, the surroundings 1–9 cells across by line thickness, cells a stepover apart; the lines are cut flat at the depth in rows. The newest bitmap is used; no component-level boundary.

### Lasers [obs] (help: Laser Cut - Fill, Laser Picture, Adapting a post-processor for Lasers)
- Laser tools come from the tool database's laser settings: **Power** (% of the machine's maximum), **Move Speed**, **No. Passes**, and a kerf.
- **Laser Cut / Fill:** Cut Outside / Right and Cut Inside / Left (offset by the kerf so the part or hole is the drawn size, plus an **Allowance**), Cut On, Hatch Fill (**Stepover**, **Hatch Angle**, **Cross Hatch**, **Outline**, Fill Together / Fill Regionally).
- **Laser Picture:** burns the selected bitmap along Raster, Hatch or Selected vectors; power follows the tones, or **Dither Image** uses only burnt / unburnt; **Line Spacing** from the kerf (densest) to 30 × the kerf; **Line Angle**.
- **Posts:** `LASER_SUPPORT = "YES"` marks a laser post; power prints through `VAR POWER` (`[P]`, 1–100 %, e.g. `[P|C|S|1.0|10.0]` for grbl's 0–1000), blocks `JET_TOOL_ON` (e.g. `M4[P]`), `JET_TOOL_POWER` (`[P]`), `JET_TOOL_OFF` (`M5`); the laser comes on only once it is at its Z, never during a plunge.
- **[inf]** Cut Craft: a laser toolpath holds a power per move (0 on rapids); the post turns the jet on before the first feed move after a rapid, prints JET_TOOL_POWER when the power changes, and off before rapids and at the end; a post without LASER_SUPPORT refuses laser toolpaths. Cut Craft's own FluidNC posts are not laser posts. Passes repeat at the surface; Fill Regionally is not done.

### Array Copy and Merged toolpaths [obs] (help: Array Copy Toolpath, Merged Toolpath)
- **Array Copy:** a grid of Rows (Y) × Columns (X) copies of the visible toolpaths, above and to the right of them, spaced by **Offset** (start to start) or **Gap** (between extents). Editing a source updates its copies; sources show under the array in the toolpath list, and hiding a source leaves its copies out when saving. Each copy is normally cut whole before the next; **Minimize tool changes** (several tools) groups each tool's work across the copies.
- **Merged:** merges all visible toolpaths that use the same tool into one, trying several orderings and keeping the quickest; **Merge By Part** cuts each part's contours together, respecting the toolpath list's order; feeds and speeds change part-way as needed.
- **[inf]** Cut Craft: an array toolpath per source (the tool-grouped order), each recalculated from its source. Merged refuses toolpaths with different tools, splits them into their plunge-to-retract cuts, and keeps the toolpath list's order or a nearest-next order, whichever travels less — since the course review: the Ordering ticks Left to Right, Bottom to Top, Grid (rows an eighth of the job high) and Shortest Path, the quickest of those ticked kept (none: the list's order); Merge By Part groups each outermost closed cut with the cuts starting inside it, orders the parts the same way and keeps the list's order within a part; the merged toolpath's right-click Ungroup removes it and shows its sources again.

### Inlay (pocket / plug) [obs] (help: Inlay Toolpath)
- Profile or pocket toolpaths that compensate for the tool radius so a cut-out part fits its cavity; both halves must use the same (finishing) tool. A cut-out leaves inside corners rounded and a pocket leaves outside corners rounded, so the inlay rounds both.
- **Male:** Straight (a profile outside only, external corners rounded) or Stepped (push-through: a Step Depth from the start down and a Step Width out from the vector, forming a shelf). **Female:** Pocket (standard pocket options, two tools allowed, internal corners rounded) or Hole (a profile inside only).
- **Allowance:** an offset on either side so the parts fit (0.25–0.5 mm typical), usually on the female side.
- **[inf]** Cut Craft: the inlay shape is the shapes opened then closed by the tool radius; the male is cut round it (shrunk by the allowance), the female pocket or hole grown by it.

---

## P5 — Directories and configuration

- **[obs]** The install directory is read-only data (fonts, icons, help, languages). User data lives in a separate **Application Data Folder**, which has its own menu item to open it.
- **[obs]** `Default.prf` is the default "profile": a name line followed by one positional numeric value per line, about 60 values. **[unk]** What each value means.
- **[obs]** `ProductInfo.ini`: `[ProductInfo] product_name, product_version=12.0.0.9, install, update`.
- **[obs]** The Options dialog has five sections: Window Layout, 3D View Settings, Toolpath Settings, General Settings, Confirmation Choices ("don't ask again" answers).
- **[obs]** Snap settings and view-toolbar toggles persist between sessions.
- **Cut Craft difference (deliberate):** Options adds an **Advanced** section that lists only while *Show advanced settings* is ticked (off at every start), under a warning. It holds system values (processor cores to use, and later 3D detail limits, gadget limits and toolpath accuracy) that can freeze the app or spoil a cut. Its values are saved with a keyed check; if `options.json` was edited outside Cut Craft they go back to their defaults and the status bar says why. Every Options page has **Reset to defaults** for that page. Aspire has neither; a novice should not meet these values by accident. `docs/settings-inventory.md` lists which hard-coded values become settings.
- **Recommendation:** keep our `config/` but store **keyed** JSON (serde), never positional. Put user data under the XDG dirs (`$XDG_CONFIG_HOME/cut-craft`, `$XDG_DATA_HOME/cut-craft`) rather than the working directory. The current `main.rs` creates `assets/ config/ output/` in whatever directory the binary is launched from.

---

## P6 — Input handling

### Mouse [obs]
| Action | 2D view | 3D view |
|---|---|---|
| Pan | middle-drag; Ctrl+right-drag; Pan tool (Ctrl+P) + left-drag | middle-drag; Ctrl+right-drag; left+right drag |
| Zoom | wheel; Shift+right-drag | wheel |
| Zoom box | Z tool: drag a box; click zooms in, Shift+click zooms out | zoom-to-box button |
| Rotate | — | right-drag ("twiddle") **[unk: the component-tree page says left-drag; the two help pages conflict]** |
| Fit | F / F6 = job limits, Ctrl+F = selection | zoom-to-fit / zoom-to-selection buttons |

### Selection and transform [obs]
- Click selects. **Shift+click** toggles an object in or out of the selection.
- A drag **left→right** selects objects *fully inside* the box. A drag **right→left** selects objects *touching* it.
- Ctrl+A selects all vectors. Esc or a click on empty background deselects.
- **Smart part selection [inf]** (help: What's New in Aspire V12.5): "Select all vectors internal to a selected boundary using the right click context menu in both 2D and 3D view", so complete parts can be picked "without requiring manual grouping first".
  - **[inf]** Cut Craft: right-click ▸ **Select Vectors Inside** (also Edit ▸ Selection) keeps the selection and adds every vector lying wholly inside a selected closed shape; with no closed shape selected it says to select one first. 2D view only so far.
  - **[inf]** Cut Craft, a deliberate difference: a plain **click inside a closed shape** (away from any line, picture or component) also picks the part: the smallest closed shape round the click plus everything wholly inside it (its holes, open vectors such as grooves, and islands in its holes). A novice clicks inside a shape to pick it on day one and would not find a menu item. Clicking inside a hole picks the hole and what is in it; Shift-click adds the part, or takes it away when it is all selected already. Overlapping shapes are never each other's holes (a closed shape must be smaller and wholly inside). Lines, pictures and components under the click still win, so nothing that was pickable before stops being so.
- Selected vectors draw as **dotted magenta**; groups and text draw as solid magenta.
- **Click an already-selected object again** to show the transform handles:
  - **Centre handle:** move. Alt restricts the move to one axis. Clicking it again switches to the rotation centre, which can be dragged.
  - **White corner handles:** proportional scale. Alt makes it non-proportional, Shift scales about the centre.
  - **Edge handles:** scale along one axis. Shift keeps it proportional.
  - **Black corner handles:** rotate. Alt snaps to 15° steps.
  - **3D view differences:** the edge handles are black, and a rotation arrow sits on top.
  - **Cut Craft differs, deliberately:** one rule for the keys on every handle, 2D and 3D:
    Shift or Ctrl scales in proportion, moves along one axis, and snaps rotation (Shift to
    5.625°, Ctrl to 1°); Alt scales about the centre. The status bar names the keys over
    each handle. Aspire's Alt-for-15° and per-handle key meanings are harder to learn.
  - **Cut Craft's 3D handles** sit level with the highest surface inside the selection (on
    top of a raised component, not under it) inside a dashed box; they work on vectors
    as well as components; Escape cancels a drag; letting go over the horizon changes
    nothing. [inf: where Aspire draws them on a raised component is not confirmed.]
  - Grabbing any handle focuses an **edit box** where an exact value can be typed, then Enter.
- **Node editing:** N toggles it on and off.
  - Drag nodes, Bezier control points or spans. Shift / box selection picks several nodes.
  - Ctrl+drag on an arc or Bezier span moves the whole span.
  - A "keep Bezier tangency" toggle.
  - Right-click a node or span to insert/delete a point, cut the vector or move the start point.
- **Nudge:** Ctrl+Shift+arrow keys move by the fixed nudge distance.

### Snapping [obs]
- **Rulers** along the top and left of the 2D view, in job units, subdividing into 10ths as you zoom.
- **Guides:** drag one out of a ruler. It snaps to ruler units (Shift overrides). Ctrl+drag from an existing guide creates a relative guide. Right-click opens guide properties: position, angle (degrees counter-clockwise from X), lock. Clicking the top-left corner of the view toggles guide visibility.
- **Snap options (F4):**
  - Cursor coordinate text.
  - Snap to guides, guides snap to geometry.
  - Grid with a spacing setting.
  - Zoom-relative snap distances.
  - Job centre and corners.
  - Fixed nudge distance.
  - **Snap radius in pixels.**
  - Geometry snaps: object centres, span end/mid points, arc centres, intersections, horizontal/vertical/angle alignment.
- **Smart snapping:** hovering "wakes" nodes and vectors, and the most recent few take priority (with a cap on how many). It draws dashed snap lines for bounding boxes and centres, horizontal and vertical lines through nodes, tangents, perpendiculars to tangents, lines connecting two woken nodes (and their midpoint), the span geometry itself, angle constraints, and job edge and centre lines.
- **Hold Shift** to turn snapping off temporarily.

### Keyboard shortcuts [check] (help: keyboard shortcuts; to re-confirm in the trial)
| Key | Command | Key | Command |
|---|---|---|---|
| Ctrl+N / O / S | New / Open / Save | Ctrl+Alt+S | Increment and save |
| Ctrl+Z, Alt+Bksp / Ctrl+Y | Undo / Redo | Ctrl+X, Shift+Del / Ctrl+C, Ctrl+Ins / Ctrl+V, Shift+Ins | Cut / Copy / Paste |
| Ctrl+Del | Delete selection | Ctrl+A | Select all |
| Ctrl+I | Import vectors/bitmap | Ctrl+D | Show Drawing tab |
| Ctrl+L | Layers dialog | F8 | Component manager |
| M / R / T | Move / Rotate / Set size | F9 / F10 | Centre in material / Align |
| J | Join vectors | N | Node edit toggle **[inf: id unnamed; from docs]** |
| Ctrl+M | Measure | Ctrl+Shift+D | Convert to contours |
| F4 | Snap options | Ctrl+Q | Quick Engrave |
| F2 / F3 | 2D / 3D view **[inf: ids unnamed; from docs]** | F11 / F12 | Drawing / Toolpaths layout |
| F5 | Refresh 2D | Ctrl+P | Pan mode |
| Z | Zoom box | F, Shift+F, F6, Shift+F6 | Zoom to job |
| Ctrl+F (Ctrl+F6) | Zoom selected | PgUp / PgDn (any modifier) | Tile views |
| = | Single/double-sided view | 1 / 2 | **[unk]** (likely top/bottom side) |
| Ctrl+F1…F9 | **[unk]** (unnamed command ids 33786–33796) | | |

---

## Post-processor `.pp` format (for G-code output) [obs]
A plain-text, line-based format. Supporting it lets users reuse the `.pp` file for their machine.
- **Comments:** `+` at the start of a line (a `+====` banner by convention).
- **Global statements:**
  - `POST_NAME="…(*.ext)"`, `FILE_EXTENSION`, `UNITS="MM"|"INCHES"`
  - `LINE_ENDING="[13][10]"` (decimal codes), `LINE_NUMBER` / `_INCREMENT` / `_MAXIMUM`
  - `SUBSTITUTE="O1 S1 O2 S2"` (character substitution), `SPINDLE_SPEED_RANGE=min max rpm_min rpm_max`
  - `RAPID_PLUNGE_TO_STARTZ`, `DIRECT_OUTPUT`, `PRINT_DIRECT`, `ROTARY_WRAP_X/Y=A|B`
  - `INVERSE_TIME_MODE`, `LASER_SUPPORT`, `MIN_ARC_RADIUS` / `MAX_ARC_RADIUS`
  - `TAPE_SPLITTING=max_lines tol "fmt" start "YES"`
- **Variables:** `VAR NAME = [LABEL|WHEN|PREFIX|FORMAT|MULT]`.
  - `WHEN` is `A` (always) or `C` (only when changed).
  - `FORMAT` = `[flags]width.decimals`. The flags are `-` (left-justify), `+` (always print sign), `0` (zero-pad) and `#` (always print the separator), and the separator character itself may be `,`.
  - `MULT` is an optional multiplier.
  - Labels: X Y Z A, XH YH ZH, F FC FP S P T TP N, I J, SAFEZ, XMIN/XMAX…, XLENGTH…, TOOLNAME, TOOLPATH_NAME, TP_FILENAME, TP_DIR, TP_EXT, PATHNAME, DATE, TIME, PRODUCT, FILE_NOTES, TOOLPATH_NOTES, TOOL_NOTES, TDIA, WRAP_DIA, DWELL, X_ORIGIN_POS / Y_ORIGIN_POS / Z_ORIGIN / XY_ORIGIN.
- **Blocks:** `begin NAME` followed by quoted template lines. The names:
  - HEADER, FOOTER, SPINDLE_ON, TOOLCHANGE, NEW_SEGMENT, FEED_RATE_CHANGE.
  - INITIAL_RAPID_MOVE, RAPID_MOVE, FIRST_FEED_MOVE, FEED_MOVE, FIRST_PLUNGE_MOVE, PLUNGE_MOVE, RETRACT_MOVE, DWELL_MOVE.
  - FIRST_CW_ARC_MOVE, CW_ARC_MOVE, FIRST_CCW_ARC_MOVE, CCW_ARC_MOVE, and the `*_HELICAL_ARC_MOVE` / `*_HELICAL_ARC_PLUNGE_MOVE` variants.
  - JET_TOOL_POWER / ON / OFF (lasers).
- **Feed units [obs]** (help: post-processor-editing): a post prints feed rates **per minute** (mm/min or inches/min, by `UNITS`) unless its variable has a multiplier. The help's examples multiply by `0.01666` to convert "from inches per minute to inches per second" and "from mm per minute to mm per second". Real grbl / FluidNC posts use multiplier 1 (mm/min).
- **[inf] A new feed rate starts a new feed run:** real posts print `[F]` only in `FIRST_FEED_MOVE` (and `FIRST_PLUNGE_MOVE`), so the `FIRST_` block must be used whenever the rate changes (plunge to cut), not only after a rapid; otherwise the cut would run at the plunge rate.
- **Inverse time [obs]:** `INVERSE_TIME_MODE = YES` makes feed moves print `[FI]` instead of a rate: the move takes 1/F minutes (G93; G94 is units per minute), so F = feed rate ÷ move length.
- **More variables and blocks [check]:** the post-processor editing guide and the plain-text `.pp` files Vectric ships list more (rotary, laser, scripts after output, `%IF` templates); add them from those as needed.
- **Meanings [obs]** (help table): `XLENGTH` etc. are the material size; `Z_ORIGIN` is "Table or Material Surface" (**[obs]** the UI words are "Material Surface" and "Machine Bed"); `XY_ORIGIN` is the X, Y origin; `TOOLS_USED` and `TOOLPATHS_OUTPUT` list the tools / toolpaths in order of use; `DATE` / `TIME` are when the file was made; `PRODUCT` is the program name and version; `DWELL` is seconds when drilling; `WRAP_DIA` is the cylinder's diameter.

---

## Fonts: `.vfont` [obs] (the 23 fonts the trial installs, read as files with `re/vfont.py`)

- An OLE compound file (`D0 CF 11 E0`, version 4, 4096-byte sectors; the last sector may be
  cut short). Two streams: `VectricFontInfo/Info` and `VectricFontData/GLYPHS_0_255`.
- Both streams are MFC `CArchive` serialization: an object is `FFFF`, schema (u16), name
  length (u16), class name, then its fields; a later object of a known class starts with
  `8000 | n`, where n numbers classes and objects together from 1; `0000` is a null object.
- `Info`: u32 version (3; 1 in the older `Century`), u32, name (u8 length + bytes), and in
  version 3 six f64 (`[1, 1/h, 1, 0, h, 1]`, h = capital height in font units: 1500 or
  15000), then a `txtKerningPairs` object or null: u32 version, u32 count, f64, then count ×
  (u16 first, u16 second, i32 extra advance in font units). The older version stores no
  height; the top of `H` stands in.
- `GLYPHS_0_255`: u32, then one object per code 0–255 (null for missing glyphs).
  - `txtVectricChar`: u32 version, u32 code, f64 advance, a `vdContourGroup` or null.
  - `vdContourGroup`: u32 version, u32 count, the contours; version 2 adds a u8.
  - `vdContour`: u32 version, i32, f64 tolerance, f64, u16, u32 span count, the spans;
    version 2 adds 16 bytes after them. Closed when the last span ends at the first's start.
    Single-line ("1L") fonts are open strokes; the others are closed outlines.
  - Spans: u32 version, u32 kind, start x y z, end x y z (z always 0), u32; then
    `vdLineSpan` nothing, `vdArcSpan` f64 bulge (tan(sweep/4), positive counter-clockwise),
    `vdBezierSpan` two control points (x y each).
- Codes are Windows-1252, except three fonts (AvCO, Avntgrd 1L, Times New Roman 3L) in code
  page 437; those are the ones with a glyph at 129. **[inf]**
- Text Height is the capital height (the Info height matches the top of `H` in every v3 font).
  The files hold no line spacing; Cut Craft uses 1.6 capital heights. **[inf]**
- Cut Craft reads `.vfont` files from `~/.config/cut-craft/fonts` (a link to a Vectric
  install's `Fonts/` will do); none ship with it.

### Create Text form [obs] (help: Create Text, Text on a Curve)

Text / Text in a Box radios; Text box (with Larger Edit …); Font: TrueType / Single Line radios
over one font list, Bold, Italic; Text Alignment: Left, Center, Right; Text Height; Anchor
Point X, Y "(Click on drawing to set position)"; Apply, Close. Text in a Box swaps height and
anchor for Bounding Box Width / Height, Margin Size (None, Normal, Wide), Vertical Stretch and
Horizontal Stretch drop-downs.

## Job files: `.crv3d` [obs] (samples saved from the Aspire trial, read with `re/crv3d.py`)

- The same OLE compound container as `.vfont`. Streams: `VersionData/Version`,
  `PreviewData/Preview2D_GIF` and `Preview3D_PNG`, `SideData/Sides`, `VectorData/MaterialSize`,
  `VectorData/DocumentData`, `VectorData/2dDataV2`, `ModelObjects/ObjectData`,
  `ModelSettings/Settings`, `Toolpaths/ToolpathData`, `ToolpathDataEx`, `Simulation`.
- `MaterialSize`: u32 8, u8, u8, then six f64: the material's x min, y min, z min (minus the
  thickness), x max, y max, z max (0), in the job's units. There is no units flag in it or
  in `DocumentData`; **[inf]** the units show in every `vdContour`'s fitting tolerance and in
  the toolpath settings: 0.001 and plain values in a mm job, 0.001/25.4 (3.94e-5) and values
  divided by 25.4 in an inch job (New-inch / New-mm, the same job saved in each; the
  geometry numbers stayed the same, so the job's units are a label on the numbers).
  A rotary job is stored unwrapped: a 50 mm diameter × 20 mm long job is x ±10, y ±(π × 25)
  = ±78.54, z from −25 (the radius) to 0, the same unwrapping Cut Craft uses.
- `2dDataV2` is MFC `CArchive` serialization (as in `.vfont`): u32 version 2, u32, u32 layer
  count, then each `vcCadLayer` (v4): 16-byte GUID, UTF-16 name (`FF FE FF` + u8 length,
  or `FF` + u16), flags, then its records. Every record sits behind a `Vectric__Version`
  wrapper: u32 16, that name, u32 1, u32 36, a 36-char GUID string, u32 wrapper version. The
  GUID names the record (it is not fixed per kind); a nil GUID marks a component's 2D preview
  (`vcCadModelPreview`), whose contour is the component's outline, not a drawn vector.
- A drawn vector is a `vcCadContour` (v10: own GUID, layer GUID, …, bounding box as 4 f64,
  a 3×3 f64 transform) wrapping a `vdContour` **version 7**: i32, f64 tolerance (4e-5),
  f64, u16, u32 span count, then the spans **inline** (not as objects, unlike `.vfont`):
  u8 kind (0 line, 1 arc, 2 Bézier), u32 span version (2), u32 type, start x y z, end x y z,
  u32 flags, then an arc's f64 bulge (tan(sweep/4), positive counter-clockwise; a circle is
  four quarter arcs) or a Bézier's two control points. Coordinates are in the job's units.
- Text is a `txtBlock` record (font name, the string, every glyph as `vdContourGroup`s
  of `txtChar`s) with `_txtAL_*` and `_txtblk*` parameters, not plain contours.
- Layers seen: "Toolpath Previews" (always first), the user's, and after an STL import
  "Zero Plane", "2Rail Sweep Rails", "Bounding Box" and one named for the model. "Sheet 1"
  is a `vcCadSheet`.
- `ModelObjects/ObjectData`: `sdModelManager`, `vmmObjectNode` / `cmModelNode` per component,
  a `RawRotaryModelImportData` holding the imported model's source path and name (UTF-16),
  a GUID wrapper, then a readable header (three f64::MAX, f64s that look like a transform)
  and a large block at 8.00 bits/byte (336 KB for a 217 MB STL): compressed or encrypted,
  not zlib. **[unk]** Its layout. Cut Craft could re-import the model from the stored path.
- `re/crv3d.py` extracts the contours per layer; `fmt-import-crv3d` imports them.
  **[unk]** the remaining fields of layers and contours (colours, visibility, groups), the
  toolpath and sheet streams, and `.v3m`.

## Toolpath, post, tool and 3D view work of 2026-10-27 [obs] / [inf]

What was found, and what Cut Craft does where Aspire's behaviour is not known.

### Arc output in posts; Plunge (Z2) and Home

#### Aspire behaviour found (sources)
- Material Setup help (`re/extracted/help-12.5/material-setup-flat.txt`, "Rapid Z Gaps above
  Material"): Clearance (Z1) is the safe height for rapid moves; Plunge (Z2) is a smaller gap
  the tool rapids down to before changing to the plunge rate; by default Z2 equals Z1 (no rapid
  plunges). Home / Start Position is "the absolute position that the tool will start moving
  from" and can be returned to at the end.
- Material Setup form [check]: the home group is X, Y and
  "Z Gap above Material", so home Z is a gap above the material, not a machine Z.
- Post help (`post-processor-editing.txt`): the arc variables and blocks above;
  `MIN_ARC_RADIUS` turns smaller arcs into a straight line, `MAX_ARC_RADIUS` polygonizes larger
  ones (example value 1000.0); the help says the arc angle can be multiplied by 57.2957795 to
  print degrees, so `[Angle]` is in radians.
- `RAPID_PLUNGE_TO_STARTZ = "YES"` "indicates that plunge moves to Plunge (Z2) height ... are
  rapid moves".

#### Guessed / not done
- [inf] The move down to Z2 is always written as a rapid; `RAPID_PLUNGE_TO_STARTZ` is still
  ignored. Its help line might mean posts without it write that move at the plunge rate; the
  Material Setup help says it is rapid, so rapid was chosen.
- [inf] `[Angle]` is signed, anticlockwise positive; Aspire's sign is not known.
- Arcs are capped at a half turn so `[Radius]` (R-format) posts are never ambiguous; a full
  circle is two or three arcs.
- Under `MIN_ARC_RADIUS` Cut Craft keeps the original lines rather than one straight line.
- The `*_HELICAL_ARC_PLUNGE_MOVE` blocks are never written (arcs are fitted to Cut moves only).
- Z2 and home are applied at post time; the 3D toolpath preview still shows the plunge from
  the clearance.
- The toolpath tolerance is not a user setting yet.

### Profile order, start points, nesting and sharp corners

#### Aspire behaviour found (sources)
- help `uiProfileMachineForm`: nested vectors cut on the correct side, inner always before
  outer; Order options may be combined and the shortest machining time wins; Start At three
  options; "Closest on Bounding Box ... nearest point from all of the spans' endpoints".
- Order tab [check]: five options incl. **Shortest Path** (not in the help), and "the
  option which results in the shortest rapid move join ups will be used" — so the measure is
  rapid link length, which is what is used.
- Start tab [check]: Keep Current / Optimize / "Move Start Points to be closest to
  selected point on vector bounding box" with 8 radio buttons (corners and mid-edges, no
  centre).
- Corners tab [check]: "Sharp external corners", "Sharp internal corners (3D)",
  "Allowance for cut out tool".

#### Guessed defaults
- All five order options ticked (Aspire's defaults unknown; trying them all costs little).
- Start At defaults to Optimize (the old help's "Use Vector Start Point" was off by default,
  which meant optimise); bounding-box point defaults to top left (the first radio button).
- Mitre limit 4 tool radii (unknown in Aspire).
- Bounding box is the centre line's own box, not the vector's: round an offset corner every
  arc point is equally near the vector's box corner, so the vector's box gives no answer.
- The tool starts from the origin (the job has no home position yet).

#### Not done
- Sharp Internal Corner (3D) V-bit move and its "Allowance for cut out tool".
- Open paths are never reversed to shorten links.
- Overlapping (not nested) vectors are now taken even-odd: their overlap becomes a hole.
  Before they were unioned. Aspire's handling unknown.
- The `corners` form widget only knows corners and centre; `start_near` uses the plain
  choice list (9 anchors incl. centre). A widget with mid-edge spots would match Aspire's
  picker (app code, not touched).
- Sequencing is O(n²) per ordering (fine for hundreds of parts).
- re/SPEC.md "Not yet: ... Aspire's order options ..., sharp corners" line is now stale
  (shared file, not edited).

### Cutting data per material, rate units and thread mills
- **Material Management** [obs: help]: Add Material; Duplicate Material, optionally with Copy Cutting Data (every tool's data for the original copied for the new material); Delete Material deletes every tool's data for it; Name (two materials cannot share one). Cutting data missing for a tool is created with defaults, copied from the same tool for a different material, or from an identical tool. **[inf]** Cut Craft: Duplicate Material (a "with cutting data" tick) names the copy as typed or "… copy", numbered while taken; Copy settings from, over a tool's cutting data, copies from another material or the default. Data is per material only; machines carry none.

#### What the vendor files contain (Amana ×2 copies, CIC, IDC Woodcraft, SpeTool `.vtdb`)
- Each file has exactly one machine ("Imported", "iQ 2x3 - Default", "Desktop") and one material
  ("Hardwood", "All Material", "Softwood", "Acrylic").
- Every geometry has two `tool_entity` rows. One has no material and no feed rate (a
  placeholder). The other has the material and its data.
- `rate_units` values seen are 1 and 4, plus a single 2 in IDC: 1.27 / 0.381 for a 1/8" end mill
  at 22000 rpm, which is plausible as m/min.
- There is no thread data anywhere: `tooth_*` and `thread_*` are all empty. `tool_type` 12 is a
  laser (SpeTool, name format "{Max Power}W").
- `tool_number` is per cutting data (IDC uses 1–215).
- The SpeTool Fusion library has 1–4 presets per tool: SOFTWOOD, HARDWOOD, MDF/Laminate,
  ALUMINUM…, plus "default", "Default Preset" and 默认预设.

#### Aspire behaviour found (sources)
- Cutting data is per machine and per material. The active material and machine are combo boxes
  in the Tool Database dialog, and switching them switches the data shown. Source: help
  `Tool Database.txt` § Material / Machine Management.
- Add Material makes the new one active. Delete Material deletes all its cutting data. Names are
  unique. Source: help `tool-database-material-management.txt`.
- Rate units: "distance per second or per minute"; the list of labels is [check].
- Thread mill S, H, D, A, O: O must be bigger than half the tool size. H, S and A are linked,
  and changing H with A fixed changes S. Source: help `ThreadMillingStrategy.txt`.
- The tool number is per machine, not per material. Source: `Tool Database.txt` § Tool Number.

#### Choices and guesses
- **No job material field.** Aspire keeps the active material in the tool database, not in the
  job, so Cut Craft does the same (`ToolDatabase.material`).
  - Also, `Job` is `Copy`: adding a `String` would break more than 26 `..doc.job` call sites in
    other agents' crates.
  - If a per-job material is wanted, the cheapest route is a `Document.material: String` plus a
    `SetMaterial` edit in `change.rs`, and having the toolpath forms pick by it.
- **Data is keyed by material only, not by machine.** Every vendor file has one machine, and its
  name never matches the user's machines. A second machine's data for the same material is
  dropped on import (`ponytail:` in vtdb).
- **[guess]** A thread mill's vtdb `tool_type` number is unknown, so the reader goes by
  `tooth_size > 0`.
- **[guess]** Thread mill profile: the core is taken to reach the tool's bottom.
- The vtdb default is the first material's data; in practice that is the file's only material.

#### Not done
- Toolpath forms still show the tool's feed in mm/min (or in/min in inch jobs) through the
  schema form's `x-unit`, not in the tool's rate units. Only the Tool Database window uses
  rate units.
- No thread mill picture: `tool_database/thread_mill.png` does not exist, so none is drawn. It
  would come from `scripts/build-icons`.
- Multi-point thread mills (threaded length), the tool's own pitch, per-machine data, tool
  number per machine, and renaming or duplicating materials (Duplicate with "Copy Cutting
  Data").
- The remaining thread-milling form fields (pitch etc.) still live on the form, as before.

### 3D Finishing with several tools; Project toolpath onto 3D Model

#### Aspire behaviour found (sources)
- Help "Finish Machining Toolpath": any number of tools in run order; first removes as much as it can,
  later ones only previously unmachined areas; Minimum Detail; Boundary Offset (rest machining).
- Help uiProfileMachineForm / Fluting Toolpath / Texture Toolpath / PhotoVCarveStrategy: "Project
  toolpath onto 3D Model" — after calculating, the toolpath is dropped in Z onto the model; depth below
  the material surface becomes depth below the model surface; limited to the material bottom; only
  available when a 3D model exists.

#### Guesses
- [inf] Names of a several-tool set: "<name> [1]", "<name> [2]"… from the "%s [%d]" string; which code
  uses it was not traced. A single tool keeps the plain name.
- [inf] Projection moves the tool tip (point drop onto the model's cell height), not the whole cutter;
  a wide tool's sides can touch a steep slope. Heights are nearest-cell, not interpolated.
- [inf] Rapids are not projected.

#### Not done
- Profile and Pocket projection (other agent's crates).
- The option is not hidden when there is no model (show-when only looks at string params); running
  with it on and no model gives "there is no 3D model to machine: create a shape first".
- Each toolpath of a set recomputes the rest areas of all the tools before it (O(n²) model builds);
  fine for 2–3 tools.
- Aspire's Machining Limit Boundary, Offset strategy, raster angle and stepover retract for finishing
  are still not done (unchanged).

### 3D view: ViewCube, perspective, tilt, hidden overlays

#### What was built
- **ViewCube** (`view3d::ViewCube`) in the top right of the 3D view. Each face facing the
  viewer is cut in thirds both ways: the middle tile looks from that face, the strips from
  that edge, the corner squares from that corner (26 directions). The tile under the pointer
  is highlighted (in every face that shares it). A click turns the view to look from there, a
  double-click looks from the opposite side, and a drag on it orbits like the view. Face names
  (Top, Bottom, Front, Back, Right, Left) are drawn on the faces that have room.
  The cube's size comes from its longest name in the theme's Small text style plus item
  spacing; its colours come from the theme (inactive widget fill, selection fill,
  noninteractive stroke, text colour).
- **Perspective**: a "Perspective" toggle in the 3D view's button row (beside Top, Front,
  Right, Iso, Fit). It lives in `Framing.perspective`; Fit, the standard view buttons,
  double-click-off-the-model and F keep it (`Framing::fitted`). The eye stands just far
  enough off for the block's bounding sphere to fill a 45° field of view, so turning still
  never changes the block's size and it always fits the view.
- **Tilt past side-on**: pitch now runs from straight down (0) to straight up from below (π),
  so up stays up on screen. `Orbit::facing(direction)` / `Orbit::towards_viewer()` added.
  Seen from below, the material block's bottom is drawn (when the block's sides are).
- **Overlays hidden behind the surface**: vectors, the modeling plane and the origin axes are
  cut a cell at a time and only the parts in sight are drawn (`view3d::seen_of`,
  `view3d::visible_runs`). Works in orthographic and perspective.
- Block sides (and the bottom) are now drawn after the surface rather than sorted in with
  it: the block is all behind its own front faces, so this is exact where the depth sort was
  approximate.

#### Approach and why (no GPU paint callback)
eframe is configured with the **wgpu** renderer (`Cargo.toml`: eframe features `wgpu`), so a
`egui_wgpu` paint callback was possible. I didn't use one. The surface is a height grid, so
whether an overlay point is hidden is answered exactly (to a cell) by stepping a ray from the
point towards the eye over the grid until it is above (or below) everything or off the
material. That needs no depth buffer and no change to the mesh, which stays egui's own. It
also stays testable headless. A paint callback would mean WGSL shaders, a pipeline and depth
texture of its own, the overlays redrawn on the GPU, and no headless test coverage. This is
the `ponytail:` comment on `seen_of`.
Cost: early exit as soon as the ray is above the highest point. The worst case is side-on
over a low surface, about (footprint / half a cell) steps per sample; samples are one per
cell along each overlay. Not profiled in the real app. If it shows up, walk the cells (DDA)
instead of half-cell steps.

#### Aspire behaviour (sources)
- `re/extracted/help-12.5/3d-view-controls.txt`, "View Controller": drag it to rotate
  smoothly; highlight a face and click to move smoothly to face it; double-click a
  highlighted face to rotate to the opposite side. Also "Orthographic or Perspective view:
  toggle between Orthographic and Perspective camera projection". Both are implemented,
  except that the turns are instant, not animated.
- `re/SPEC.md` lines 58–59: the 3D view overlay (`View_Overlay/*.png`) has an
  ortho/perspective toggle, and there is a ViewCube.
- Guessed (not in the help): edge and corner clicks (Aspire's help only mentions faces, but
  its cube has the usual edge/corner highlights; worth a look in Aspire). The 45° field of
  view. A cube click leaves the zoom/pan alone. The cube sits top right.

#### Hardcoded GUI values noticed (for rectification)
- `view3d.rs`: `LIGHT`, `AMBIENT`, `DEPTH_SHADE` (already ponytail-marked), `PICK_REACH`
  4 px, zoom clamp 0.05–200, the tool cylinder's 24 rim points, and my new `FIELD_OF_VIEW`
  (45°; it belongs in Options ▸ 3D View Settings, which I didn't touch because options.rs is
  shared).
- `app/panels.rs` `overlays_3d`: origin axis colours `Color32::RED/GREEN/BLUE`, axis length a
  tenth of the job, axis width twice the stroke.
- `app.rs`: `TOOL_HEIGHT_DIAMETERS` 4, `SETTLE_SECONDS`, `MESH_CELLS` / `FINE_MESH_CELLS`.

#### What's left
- Smooth (animated) turns for cube clicks, as in Aspire.
- The playing tool's cylinder is still drawn whole over the surface (ponytail-marked).
- Overlays are still left off the auto-wrapped rotary view (as before).
- Toolpaths aren't drawn as lines in the 3D view at all (only the preview's cut material),
  so there was nothing to depth-test there. Aspire draws them.
- The overlay toolbar's other toggles (keep-out zones, 2D toolpaths, components, bitmap, plan
  view), zoom to box/selection in 3D, and rotary CW/CCW buttons.
- Perspective's zoom scales the picture (like a longer lens) rather than moving the eye in.
- Not tried by hand: how the cube looks and feels, label fit at other font sizes, and
  side-on overlay speed on a big job.

## Drawing in the 3D view [inf] (help: What's New in Aspire V12.0, "Drawing in the 3D View")

#### Aspire behaviour (sources)
- "The software's drawing tools are now available in the 3D view", so a user can stay in the
  3D view instead of switching views [inf: What's New V12.0]. The help does not say how the
  pointer maps onto the model, nor how the view is turned while a tool is open.

#### What was built, and the choices made
- The open drawing tool takes the 3D view's left button; its gestures are the 2D view's
  (`view2d::gesture_of`), with each screen point taken to where its line of sight first meets
  the model's solid (`view3d::Camera::on_surface`: stepped half a cell, then halved to a hair),
  off the material the top plane (`Camera::on_floor`). [inf] Projecting onto what is under the
  pointer is our guess; it means what is drawn lies on what the user sees, which a novice
  expects, where the top plane would put it millimetres off on a raised or cut surface.
- Right-drag turns the view and middle-drag pans while drawing (the sculpting rule), so the
  left button never does two things; the status bar says so. Double-click does not pick a
  component or fit while drawing (it would fight the polyline's clicks).
- The shape being drawn is drawn on the model in the selection colour, flat at the height a
  finished vector shows (`view3d::vector_plane`), hidden where the model hides it.
- Not offered on the wrapped (rotary) view, while importing a 3D model, or with a view tool on.

#### What's left
- Node editing and the selection's 2D handles in the 3D view; selecting vectors by clicking
  them there.
- Drawing before any component exists (the 3D view shows "Nothing in 3D yet").
- Drawing on the wrapped rotary view.

## Tool databases: `.vtdb` [obs] (vendor files from Amana, IDC Woodcraft and CIC; Aspire 12.5)

- A SQLite file: `tool_geometry` (name_format with `{Tool Type}`, `{Diameter|F}`,
  `{Included Angle|.0}`, `{Units Short}` fields; tool_type; units; diameter; included_angle;
  flat_diameter; num_flutes; tip_radius; thread_pitch; tooth_*; an `outline` blob for form
  tools), `tool_cutting_data` (rate_units, feed_rate, plunge_rate, spindle_speed, stepdown,
  stepover, clear_stepover, thread_*, laser_*, line_width, length_units, tool_number),
  `tool_entity` (geometry × material × machine → cutting data), `machine`, `material`,
  `tool_tree_entry` (groups), `drill_bank_*`, `tool_name_format`, `migration`, `version`.
- **[inf]** tool_type: 0 ball nose, 1 end mill, 2 radiused end mill, 3 V-bit, 4 engraving,
  5 tapered ball nose, 6 drill, 8 form tool, 9 diamond drag (7 unseen). units / length_units:
  0 mm, 1 inches. rate_units: 1 mm/min, 4 in/min (the rate list is mm/sec, mm/min, m/min,
  in/sec, in/min, ft/min). Entries whose cutting data has no feed rate are placeholders.
- `crates/vtdb` reads them into Cut Craft tools (Tool Database ▸ Import).

## Aspire 12.5 trial (12.5.1.1) against 12.010 [obs]

- Help 242 → 248 pages. New: the **Advanced Machining Module** (paid add-on: cabinet
  cut-list import from CSV + DXF parts with a template job, cutter compensation on profile
  toolpaths, Drill Bank toolpath, Print Part Labels with barcodes), a laser sketch-carving form,
  getting-started guides. Help text: `re/extracted/help-12.5/` (gitignored).

### Cut lists and part labels [inf] (Vectric's public Advanced Machining Module product page and help page, V12.5; not tried in the trial)
- **What Vectric documents [inf]:** Cabinet Import brings in parts exported from cabinet
  programs (CabinetParts Pro, CabinetSense, KCD, Mozaik), or others through a Custom Mapping
  File, and nests and toolpaths them from a template job. Labels can be made for each part
  during import, with a barcode, QR code or Data Matrix holding the part's information, in
  one of seven standard sizes for common label printers. The help does not list the label
  fields, the sizes or how printing works.
- **Cut Craft, a deliberate difference:** the same results are reachable without the add-on
  or a cabinet program, in steps a novice can follow, and labels work on any job, not only
  imported ones.
  - Import Cut List reads a plain CSV (what any spreadsheet saves) rather than the cabinet
    programs' formats or a mapping file: columns found by heading (Name, Length, Width,
    Quantity, Material; synonyms such as Qty, Part, Description), or Name, Length, Width,
    Quantity without headings. Each part is a named rectangle; a material picks the layer.
    Nest Parts opens next, set to move the parts, instead of a template job. Mistakes name
    the row and the fix.
  - Parts carry a name (Name Part; kept on the job by vector group, undoable). A part is a
    group or closed vector with what lies inside its box; Nest Parts' own rule (boxes that
    overlap) is not used, so parts nested close together stay apart on the list.
  - The cut list lists length (the longer side) and width (the shorter), so a part turned
    by nesting stays on the same row; grain direction is not kept.
  - Labels: name, "3 of 12", size, sheet, job name and a QR code holding those same lines as
    plain text (any phone camera reads it; no database is needed). Sizes are our choice of
    common rolls: 4 × 6, 4 × 3, 4 × 2, 3 × 2, 2.25 × 1.25 in, 100 × 50 and 62 × 29 mm, plus
    plain paper with cut lines for people without a label printer. Printed through the web
    browser, which every platform has, rather than a print dialog of our own.
  - Engrave Part Names puts the label's number and name on the part itself as Draw Text on
    a Part Names layer, for an engraving toolpath; Vectric's page does not mention this.
- **Platform notes:** Excel on Windows saves "CSV" in the Windows code page, read here as
  Latin-1; European spreadsheets use semicolons and decimal commas, both read. The labels
  page is written to the system temp folder and opened with the platform's opener.

### Drill Bank Toolpath [inf: help 12.5 "Drill Bank Toolpath" and "Post-Processor Editing"]
- A variant of Drilling for a drill bank: "multiple individual drills arranged on horizontal
  and vertical arms" that are engaged (extended) or disengaged (retracted) before plunging.
  Part of the Advanced Machining Module.
- Layout: Horizontal Drills and Vertical Drills (how many on each arm), Central Drill (X, Y)
  (the drill both arms share, by its index on each), Pitch (even spacing). Per drill: Drill
  Number (the controller's; the shared drill may have one on each arm "for machines that
  number the horizontal and vertical drill arrays independently"), Diameter, and Reference
  Drill ("the origin point when positioning the drill bank"; one at a time). Pitch Tolerance
  and Diameter Tolerance are used "when looking for matching drill patterns". Peck drilling,
  retract gap and options as Drilling; Project onto 3D Model; Vector Selection; Name.
- **[inf]** The help shows the layout in the toolpath's own Template Creation form; the
  `.vtdb` has `drill_bank_*` tables, so Aspire may keep banks in the tool database.
  **[dec: clarity]** Cut Craft keeps the bank with the machine (Machine Configuration ▸
  Drill Bank): it is part of the machine, set once, and the toolpath form takes it from the
  active machine each time it opens (editing a toolpath picks up a changed bank).
- **[inf]** The arms are taken to cross in the XY plane (a cross, T or L): horizontal along
  +X, vertical along +Y, all drills pointing down. Not edge (horizontal) boring.
- **[inf]** Matching (the help does not say how): from the first hole not yet drilled, the
  head is placed for each drill that fits it by size, and the placement dropping the most
  drills onto other holes (within the tolerances) is taken. Holes no drill fits stop the
  toolpath, naming their sizes and the bank's (Aspire's behaviour is not known).
- Post (help 12.5): mode by blocks, mutually exclusive: pattern (`DRILL_BANK_PATTERN`, on
  pattern change) or per drill (`DRILL_BANK_EXTEND_DRILL` / `DRILL_BANK_RETRACT_DRILL`, on
  each drill's change). Both need `SETUP_DRILL_BANK` (entering drill bank mode, "at the
  start of every block of drill bank moves") and the final action (leaving it). Missing ones
  or an invalid mix "will prohibit the saving of drill bank toolpaths". The help heads the
  final block `DRILL_BANK_FINAL_ACTION` but its `begin` line is `FINAL_DRILL_BANK_ACTION`;
  Cut Craft reads both.
- Variables: `DRILL [DB]` bitwise OR of the engaged drills' numbers, `DRILL_X [DBX]` /
  `DRILL_Y [DBY]` the same for horizontal-only / vertical-only drills, `DRILL_LIST [DL]` the
  list, formatted `[DL|prefix|separator|first_separator]` (1, 3, 5 with `[DL|D|,| ]` print
  `D1 D3,D5`), `CURRENT_DRILL [CD]` the drill being extended or retracted.
- **[inf]** Cut Craft's reading: `[DBX]` / `[DBY]` are each arm's engaged drills including
  the shared one by its number on that arm; `[DB]` is both ORed; `[DL]` and `[CD]` give the
  shared drill its horizontal number. New banks are numbered 1, 2, 4, 8... so the OR is a
  bitmask. The pattern or extends are written before the hit's move to the head's position
  at safe Z; per-drill posts retract every drill before the final action. The G-code's X / Y
  are the reference drill's position.
- **[inf]** Cut Craft's toolpath drills each hole in its moves (so the preview, simulation and
  checks show every hole) and keeps each plunge of the head with them; the post writes one
  plunge per hit.

## Open questions (need trial testing, the docs or sample files)
1. `.v3m` byte layout, and the rest of `.crv3d` (see Job files). `.3dClip` (the 22 sculpting
   brushes the installer puts in `Public/Documents/Vectric Files`) is **not** a compound file:
   a plain header (u16s 2, 1, 1; the GUID `7d82aea4-9169-f743-a375-09ddf91359e7` in every
   file; UTF-16 "Aspire" (the creating application), "DLL", then the exported component's
   name, e.g. "Clouds - Baked") and then 7.99 bits/byte, so the height data is encrypted
   and is left alone.
2. Meanings of the `Default.prf` fields.
3. Which mouse button orbits the 3D view (the help pages contradict each other).
4. The commands behind keys 1, 2 and Ctrl+F1…F9.
5. Status bar contents.
6. Toolpath algorithms (V-carve, pocketing, 3D raster). Write our own; the docs describe only parameters and results.

### Toolpath groups [obs: Toolpath Groups Guide, Save Toolpaths FAQ]
- The toolpath list's right-click menu groups toolpaths: Group Visible (a new group of the shown toolpaths, named Group 1, Group 2…), Create Empty Group, Rename, and Ungroup or Delete (asked whether the group's toolpaths go too). A toolpath is dragged into a group; a toolpath made while a group is highlighted joins it.
- Clicking a group's name picks it: Preview previews its toolpaths in list order, and Save Toolpaths' Selected mode saves them as one file named after the group. Toolpath templates keep the groups.
- **[inf]** Cut Craft: Group ▸ Group This / Group Visible / Create Empty Group / Move to Group / Remove from Group on a toolpath's menu, and a New Group button over the list, stand in for dragging; a group's menu has Rename, Ungroup and Delete Group and Its Toolpaths. The group list is kept with the job; a template's toolpaths bring their group names in.

### Freehand Drawing [obs: help Freehand Draw; course qjJXVHB0VM4]
- A left drag draws; the path follows the cross-hair's centre. **Smoothness**: a slider, less to the left (angles kept, more nodes) and more to the right (curves, fewer nodes), applied when the button is released.
- **Stabilise**: a slider; some stabilisation draws a circle round the path's end that follows it. Pointer moves inside the circle do not extend the path; at its edge the path moves toward the pointer, so a sharp turn is made by going back inside and off another way. **End Vector at Cursor**: the end point at the cross-hair rather than the circle's centre when the button is released.
- Tab while drawing closes the path to its start. Ctrl-drag from an open vector's end extends it; Ctrl held over another open vector's end on release joins to it. Fade selected Bitmap dims a selected picture to trace over.
- **[inf]** Cut Craft: Smoothing as a tolerance in mm, Stabilize as the circle's radius in mm; the end moves only when the pointer is outside the circle, to a radius short of it. Not built: Tab to close, the Ctrl extend and join, the bitmap fade.

### Orientate 3D Model in a rotary job [obs: help Importing a 3D Model into Rotary job; course -rx-oCb3Crw]
- **Interactive Rotation**: XYZ-View (the mouse turns the view, the model stays) or, under Model, XYZ (all three axes at once), X, Y or Z (that axis only, the view switching to look along it). A new initial orientation cancels these turns.
- **Rotation Axis Movement** (full 3D model): the model is centred on the rotation axis by default; On, the 3D view's panning moves the rotation axis instead of the view.
- **Model Scaling**: the bounding cylinder (blue) sized by length and diameter, units, lock ratio, scale to fit, reset, resize material block.
- **[inf]** Cut Craft: Drag turns (XYZ view / XYZ model / X / Y / Z) with the turn in degrees shown and typed as X, Y, Z: a turning drag adds to them, the view's height a half turn (XYZ model: sideways about Z, up and down about X). Drag moves the axis with Across and Up in mm: a pan drag moves the model, the view's height about two diameters. The fine turn is applied after the initial orientation (about X, then Y, then Z). Not built: the view swinging to look along the chosen axis, cancelling the turns on a new initial orientation.

### Text spacing in the 2D view [obs: course R2nhSGOSaoU]
- With the text form open, the pointer between two letters shows arrows pointing together: a click brings the letters closer; Shift (arrows apart) moves them apart; Ctrl makes the step bigger; Alt applies the step to every pair of the text. Between two lines, up and down arrows: a click brings the lines together, Shift apart, Ctrl by more.
- Right-click ▸ Text ▸ Break Block into Lines splits a block into one text per line, each selectable and editable on its own (then arced with its own handles). Kerning and editing keep an arc.
- **[inf]** Cut Craft: the step between letters is a twentieth of the text height, between lines a tenth of the font's line pitch, Ctrl four times either; the spacing is kept with the text's settings, so editing the text keeps it, and Break Block into Lines carries each line's kerning. A click on no gap places the anchor (as the form says). Not built: the arrow cursors.

### Lithophane preview [obs: help Preview Toolpaths; course RJogq_22TBs]
- Preview Toolpaths ▸ Machined Area Color ▸ Lithophane: the material shown as if lit from behind, white at zero thickness and the chosen material or solid colour at full thickness, whatever material is picked; a slider beside it goes from very high contrast (left) to a much lighter look, as with no backlight (right).
- **[inf]** Cut Craft: Toolpaths ▸ Materials ▸ Machined Area Color ▸ Lithophane, with Lithophane light 0 to 1 (0.5 to start). Each cell's colour mixes white and the material's colour by the share of the thickness left, raised to 4^(2 × light − 1): 0 darkens soon, 1 stays light.

### Re-Import [obs: help Import a Component or 3D Model]
- Once imported, a model can be re-imported to adjust its size, position and slicing plane, "provided it has not been changed significantly": right-click one of its components in the tree ▸ Re-Import. A model that was segmented reopens the 3D Segment dialog in its last state instead.
- **[inf]** Cut Craft: Re-import Model… on an imported component's menu reopens Orientate 3D Model with the import's settings, from the file where it was (or beside the job, once moved with it); a changed file is imported again as it is now, with a warning. Not built: reopening Segment 3D Model for a segmented model.

### Plate Production toolpaths [obs: help Plate Production]
- With toolpaths calculated for the master template, Create Toolpaths (on by default then) calculates toolpaths for every plate: one toolpath per operation per sheet of material, named "S1 - Name" after the sheet and the template's toolpath. Unticked, only the vectors are made.
- **[inf]** Cut Craft: each template toolpath (one whose vectors are in the template) is made again per sheet with its moves shifted onto each plate of that sheet, keeping its strategy, settings and tool, its vectors the plates' copies; a recalculation runs the strategy on those copies.

### Texture Area, live [obs: help Create Texture Area]
- A texture area component remembers it is one: selecting it and opening Create Texture Area (or opening the form, then selecting it) edits it with the form; Edit Textured Area Component drags a transform box on the lower-left tile to resize or move the tile, the pattern following. Resizing the component resizes the boundary tiled, not the tiles. Changing the border shape means starting again.
- **[inf]** Cut Craft: the component keeps its hidden seed and settings (and a 3D clipart made of it carries them); the form opens on a selected texture area with its settings and Apply replaces it over its own box; a move, scale, rotate, mirror or align of a texture area tiles its new box again with the tiles unchanged. Not built: resizing the tile by handles in the 2D view.

### Node editing mirror lines [obs: course jzW6RGG8wWk]
- In node editing, with several nodes box-picked (on one vector or several selected ones), H shows a line through the middle for horizontal mirroring and V one for vertical; the arrow keys then move the picked nodes together, the two sides moving opposite ways so they stay mirrored. Escape leaves node editing.
- **[inf]** Cut Craft: the lines run through the selection's middle; each toggles with its key while nodes are picked (else H and V mirror the selection as before), the arrows move the picked nodes by the nudge step (Shift, Ctrl as for the selection) with the far side of a line moving the other way; entering or leaving node editing clears the lines.

### Toolpath Summary's Machine Acceleration [obs: help Estimating Machining Times; course WjTQ9RIvaKI]
- The estimate is each move at its feed, rapids at the Rapid Rate, times the Scale Factor. The Machine Acceleration option picks a machine and its X / Y / Z / A accelerations (kept in Machine Configuration), and the time then accounts for accelerating and decelerating at corners.
- **[inf]** Cut Craft: a Machine acceleration tick, live while a machine with accelerations is active: the machine stops at each corner (a turn of more than 5° or a change of move kind runs through at the slower feed), and each move speeds up and slows down along what the slowest axis it uses allows (a short move peaks below the feed). No A axis.

### Vector Validator's V-Carving Mode [obs: help Vector Validator]
- Checked, the validator checks as V-carving needs: it ignores intersections within text, which may come from the font (solder the text and validate the result if its intersections matter).
- **[inf]** Cut Craft: crossings and overlaps between (or within) the letters of one text are let be; open vectors are found too, since V-carving needs closed ones.

### Angle and Arc Dimensions, editing [obs: help Dimensions]
- Angle Dimension: click the centre (a corner), then the two points of the sweep, then where the dashed dimension line goes, then the text; arrow heads pop outside small angles, extension lines where needed; its own Decimal Places (up to six). Arc Dimension: the first click must be on an arc span (not a Bézier), the second places the text showing the radius, or the diameter with Show Diameter; circles are recognised. Shift while selecting a dimension with the form open edits it; Ctrl+Z undoes dimensions while in the tool.
- **[inf]** Cut Craft: one drag does it: Angle from the corner (the two straight sides meeting nearest the drag's start, within 2 mm) to where the arc goes, the text outside the arc's middle, the arms drawn out to the arc; Arc from a point on an arc or circle to where the text goes, with a leader and arrow to the arc. The dimension's settings stay with its group, so Shift-click opens them in the form and Apply replaces it. No separate angle decimal places, no dashed lines, no leader snapping.

### Guide Properties and angled guides [obs: manual, Rulers, Guidelines and Snapping; course lJUAcgT77eA]
- Guides come off the rulers (snapping to the ruler's units unless Shift is held), are moved by dragging or by typing a New Position in Guide Properties (right-click the guide). New Angle, typed or by slider, turns a guide (degrees anticlockwise from the X axis); an angled guide gives only relative parallel guides. Lock Guide stops dragging. Create New Guide adds guides at absolute or incremental positions; Ctrl-drag from a guide copies it (the distance shown). Hovering shows a guide's position. Guides toggle with the ruler corner or View ▸ Guide Lines.
- **[inf]** Cut Craft: a guide turns about where it crosses its axis (its position); the 2D view's right-click menu on a guide has Guide Properties…, Lock / Unlock Guide and Delete Guide, and elsewhere Guidelines ▸ Insert Horizontal / Vertical / Angled Guide (the angled one at 45° through the click, its properties opened). Angled guides are drawn far past the view, dragged through the pointer keeping their angle, and snap the pointer onto them. Not built: Ctrl-drag copies, the position at the cursor, ruler-unit snapping of a dragged guide.

### Sculpting custom brushes [obs: help Sculpting; course IHPQ65Rn0YM]
- In Deposit or Remove, a custom brush made from a model (a .3dclip, .v3m or bitmap) can replace the round brush, leaving the impression of pushing the model through the surface. A component's right-click Create Sculpting Brush makes one, optimised and put in the brushes folder; Load Brush picks one. Angle (slider, or R held with the wheel) turns it; Dynamic Rotation turns it to follow the stroke, an arrow showing its turn. Merge: a stroke's result merges with the strokes before it instead of being added onto them. A click stamps, a drag strokes; H or V held keeps a stroke straight.
- **[inf]** Cut Craft: a brush is a component saved as Cut Craft 3D clipart in the brushes folder; its heights count 0 (lowest) to 1 (highest), its larger side spans the brush diameter, and at full strength it presses a fifth of the diameter deep. A stroke's impression is the highest of its dabs; added, it goes onto the surface; merged, it is laid on the surface from before sculpting and the higher (Remove: the lower) kept. Not built: R + wheel, the arrow and ghost, bitmap brushes.

### Text spell checker [obs: help Create Text; course admEKCLmzX8]
- The text box underlines misspelled words in red; clicking one offers corrections, Add Word, and Remove Word (only for words the user added). The language is the software's.
- **[inf]** Cut Craft: Hunspell dictionaries (one-letter flags; PFX and SFX rules with conditions and cross products) from Cut Craft's `dictionaries` folder, else the system's (`/usr/share/hunspell`, `/usr/share/myspell`, `/Library/Spelling`; English first). The misspelled words are listed under the Draw Text form, each with up to six suggestions one edit away (letters from the dictionary's TRY line) and Add Word; user words live in `words.txt` in the settings folder. `!!variables!!` and words with digits are let be. Not built: underlining in the text box, compound rules, long or numeric flags.

### Copy Along Vectors: Copy Circles [obs: help Copy Along Vectors]
- Copy Object copies the first vector or group selected along the others; Copy Circles copies circles of the diameter entered along the selected vectors.
- **[inf]** Cut Craft: with Copy Circles every selected vector is one to copy along, the circles centred on the points the spacing gives, on a new layer when asked.

### Space Selection inside the last vector [obs: help Alignment Tools]
- Space Selection spreads the objects evenly, the outermost staying put; with Ctrl the order of selection counts; with Inside last vector the items are spread within the bounds of the last object selected.
- **[inf]** Cut Craft: inside the last vector, the gaps at its two edges equal the gaps between the objects; the last vector does not move.

### Text arching [obs: help Text Selection; courses R2nhSGOSaoU, XJ8fKNIIbTg]
- With text selected for editing, two green handles sit in the middle of the text: drag the bottom one to arc the text down, the top one to arc it up; drag back to flatten. Arced text then shows red handles (rotate about the arc's centre, Ctrl in 15° steps), blue handles (change the radius, the centre kept) and white handles (move: one in the text's middle, one at the arc's centre). Text placed on a curve has no arc handles.
- **[inf]** Cut Craft: Draw Text keeps Arc angle (degrees the line turns through; above zero bends up) and Arc rotation (about the arc's centre). The middle of the text stays put; each letter stays whole, its middle moved onto a circle round the centre and turned to face out (bent down: in). The handles sit a quarter of the text height above and below the unbent text's middle; a drag sets the angle so the text's middle rises (or falls) as far as the pointer moved, from what it rose before. Not built: the red, blue and white handles; kerning clicks on arched text.

### Text in a Box stretch [obs: help Create Text]
- Vertical Stretch (text that fits the width, room above and below): none, Stretch Line Space to fit, Stretch Characters to fit. Horizontal Stretch (text that fits the height, room at the sides): none, Stretch Spaces between words, Stretch Kerning, Stretch Character size.
- **[inf]** Cut Craft: line space grows the line pitch until the block's ink fills the box height (one line: no change); characters scale the block in Y only; spaces and kerning share the room each line lacks equally among its spaces or among all its letter gaps (by ink, so each line reaches the box width); character size scales in X only. The margin still applies to the width.

### Create Shape: Preserve Internal Corners, Blend To Inner Vectors [obs: help Create Round Shape, Create Custom Shape]
- Preserve Internal Corners: inside corners rise as straight, mitred faces meeting in a sharp crease (as a roof's valley), not curving in to a point. Blend To Inner Vectors: the profile is applied from the outside vectors to the inner vectors.
- **[inf]** Cut Craft: a cell whose nearest point on an outline is a corner (inside the shape only an inside corner can be) takes the distance to the farther of the two sides' lines, not to the corner. Blending, vectors inside another are the top (flat at the full rise); between, a cell's distance in is its share of the way from the outer edge to the inner vectors (outer / (outer + inner)) times the widest gap between them. The shape keeps its settings and outlines with the component, so E opens the form on it and a change remakes it in place, keeping its level, group, colour and component properties.

### Trace Bitmap area and colour picking [obs: help Trace Bitmap]
- Dragging over the selected bitmap defines a rectangle (dashed black) traced alone; clicking the bitmap again removes it. Colours can be selected directly from the image in the 2D view.
- **[inf]** Cut Craft: a pixel is traced only when its centre is inside the box. A click, with a box, clears the box; without one, in Colors mode, the colour of the pixel clicked (its number in the cut-down palette) is added to Colors to trace, or taken out if it is there. Not built: the swatches with tick boxes and linked colours.

### Sketch Carving highlight [obs: help Sketch Carving]
- The green highlighted area shows what the toolpath will cut, before it is calculated.
- **[inf]** Cut Craft: each row's run of line cells inside the boundary, filled translucent green while the form is open, found again when the settings, the job or the selection change.

### Segment 3D Model: tree and layout [obs: help Segment 3D Model]
- Each Segment makes two children of the current segment (above and below the plane); any segment can be picked and cut again; Delete children joins them back; segments dragged onto each other merge. Layout mode cycles each segment's orientation with arrows (O shows a rotation handle) and can highlight undercuts in pink. Import Options: Create Backface Caps, Resize Material To Fit Slices, Create boundary vectors.
- **[inf]** Cut Craft: Cut Segments Again lists cuts by segment name: cutting segment N gives N.1 (beyond the plane, toward the axis's positive side) and N.2, each nameable by a later cut; the plane's position is a share of that segment's own size. Segment Orientations sets a face up per segment (top, bottom, front −Y, back +Y, left −X, right +X); segments not listed follow Turn the second segment over. Not built: the plane's 3D handles, merging, the O rotation handle, pink undercuts (a note names them), the Import Options.

### Imported models: steep sides [inf]
- Cut Craft: top-down rasterising, a cell whose highest face is near vertical (its area seen from above under a tenth of its own area) takes the height of the non-steep top next to it, spreading up to 4 cells in, so near-vertical sides leave no ring of low cells. A wider steep band keeps its slope.

### 3D view: the model solid [obs: Matthew, 2026-11-01]
- The model should look solid down to the modeling plane (to the axis on a rotary job), not a skin.
- **[inf]** Cut Craft: walls hang from the edge of the drawn surface (a quad edge with a quad on one side only) down to z = 0, with a bottom there; wrapped, down to the axis with no bottom. GPU only.

### Checks before saving toolpaths [inf: Cut Craft's own; Aspire makes none of these]
- Aspire saves whatever is asked and leaves crashes and air cutting to the preview. Cut Craft checks the toolpaths being saved first, simulating them in cutting order (file by file, in list order) from uncut material, as a novice cannot be expected to spot a rapid move into the material in a preview. A deliberate difference from Aspire.
- **Errors stop the save**, with the fix in the message: a rapid move more than 0.5 mm into material still there (the cells are coarser than the cut, so a skim along a fresh wall is not one); a toolpath with no moves; no feed rate (with cutting moves) or plunge rate (with plunges); toolpaths covering more than the machine's bed (rotary jobs aside) or spanning more than its Z travel.
- **Warnings** are asked about (or told in the status bar, per Options ▸ checks before cutting): air cutting, where 30% and 100 mm or more of a toolpath's cutting takes nothing off (the tool 0.5 mm clear of everything, or where an earlier toolpath already cut; a toolpath's own neighbouring rows do not count, as cells are coarser than a finishing stepover), with the time it wastes; an out-of-date toolpath; no spindle speed (not lasers); a spindle speed outside the machine's range; a feed or plunge rate over the machine's max rate; a tool not loaded in the tool changer; a cut over 1 mm below the material; and the flute-length and taper checks.
- The checks are listed in the Save Toolpaths form as Must fix / Check lines, worked out again when the plan, the job or the machine changes; Save is greyed while there is a Must fix.
- **Tool Changes** (Cut Craft's own; Aspire leaves it to the post): **Separate File for Each Tool** splits the plan wherever the tool changes into files run in order, named after the tool, with no tool changer call (M6) for the first tool on a machine without a tool changer; it is the default without one, and the only way with a post without a TOOLCHANGE block (Aspire refuses that save instead [inf]). **Tool Changer (ATC)**: one file through the post's TOOLCHANGE; the default when Machine Configuration has tool changer slots, greyed without. **One File, Pause to Change the Tool**: the post's TOOLCHANGE when it already stops (M0/M1, no M6), else lift to safe Z, M5, a note naming the tool, M0, spindle on, a 3 s dwell; the header's M6 becomes a note to fit the first tool. Its hint warns that GRBL cannot jog while stopped.
- **Merge Toolpaths in a Row That Use the Same Tool** (Aspire's Group Where Possible, renamed): only toolpaths next to each other in the list share a file; the cutting order never changes (Matthew, 2026-11-04). In one file, toolpaths in a row with one tool never get a tool change between them.
- Not checked yet: the tool holder or collet hitting the material (no holder sizes in the tool database), and moves past the machine's soft limits from the job's datum (the machine's home position is not known).
