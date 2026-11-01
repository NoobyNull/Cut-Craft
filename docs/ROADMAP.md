# Cut Craft roadmap

Where Cut Craft stands against Aspire 12, what is still to do, and what is causing trouble.
Updated **2026-11-01**, at version **0.4.2** (tag `v0.4.2`, commit `ec83b2b`).

Behaviour comes from the clean-room spec (`re/SPEC.md`: Aspire's help, resources and
strings, and black-box comparison under Wine). The tool-by-tool list is
`docs/tool-inventory.md`. How commands are built is
`docs/superpowers/specs/2026-10-23-command-architecture-design.md`. The training-course
check is `docs/courses.md` (made by `scripts/courses-report.py`).

Sizes: **S** a day or less, **M** a few days, **L** a week or more.

## 1. State at a glance

| Measure | Now |
|---|---|
| Tests | 2204 pass (`.githooks/pre-commit`: fmt, clippy pedantic `-D warnings`, nextest) |
| GitHub issues | 0 open, 99 closed |
| Course capabilities (`docs/courses.md`) | 139: 135 yes, 0 partly, 0 no, 4 out of scope |
| Menu items that say "not implemented yet" | 10 (section 4) |
| `ponytail:` stand-ins in the code | 171 (`grep -rn "ponytail:" crates`) |
| Guessed behaviour in `re/SPEC.md` | 58 `[inf]`, 14 `[unk]` |
| Pushes | **None until 2026-11-04** (Matthew, 2026-11-01). Commit locally until then. |

## 2. Done

Phases A to F of the 2026-10-24 plan are done, and most of G. In short:

| Area | What works |
|---|---|
| Foundation | Command crates (params in, change out), generated forms, undo, layers, sheets, double-sided and rotary jobs, levels, groups, document variables, packed job files, atomic saves and crash recovery, Options kept between runs, mm / inch switch. |
| Design (2D) | Every drawing tool, text (TrueType and single-line, text in a box with stretch, text on a curve, text on an arc, spelling, kerning and line spacing by clicking), node editing, transform handles, trim, fillets, join / close, offsets, booleans, copies, nesting (true shape), trace bitmap (black and white, colour, area), guides, snapping, dimensions, measure, validator, keep-out zones. |
| Models (3D) | Create Shape (all profiles, custom, limit, scale, total height, mitred corners, blend to inner vectors, E to edit), extrude and weave, two-rail sweep, turn and spin, sculpting, distort, texture areas, offset, emboss, slice, split, draft, replace below, bake, levels with clipping and mirroring, STL / OBJ import with orientation and re-import, segment 3D model (tree, face up), 3D clipart, the 3D view editing with handles, a solid model view. |
| Toolpaths | Every strategy in `re/SPEC.md`, rest machining, tabs / ramps / leads, clearance tools, project onto 3D model, animated preview, tiling, templates, groups, merging, plate production. Grid kernels run across the cores and on the GPU. |
| Machines and output | Tool database (`.vtdb` and Fusion 360 import, cutting data per material, tool changer slots), machine configuration, post-processor management, Cut Craft's own GRBL / FluidNC / Mach3 posts, machine limits, toolpath summary, job sheets. |
| Releases | Linux, Windows and macOS packages from the tag workflows; AppImage and Flatpak. |

## 3. Fixes pending

Things that exist but do not behave as they should.

1. **The 3D view's CPU fallback draws the model as a skin.** With a GPU the model is solid
   down to the modeling plane (`view3d::solid`); without one (and in the headless tests) the
   walls are left off, because the painter draws in order and the walls would show through.
2. **The modeling plane is always drawn while a model is imported**, even with View ▸ Draw
   Modeling Plane off. Draw Material Block is in the View menu only, not in the 3D view's
   right-click menu.
3. **Double-sided jobs:** Zero off same side is kept but the posts do not use it yet. An
   animated preview plays this side only.
4. **Display settings:** the 3D view toggles are saved with Options; the Toolpath Drawing
   toggles (rapids, plunges, retracts, joins) last only for the session.

## 4. Missing: still to build

### From Aspire's menus (greyed in Cut Craft)

