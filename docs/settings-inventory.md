# Settings inventory: hard-coded values

An audit (2026-11-02) of the constants and magic numbers in the code that change how Cut Craft
behaves, what each one does, and where it should live. Trivial values (UI spacing shares,
picture sizes in dialogs, file-format magic numbers, ids) are left out or grouped.

Each value gets one of three calls:

- **(a) Setting**: a novice could reasonably want it different. Goes in Edit ▸ Options with a
  plain label, a one-line explanation and the current value as the safe default.
- **(b) Advanced**: a system value. Changing it can slow the computer down, use up memory, or
  make toolpaths less accurate. Goes in an **Advanced** section that stays hidden until
  *Show advanced settings* is ticked, with a warning, and is protected against edits made
  outside the app (see *Storage* below).
- **(c) Constant**: stays in the code. Maths, unit conversions, file formats, values a shader or
  GPU buffer layout depends on, or values already reachable another way.

## Already settings (no change needed)

These were hard-coded once and are already user-tunable, so they are listed only so nobody
moves them again.

| Value | Where it is tuned |
|---|---|
| Toolpath save tolerances (2D, 3D, V-carve: 0.00254 mm) | Options ▸ Toolpath Settings |
| Recent files kept (`files::RECENT_COUNT` = 4) | Options ▸ General |
| Imported model preview faces (250,000) | Options ▸ Importing |
| Snap radius, nudge distances, grid, rotate snap angles | Snapping Options (kept between runs) |
| Job defaults: clearance 5 mm, home, sizes | Job Setup ▸ Set as Default |
| Tool feeds, speeds, stepovers | Tool Database |
| Each toolpath's own defaults (stepover, passes, tabs) | the toolpath's form; tool database |
| Simulation detail (`display::STANDARD_CELLS` ×1..25) | View ▸ Preview Quality menu |
| SpaceMouse speeds (`spacemouse::*_PER_SECOND`) | Options ▸ SpaceMouse ▸ Speed |
| Post arc limit (`post::LARGEST_ARC` 1000 mm) | the post's `MAX_ARC_RADIUS` |
| STL export detail (`export-as-stl::DETAIL`) | Export as STL form, saved |

## (a) Settings: normal Options

