# Notes on the CNC video courses

What each video teaches, written while reading the captions (fetched by `scripts/courses`;
video ids are YouTube ids). The capability table `docs/courses-capabilities.tsv` and the report
`docs/courses.md` are built from these.


## Toolpath FAQs (V12)
- YNfSato4LvM Material Setup: active sheet shown; thickness; XY datum 5 positions + Use Offset; Z zero surface/bed;
  rapid clearance Z1, plunge Z2; home X/Y/Z gap; "show detailed summary" panel on every toolpath form;
  3D: model thickness shown, slider places model, gap above / below typed, double-click line = top/middle/bottom;
  "Set" model thickness (rescale model Z) from the form.
- 0FkoKHrktJE Tool Database: material drop-down + Material Management (add, copy, remove with warning);
  machine drop-down + Machine Management (add/copy/remove, name, manufacturer, model, controller, width, height,
  units, posts list, install offline package, search online); geometry vs cutting data per material+machine;
  greyed tools without data; "Copy settings from" (machine + material); right-click Hide Unset Tools;
  notes field output by post; tool name format with variables (right-click insert: geometry, cutting params,
  feeds, material, machine; diameter, fraction, units, unit short, tool type), default format per tool type,
  apply to existing tools; warning on unapplied changes; import/export database; tool groups.
- 6ShnHsNfkhg Keep-out zones: from selected closed vectors, clearance border, red in 3D, toolpaths re-routed
  (rapids lifted over), live update when clearance changes, zones independent of vectors after creation,
  recalc warns, Save Toolpaths refuses, Clear button.
- Txafg3oN8c0 Save Toolpaths: machine + post drop-downs (machine config button beside); modes: selected toolpath,
  visible to one file, visible to multiple files (numbered in list order), group where possible (by tool, named
  after first); toolpath groups in the list (right-click Create Empty Group, Group Visible, Ungroup; a group saves
  as one file); errors: visible use different tools and post lacks ATC; same tool number, different geometry;
  tool number edited on the toolpath's tool; Output tiled toolpath; Add side to toolpath name (two-sided);
  Options ▸ File dialog default: Global / Operation / Job.
- W2P2EhRRjg8 Machine Configuration: machines (name, manufacturer, model, controller, width, height, units),
  associated posts per machine (+ add from full list, type-ahead), warning when none; post version pinning and
  "not latest" warning; per-post Edit menu / right-click: Change Log, Customize (custom copy, pen icon, edit
  in text editor via Open File Location), View, Delete custom, Make Default (bold, first in Save Toolpaths),
  Make Default for Laser, Remove; install post to custom directory; download latest post database;
  install offline machine package; search online (manufacturer / series / model / configuration incl. rotary)
  installs posts + tools; Machine menu: Add Machine, Install Offline Package, Search Online, Post-Processor
  Management, Install Post-Processor, Update Post-Processor Database; kickstarter wizard on first run.
- WjTQ9RIvaKI Toolpath Summary: opens with all toolpaths shown, list ticks change total; rapid rate with units
  (mm/s, mm/min, m/min, in/s, in/min, ft/min); scale factor; notes per toolpath (Apply); Machine Acceleration
  option: machine drop-down, accelerations X/Y/Z/A with units (…/s², …/min², rad/s², deg/s²) kept on the
  machine in Machine Configuration; time accounts for accel/decel at corners.
- Xnj195VAW04 Preview Toolpaths form: active sheet / all sheets; Edit material settings palette (material
  library categories: woods, metals, stone, plastics; Use Solid Color; New Category; Add New Texture from a
  photo); Machined area color: Material Color / Surface Color (painted surface, wood revealed) / Global Fill
  Color / Toolpath Color (per toolpath) / Lithophane (slider); Draw Tool (to scale) + Animate Preview + speed
  slider; Preview Selected / Visible / All Toolpaths / All Sides (double-sided flips); Reset Preview;
  Run / Pause / Stop / Single Step / Run to Retract; Undo Last (undo last previewed toolpath); double-click a
  cut-free waste area in 3D to remove it; Save Preview Image (bitmap); close.
- EbGvAQ3tDtY Form Tool: tool type "Form Tool" from a selected open vector (right half of profile; node edit
  Cut Vector / Delete Span to prepare it); diameter shown from the vector; Create Settings then cutting data;
  no undercuts. Used by profile-like cuts and drawn in preview with that profile.
- nzMCOZUQY68 Online tool database: vendor portal upload / download / shared links (vendor service, out of
  scope); shared databases read-only, right-click Copy to Local Database (machine + material); toolpath
  form's tool Edit changes the tool's settings for that toolpath only.
- WHFiP-5FMYU Toolpath list: double-click or right-click Edit; Duplicate; Delete key; right-click Delete ▸
  Delete This / All Invisible / All Visible / All; Recalculate ▸ This / Visible / All; Recalculate All button.
