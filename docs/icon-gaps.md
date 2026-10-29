# Aspire icons Cut Craft does not use yet

Made 2026-10-26 from `re/icon_labels.json` (every picture extracted from Aspire: 199 toolbar
strip slots, 36 icons, 243 PNGs, 364 bitmaps) against `re/icon_set.json` (the 204 traced
into `assets/icons/`) and the icon paths and command ids in `crates/`. Each item names the
help page (`re/extracted/help/`) that says how it behaves. Work down in order; tick as done.

Not gaps: the snapping, layer-visibility and Job Setup pictures are used through names
built in code; strip 35062 is a smaller copy of the toolpath strip (35070); the laser
icons are skipped (Matthew: no laser features).

## Icons whose tool exists but is not wired to them

- [x] Align: Left To Right / Right To Left / Top To Bottom / Bottom To Top
      (`drawing/align/*`): the four spacing commands, by pick order.
- [x] Create Shape: Round / Angled / Smooth / Flat (`modeling/create_shape_*`): Create Shape
      with that profile chosen (help: Create Round / Angled / Smooth Shape, Create Flat Plane).
- [x] Texture (`modeling/texture.svg`): traced from Aspire's Smooth Selected Components art
      (png/34764), so it is that tool's icon.
- [x] Component Levels (`modeling/component_levels.svg`): the Components tab.
- [x] Import Vectors / Export Vectors (`file/*`).
- [x] Edit / Preview / Hide toolpaths, Save Toolpath Template, Save All as Template
      (`toolpaths/*`): the toolpath list's buttons (help: edit-toolpath, Preview Toolpaths,
      save-toolpath-template, save-all-visible-toolpaths-as-a-template).
- [x] 2D toolpath drawing on / off / solid (`view/*`): Toolpath Drawing toggles.
- [x] Expand / Collapse (`view/*`): the component tree's levels.
- [x] Start page social links (strip 35069 slots 14–25: YouTube, Facebook, Twitter,
      Instagram, Pinterest, forum): beside the Links (needs Matthew's addresses).

## Icons whose tool is missing

- [x] Join: straight line, smooth curve, move end points (`drawing/join/*`; help:
      join-close-vector-with-a-straight-line, -with-a-smooth-curve, join-close-by-moving-end-points).
- [x] Convert Text to Curves (`drawing/create_vectors/convert_text_to_curves.svg`; help:
      Convert Text To Curves) and Edit Text (`edit_text.svg`).
- [x] Vector Boundary (`drawing/create_vectors/vector_boundary.svg`; help: Vector Boundary).
- [x] Paste in Place (`file/paste_in_place.svg`; help: paste-operation).
- [x] Open Folder / Open Clipart Folder (`file/open_folder.svg`, `open_clipart_folder.svg`).
- [x] Weld (cross) (`drawing/edit_objects/weld_cross.svg`): sits where Aspire's Align Selected
      Objects is (Edit Objects strip, between Set Size and Mirror) [inf]; the Align Objects group covers it.
- [x] Vector Texture fill (`drawing/weld_and_offset/vector_texture_fill.svg`): the fifth of the
      Weld / Subtract / Overlap / Offset strip, parts packed in a sheet: Nest Parts [inf].
- [ ] Rounded shape / span (`drawing/create_vectors/rounded_shape_span.svg`).
- [x] Engraving, Circular pocket (`toolpaths/engraving.svg`, `circular_pocket.svg`): a carved
      portrait and a ridged hole, PhotoVCarve and Thread Milling [inf].

## Tools with no icon that now have one

- Interactive Vector Trim (its path pointed at the wrong folder), Smooth Selected
  Components, Nest Parts, PhotoVCarve, Thread Milling; Keep-Out Zones and Toolpath Tiling
  added to the Toolpaths page's Output group.
- Create Component from Bitmap: Aspire's art (png/34199) traced.
- Still text, Aspire having no icon found for them: Join Vectors, Plate Production, Vector
  Unwrapper, Drag Knife, Inlay, Material Setup; Rounded shape / span is still unplaced.