| Item | Note | Size |
|---|---|---|
| Edit ▸ Object Properties | the selection's own properties form | S |
| Move to Group (component right-click) | move a component into a component group | S |
| Copy to Sheet (selection right-click) | copy vectors to another sheet (Move / Copy toolpath to sheet exist) | S |
| View ▸ Draw Keep-Out Zones in 3D View | the zones drawn over the 3D view | S |
| View ▸ Use Shadow Shading | shadows in the 3D view | M |
| Window ▸ Tile Horizontal / Vertical, Cascade | the 2D and 3D views side by side exists (split); these three do not | S |
| Toolpaths ▸ Import Toolpaths | Aspire's PhotoVCarve / Machinist / Cut3D toolpath files; format unknown | ? |
| Machine ▸ Update Post Processor Database | a vendor service; out of scope unless decided otherwise | — |

### Inside tools that exist

| Tool | What is left | Size |
|---|---|---|
| Draw Text, text on an arc | the red (rotate), blue (radius) and white (move) handles; kerning clicks on arched text | M |
| Draw Text | TrueType kerning (`kern` / `GPOS` tables); line spacing is a guess | M |
| Trace Bitmap | colour swatches with tick boxes in the form (colours are picked on the picture or by number) | S |
| Segment 3D Model | the plane placed with handles in the 3D view; merging segments by drag and drop; undercuts painted pink (they are listed); Import Options (backface caps, resize material, boundary vectors) | L |
| Sketch Carving | the Selected Level boundary | S |
| Create Shape | its own live 3D handles (the 3D view's height handles work on it) | M |
| Split Component | open vectors extended to the edges (closed vectors only now) | S |
| Groups | nested groups; a group's own layer | M |
| Cutting data | per machine (it is per material now); feeds on toolpath forms in the tool's own units | S |
| Posts | Lua-scripted posts | out of scope |
| Lasers | not planned: Matthew skips laser features (no laser tool type, Fill Regionally, focus passes) | — |

## 5. Not tried by hand

The work below has tests but has not been used in the running app. `docs/hand-test-3d.md`
has the checklists.

- 2026-11-01: text on an arc and its green handles; Text in a Box stretch; Sketch Carving's
  green areas; Create Shape's mitred corners, blend to inner vectors and E to edit; Trace
  Bitmap's area box and colour clicks; the segment tree and face-up list; the solid model
  view and the steep-side spike fix (on a real relief); Draw Material Block in 3D.
- Earlier batches are in `docs/hand-test-3d.md` and `docs/tool-inventory.md`.

## 6. Guessed behaviour

Marked `[inf]` / `[unk]` in `re/SPEC.md`; each needs a black-box comparison with Aspire
under Wine (`~/.wine-aspire`) before it can be called exact.

- Fluting's Smooth curve; prism, photo V-carve and texture geometry; sketch-line contrast
  rule; Emboss's split into shape and detail; Trace Bitmap's Corner Fit scale.
- 2026-11-01 additions: the arc handle mapping (the middle of the text rises as far as the
  drag), the box stretch arithmetic, mitred corners as the farther side's line, blend as the
  share of the way from the outer to the inner vectors, the segment naming (2.1 beyond the
  plane), the steep-face limit (a face whose area seen from above is under a tenth of its
  own) and the 4-cell raise.

## 7. Decisions needed

1. **Gadgets:** leave them out, or design a Cut Craft scripting interface?
2. **Import Toolpaths:** worth reverse engineering the formats, or leave out?
3. **Segment 3D Model handles:** build Aspire's 3D plane handles and drag-and-drop tree, or
   keep the form lists?

Settled since 2026-10-24: system and single-line fonts both; double-sided jobs and sheets
built; machine configuration built; no laser features.

## 8. Out of scope unless decided otherwise

- **Gadgets:** Aspire runs Lua scripts against its own API (the rotary layouts and Celtic
  Weave are built as tools instead).
- **Vendor services:** Search for Machine Online, Update Post-Processor Database,
  licensing, the clipart shop.
- **Vectric file formats:** `.crv3d`, `.v3m`, `.3dclip` (encrypted); Cut Craft uses `.ccjob`
  and `.ccclip`.
- **Lasers.**

## 9. Order of work

1. Hand test the 2026-11-01 batch; fix what it finds.
2. The small missing items (Object Properties, Move to Group, Copy to Sheet, keep-out zones
   in 3D, window tiling, Selected Level boundary, Trace Bitmap swatches).
3. The larger in-tool gaps (arc handles, TrueType kerning, segment handles) by need.
4. After each batch: update this file, `docs/tool-inventory.md` and `docs/courses-capabilities.tsv`.
