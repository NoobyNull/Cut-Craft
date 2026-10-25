# Aspire 12.0 — clean-room behaviour spec

Static analysis of the installed Vectric Aspire 12.0.0.9 (`~/Desktop/Aspire 12.0/`). No Aspire binary was run.
This file describes **behaviour** to reimplement. Don't copy Vectric code, icons, `.vfont` fonts, help text or `.pp`
files into this repo. Write our own implementations from this description.

Confidence tags: **[obs]** read directly from a shipped file · **[dec]** read from `Aspire.exe`'s data or decompiled code (Ghidra, `re/ghidra_scripts/`) · **[inf]** inferred from several observations · **[unk]** not established.

## Sources

| Source | What it gave | How |
|---|---|---|
| `Language/ENU/x64/Resources_ENU.dll` | 39 menus, 1 accelerator table (64 keys), 20 toolbars, 885 strings, 273 dialogs, 107 `RT_XAML` | REA `inspect_pe_resources` (Evidence `ev_cf7650e8ff7c8f62e4c5ef0c002cedfe5839fbba3f5973766872a107771200ca`) → `re/peres.py` decodes payloads |
| `Language/Lang/*/LC_MESSAGES/*.mo` | English UI strings split into 35 subsystem catalogs | `re/mo.py` (stdlib gettext) |
| `Language/ENU/Help/{form,page}/*/index.html` | 241 help topics = full feature list and behaviour | `re/help2txt.py` |
| `x64/`, `Language/ENU/Help/third-party/` | dependency and architecture map | file inventory |
| `x64/Aspire.exe` | the post-processor vocabulary table, unit labels, date formats, form names (MFC runtime-class records). **Its code section is encrypted on disk** (7.99 bits/byte; most of the decompile is Ghidra decoding encrypted bytes), so only data is read statically. | Ghidra 12.1.2 project `re/extracted/ghidra/` (gitignored, about 2 h to analyse); `re/ghidra_scripts/` decompiles one function, finds a string's users, or dumps everything (`dump-decompiled`, about 50 min on 12 processes) |

Regenerate the raw dumps:
```bash
python3 -I re/mo.py "$HOME/Desktop/Aspire 12.0/Language/Lang/de_DE/LC_MESSAGES" /tmp/mo
python3 -I re/help2txt.py "$HOME/Desktop/Aspire 12.0/Language/ENU/Help" /tmp/help
# peres.py needs the JSON from REA inspect_pe_resources on Resources_ENU.dll:
python3 -I re/peres.py rea.json "$HOME/Desktop/Aspire 12.0/Language/ENU/x64/Resources_ENU.dll" /tmp/res
```

## How the original is built (reference only)

- **[obs]** UI: MFC + Codejock Xtreme Toolkit Pro (`XTP_*` resource names) and embedded XAML for newer forms. The same resource DLL is shared with VCarve Pro / ShopBot builds, and product edition decides which menu items appear.
- **[obs]** 3D: OpenSceneGraph (`osg*.dll`), with a Mesa llvmpipe software-OpenGL fallback (`x64/swrndr/opengl32.dll`). `VectricGLDetector.exe` probes the GPU.
- **[obs]** I/O libraries: ODA (DXF/DWG; `IoOda`, `Oda` catalogs), pstill (EPS/AI/PDF; `IoPScript`), GraphicsMagick (bitmaps; `IoMagick`), OpenNURBS (`.3dm`), SketchUp SDK (`.skp`).
- **[obs]** Scripting: Lua + LuaBind, used for Gadgets and Lua-based post-processors (`Lua/ppVariables.lua`).
- **[obs]** Cloud and support: curl/OpenSSL/JsonCpp (clipart library, machine database, licensing, makerspace accounts) and BugSplat crash reporting.
- **[inf]** Subsystem split, from `.mo` catalog names: `App`/`AppCmd` commands, `Io*` file I/O, `Vd` vector geometry, `Cm` layers, `Tri*` triangle meshes, `Mc*` machining and post-processing, `Nest` nesting, `Sd` two-sided jobs, `V3d` 3D view, `UiP`/`VfUi`/`Uvs*` UI forms.

Our equivalents so far: wgpu instead of OSG/OpenGL, winit instead of MFC. Every dependency is still open below.

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

