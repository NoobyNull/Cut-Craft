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
crates/core                aspire-core: geometry, Document, Edit/Change, History,
                           Command trait, Registry, contract tests (feature "testing")
crates/commands/<name>     cmd-<name>: one command — Params, run(), tests
crates/app                 aspire-clone binary: egui UI, icons, forms, tools, history
.githooks/pre-commit       fmt --check, clippy -D warnings, nextest
```
Command crates depend on `aspire-core` only (plus other command crates they cascade
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

## Contract tests (`aspire_core::contract_tests!(Type)`)
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
1. Workspace, lints, hook; move existing app into `crates/app`; delete placeholders.
2. Core: geometry, Document, Edit/Change/History, Command/Registry, contract tests.
3. Create vectors: rectangle, circle, ellipse, polygon, star, line/polyline, arc.
4. App: registry wiring, Drawing tab from metadata + icons, generated forms,
   2D rendering of vectors, undo/redo (Ctrl+Z/Y).
5. Edit: select all / unselect, delete, move, set size, rotate, mirror,
   align family (centre-in-material = centre-H + centre-V cascade).
6. Interactive: drag-to-draw rectangle/circle/ellipse + quick keys; click-select,
   window/crossing select.
7. Files: save/open job (JSON), export SVG, import SVG.
8. Vector ops: join, offset, weld, subtract, intersect (polygon booleans).
9. Layers and job setup form.
10. Toolpaths: profile, pocket, drilling + tool database; preview / preview-all cascade.
11. Post-processors: `.pp` parser and G-code output.
12. DXF import/export, STL export.
13. 3D components (height maps), create shape, combine modes.