- _IwpNdUitfA / hKbtMLP4Jdc Toolpath Tiling: Tile toolpaths tick; individual tiles / feed through X / Y; tile
  width, height, overlap; machine smallest tile first; Update Tiles; Delete All Tiling Data; Active tile
  (drop-down, or double-click a tile's label in 2D) — preview and 3D show only that tile's toolpaths;
  Draw toolpaths in original position for visualization (else at the datum); Save: Output tiled toolpath
  (default when tiled) files prefixed T1, T2…; move toolpaths up / down in the list so same-tool ones group.

## General Topics (V12)
- EC0F23IihuY Interface: start page (new, from template, open, 4 recent files, tutorial browser + tutorial files
  folder, online links, social); Job Setup (single / double / rotary, size, Z zero, XY datum, modelling
  resolution, material appearance); menus; side tabs Design / Sheets / Layers / Components / Clipart;
  Toolpaths tab on the right, pin / unpin, auto-hide pop-out, F11 / F12 switch; job dimensions shown in
  Design tab; 2D view toggles (3D parts, bitmaps, toolpaths wireframe / solid), pan / zoom; 3D view toggles
  (toolpaths, origin, block, modelling plane, bitmaps, vectors, 3D parts), ViewCube; document bar: 2D / 3D,
  tile horizontal / vertical, two-sided single / double view, flip side, sheets, layers, levels drop-downs,
  snap toggles; right-click context menus (vector, node edit).
- v2rJ6lOdIuY Layers: one layer per object; active layer bold; new objects on active layer, copies / arrays
  on original layer, offsets on active layer; layer from bitmap import; DXF / DWG / SKP layers kept; PDF a
  layer per page; EPS / AI one layer named after file; Dimensions "place on own layer"; vector texture,
  plate production, zero plane, slice make layers; visibility eye, colour swatch (+ more colours), page icon
  (empty / has vectors; double-click selects layer's vectors), name; reorder up / down; drag the layers
  panel out as a floating pinned/unpinned window; rename (double-click / right-click / menu); hidden active
  layer turns red, drawing on it shows it; Copy to Layer ▸ / Move to Layer ▸ layer or New Layer (dialog:
  name, colour, visible, active); group puts members on active layer; Ungroup onto original object layers /
  onto group's layer; Delete layer with data: move data to layer or delete; layer order decides what draws
  on top; menu: Activate, Show ▸ This / Only This / All But This / All, Hide This / All, top toggle all,
  Lock / Unlock (no selecting), Insert New Layer, Delete ▸ This / Visible / Empty, Rename, Select Layer
  Vectors, Merge Visible (onto one layer).
- qOIZmdKOno0 Import / export 3D: Clipart tab (folders of clipart, double-click = centre, drag onto 2D at a
  point, Add Folder, right-click Import to Level ▸ current / new level, variants); Import Component / 3D Model
  (v3m, STL, 3dclip, crv3d, OBJ…); orientation picks + red / green / blue rotation handles, rotate about Z;
  size X/Y/Z with lock ratio; units mm / in; Centre model; Scale mm↔in; Position and Import step: depth
  slider (double-click top / middle / bottom), discard below zero plane, Create Both Sides (two components,
  front and back), Highlight Undercuts (purple); components overlapped after import; Components tab right-
  click Re-Import (back to the import form); crv3d imports as one component, 3dclip keeps its component
  tree; Create Component from Bitmap (import directly, or from selected bitmap); combine modes (add,
  subtract, merge, low, multiply) from tree or 3D right-click ▸ Component ▸; component as texture overlay
  (multiply / ordering); Smooth; Ungroup group in tree, right-click Export as 3D Clipart (component or
  group; shift-click range select in tree, Group); Export Visible Model as STL: tolerance, max triangles,
  Triangulate preview with triangle count, back: open / inverted front / flat plane; Save as STL / OBJ /
  POV; Export as Grayscale Bitmap: BMP / JPG / GIF / PNG / 16-bit TIFF.
- WfexakRNSpE Sheets (multi-material): Sheets tab + top drop-down; Add New (copies size), name; double-click
  sheet in 2D or list to activate; right-click Copy to Sheet ▸ / Move to Sheet ▸ (keeps XY); Edit: width,
  height, thickness, Z zero, XY datum per sheet; multi-select sheets and edit together; hide inactive
  sheets; Job Setup size change applies to all sheets; toolpaths per sheet; material setup per sheet;
  preview appearance per sheet (palette: material or solid colour); sheet drop-down in Toolpaths tab.
- WpzMKF9LpDg Sheets (12.5): right-click Rename, Duplicate sheet (copies vectors, toolpaths, components,
  level effects); new sheet takes last active sheet's size; visibility dots per sheet; toolpath right-click
  Copy To ▸ Copy to Sheet (copy is unset "!" until its vectors are picked); preview palette per sheet with
  surface colour (painted MDF).
- JRPg0oexpN8 Layers (12.5): as above plus drag-reorder in list, Duplicate layer, Delete Invisible, filter
  "show layers on the active sheet only" (tab + top drop-down), layer status in status bar, drag the tab
  out to float / auto-hide.
- lJUAcgT77eA Snap grid + guides: top-bar grid toggle shows grid; grid spacing in Snap Options (F4);
  Options ▸ 3D view: snap grid colour and point width, guide colour, locked guide colour, guide thickness,
  Restore Defaults; right-click ▸ Guidelines ▸ Insert Horizontal / Vertical / Angled (typed position,
  angle, rotate handle, drag); Delete key in 3D; 2D right-click guide ▸ Guide Properties: position, move to,
  angle + slider, Lock, Create parallel guides absolute / relative (count, spacing), Select New Guide,
  Delete; ruler-corner button hides guides; drag from rulers; View ▸ Guide Lines ▸ Visible, Delete All,
  Lock All, Unlock All; drawing snaps to guides and their crossings.

## Creating Vectors (V12)
- Shape tools common: drag from centre / corner, live size field beside cursor (typed + Enter, Tab to next),
  click again to place another at the same size, Shift+click an existing shape to edit it in the tool,
  handles stay in transform mode (radius, side length, inner / outer radius), E reopens the creating form.
- xKEMBBssl48 Circle: radius / diameter radio, X/Y centre, typed radius handle.
- evD7YgrMgl0 Ellipse: width / height handles, corner handles proportional.
- 3VLd_-gHGeI Rectangle: drag direction sets anchor corner; Alt keeps proportion; corner type Square /
  Radius External / Radius Internal with a corner handle and radius field.
- aH2x_vEKThY Polygon: sides; radius or side length.
- UZeMhbWFfjQ Star: points; outer / inner radius handles.
- WHG7Ko4_6wU Polyline: click points, typed length + Enter along the pointer direction, smart snapping lines
  ("wake" a point), click start or Tab to close, click-drag makes a curve (Bézier), S for straight,
  Backspace undoes last span; form entry of points.
- 3VzE89aug5o Arc: through 3 points (start, end, bulge) or centre / start / end; form: start, end, radius
  or height.
- qjJXVHB0VM4 Freehand: mouse or pen; Smoothness slider; Stabilize (circle lag); End vector at cursor;
  Tab closes to start; bitmap fade slider in the form.
- BeIZ7BD514s Vector Texture: angle −90..90 slider, max spacing, variation, amplitude, wavelength, noise,
  "place vectors on a layer" (named), Preview, inside selected vectors; re-selecting a texture reopens its
  settings (texture remembers params) until edited; tileable when angle 0/90 and wavelength divides width;
  profile toolpath with V-bit / ball on texture lines.
- admEKCLmzX8 Text: text box + Larger Edit window; spell checker (right-click: suggestions, Add Word, Remove
  Word); TrueType list with type-ahead and recent-fonts list at top (count in Options); single-line fonts;
  bold / italic; alignment; anchor X/Y; text stays editable (select + Draw Text).
- R2nhSGOSaoU Text spacing (kerning in place): click between letters to tighten, Shift to loosen, Ctrl for
  bigger steps, Alt for all letters; line spacing arrows between lines; right-click ▸ Text ▸ Break Block
  into Lines; arc text: green top / bottom handles bend the line into an arc, red handle rotates round the
  arc centre (Ctrl 15° steps), blue handle changes radius; kerning and editing keep the arc.
- nESYbpc9r6o Convert Text to Curves.
- 7BdkWRxDWic Text on a Curve: maintain size / scale to fill %, spacing slider, above / on / below,
  other side, offset, alignment left / centre / right + draggable blue position box, align to curve /
  keep vertical; stays editable text (curve kept after deleting the vector; kerning, font change).
  Node edit C cuts a circle into two arcs for top / bottom text.
- dv9m9WaQkkY Trace Bitmap: colour mode (reduced to ≤16 colours, slider, palette picks or click colours in
  2D, trace colour), black-and-white mode with level slider; selection box limits tracing to an area;
  noise filter (pixels); corner fit loose ↔ tight; group vectors; Preview / Apply.
- IbdAVuCvW7g Dimensions: length (any angle), vertical, horizontal, angle (centre + 2 points, either sweep),
  arc (radius / show diameter); font, text height, decimals, offset, custom text, "place on layer"
  (Dimensions layer default); Shift+click an existing dimension to edit; smart snapping to centres.
- bn1qBrFMsW0 Polyline (old): S toggles a curve mode (clicks make smooth curves), mode remembered across
  sessions; right-click exits.
- Q5J-RJ-a61k Text in a Box: box from selected closed vector (or job), width / height editable, margin none /
  normal / wide; vertical stretch none / space lines / stretch characters; horizontal stretch none / spaces /
  kerning / characters; box remembered without the vector.
- jzW6RGG8wWk Node editing: N or right-click; start node green with direction arrow; typed X/Y fields for a
  node; drag a span midpoint to make a node; right-click node: Delete Point (D), Smooth Point toggle,
  Cut Vector (C), Make Start Point; span: to Bézier (B), to Arc, to Line; node-edit while in a shape form;
  open vector node ▸ Edit Polyline (continue drawing, close); clicking text offers convert to curves;
  box-select several nodes and drag together; H / V mirror lines move node pairs symmetrically with arrow keys.
- PXh58eYRoWE Trim: Trim Objects (selected to last selected, clear inside / outside boundary, groups
  respected, refit curves after clipping); Interactive Trim: click or drag across sections, Shift deletes
  closed non-overlapping vectors, groups refused (struck scissors), rejoin trimmed sections on close.
- bPCCtoT6dx0 Text (12.5): height handle drag, anchor, Rotate angle field + handle in the text form.

## 2.5D Toolpaths (V12)
- h7FccWQT2TA V-Carve: start depth, flat depth, clearance tools (several, ordered, offset / raster each, ramp
  plunges) making extra toolpaths, use vector start points, use vector selection order, project onto 3D
  model; Vector Validator V-carving mode (ignores overlaps that v-carve fine); Weld on text asks Replace or
  keep a copy as text; node edit right-click two nodes ▸ Close with Smooth Curve.
- 8o1ckYbr9tY PhotoVCarve: start depth, max carving depth, stepover retract, V-bit, Raster / Hatch /
  Selected Vectors (vectors give the lines), line spacing (density), line angle, project onto 3D; toolpath
  preview colour; Edit Picture (contrast, brightness, gamma, invert, greyscale, border rectangle / oval with
  width); Crop Bitmap to a vector.
- 4_WbZoZeVvY Fluting: start depth, flute depth, tool, Ramp over complete length / at start / at start and
  end, ramp length (distance or %), linear / smooth; start points decide which end is shallow (Make Start
  Point); pocket with two tools (end mill clearance then ball nose; tool order up / down); per-toolpath tool
  Edit (pass depth) not saved to database; preview Toolpath colour; texture panels and seat from line sets.
- LXQai4Dt264 Texture toolpath: whole job or selected vectors; start depth; selected vectors as pattern;
  max / min cut depth, max / min cut length, overlap %, variation %, stepover, angle; ramp start and end
  (vectors as pattern); Save / Load texture settings; boundary vector offset (halo off the text);
  project onto 3D model.
- MM-satiPXgg Prism: start depth, prism depth, "set depth for full prism", V-bit, climb / conventional,
  order (use vector selection order), Vector Selector.
- pBkxXN4lz88 Moulding: several drive rails, one cross-section; direction arrows and side ticks; Reverse
  Direction on rail; start point of section on the rail side; position in material (model gap); finish tool;
  Vary stepover (tighter on steep sides); large area clearance tool (machine flat regions; finishing skips
  flat regions); ramp; machining allowance; Create sharp corners (closed rail); boundary offset automatic /
  user value; automatic Vector Selector (layer, style); makes two toolpaths (clearance + finish).
- izFO02etvTg Chamfer: start depth; tool V-bit (angle locked from tool) / ball nose (angle typed) /
  engraving; width ↔ cut depth linked; "z=" sums; overcut (flat tip); max cut depth read-only (ball goes
  deeper); inside / outside, slope upwards / downwards (arrows drawn in 2D); cut-through warning;
  climb / conventional; use vector selection order; Vector Selector; per-toolpath tool Edit pass depth
  ("/2=" sums) for smooth steps; countersinks with pass depth > cut depth (one plunge each); offset by
  chamfer width (copy field, paste into Offset).
- J1pR89ObO6g V-Carve Inlay: pocket depth, glue gap, surface gap with diagram; V-bit + clearance tool;
  plug sheet drop-down (copies + mirrors vectors onto it, toolpaths on that sheet); plug boundary: sheet
  limits or vector offset + boundary offset; makes pocket clearance / pocket V-carve / plug clearance /
  plug V-carve toolpaths; Plug stock field (how proud the plug sits); warnings: insignificant glue gap,
  clearance tool unused (empty toolpath).
- T20Snbn9ACc Sketch Carving: source bitmap or 3D model (composite, levels); start depth, flat depth, V-bit;
  machining limit boundary: bitmap / model boundary / selected vectors (holes in them excluded) / selected
  level; boundary offset; tracing parameter line thickness slider (green live preview of areas in 3D);
  project onto 3D; vector selector; Material Setup "Set" model thickness to fit; Vector Boundary round
  components for an outline profile.

## Offset and Layout (V12)
- 66cFpax3Q3Q Offset: outwards/right, inwards/left, both; distance; sharp corners; delete original; select
  new; open vectors offset left / right relative to the start point.
- nKh9vdfQYbs Array Copy: selected object size (copyable), rows, columns, gap or offset X/Y, block symmetry
  (mirror / rotate buttons per cell of a 2×2 block), row / column displacement, group copies.
- WZ3FxrtXOCo Circular Copy: rotation centre (handle + typed), step / total angle, count, rotate copies,
  group copies.
- 2cCMTkFRwHA Copy Along Vectors: copy object (first picked) or copy circles (diameter); distance with
  force even spacing, or number of copies (lands on nodes); align to curve; copies on new layer; reverse.
- aeAN-3_U1io Nesting: tool diameter, clearance, border gap; nest from corner; true-shape parts; parts in
  parts; copies per part (Apply; remembered per part; number drawn on part); remove original parts;
  override active sheet; mirror parts to fit; rotate parts with step angle (1° = free); nesting sheets
  list (custom: sheet, limit one / no limit, order up / down, remove, + add, overflow makes new sheets);
  nest boundary from a layer; two-sided parts nested on both sides; filler parts (star, fill the rest,
  larger fillers first); right-click Select ▸ Select Part after nesting; preview / reset.
- hRaUMei75ws Plate Production: template with !!variables!! text in a box; mode sheets or layers; sheet /
  plate size, margins (equal or each), spacing, plates X/Y auto calculate; Create Toolpaths (template's
  toolpaths copied onto every plate, named S1 / S2 per sheet); CSV with separator (comma, tab, semicolon,
  space), first row column names, variable ↔ column assignment; toolpath list drop-down "all sheets".
  Also: profile tabs placed by clicking (edit tabs), Z/3= sums, Quick Engrave diamond drag outline / fill
  with hatch angle.

## Editing Objects (V12)
- aT7myo3CccE Select: click, window left→right inside only, right→left crossing.
- CF7nszvPLTM Transform: click twice or tool; corner handles scale with typed W/H fields (proportional),
  side handles one axis, drag to move, rotation handle + typed angle, draggable rotation centre, several
  objects share one box.
- N_jHFZz9wz4 Group / Ungroup: G / U keys, right-click; ungroup onto original object layers or group layer.
- tGDpYuHVXmw Measure: two points (distance, angle, dX, dY, first point, live cursor); span / contour
  properties: type, length, start / end, centre + radius for arcs, vector area, perimeter, span count
  — all in a form.
- 8pWYBEY7Sxg Weld (asks replace / keep original for text), Subtract (first minus rest), Overlap.
- vDO-TC__TCE Vector Validator: no selection = visible vectors on current side / sheet / visible layers;
  overlaps, intersections (red squares), zero length spans (Fix button), V-carving mode; Edit ▸ Select
  Duplicate / Open Vectors; Join with tolerance.
- uPTziUXw4PI Extend: click two open ends, extended along tangents to meet (dashed preview).
- V3zxhKSNSCg Curve fit: arcs / Béziers / lines, tolerance, keep sharp corners with max angle, replace.
- Z39pqSDmLWc Vector Boundary: offset distance; rubber band (convex hull) option; several vectors.
- k7CzrIqaVco Edit Picture: contrast, brightness, gamma sliders (double-click resets), invert, greyscale,
  border rectangle / oval with fade slider, Apply.
- 0Bb_Sm-jAtA Join / close: move end points, smooth curve, straight line (buttons and right-click), Join (J)
  with tolerance and open / closed counts.
- dLD0Li8DMYw Fillets: interactive click per corner (tick / cross / warning icons), typed or dragged radius
  live, max radius warning, click a fillet to remove it; normal, dog-bone, T-bone (either side), plasma /
  drag knife loops.

## Transforming Objects (V12)
- 1-xjiMnIHE8 Move (M): absolute / relative / relative rotated; anchor (9 points) X/Y; F9 centres in job;
  centre handle typed X/Y in view.
- MuZTZnmnU5c Set Size (T): scale selection / items individually; link XY; job-aligned vs rotated box;
  anchor; units or %; Shift-drag scales from middle; typed W/H on side handles.
- 9_X3gIyPGMM Rotate (R): pivot anchor or coordinates, draggable pivot (Shift ignores snapping), absolute /
  relative angle, Alt 15° steps on handle.
- AD95uqxZWpg Mirror: mirror to left / right / top / bottom (beside), create mirrored copy, flip H / V in
  place, flip about job centre, use rotated bounds, flip about a line (last picked), keys H / V,
  Ctrl+H / Ctrl+Shift+H / Ctrl+Shift+V copies.
- bb2ybfu-5MM Distort: rotated bounds, envelope node editing (insert, smooth, arc / Bézier), Transform
  Object / Edit Envelope modes, Bake Distortion, above a single curve, between two curves (start points
  decide orientation), components too.
- JG9x0dAr3WI Align: to material (centre, centre H / V) or to the last-picked vector: centres, inside edges
  (top / bottom / left / right), outside edges; space vertically / horizontally; "inside last vector"
  spaces between the edges of the last picked; "show common tools on the drawing tab".
- 7FsMyhZ7UtE Snap options (F4): display text at cursor (X/Y, radius, L / A), snap to guides, snap guides to
  geometry, grid + spacing, job centre and corners, fixed nudge distance (Ctrl+Shift+arrows), snapping
  radius slider; geometry: object centres, span end points, midpoints, arc centres / quadrant points,
  intersections; smart snapping: object bounds, horizontal / vertical, tangents, perpendicular (T key),
  from nodes and midpoints, span geometry, snap angle increment 15°; Space restarts.

## 2D Toolpaths (V12)
- VAvj8KqOuE8 Profile: start / cut depth (z=), tool + per-toolpath Edit (pass depth), outside / inside / on,
  climb / conventional; ramp plunge (distance); tabs: length, thickness, Edit Tabs: constant number or
  constant distance with min / max count, avoid corners and curved regions, first tab at start point,
  click to add / remove, drag to move; nested shapes cut inside; 2D solid toolpath drawing.
- 3I5rZg8hE-s Pocket: offset / raster (angle) + profile pass first / last; ramp; allowance (+ under, − over);
  Ctrl+Calculate keeps the form open; offset tails into corners at large stepover; islands alternate;
  vector selection order; several tools largest first (raster only for the largest; per-tool edit,
  ramp, allowance) making clearance toolpaths.
- HNyIDbF7kww Drilling: tool any size (centre of each closed vector); peck drilling (pass depth), retract
  above start depth or above previous pass, retract gap; dwell seconds; vector selection order.
- JoYrjnIRXhk Thread Milling: start / max depth, thread length (read-only), single / multi-point tools;
  pitch presets (ISO metric / UTS), pitch, major diameter (from circles), fit tolerance, internal /
  external, left / right hand, top→bottom / bottom→top; Create Circles for Internal Threads (pocket
  vectors); exceeds thread length warning; small chamfer loop at the top.
- -gbQ2eUJ4kw Inlay toolpath: straight (male), pocket (female, offset / raster), stepped male (step depth,
  step width), hole; same tool both; overcut / allowance.
- MOiVl-R3xhM Quick Engrave: save toolpath from the form (machine + post), depth / pressure, outline / fill:
  offset / hatch angle / cross-hatch, stepover; nose cone (protrusion, passes); vector selector; add side
  to name; output direct to machine; engraving post with no spindle; Preview Simulation Quality.

## Efficiency Tools (V12)
- wzS6M3bhHgo Job templates: Save As Template (.crvt) with vectors and toolpaths; New from Template
  (template folder list); Vector Selector on each toolpath (closed / open, from selected layers, associate
  with toolpath) so Recalculate All re-picks layer vectors after artwork / text swaps.
- mucdprsJQkw Job sheet: Ctrl+click opens in browser; job layout shows only visible layers' vectors;
  material block, home, Z zero, XY datum, clearance pictures; toolpath summary (name, tool + number,
  time); per toolpath: type icon, feed, plunge, spindle, tool name / type, max cut depth (red when
  cutting through), pass depth, tool number, stepover; notes; multi-sheet jobs write _Sheet N files;
  two-sided: job type, side, flip direction, _Top / _Bottom; Setup Sheet Editor gadget: logo, title,
  colours, border, sections on / off, restore defaults.
- pTEt3t9ITqc Array Copy Toolpath: selected toolpaths (tick all), rows, columns, offset or gap, minimise
  tool changes (group by tool across copies), parent toolpath in list; edits to children propagate;
  vectors not copied (layer-associated); ATC error on save.
- sIHxUY9crJM Merged toolpath: same tool required (warning otherwise), ordering left→right / bottom→top /
  grid / shortest path, merge by part (keeps order per part), warning when order may change; edits to
  children propagate; right-click Ungroup un-merges.
- pH8zxn5uJz4 Toolpath templates (cabinets): Vector Selector filters (open / closed, all circles, circles
  matching tool diameter, circles of given diameter, from selected layers, associate); status bar shows
  layer and size of a picked vector; copy / paste layer name into toolpath name; Save All Visible
  Toolpaths as Template; DXF Batch Processor gadget (folder, subdirectories, per-row count, gaps, job
  size, datum, Z origin); nesting (90° rotation only, remove originals); Load template applies to all
  sheets (skips toolpaths with no vectors on a sheet).

## Modeling Tools (V12)
- glEsqGe_8cU Intro: modelling resolution standard 1M / high 2M / very high 4M pixels over the job; job
  only slightly bigger than the part; components, levels, groups, combine modes; workflow; 3D rough +
  finish; projecting V-carve onto models; export mesh for printing.
- rA1ZDH1qY4c / nMM9s51Ee2c / zCVBsLEBi4w / 57VUdzl9jiI Create Shape (round / angled / smooth / custom
  profile): per-shape tool icons on the Design tab; interactive 3D handles: angle (−90..90 round, −80..80
  angled; typed), base height (double-click resets), limit %, scale %, total height (scales base + scale
  together); properties handle: combine mode, fade (set two anchor clicks, %), tilt (two anchors, angle);
  shape kept editable (E reopens the creating tool, N node-edits outline / custom cross-section with live
  update) until transformed (then only Component Properties); new shape per vector picked; name; between
  two closed vectors (ring); preserve internal corners; smooth: top and base sliders; blend to inner
  vectors (smooth, custom); custom profile: open cross-section (with or without a leg), width vs radius
  decides flat top.
- e08s0KipMEY Flat shape: height handle (typed), combine mode, fade / tilt, live node edit of outline;
  Shift+click Flat Shape = zero plane (layer "Zero Plane", greyscale hidden in 2D).
- Fdq4nhMahF8 Two Rail Sweep: Use Selection (first red, second green rail, start points + arrows); open
  cross-section(s); scale cross-sections with width, or scale to exact height; several cross-sections
  placed by clicking on the rail (coloured squares match section to position), drag to move along rail;
  sweep between spans (needs equal span counts; numbers shown when they differ); right-click section node:
  smooth / unsmooth, delete, Add to All Rail Nodes (equal node counts), Remove All; Reverse Rail, Swap
  Rail Order; closed rails (frames; corner sections fix twisting); Fill Centre of Inner Closed Vector
  Rails (cross-section leg makes a raised plaque); node edit rails / sections inside the form; Start New
  Component, Reset, Clear Rails.
- xyKVB69c734 Extrude and Weave: drive rail(s) (start point, arrows), vector cross-section(s) placed at start
  / end / middle (sweep between spans, span count warning, smooth / unsmooth, delete, drag to move),
  scale to exact height; closed rails with Create Square Corners; Reverse Rail; component along rail
  (repeat with overlap %, stretch along rail length); weave under / over crossings: scale shape (Z under
  50 %, Z over 150 %) or add base (70 / 130 %); several rails at once.
- eJ8FgjO4S90 Turn and Spin: turn about start→end line (height = widest point, needs legs for closed
  silhouette), spin round the left end point (centre it on the job), scale to exact height, combine,
  name, node edit the profile inside the form; status bar Z under cursor.
- 9Lc7XZHrvP8 Lioness: levels per body part (insert, rename, combine Merge), round shapes with steep angle
  and squashed total height for detail lines; Create Component from Visible Model on a new "sculpt" level,
  Show Only This level; Smooth filter (preserve transparency); Sculpt: 5 brushes numbered 1–5 (smooth,
  smudge, deposit, remove, undo), component brush, diameter / strength / smoothness, preserve
  transparency, show visible model, sculpt mode normal / raise / lower, overlay image (bitmap on model),
  presets; sculpting in the 3D view with red brush circle, Ctrl+wheel brush size, up / down arrows
  strength, both buttons to rotate, Ctrl+Z inside, Keep; pen tablet.
- IHPQ65Rn0YM Custom brushes: component right-click ▸ Save as Sculpting Brush; Duplicate component;
  deposit / remove with Use Custom Brush ▸ Load (preview picture, ghost on model).
- IHPQ65Rn0YM (cont.) custom brush: stamp (click) or stroke; add vs merge brush mode; H / V held for straight
  strokes in top view; diameter (←/→, Ctrl+wheel), strength (↑/↓), smoothness = linear fall-off; Ctrl
  while dragging extra smoothing; R + wheel rotates brush; Dynamic Rotation follows the stroke (arrow).
- 8KjtmaE-Pr0 Texture Area: seed component (any clipart) tiled over job or inside closed vectors (several,
  holes = frame); spacing X/Y %, XY shift rows / columns %, reflection per tile of a 2×2 group, combine,
  name; Edit Texture Area resizes the seed in 2D; stays live (scaling the result re-tiles) until baked;
  Export as 3D Clipart keeps the texture settings; seed orientation remembered (bake first to rotate).
- s4ofeDBlkbw Dish recess: clipart dome, combine Subtract, zero plane, Scale shape height, modelling plane
  pink where proud; levels: rename, insert; Ctrl-drag copies components between levels; Multiply level
  combine mode (positive copy of dish scaled to 1 multiplies the model into the dish); flip copy with H.
- 7TLntCKMjww Carved recess: base height + scale; Vector Boundary round component; offsets; smooth shape
  blend to inner vectors as Subtract dish; sculpt only dish + zero plane (bake prompt), Show Visible Model
  off; smudge outward; Trace Bitmap on the component's greyscale to get an outline.
- RJogq_22TBs Lithophane: Create Component from Bitmap, set size with scale Z, combine Subtract (dark
  high), modelling plane toggle, Material Setup gap below model, 3D roughing (ball nose, large stepover,
  allowance warning > 20 % of tool), 3D finishing raster 45°, Preview Lithophane mode with light slider,
  duplicate toolpath with smaller tool.
- DNuqgWjC9n4 Component texture: component from selected bitmap on a new layer; Clear Area of Selected
  Component Outside Selected Vector; smoothing filter then Bake; height-map TIFF import.
- 0z0zpqyuaKc Zero plane: Add Zero Plane component on a hidden "Zero Plane" layer.

## Wrapped Rotary (V11, V10, V9 duplicates)
- bNYaA66G4Sg Rotary text: rotary job (length, diameter, Z zero cylinder axis / surface, datum, along X / Y,
  flip design), unwrapped height = circumference; 3D view material block + toggle automatic wrapping;
  rotary machine in tool database; text in a box over the job; pocket / V-carve; view buttons X / Y / Z and
  rotate ±30° steps; save with a Wrap Y→A post.
- IWkU7pBJ3zU Fluted column: Gadgets ▸ Wrapping ▸ Fluting Layout (flutes, start / end offsets, coves); layers
  "Fluting Vectors", "Cove Vectors", zero plane, bounding box; profile on the lines with a ball nose.
- 4-84MaPxrq4 Spiral column: Spiral Layout (strands, pitch or spacing, offsets, left / right twist, coves;
  reports revolutions); vectors run off the job and wrap.
- htEfRTdLDVo Rounding toolpath gadget: blank square (side) or round (diameter), radial / raster /
  optimised raster (corners first, square only), allowance, tool, name.
- -rx-oCb3Crw Unwrap full 3D model: Orientate 3D Model full / flat; initial orientation, rotate about Z;
  interactive rotation: view, XYZ move, rotate about X / Y / Z by dragging (Ctrl+Z); rotation axis
  movement (Ctrl+drag the red axis to bury it inside the model to avoid undercuts); model units mm / in;
  scale model to fit material; lock ratio; zero plane base height as a ring tab holding the part; Material
  Setup gap outside; 3D rough (Z level) + finish raster 0° (along) or 90° (around).
- LF9rIn8CYO4 Relief onto rotary: flat model import (top, +90° about Z, size X 9″); prompt to add a modelling
  (zero) plane so the relief is not distorted; Material Setup model thickness "Set" and slider shows
  distortion as the model goes deeper; coves drawn with polyline at the ends.

## Older series, unique topics (skimmed)
- ZPLMEYFFiAA Segmenting: edit mode with plane gizmo (arrows move, rings rotate, typed values), segment tree
  (rename, click to isolate, delete children), several cuts, layout mode with per-part orientation,
  highlight undercuts, back face caps / shells, import all parts; then nesting, limit-plane level, tabs,
  two-sided toolpaths.
- aZIIyhN5V1A Slicing: auto Z scale, slice thickness = material thickness or number of slices or custom;
  slice list with each thickness editable, bar to drag boundaries, double-click to add a slice, remove;
  colours (top green, selected red, below grey); components + layers + boundary vectors per slice; onion
  skin via zero plane base height.
- NidW-d0ZBYk / D03 / D04 / A07 Import Toolpaths (.tp from Machinist / Cut3D / PhotoVCarve; vendor formats).
- wIOxqcLbJt4 / EQtXyNwx0NQ / KEDjuvUAx2M Two-sided: Z zero top / bottom, flip along X / Y; copy everything to
  other side; dowel holes (symmetrical / non-symmetrical patterns into spoil board); limit plane (zero plane
  base height below zero); machining limit boundary: model, material, selected vectors, selected level;
  multi-sided 3D view showing both sides; Preview All Sides.
- b88wkQ_H8iE Toolpath groups: right-click Group (selected / visible), Create Empty Group, rename, new
  toolpaths join the highlighted group, ungroup keeping or deleting children, preview a group by clicking
  its name, save a group as one file named after it.
- JfxWD0_WxJo Level clipping: non-destructive, Update after moving vectors / components, works with
  clipart panels, text level, Vector Boundary of a component as the clip.