### Startup sequence [obs] (help: how-to-get-started, create-a-new-file, Job Setup Single; MENU 128; screenshots)
- **Start page, no job open:** the left panel holds *Startup Tasks*: Create a new file, New file from template, Open an existing file. Below them is *Recently opened files*, a rolling list of the last 4 jobs, empty on a first run. The rest of the panel is vendor links (tutorial videos, online resources, social media, clipart shop), which are out of scope. The centre shows the product splash. There are no Drawing or Toolpaths panels, and no 2D/3D view.
- **No-job menu (MENU 128):** File (New, New from template, Open, Open Application Data Folder, Recent files, Close, Exit), Edit (Options only), Machine (as with a job), Toolpaths (Materials, Tool Database), Gadgets, Help. Model and View are absent.
- **Create a new file → Job Setup:** the form fills the left panel, with OK and Cancel at the bottom. The 2D view (rulers, zoom buttons) appears beside it, and the full menu bar is back. Sections, top down: Job Type, Job Size + Units, Z Zero Position, XY Datum Position (+ Use Offset X/Y), Modeling Resolution, Material Settings.
- **OK** creates the new empty job: a grey rectangle in the 2D view, with dotted lines through X0 and Y0. **[inf]** Cancel abandons the new job and goes back to the start page; File ▸ Close closes the job and goes back to the start page too.
- **Open an existing file** takes native jobs and importable 2D vector files (DXF, EPS, AI, PDF). Opening a 2D file starts a job at the drawing's size (see Import).

### Right-click menus [obs] (popup MENU resources in `Resources_ENU.dll`)
- **Families that repeat across kinds of item:**
  - **Show / Hide:** Show This, Show Only This, Show All But This, Show All; Hide This, Hide All. On layers (967), toolpaths (968, 669; toolpaths also have Show All With This Tool), components (613) and levels (1046).
  - **Delete:** This, Visible, Invisible, Empty, All. On layers and toolpaths.
  - **Combine Mode:** Add, Subtract, Merge, Low, Multiply. On a component (613), several components (615), the active component (616), a component in the 3D view (619) and levels (1046).
  - **Rename, Properties:** on every kind.
- **Layers (967, 982):** Activate, Show / Hide, Lock, Unlock, Insert New Layer, Delete (This, Visible, Invisible, Empty), Rename, Merge Visible, Select Layer Vectors. With no layer under the pointer: New Layer.
- **Toolpaths (968):** Show / Hide, Activate Sheet, Edit, Rename, Duplicate, Recalculate (This, Visible, All), Create an Empty Group, Group Visible, Ungroup, Delete (This, All Invisible, All Visible, All).
- **Components (613, 614, 615):**
  - On a component: Combine Mode, 2D preview order, Export as 3D Clipart, Save as Sculpting Brush, Re-Import, Copy, Duplicate, Copy / Move to other side, Ungroup, Delete, Rename, Show / Hide, Properties.
  - With no component under the pointer: Paste, Import, Show All, Hide All.
- **Levels (1046, 1048):** Combine Mode, Show / Hide, Select Components, Clipping, Mirror Mode, Wrapping, Insert New Level, Delete Level, Rename Level, Export as 3D Clipart.
- **2D selection (318):** Undo, Redo, Cut, Copy, Paste, Delete, Move to Front / Back, Ungroup Component, Object Properties.
- **Node editing (142, 317, 635, 636, 245):** span to Line / Bezier / Arc, insert / delete / smooth point, cut vector, make start point, mirror, close or join with a line / curve / moved end points, reverse direction, exit node editing.
- **Also:** sheets (5030), post-processor list (5093: make default), clipart library (1050).

### Menus [obs] (resource MENU 129 = job open, 128 = no job)
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
- **Help:** contents, shortcuts, tutorials, licence, updates, about. Licensing and vendor web links are out of scope.

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

- **[obs]** The format is versioned. The loader rejects "written by an unsupported later version", can save down to 11.0, and checks trial-version and machine binding (`Io.mo`).
- **[unk]** Internal byte layout. There are no sample files in the install, and reversing it would mean decompiling `Aspire.exe` (80 MB).
- **Recommendation:** use our own serde/JSON job format (serde_json is already a dependency) and treat `.crv3d` import as a much later stretch goal.

