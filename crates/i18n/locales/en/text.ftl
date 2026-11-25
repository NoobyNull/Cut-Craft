# The app's own text with no name of its own: headings, hints and choices in its windows,
# each keyed by its English words (`text-`, its first words and a check of the whole text),
# so a key changes when its English does. Each line says where the text is.

## src/app/canvas_2d.rs
text-nothing-drawn-yet-226f94 = Nothing drawn yet
text-draw-a-shape-with-the-189188 = Draw a shape with the tools on the Design page, or drop a picture, SVG or DXF here.
text-open-a-recent-file-d3dffd = Open a recent file
text-try-a-sample-project-e547e8 = Try a sample project

## src/app/dialog_windows.rs
text-tick-each-under-checks-before-53a045 = Tick each ⚠ under Checks before cutting to say you have read it, then save

## src/app/gestures_2d.rs
text-inside-a-group-click-an-37b17e = Inside a group: click an item to move, size or change it on its own, double-click a group in it to go further in or text to edit it; Esc or a double-click on empty space steps out

## src/app/panels/layers.rs
text-move-the-active-layer-up-2a9744 = Move the active layer up the list
text-move-the-active-layer-down-a2a1a7 = Move the active layer down the list
text-hide-0c7bcd = Hide
text-show-742e5c = Show
text-unlock-799975 = Unlock
text-lock-b42d62 = Lock
text-the-layer-s-own-colour-343d43 = The layer's own colour: click to change it
text-the-usual-vector-colour-click-96fce9 = The usual vector colour: click to give the layer its own

## src/app/selection_2d.rs
text-moved-the-picture-0422c5 = Moved the picture
text-transformed-3bb93a = Transformed
text-shift-click-one-this-tool-eb969b = ; Shift-click one this tool drew to edit it
text-pan-drag-to-move-the-15e0c2 = Pan: drag to move the view
text-zoom-box-drag-a-box-3c6ab5 = Zoom Box: drag a box round what to zoom in on
text-node-editing-click-a-node-c2f7c8 = Node Editing: click a node to pick it, or drag a box round nodes (Shift adds); drag a picked node to move them; right-click for more
text-measure-drag-from-one-point-97b00b = Measure: drag from one point to another to measure, or click a vector to inspect it
text-transform-drag-a-handle-to-6c3149 = Transform: drag a handle to scale or rotate, the middle to move; click the middle for the rotation handles
text-trace-bitmap-drag-a-box-b3795b = Trace Bitmap: drag a box over the picture to trace that part, or click a colour to trace it
text-create-fillets-click-a-sharp-9b5a77 = Create Fillets: click a sharp corner to round it, or a fillet to take it out
text-tabs-click-a-selected-vector-aa7fa1 = Tabs: click a selected vector to add a tab, click a tab to take it off, drag a tab to move it
text-join-open-ends-now-click-64ed2a = Join Open Ends: now click the end to join it to (the same vector's other end closes it); Escape or a click on the first end lets it go
text-join-open-ends-click-the-72b8f9 = Join Open Ends: click the end of an open vector (each has a circle on it)
text-nothing-drawn-yet-pick-a-5be8e0 = Nothing drawn yet: pick a drawing tool on the Design page, or drop a picture, SVG or DXF on the sheet
text-click-a-vector-to-select-374182 = Click a vector to select it, Shift-click to add more, or drag a box round several (right to left takes those it touches too)
text-drag-the-selection-to-move-21f50b = Drag the selection to move it, a handle to scale or rotate; Shift-click to add or take away
text-draw-polyline-click-the-first-20a8ee = Draw Polyline: click the first point (drag for a curved span)
text-click-the-next-point-drag-024c4d = Click the next point (drag for a curved span); click the first point to close it, or double-click, right-click, Space or Escape to finish it open; Backspace takes a point back; or type the next one (20, x,y or 90A 20L) and press Enter
text-draw-rectangle-drag-from-one-e999d0 = Draw Rectangle: drag from one corner to the opposite one
text-draw-circle-drag-from-the-1c15c7 = Draw Circle: drag from the centre out to the radius
text-draw-ellipse-drag-a-box-65e661 = Draw Ellipse: drag a box for it to fit
text-draw-polygon-drag-from-the-36c6f4 = Draw Polygon: drag from the centre out to a corner
text-draw-star-drag-from-the-9b8d3d = Draw Star: drag from the centre out to a point
text-dimensions-drag-from-the-first-f12fe1 = Dimensions: drag from the first point to the second
text-draw-freehand-drag-to-draw-6cf833 = Draw Freehand: drag to draw
text-sculpting-drag-over-the-model-5af148 = Sculpting: drag over the model to work it with the brush
text-cross-section-drag-a-line-8368f2 = Cross Section: drag a line across the model
text-trim-click-a-piece-of-7e52b4 = Trim: click a piece of a vector to cut it away back to the crossings either side, or drag across pieces to cut each one
text-draw-text-click-to-place-185052 = Draw Text: click to place the text; click between two letters to bring them together (Shift: apart, Ctrl: by more); Shift-click text to edit it

## src/app/surfacing.rs
text-this-cuts-your-spoilboard-not-63183d = This cuts your spoilboard, not your material. Take everything off the bed first. Zero X and Y at the spoilboard's front-left corner, and Z on top of the spoilboard.
text-select-the-closed-outline-of-6674fc = Select the closed outline of what to flatten

## src/app/tool_sections.rs
text-not-in-the-tool-database-406cae = Not in the Tool Database: Select… one, or add it there
text-no-tool-picked-yet-select-608b98 = No tool picked yet: Select… one
text-clear-with-bigger-bits-first-124334 = Clear with bigger bits first
text-bigger-bits-take-out-most-7ff236 = Bigger bits take out most of the material first, so the Tool above only cuts what they could not reach: quicker, and easier on a small bit
text-biggest-first-the-tool-above-0f1f43 = Biggest first. The Tool above finishes: it cuts only what these left.
text-clear-the-flat-bottom-with-2c9191 = Clear the flat bottom with end mills first
text-end-mills-clear-the-flat-a6f267 = End mills clear the flat bottom first, so the V-bit only carves the slopes and what they could not reach
text-biggest-first-the-v-bit-863877 = Biggest first. The V-bit finishes: it carves only what these left.
text-bits-that-finished-before-c3f92b = Bits that finished before
text-none-the-pencil-bit-runs-ca0b90 = None: the pencil bit runs along every valley. Add the bit your 3D Finishing toolpath used, to cut only what it left
text-tools-0196fc = Tools
text-no-tool-picked-yet-add-a9a528 = No tool picked yet: Add a tool… to pick one
text-all-visible-layers-6cbebc = All visible layers
text-current-layer-only-e1bbf3 = Current layer only
text-selected-layers-d4275e = Selected layers

## src/app/views.rs
text-drop-to-open-or-import-8b9385 = Drop to open or import
text-2d-over-3d-a87e26 = 2D over 3D
text-2d-beside-3d-44dcf2 = 2D beside 3D
text-one-view-in-a-corner-89957f = One view in a corner of the other
text-bottom-side-652dcd = Bottom Side
text-top-side-3356bf = Top Side

## src/app/web_api.rs
text-web-api-running-adcc92 = WEB API Running
text-web-api-not-running-8e060b = WEB API Not Running