| Value | Location | Cause and effect | Proposed setting and default |
|---|---|---|---|
| `AUTOSAVE_SECONDS` = 30 | `app/src/app.rs:378` | How often unsaved work is copied to `recovery.ccjob` for crash recovery. Lower = less lost work, more disk writes. | General ▸ **Save a recovery copy every** 30 s (10 to 600) |
| `TEXT_SCALE` = 1.15, `LINE_SCALE` = 1.5 | `app/src/app.rs:441-443` | Size of all text and controls, and line thickness. Already marked "fixed until Options has a text size". Big help on high-DPI or for poor eyesight. | Appearance ▸ **Text size** 115% (80 to 200%); line thickness follows it |
| `UI_FONTS` list | `app/src/app.rs:446` | The first installed font of Noto Sans, Open Sans, DejaVu Sans, Segoe UI, Arial is used. **Platform-specific**: Linux usually gets Noto/DejaVu, Windows gets Segoe UI. | Appearance ▸ **Interface font** (Automatic, or any of the list that is installed). Low priority |
| `flyout::HOLD_SECONDS` = 0.4 | `app/src/flyout.rs:14` | How long you hold a toolbox button before its flyout opens instead of running the tool. Slower hands may open tools by accident. | General ▸ **Hold to open a tool group** 0.4 s (0.2 to 1.5) |
| `PLAYBACK_SPEED` = 200 mm/s | `app/src/app.rs:448` | Speed a toolpath preview starts playing at (the slider changes it per run). | Toolpath Settings ▸ **Preview playback speed** 200 mm/s; keep the 5..20000 range as a constant |
| `handles::ROTATE_STEP` = 22.5°, `FINE_ROTATE_STEP` = 1° | `core/src/handles.rs` | Angles a rotate drag snaps to with Shift and Ctrl. | Snapping Options ▸ **Rotate snap angle with Shift**: 90°, 45°, 22.5° (default), 10° or 5°; **with Ctrl** 1° |
| `view3d::PICK_REACH` = 4 px | `app/src/view3d.rs:20` | How close the pointer must be to a surface to pick it in the 3D view. Marked "fixed until the 3D view has settings". | 3D View ▸ **Click reach** 4 px (1 to 20) |
| `view3d::FIELD_OF_VIEW` = 45° | `app/src/view3d.rs:23` | Perspective lens width. Wider looks more dramatic, narrower flatter. | 3D View ▸ **Perspective angle** 45° (20 to 90) |
| `view3d::AMBIENT` 0.35, `DEPTH_SHADE` 0.5, `LIGHT` | `app/src/view3d.rs:13-17` | How dark shadowed sides and deep cuts look. Marked "fixed until the 3D view has light settings". | 3D View ▸ **Shadow strength** and **Darken deep cuts** (0 to 1). Light direction stays a constant |
| `SETTLE_SECONDS` = 0.25 | `app/src/app.rs:376` | How long the 3D view must be still before it redraws at full detail. | 3D View ▸ **Wait before drawing full detail** 0.25 s; only shown with *Simplify while moving* on |
| `MESH_CELLS` = 250 | `app/src/app.rs:354` | 3D view detail while turning. Higher = smoother but laggier on slow computers. | 3D View ▸ **Detail while moving** (Low 150 / Normal 250 / High 500) |
| `BRUSH_STEP` 0.1, `BRUSH_FINE_STEP` 0.02 | `app/src/app.rs:384` | How much one wheel notch grows the sculpting brush. | Sculpting ▸ **Wheel step** 10% (fine 2%). Low priority |
| `KERN_STEP` 0.05, `LINE_STEP` 0.1 | `app/src/app/views.rs:710` | How much one wheel notch changes text letter and line spacing. | Leave as constants unless asked (very niche) |
| `checks::LEFT` = 0.1 mm | `core/src/checks.rs:13` | Material left behind that the *Checks before cutting* warn about. Marked "a setting if it proves too fussy". | Confirmation Choices ▸ **Warn when material left is over** 0.1 mm |
| `import-bitmap::MM_PER_PX` = 96 DPI | `formats/import-bitmap/src/lib.rs:11` | Size given to an imported picture. Phone photos at 96 DPI come in metres wide. | Importing ▸ **Picture size when the file doesn't say** 96 px per inch |
| `create-job-sheet::RAPID` = 5000 mm/min | `formats/create-job-sheet/src/lib.rs:18` | Rapid speed used for the job sheet time estimate. **This is really a bug**: it should use the machine's rapid rate (as Toolpath Summary does), not a new setting. | Fix: read the active machine's rapid rate; fall back to 5000 |
| `export-pdf::LINE` = 0.7 pt | `formats/export-pdf/src/lib.rs:11` | Line width in exported PDF/AI/EPS. | Better as a field on the export form. Low priority |
| sculpt `DAB_SPACING`, `DEPOSIT_RATE`, `BRUSH_DEPTH` | `commands/sculpt/src/lib.rs:111-276` | Feel of the sculpting brush. All guesses (`[inf]`). | Better as fields in the Sculpt form than global settings |
| merged toolpath `GRID_ROWS` = 8 | `commands/merged-toolpath/src/lib.rs:130` | Rows the Grid cutting order splits the job into. | Better as a field in the Merged Toolpath form |

## (b) Advanced: hidden, warned, tamper-checked