### Import [obs]
- **2D vectors:** DXF/DWG, EPS, AI, PDF. Vectors keep their original size and scale. Opening (not importing) a DXF/EPS/AI opens Job Setup pre-filled with the design extents, and EPS/AI are placed with their lower-left corner at X0,Y0. Each page of a multi-page PDF becomes its own layer. When the result is empty, the importer offers to check the file and retry.
- **Bitmaps:** BMP, JPG, TIF, GIF, PNG. Used to trace over, auto-trace into vectors, or turn into a 3D component (greyscale → height).
- **3D:** CRV3D, 3DCLIP, V3M (placed at their saved size and position), plus STL, 3D-mesh DXF, 3DS, 3DM (render meshes only), SKP. Third-party meshes open an **Orientate 3D Model** step first (orientation, scaling, then conversion to a component).
- **[unk]** SVG import: SVG appears under Export, but the import help page doesn't list it.

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

### Create Fillets [inf] (no help text read yet; general CNC practice)
- Works on the sharp corners where two straight sides meet; corners touching curves are left as they are.
- **Round:** an arc of the given radius tangent to both sides. **Dog-Bone:** a circle the cutter's radius, centred on the line halving the corner and passing through it, so a round cutter clears the corner fully. **T-Bone:** the same circle, notched into the longer side instead.
- **Corners:** All, Outer (pointing out of a closed shape: what a cutter inside a pocket cannot reach) or Inner (pointing in: what a cutter outside the shape cannot reach). Open vectors have no inside and get every corner.
- A fillet may use at most half of each side it touches, so neighbouring fillets never overlap; corners that need more are left sharp. Aspire's tool is clicked corner by corner; ours does every corner of the selection at once.

### Array Copy and Circular Copy of vectors [inf] (modelled on Array Copy toolpath above)
- **Array Copy:** Rows (Y) × Columns (X), the original's row and column included, placed above and to the right of the selection. Spacing by **Gap** (between neighbouring copies' extents) or **Offset** (start to start).
- **Circular Copy:** a number of items in all, the original included, round the **job centre** or a typed point, spread counter-clockwise over an angle (360° spaces them evenly; less puts the last one at the end of the angle). Copies turn with the circle unless **Rotate copies** is off, when they only move.
- Copies stay on their originals' layers and the originals and copies end up selected.

### Close / Extend, Vector Validator, Copy Along Vectors [inf]
- **Close / Extend Vector:** closes the selected open vectors with a straight line from end to start, or **Extend** runs the first and last lines on until they meet (a closing line when they never do). **Max gap** leaves vectors whose ends are further apart.
- **Vector Validator:** checks the selection, or every editable vector when nothing is selected, for open vectors, closed ones crossing themselves, vectors on top of another (either direction) and vectors or spans shorter than a tolerance, and selects the ones that fail. Aspire's validator also marks the trouble spots; ours selects whole vectors.
- **Copy Along Vectors:** the last vector picked is the path; the objects picked before it are copied along it, a number of copies spread end to end (all round when it is closed) or one every so far from its start, turned to follow the path as if facing +X, or not.

### 3D components [obs]
- **Component tree:** the composite model is built from the **bottom of the tree upwards**. Combine modes are **Add / Subtract / Merge / Multiply**. Each component belongs to one **Level**: components inside a level combine first, then levels combine with each other. Per-component settings include combine mode, Fade, Tilt and Appearance (colour or material). Blue arrows reorder components, and double-clicking a component opens Component Properties.
- **Create Shape** (from closed vectors): Round, Angled, Flat, Custom and Smooth profiles. The other creation tools are in the Model menu: Two Rail Sweep, Extrude & Weave, Turn & Spin, Texture Area, Zero Plane and From Bitmap.
- **Modify:** Sculpt, Smooth, Emboss, Distort, Offset Model, Slice, Add Draft, Scale Z Height, Replace Below, Split, Bake, Segment 3D Model.

