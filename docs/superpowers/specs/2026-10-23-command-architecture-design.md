# Command architecture — design

Date: 2026-10-23 · Status: approved in chat (crate per command, Params in / Change out,
generated forms, local pre-commit hook).

## Goals
- Every user-visible operation is a **command** in its own crate. Editing one command
  recompiles that crate and relinks; nothing else changes.
- Every command has the **same contract** (typed params in, invertible change out), so
  new commands drop in as plugins and all share undo/redo, forms, scripting and tests.
- Composite commands **call the smaller command** (cascade), never copy it (DRY).
- Test-first: every command gets the shared contract tests plus its own behaviour tests.
- rustfmt + clippy (warnings are errors) + tests run on every commit (pre-commit hook).

## Layout
```
Cargo.toml                 virtual workspace: members, shared deps, shared lints
crates/core                cut-craft-core: geometry, Document, Edit/Change, History,
                           Command trait, Registry, contract tests (feature "testing")
crates/commands/<name>     cmd-<name>: one command — Params, run(), tests
crates/app                 cut-craft binary: egui UI, icons, forms, tools, history
.githooks/pre-commit       fmt --check, clippy -D warnings, nextest
```
Command crates depend on `cut-craft-core` only (plus other command crates they cascade
from) — never on egui. The app registers every command explicitly in
`crates/app/src/commands.rs`; a test fails if a `crates/commands/*` crate is missing.

## Contract
```rust
pub trait Command: 'static {
    const ID: &'static str;          // slug of the UI label, e.g. "rectangle"
    type Params: Serialize + DeserializeOwned + JsonSchema + Default + Clone;
    fn meta() -> Meta;               // label, group, icon path, shortcut
    fn run(doc: &Document, p: &Self::Params) -> Result<Change, CommandError>;
}
```
- `Change` = ordered `Edit`s; every `Edit` stores before/after so `inverse()` is exact.
  `Document::apply` validates (ids exist / don't exist) and is the only mutation path.
- `Change::chain(doc, steps)` runs steps in order against a scratch copy, so later steps
  see earlier results; this is how cascades compose.
- New entity ids come from `doc.next_id()` (max id + 1) via a `ChangeBuilder`, so ids in
  one change never collide.
- The registry stores commands type-erased (`DynCommand`): `schema()`, `default_params()`,
  `run_json(doc, &Value)`. The app only ever uses this; it is also the plugin/JSON door.
- Params schema comes from `schemars`; the app generates the side-panel form from it
  (number/integer/bool/string/enum, `title`, `description`, `minimum`/`maximum`,
  `x-unit`). A command with no params runs immediately on click.

## Contract tests (`cut_craft_core::contract_tests!(Type)`)
default params serialize ↔ deserialize · schema is an object · `run` on the sample
document succeeds · apply then inverse restores the document exactly · id is a slug and
meta has a label.

## Document (2D first)
Job (material width/height/thickness, units mm/inch, XY datum, Z-zero), vectors
(`Path` = start + segments: Line / Arc / Cubic, closed flag), selection. Layers, sheets,
components (3D height map) and toolpaths are added by later backlog items as new `Edit`
variants — existing commands are untouched.

## Decisions
- Units: document stores **mm, f64**; inch is display-only.
- Icons: `assets/icons/<meta.icon>` loaded at runtime (generated, gitignored); text
  fallback when missing.
- Interactive drawing (drag + quick keys) is an optional `Interactive` trait in core,
  implemented inside each command crate; the app only forwards pointer/key events.

## Backlog (in order; each item = failing tests → code → hook green → commit)
Done:
1. ✅ Workspace, lints, hook; app moved into `crates/app`; placeholders deleted.
2. ✅ Core: geometry, Document, Edit/Change/History, Command/Registry, contract tests.
3. ✅ Create vectors: rectangle, circle, ellipse, polygon, star, line/polyline, arc.
4. ✅ App: registry wiring, Drawing tab from metadata + icons, generated forms, 2D view, undo/redo.
5. ✅ Edit: select all / unselect, delete, move, set size, rotate, mirror, align family (cascades).
6. ✅ Interactive: drag-to-draw; click, shift-click, window and crossing selection.
7. ✅ Files: versioned job save/open, SVG import/export, unsaved-work checks.
8. ✅ Vector ops: weld, subtract, overlap, offset, join (i_overlay in core).
9. ✅ Job setup form (forms start from the document).
10. ✅ Toolpaths: profile, pocket, drilling; toolpath panel, preview, recalculate / recalculate-all cascade.
11. ✅ G-code: generic exporter; `.pp` post-processors (`crates/post`).
12. ✅ DXF import/export (lines, arcs, bulged polylines, ellipses, splines).
13. ✅ Layers: current layer, hidden/locked layers (not drawn / not pickable), layer
    commands (add, delete, move to, select on, properties, set current), Layers tab.
14. ✅ Tool database: shared tools in `~/.config/cut-craft/tools.json`; toolpath forms
    pick from it and save back; Tool Database window edits and deletes. Toolpaths keep a copy.
15. ✅ Toolpath preview: material height grid cut by the toolpaths (Preview All = Preview
    Selected for each); orbitable 3D view of the block, drawn as a lit mesh.
16. ✅ 3D components (height grids; add / subtract / merge / low / multiply) combined into the
    model; Create Shape (round / angular / flat); Modeling tab; model in the 3D view; STL export.
17. Strategies, part 1: ✅ tool shapes (end mill, ball nose, V-bit) cut with their own profile
    in the preview; ✅ V-Carve / Engrave (concentric offsets, holes respected, flat depth);
    ✅ 3D Finishing (zigzag raster, drop cutter, adaptive refinement so straight moves stay
    within 0.01 mm of the model; refuses models taller than the material).
18. ✅ 3D Roughing (levels by pass depth, vertical allowance). Both 3D toolpaths share a
    zigzag that checks every move exactly against the model (the simulation's own
    lowest-edge maths), halving or lifting moves that would cut in.

Next:
19. ✅ Fluting; ✅ V-carve inlay (and V-Carve Start Depth); ✅ prism; ✅ texture; ✅ 3D toolpaths use the job model resolution; ✅ quick engrave; ✅ keyhole; ✅ drag knife; ✅ chamfer; ✅ thread milling; ✅ moulding; ✅ rounding; ✅ bitmaps; ✅ photo V-carve; ✅ sketch carving. Next: the other strategies.