| Value | Location | Cause and effect | Risk if changed |
|---|---|---|---|
| Worker threads (`available_parallelism`, `BANDS_PER_CORE` 4, clipart preview workers = cores − 1) | `core/src/simulation.rs:16-20`, `app/src/clipart/preview.rs:458`, `formats/import-stl/src/lib.rs:105` | Simulations, previews and STL imports use every core. | **Advanced ▸ Threads to use** (Automatic, or 1..cores). Too many on a shared PC makes the rest of the desktop stutter; too few makes previews slow |
| `PARALLEL_CELLS` = 4096 | `core/src/simulation.rs:12` | Below this, work stays on one thread. | (c) really: a tuning value with no user-visible meaning. Keep constant |
| `FINE_MESH_CELLS` = 1200 | `app/src/app.rs:359` | Most 3D view detail when still. 1200² vertices is about 40 MB on the graphics card. | **Advanced ▸ Most 3D view detail** 1200 (400 to 3000). Too high runs the graphics card out of memory and can crash on integrated graphics |
| Gadget limits: 200M instructions, 64 MB, 10,000 commands | `scripting/src/lib.rs:36-42` | Stops a Lua gadget that loops forever or eats memory. | **Advanced ▸ Gadget time, memory and command limits**. Raising them lets a stuck or hostile gadget freeze the app |
| `polygon::TOLERANCE` = 0.01 mm | `core/src/polygon.rs:16` | Curves are cut into straight pieces this close before offsets and booleans. | **Advanced ▸ Curve accuracy for offsets**. Coarser = visible facets in cuts; finer = much slower offsets and pockets |
| 3D roughing `TOLERANCE` 0.01, finishing `GOUGE` 0.002, `HALVINGS` 6 | `commands/3d-roughing-toolpath/src/lib.rs:56`, `commands/3d-finishing-toolpath/src/lib.rs:31`, `core/src/toolpath.rs:1175` | How far a straight move may dip below the surface before it is split. | **Advanced ▸ 3D toolpath accuracy**. Raising it can **gouge the part**; lowering makes huge files. Strong warning |
| `simulation::REST_TOLERANCE` = 0.05 mm | `core/src/simulation.rs:622` | How much leftover material counts as "not machined" for rest machining. Marked "a setting if finishes need finer". | **Advanced ▸ Rest machining threshold** |
| `import-dxf::SPLINE_SAMPLES` = 32 | `formats/import-dxf/src/lib.rs:176` | Points per spline span on DXF import. | **Advanced ▸ DXF spline detail**. Low = faceted curves, high = heavy files |
| clipart `MOST` = 100,000 files, `RELIST` 5 s | `app/src/clipart.rs:29-31` | Clipart folders are rescanned every 5 s, and listing stops at 100k files. | **Advanced ▸ Clipart library**. A huge limit or fast rescan on a network drive hammers the disk |
| clipart preview cache: `UNUSED` 60 days, `KEEP` 400 on the card | `app/src/clipart/preview.rs:95,408` | How long cached previews last; how many stay in video memory. | **Advanced**. More = more GPU memory |
| nesting `MOST_SHEETS` 50, `MOST_FILLERS` 2000 | `commands/nest-parts/src/lib.rs:129-131` | Ceiling on sheets and filler parts one nesting makes. | **Advanced**. Raising risks very long nest runs. Could be (a) for shops cutting big batches |
| `machines::MOST_SLOTS` = 99 | `machines/src/manager.rs:341` | Most tool changer slots. Comment says raise for big machining centres. | **Advanced**. Mostly harmless; listed so pro users can lift it |
| `orientate-3d-model::SETTLED` 3 s | `commands/orientate-3d-model/src/lib.rs:34` | A model file must be untouched this long before a cached read is reused. | (c) keep constant |

## (c) Constants: stay in code

Grouped, with the reason.

- **Unit conversions and maths**: every `INCH`/`MM_PER_INCH` = 25.4, `POINTS_PER_MM`, golden
  ratio `SHRINK`, `cos 5°` values, `EDGE_ON`, `TOUCHING`, test-only `TOLERANCE`s. They are facts.
- **File formats and versions**: `file::VERSION`, `export-as-3d-clipart::VERSION`, `vfont` OLE
  markers, `fusion` zip signatures, `packed::SIZE`, every `const ID` and `FileFormat`. Changing
  them breaks reading files.
- **GPU and shader layouts**: `compute.rs` (`PARAMS_BYTES`, `MOVE_WORDS`, `WORKGROUP`,
  `MOST_GROUPS`), `gpu.rs` (`DEPTH_BITS`, `UNIFORM_BYTES`, `FACE_WORDS`). Must match the WGSL.