### Toolpaths (beyond P4, listed for completeness) [obs]
- **Strategies:** Profile, Pocket, Drilling, V-Carve, Quick Engrave (Ctrl+Q), Inlay / V-Carve Inlay, Fluting, Prism, Chamfer, Keyhole, Thread Milling, Drag Knife, Moulding/Extruded, Texture, Rough and Finish Machining (3D), Rounding, PhotoVCarve, Sketch Carving, Laser Cut/Fill and Laser Picture, Array Copy, Merged.
- **Support:** Tool Database (with remote databases), Materials, Templates, Tabs, Keep-Out Zones, Toolpath Tiling, Preview (simulation), Job Sheet, Summary.

### Tool database: feeds and speeds [obs] (help: Tool Database)
- **Tool types:** V-Bit, Engraving, Tapered Ball Nose, Ball Nose, End Mill, Radiused End Mill (a toroidal or bull-nose cutter: flat bottom, rounded corners of a tip radius), Form Cutters (profile imported from vectors), Diamond Drag, Drills.
- **Geometry vs cutting data:** a tool's geometry (diameter, tip radius, angle…) is one record; its cutting data (pass depth, stepover, spindle speed, feed rate, plunge rate) is kept **per machine and per material**, and the active machine / material picks which set a toolpath gets. The tool number is per machine only. Copying a tool copies its geometry without cutting data.
- **Fields:** Number of Flutes; Pass Depth (sets the number of Z passes, e.g. 25.4 deep at 6.35 per pass = 4 passes); Stepover (stepovers over 50 % add "tail" moves in corners of offset strategies; V-bits have Final Pass and Clearance Pass stepovers instead); Spindle Speed in rpm; Feed Rate and Plunge Rate (plunge also used for ramping); Maximum Burn Rate (lasers, simulation only); Notes.
- **Rate units** are chosen per tool: distance per second or per minute. **[dec]** The labels offered are mm/sec, mm/min, m/min, inches/sec, inches/min and feet/min.
- **Chip Load** is shown, calculated from flutes, spindle speed and feed rate (feed per tooth: feed ÷ (rpm × flutes)), to compare with the maker's recommendation.
- **Radiused End Mill [obs]** has a Tip Radius (its name template offers `{Tip Radius}` and `{Flat Diameter}`). **[inf]** The shape is a torus: flat across `diameter − 2 × tip radius`, then a quarter circle of the tip radius up to the full diameter, so its edge rises `tip − √(tip² − d²)` at `d` past the flat.

### 3D Finishing: several tools (rest machining) [obs] (help: Finish Machining Toolpath)
- Any number of tools can be chosen, in the order they will run. The first removes as much as it can; each later one removes only material the earlier ones left ("previously unmachined areas"). An allowance may be left for the last tool.
- **Minimum Detail** (value and slider): unmachined areas smaller than this are ignored by that tool — a trade between detail and machining time.
- **Boundary Offset**: every unmachined area is grown by this before it is machined; small values give many small areas, large values fewer, larger ones.
- Finishing is always one pass at full depth (it ignores pass depth); the usual tool is a ball nose with an 8–12 % stepover.

