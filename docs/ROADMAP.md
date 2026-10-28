# Cut Craft roadmap

Where Cut Craft stands against Aspire 12 and the plan for the rest. Behaviour comes from the
clean-room spec (`re/SPEC.md`, from Aspire's help, resources and strings); the detailed
working / not-built list is `docs/tool-inventory.md`; how commands are built is
`docs/superpowers/specs/2026-10-23-command-architecture-design.md`.

Sizes: **S** a day or less, **M** a few days, **L** a week or more. Updated 2026-10-24 (Phase A done).

## 1. Done

| Area | What works |
|---|---|
| Foundation | Command architecture (crate per command, generated forms, undo), layers, job files, SVG / DXF import and export, job setup (flat and rotary), tool database. |
| Drawing | Rectangle, circle, ellipse, polygon, star, polyline, arc; select (click, window, crossing); move, rotate, set size, mirror, align (10) and space (4); weld, subtract, overlap, offset, join; cut / copy / paste; select all / open / duplicates / on layer. |
| Modeling | Create Shape; height-grid components with add / subtract / merge / low / multiply; zero plane, scale Z, from bitmap, from visible model, from toolpath preview, bake, smooth, reset inside / outside vectors; STL and grayscale bitmap export. |
| Toolpaths | Every strategy in the spec: profile, pocket, drilling, V-carve, V-carve inlay, inlay, quick engrave, chamfer, fluting, prism, thread milling, keyhole, drag knife, moulding, texture, photo V-carve, sketch carving, 3D roughing, 3D finishing (rest machining), rounding, laser cut / fill, laser picture, array copy, merged. Preview simulation, recalculation. |
| Output | `.pp` post engine (Aspire's vocabulary, rotary wrapping, inverse time, laser blocks); default FluidNC tool-changer posts, flat and wrapped onto A. |
| Views | 2D view (zoom, pan, rulers, toolpath drawing options); 3D view (rotate, pan, zoom, standard views, origin, modeling plane, vectors, lighting and shading toggles). |
| Menus | All but 57 menu items (listed in `docs/tool-inventory.md`); Help (about, shortcuts, licences); increment and save. |

## 2. Known issues

These shape the plan: Phase A exists to fix the first three.

1. ~~No component or bitmap selection~~ (Phase A). Components are picked in the Components
   tab only, not yet by clicking them in the 3D view.
2. ~~The selection has no order~~ (Phase A). Align still aligns to the selection's extent,
   not to the last-picked object: Aspire's help does not say what a window selection does.
3. ~~No groups~~ (Phase A). Vector groups only; they do not nest, and components and
   bitmaps cannot be grouped yet.
4. **Not tried by hand.** The 2026-10-24 work has tests but has not been used in the
   running app.
5. **Settings are not saved.** Display options last for the session (needs Options, F2).
6. **Guessed behaviour** (marked [inf] / [unk] in the spec): Fluting's Smooth curve, prism,
   photo V-carve and texture geometry, sketch-line rule. Ghidra found no code reference to
   the fluting strings; the decompile dump could still be searched by behaviour.
7. **Tool types missing:** thread mills and lasers have their settings on toolpath forms.
8. **Lasers** need a post with `LASER_SUPPORT`; Cut Craft's own FluidNC posts are not
   laser posts, on purpose (`M4` would start the router on the tool-changer machine).
9. **3D view** draws overlays over the surface (no depth test), has no ViewCube or
   perspective, and cannot tilt past side-on.

## 3. Plan

Phases are in dependency order. Within a phase, items are independent.

### Phase A — Selection foundations ✅ (2026-10-24)

Done: selection keeps pick order (`Selection`, `last()`), moulding takes the last-picked
profile; components picked in the Components tab (`chosen_component`), outlined in 2D;
bitmaps picked in the 2D view, outlined, deleted, moved and resized (Bitmap Properties),
used by the picture tools (`chosen_bitmap`); Group / Ungroup with whole-group picking.
Left for later: picking components in the 3D view, nested groups, grouping components.

| Item | Approach | Size |
|---|---|---|
| Selection order | Keep the order things were picked (a `Vec` beside the set, or the set replaced by an ordered set) so "last selected" exists. Then: align to the last selected, space between first and last with Ctrl, moulding / sweeps take the last as profile. | S |
| Component selection | Click a component in the Components tab or in the 3D view (pick by the topmost height under the cursor); selected components outlined in 2D and tinted in 3D. Selected-component commands default to it. | M |
| Bitmap selection | Bitmaps join the pickable entities: select, move, scale, delete, layer, hide. Picture tools use the selected bitmap. | M |
| Groups | Group / Ungroup (Ctrl+G / U) for vectors and components; picking a member picks the group. | M |

### Phase B — Component tools on height grids

All work on the existing `Component` height grid; each is a command crate with tests.

| Item | Aspire behaviour | Approach | Size |
|---|---|---|---|
| Replace Below | Trim or flatten the bottom off components; several are baked first. | Clamp heights below a level (to that level or to nothing). | S |
| Offset Model | 3D offset of the composite model, optionally clipped to the zero plane. | Ball dilation / erosion of the height grid (max over a disc of h + sphere profile). | S |
| Emboss | Lower a model's height keeping surface detail. | Split into smooth base and detail (blur); scale the base, keep the detail. | S |
| Split Component | Cut a component into pieces with vectors (inside / outside each). | Like Reset Inside / Outside, but each part becomes a new component. | S |
| Slice Model | Divide the model into Z slices, each a component, for parts thicker than the machine or tool allows. | Count or thickness; each slice is the model clamped to its band, shifted to zero. | S |
| Add Draft / Component with Draft | Give steep walls a minimum draft angle (for moulds); bakes the visible model. | Limit the slope: iterative distance pass so no cell is more than tan(angle) × cell below a neighbour. | M |
| Vector Boundary round components | Outline vectors round the selected components. | Marching squares on "has shape" cells, then curve fitting (shares Fit Curves). | M |
| Cross Section Vector | An open vector of the model's profile along a drawn line. | Sample heights along the line into an open polyline drawn at the line's place. | S |
| Create Texture Area | Tile one component over a region (spacing, overlap, symmetry, boundary). | Copy the grid at an array of offsets, merged, clipped to the vectors. | M |
| Distort | Bend vectors or components with an editable envelope. | Needs node editing (Phase D); then a bilinear-patch map of the envelope. | L |

### Phase C — Files, output and job data

| Item | Approach | Size |
|---|---|---|
| Import Component / 3D Model | STL first (binary and ASCII): rasterise triangles top-down into a height grid at the job's model resolution; size and position form. OBJ next; Aspire's own formats are out of scope. | M |
| Export PDF / EPS / AI | Write vectors as PDF and EPS ourselves (lines and Béziers map directly); AI as PDF-compatible. | S |
| Print / Print Preview | Render the 2D view to PDF and hand it to the system (`xdg-open` / `lp`). | S |
| Toolpath Summary | Machining time per toolpath from feeds and lengths plus a rapid rate. | S |
| Notes | A job field and a text window; opens itself when the text starts with "."; `[FILE_NOTES]` in posts already exists. | S |
| Job templates | New from Template / Save As Template: a `.ccjobt` that always saves as a new file. | S |
| Toolpath templates | Save one or all visible toolpaths' strategies and settings; Load makes them uncalculated, creating missing layers if asked. | M |
| Job Sheet | An HTML summary (job, material, toolpaths, tools, times) opened in the browser. | S |
| Save Shaded Image | Capture the 3D view (egui screenshot) to PNG. | S |
| Document Variables | Named values usable in form number fields (`=` expressions). Needs expression parsing in forms. | M |
| Import Toolpaths | Aspire's PhotoVCarve / Machinist / Cut3D toolpath files. Formats unknown. | ? |

### Phase D — Drawing tab vector tools

| Item | Approach | Size |
|---|---|---|
| Node editing (N) | Select, move, add, delete nodes; line ↔ curve; smooth; make start point; cut. The base for Distort, Trim, Fillet, Close / Extend. | L |
| Transform (T) | Interactive move / scale / rotate handles on the selection. | M |
| Trim, Close / Extend | Trim at intersections by clicking; close open vectors; extend to the nearest boundary. | M |
| Fillets | Round, dog-bone and T-bone fillets at clicked corners. | M |
| Fit Curves | Arcs, Béziers or lines fitted within a tolerance. | M |
| Validator, Measure | Find open, overlapping and self-intersecting vectors; measure distances and angles. | S each |
| Curve, Freehand | Bézier drawing with drag handles; freehand smoothed into curves. | M |
| Text, Text on a Curve, Auto Layout | Fonts from the system (TrueType / OpenType outlines via a font parsing crate; no Vectric `.vfont` files). Text on a curve and auto layout build on it. | L |
| Dimensions | Linear, angular and radius dimensions as vectors and text. | M |
| Vector Texture | Wavy or random line patterns in a region (shares Texture's generator). | S |
| Trace Bitmap | Threshold or colour regions, marching squares, Fit Curves. | M |
| Copy Along Vectors, Circular Copy, Array Copy (vectors) | Copies along curves, round a centre, in a grid. | S each |

### Phase E — Large modeling tools

| Item | Approach | Size |
|---|---|---|
| Turn and Spin | Turn a profile about the line between its ends, or spin it round a centre: heights from the profile by distance. | M |
| Extrude and Weave | A profile (or component) along one or more rails, with start / end shapes and weaving over / under at crossings. | L |
| Two Rail Sweep | Cross sections between two rails, blended along them. | L |
| Sculpting | Interactive brushes (smooth, smudge, deposit, remove, erase) on a baked component in the 3D view. | L |

### Phase F — Machine, settings and sides

| Item | Approach | Size |
|---|---|---|
| Snap Options and Guide Lines | Snap to grid, nodes, midpoints, centres, intersections, guides; guides dragged from the rulers; the View ▸ Guide Lines items. | M |
| Options | Program options (units, view, confirmations), saved to `~/.config/cut-craft/`; display settings become saved. | M |
| Materials | Material library for the 3D view's colour and texture; Toolpaths ▸ Materials. | S |
| Machine Configuration | Machine definitions (size, post, tool database, A axis) and Add Machine; Post-Processor Management. | L |
| Keep-out Zones | Closed vectors no toolpath may pass over; toolpaths refuse or detour; shown in 3D. | M |
| Draw Join Moves | Needs toolpaths to mark join moves (feed moves linking cuts without a retract). | S |
| Double-sided jobs and Swap Sides | A job with top and bottom sides, alignment, flipping. | L |
| Sheets | Several sheets per job, sheet tabs, V-carve inlay's plug sheet, nesting onto sheets. | L |
| Nesting | Fit parts onto sheets (true-shape, rotation steps, gaps, borders, priority). | L |
| Auto-Wrapping View, Shadow Shading, Tile / Cascade | Rotary 3D shown wrapped round the cylinder; shadows; 2D and 3D side by side. | M, M, S |
| 3D view: ViewCube, perspective, depth-tested overlays | A GPU paint callback (wgpu) instead of the CPU mesh. | L |

### Phase G — Gaps inside built strategies

| Item | Size |
|---|---|
| Project toolpath onto 3D model (fluting, texture, photo V-carve, laser, profile...) | M |
| Clearance tools for V-carve inlay, inlay pocket and moulding | M |
| Laser Fill Regionally; laser passes lowering the focus | S |
| Merge By Part | M |
| 3D Finishing's several-tools form (one toolpath per tool in one go) | M |
| Thread-mill and laser tool types in the tool database | M |
| Feeds and speeds per machine and material; rate units per tool | M |
| Arc output in posts (fit arcs to straight moves) | M |
| Toolpath options still missing: tabs placed by clicking or away from corners; Selected Level boundary; Z Level profile pass, avoid machined areas, reverse step; Stepover Retract; Pocket Allowance; climb / conventional (ramps, leads, tabs, raster / offset, Z level / depth first and machining boundaries are done) | M |

### Out of scope unless decided otherwise

- **Gadgets:** Aspire runs Lua scripts against its own API. Copying that API means
  recreating Vectric's scripting interface; a Cut Craft scripting interface could be
  designed instead.
- **Vendor services:** Search for Machine Online, Install Offline Package, Update
  Post-Processor Database, licensing, clipart shop.
- **Vectric file formats:** `.crv3d`, `.v3m` 3D clipart, `.vfont`; Export as 3D Clipart
  would need a format of our own.
- **Help Contents:** Aspire's help text cannot be copied; our own help would be written fresh.

## 4. Order of work

1. Phase A (selection order, component and bitmap selection, groups).
2. Phase B, then STL import from Phase C (most use of the model tools comes with imported models).
3. The small Phase C items (PDF / EPS, Toolpath Summary, Notes, templates, job sheet).
4. Phase D's node editing, then the tools built on it; text when the font source is settled.
5. Phases E and F by need; Phase G alongside, as each strategy is used.

After each phase: try it in the running app, update this file and the inventory.

## 5. Decisions needed

1. **Gadgets:** leave out, or design a Cut Craft scripting interface?
2. **Text fonts:** system fonts (OpenType outlines), or bundled open-licence fonts as well?
3. **Double-sided jobs and sheets:** needed soon for your work, or later?
4. **Machine configuration:** worth it for one machine, or keep the post plus job setup?