- **Depth add-on model inputs**: `SIDE` 518, `MEAN`, `SPREAD`, `CLASSES`, `EMBED`. Fixed by the
  trained model.
- **Algorithm internals with no user meaning**: `post::arcs::FEWEST`, `entry::PER_LOOP`,
  `entry::CORNER`, `texture::DIP_STEPS`, `keep_out::PASS_BY`, `import-stl` `ABOVE`/`STEEP`/
  `SIDE_CELLS`, `distort-object` and `vector-unwrapper` sampling, `scripting::HOOK_EVERY`,
  `view3d` wood `GRAIN_*`, `RIM`, `CYLINDER_SIDES`.
- **Layout proportions**: dialog and panel shares, `*_ROWS`, `*_SHARE`, title bar sizes,
  `GRID_GAP`, ruler ticks, `SCROLL_NOTCH`. Adjusting them is a design change, not a preference;
  the text size setting covers the real need.
- **Diagnostics**: bug report `KEPT_LINES` 200, `URL_LOG_LINES` 30, `CRASH_TITLE_CHARS`,
  SpaceMouse `RETRY` 5 s, icon preload wait 5 s.
- **Built-in tables**: post processors, materials library, machine packages, toolbox variants.
  These are data, edited through their own managers.

## Counts

| Call | Items (grouped) |
|---|---|
| Already settings | 11 |
| (a) New normal settings | 13 (plus 1 bug fix, 4 better as tool form fields) |
| (b) Advanced | 12 |
| (c) Constants | everything else (about 480 of the 543 `const`s, mostly ids and formats) |

## Storage

- Settings stay in `options.json` in the config folder: `%APPDATA%\cut-craft` on Windows,
  `$XDG_CONFIG_HOME/cut-craft` (or `~/.config/cut-craft`) on Linux, `~/Library/Application
  Support/cut-craft` on macOS (`core/src/file.rs`).
- The Advanced section carries a checksum (keyed hash). If the file was edited outside Cut
  Craft and the checksum no longer matches, the Advanced values go back to their safe defaults
  and Cut Craft says why. Normal settings stay hand-editable.
- Both sections get **Reset to defaults**.

## Plan, one area per PR

1. Settings framework: Advanced section with *Show advanced settings*, warning, tamper check,
   Reset to defaults for both sections.
2. Appearance and general: text size, autosave interval, flyout hold time.
3. (Done, #186) 3D view: click reach, perspective angle, shading, detail while moving, settle time.
   (Done: click reach, perspective angle, wait before full detail. Shading strengths go
   through the GPU uniforms and detail while moving shares cached coarse surfaces, so both
   are left for a follow-up.)
4. (Done, #187) Toolpaths and checks: preview speed, material-left warning, job sheet rapid fix.
   (The job sheet's times use Toolpath Summary's rapid rate.)
5. Snapping: rotate snap angles. (Done: Snapping Options ▸ *Rotate snap angle with Shift*
   (a choice of 90°, 45°, 22.5°, 10° or 5°: halves of a turn down to 22.5°, then every 10°
   or 5°; 22.5° by default, Matthew's call) and *with Ctrl* (1°, 0.1 to 45°). Snapping Options are now kept between runs in
   `options.json`, where before they reset at every start.)
6. Import: picture DPI, DXF spline detail. (Done: Options ▸ Importing ▸ *Picture size (pixels
   per inch)* 96 (10 to 1200), and Advanced ▸ *DXF curve detail (points per span)* 32 (4 to
   256). The Options page "Importing Models" is now "Importing". An Advanced setting left at
   its default is left out of the file, so options sealed before it existed still pass the
   tamper check; any later Advanced setting must do the same.)
7. Performance (Advanced): threads, most 3D detail, clipart and cache limits.
8. CAM accuracy (Advanced): curve accuracy, 3D accuracy, rest threshold, nesting limits.
9. Gadgets (Advanced): Lua limits.
10. Tool form fields: sculpt brush feel, merged grid rows, PDF line width.