### Fluting [obs] (help: Fluting Toolpath; `Aspire.exe` strings)
- Cuts along each selected vector from its start point. **Start Depth** is where it begins (0 at the surface, or a pocket floor); **Flute Depth** is added below it.
- **Flute Type:** ramp over the complete length (start depth at the start → full depth at the end); ramp at start (then full depth to the end); ramp at start and end.
- **Ramp Length** is capped at the longest possible ramp (the vector's length, or half of it for start and end). **Ramp %** is a percentage of that longest ramp; **[dec]** it must be greater than 0 and less than 100 ("Ramp % is invalid").
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

### Quick Engrave [obs] (help: Quick Engrave; dialog strings in `Resources_ENU.dll`)
- Ctrl+Q. Depth to engrave (a diamond drag tool uses a Pressure instead, its preview depth from its angle and line width). Stepover is set on the form, not from the tool database.
- Strategy: **Outline** (the tip runs on the lines) or **Fill**, with the patterns **Offset, Hatch, Cross Hatch**. **Use Nose Cone** (the cutter sticks out of a sprung cone by the depth; the preview uses the depth as given). **Number of Passes** runs the pattern several times. Immediate Output saves or sends the toolpath from the form; quick engrave toolpaths cannot be output with other strategies.
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
- **[inf]** Cut Craft: an array toolpath per source (the tool-grouped order), each recalculated from its source. Merged refuses toolpaths with different tools, splits them into their plunge-to-retract cuts, and keeps the toolpath list's order or a nearest-next order, whichever travels less; Merge By Part is not done.

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
- Selected vectors draw as **dotted magenta**; groups and text draw as solid magenta.
- **Click an already-selected object again** to show the transform handles:
  - **Centre handle:** move. Alt restricts the move to one axis. Clicking it again switches to the rotation centre, which can be dragged.
  - **White corner handles:** proportional scale. Alt makes it non-proportional, Shift scales about the centre.
  - **Edge handles:** scale along one axis. Shift keeps it proportional.
  - **Black corner handles:** rotate. Alt snaps to 15° steps.
  - **3D view differences:** the edge handles are black, and a rotation arrow sits on top.
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

### Keyboard shortcuts [obs] (accelerator table; the 3 debug-only Ctrl+Alt+Shift entries are left out)
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
- **Feed units [obs]** (help: post-processor-editing): a post prints feed rates **per minute** (mm/min or inches/min, by `UNITS`) unless its variable has a multiplier. The help's examples multiply by `0.01666` to convert "from inches per minute to inches per second" and "from mm per minute to mm per second". Real grbl / FluidNC posts use multiplier 1 (mm/min). `Lua/ppVariables.lua` says "mm/sec or inches / sec"; that only matters to Lua-scripted posts, which we don't run.
- **[inf] A new feed rate starts a new feed run:** real posts print `[F]` only in `FIRST_FEED_MOVE` (and `FIRST_PLUNGE_MOVE`), so the `FIRST_` block must be used whenever the rate changes (plunge to cut), not only after a rapid; otherwise the cut would run at the plunge rate.
- **Inverse time [obs]:** `INVERSE_TIME_MODE = YES` makes feed moves print `[FI]` instead of a rate: the move takes 1/F minutes (G93; G94 is units per minute), so F = feed rate ÷ move length. **[dec]** It is refused unless wrapping is on ("INVERSE_TIME_MODE directive can only be used when wrapping is enabled!").
- **The full vocabulary [dec]** (`Aspire.exe` keeps one table of `NAME [LABEL|A||default format]`):
  - Numbers, with their default formats: `POWER [P|A||1.0]`, `SPINDLE_SPEED [S|A||1.0]`, `PLUNGE_RATE [FP|A||1.0]`, `FEED_RATE [F|A||1.0]`, `ROTARY_FEED_RATE [FW|A||1.0]`, `INVERSE_TIME [FI|A||1.3]`, `CUT_RATE [FC|A||1.0]`, `LINE_NUMBER [N|A||1.0]`, `X/Y/Z_POSITION [X|A||1.3]…`, `X/Y/Z_HOME_POSITION [XH|A||1.3]…`, `SAFE_Z_HEIGHT [SAFEZ|A||1.3]`, `DWELL_TIME [DWELL|A||1.0]`, `X/Y/Z_MIN`, `X/Y/Z_MAX`, `X/Y/Z_LENGTH` (all `1.3`), `WRAP_DIAMETER [WRAP_DIA|A||1.3]`, `X/Y_ORIGIN_POS [X_ORIGIN_POS|A||1.3]…`, `TOOL_NUMBER [T|A||1.0]`, `PREVIOUS_TOOL_NUMBER [TP|A||1.0]`, `TOOL_DIAMETER [TDIA|A||1.3]`, the arc values (`ArcStartX/Y`, `ArcMidX/Y`, `ArcMidXI/YI`, `I`, `J`, `IA`, `JA`, `Radius`, `Angle`, all `1.3`), `A_POSITION [A|A||1.3]`, `A_HOME_POSITION [AH|A||1.3]`.
  - Text: `Z_ORIGIN`, `XY_ORIGIN`, `TOOL_NAME` (`TOOLNAME`), `TOOLPATH_NAME`, `TOOLPATH_PATHNAME` (`PATHNAME`), `TOOLPATH_FILENAME` (`TP_FILENAME`), `TOOLPATH_DIR` (`TP_DIR`), `TOOLPATH_EXTENSION` (`TP_EXT`), `FILE_NAME`, `FILE_PATHNAME`, `FILE_NOTES`, `TOOLPATH_NOTES`, `TOOL_NOTES`, `TOOLS_USED`, `TOOLPATHS_OUTPUT`, `SIDE_FLIP_DIRECTION`, `TIME`, `DATE`, `POST_NAME`, `UNITS`, `PRODUCT`, `REVISION`.
  - Global statements beyond those above: `POST_BASE` (refused as no longer supported), `LINE_NUMBER_START`, `SCRIPT`, `ROTARY_SUPPORT`, `RUN_SCRIPT_AFTER`, `RUN_EXE_AFTER`, `RAPID_SPEED`; templates may use `%IF` / `%ELSE` / `%ENDIF`.
  - Blocks beyond those above: `ROTARY_A_INDEX`, `FIRST_RAPID_MOVE`, `INITIAL_STRAIGHT_RAPID_MOVE` / `FIRST_STRAIGHT_RAPID_MOVE` / `STRAIGHT_RAPID_MOVE`, `POST_INFO`, `TOOLS_USED_LINE` (the template for each line of `[TOOLS_USED]`), `REVISION_COMMENT`.
  - Parse errors name what is required: `UNITS` (MM or INCHES), `POST_NAME`, `FILE_EXTENSION` and a `FEED_MOVE` block; `MIN/MAX_ARC_RADIUS` need arc support; wrap text needs a wrap axis.
- **Meanings [obs]** (help table): `XLENGTH` etc. are the material size; `Z_ORIGIN` is "Table or Material Surface" (**[obs]** the UI words are "Material Surface" and "Machine Bed"); `XY_ORIGIN` is the X, Y origin; `TOOLS_USED` and `TOOLPATHS_OUTPUT` list the tools / toolpaths in order of use; `DATE` / `TIME` are when the file was made; `PRODUCT` is the program name and version; `DWELL` is seconds when drilling; `WRAP_DIA` is the cylinder's diameter.
- **[inf]** Date and time read like "Saturday October 24 2026" and "09:37 AM": `Aspire.exe` holds the formats `%A %B %d %Y` and `%I:%M %p`, but the code using them is not traced yet.

---

## Icons (`x64/Icons_Light.dll`) [obs]
- The DLL holds 243 PNGs, 364 BMPs and 36 icon groups (REA Evidence `ev_f158112e…`). `re/icons.py` extracts them into the gitignored `re/extracted/`.
- **The main UI icons are Codejock image strips**: square slots side by side, `png/35039`–`35070`. There are 8 strips (slot counts 31, 12, 5, 6, 14, 28, 29, 37), each at 4 sizes (24/30/36/48 px; the toolpath strip uses 28/35/42/56). The strips are:
  - Create Vectors, Edit/Transform, Weld/Subtract/Offset, Join modes, Align (14, the same as the Align menu), Modeling, File + vendor links, Toolpaths.
- The BMPs are mostly illustrations from older dialogs: tool shapes, ramp and lead-in diagrams, nesting layouts.
- **[obs]** `Aspire.exe` never contains the strip IDs, as 32-bit values, 16-bit values, `mov` immediates or data tables. It names `Icons_Light` once (a UTF-16 string). **[inf]** The slot-to-command binding is computed at runtime; following it means decompiling the code that references that string.
- **Our icon set:** `re/make_icons.py` traces the useful icons to SVG and writes them to `assets/icons/<folder>/<name>.svg` (gitignored). It traces in colour layers with potrace and uses a 96 px copy of a strip icon when one exists. `re/icon_set.json` lists each path with its label, whether the label is a guess, and its source ID.
- `re/icon_labels.json` maps every image and slot to a name. It contains IDs only, no artwork. The sources are help-manual image matches (exact for about 19 Model tools) plus visual identification; a trailing `?` marks a guess.

---

## Open questions (need decompilation or sample files)
1. `.crv3d` / `.3dclip` / `.v3m` / `.vfont` byte layouts. There are no samples here; `Aspire.exe` would need Ghidra.
2. Meanings of the `Default.prf` fields.
3. Which mouse button orbits the 3D view (the help pages contradict each other).
4. The commands behind keys 1, 2 and Ctrl+F1…F9.
5. Status bar contents.
6. Toolpath algorithms (V-carve, pocketing, 3D raster). Write our own; the docs describe only parameters and results.
