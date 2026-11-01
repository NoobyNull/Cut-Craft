---
status: read
created: 2026-10-24
updated: 2026-11-01
focus: Hand test the 2026-11-01 batch, then docs/ROADMAP.md section 9
---

## Update 2026-11-01 (version 0.4.2, `4659eb6`…`ec83b2b`, pushed with tag `v0.4.2`)

- **State:** 2204 tests pass; no open GitHub issues; `docs/courses.md` has no course gaps
  left. **No pushes until 2026-11-04** (Matthew): commit locally.
- **Landed:** text on an arc (green handles), Text in a Box stretch options, Sketch
  Carving's green areas, Create Shape's mitred corners / blend to inner vectors / E to edit
  (`Component::shape` keeps its settings and outlines), Trace Bitmap's area box and colour
  clicks, Segment 3D Model's tree and face-up list, generated forms adding the first row of
  an empty object list (`form::blank`), the 3D model drawn solid down to the modeling plane
  (`view3d::solid`, GPU only), imported models without spikes under steep sides
  (`fmt_import_stl::top_down`), Draw Material Block hiding the import outline.
- **Fixed after the release (local, not pushed):** six Model menu items named ids no command
  has, so they were greyed; they now run Texture Area, Add Draft, Offset Model, Slice,
  Split and Emboss. Found by a probe: run every menu item and list those that say "not
  implemented yet".
- **Read first:** `docs/ROADMAP.md` (state, fixes pending, missing, decisions),
  `docs/tool-inventory.md`, `docs/hand-test-3d.md`.

## Update 2026-10-26 (Windows PC "cad", `0ed6bf2`…`2f15811`, pushed to PR #1's branch)

- **Machine:** checkout at `C:\Users\CAD\Documents\cut-craft`, `core.autocrlf=false` (LF),
  hook runs under Git for Windows' bash. No Python here; Aspire 12.5 Trial is installed
  natively (`C:\Program Files\Aspire Trial Edition 12.5`), help text stripped into the
  gitignored `re/extracted/help-win/`. Windows fixes: icon paths keyed with `/`, the font
  test also accepts Arial.
- **3D modeling (the focus Matthew asked for):**
  - Component Properties' Shape / Base Height, Fade, Tilt, Bake (`Component::shaping`,
    applied as heights are read; tools that change heights bake first).
  - Move / Rotate / Mirror / Set Size / Align on selected components
    (`Change::transform_selected` takes each object's bounds; `Component::transformed`).
  - Sculpting (a stroke tool like Freehand; `Component::unsculpted` for the Undo Brush).
  - Distort Object: bounding box, along a curve, between two curves, and Create Envelope +
    Node Edit + Distort to Envelope (Coons patch), for vectors or components.
  - Levels (`Document::levels`, `Component::level`, `Edit::SetLevel`): combine, base
    height, clipping, eight mirror modes, rotary wrapping; level menu (MENU 1046), Move
    Up / Down; groups (`Component::group`, members carry it); Duplicate.
  - Rotary: Vector Unwrapper (feeds Two Rail Sweep), Orientate 3D Model's Full 3D model
    (cylindrical z-buffer), Auto-Wrapping 3D view, Material Setup's model position
    (`Job::model_position`, `Job::model_shift`, used by `placed_model`).
  - Segment 3D Model (one plane), Export / import 3D clipart (`.ccclip`, our own JSON;
    Aspire's `.3dclip` is encrypted), double-click picking in the 3D view.
- **Not tried by hand yet:** all of the above. Ask Matthew before driving his desktop.

## Update 2026-10-25 (40 local commits, `fc648d6`…`2b9c97a`, not pushed)

- **Fonts:** `.vfont` decoded from the 23 samples (layout in `re/SPEC.md`, Fonts);
  `crates/core/src/vfont.rs` reads them, `font.rs` adds system TrueType (fontdb) and layout.
  Fonts are read from `~/.config/cut-craft/fonts` (locally a link to Aspire's `Fonts/`).
  Tests use the built-in "Test Boxes" font (testing feature).
- **Built:** Draw Text (editable: `Document.texts`), Text in a Box, Text on a Curve; Fit
  Curves, Trace Bitmap, component boundary; Replace Below, Offset Model, Slice, Split,
  Emboss, Add Draft, Cross Section, Texture Area, Turn and Spin, Extrude and Weave, Two Rail
  Sweep; STL import; PDF / EPS / AI export, Print; Notes (`Document.notes`), Toolpath
  Summary, Job Sheet, job and toolpath templates; Array / Circular / Copy Along Vectors,
  Nest (box packing); Validator, Extend, Fillets, Dimensions, Trim, Measure, Vector Texture,
  Freehand, Curve, Vector Selector; node editing (`core::nodes`, N, node right-click menu);
  snapping (`core::snap`, F4); Draw Join Moves.
- **New stdlib RE helpers:** `re/cfb.py` (OLE files), `re/vfont.py`, `re/dialogs.py` (every
  DIALOG and XAML resource; dumps in `re/extracted/dialogs/`, one per tool's form).
- **Left:** see "Not built yet". Mostly big or UI-heavy: Sculpt, Distort, Transform handles,
  guides, keep-out zones, Options, document variables, machine configuration, double-sided
  jobs, sheets, true-shape nesting. Calibration against VCarve Pro under Wine still needs
  Matthew's OK.

# Handoff: Cut Craft (Rust, Vectric Aspire-style CNC CAD/CAM)

The sections below this heading are from 2026-10-24 and kept for their working notes; the
"To do next" list there is done. The current list is `docs/ROADMAP.md`.

## Where things are

- **Repo:** `/home/matthew/aspire-clone` (local folder still has the old name), also the
  public GitHub repo https://github.com/NoobyNull/cut-craft (`origin`) and Gitea
  https://git.yax.family/matthew/cut-craft (`gitea`). Push `main` to both, only when asked.
- **Health (2026-11-01):** 2204 tests pass; `.githooks/pre-commit` (fmt, clippy pedantic
  `-D warnings`, nextest) is green. Releases: the tag `vX.Y.Z` starts the Release (Linux,
  Windows) and Builds (macOS) workflows; steps in `BUILDING.md`.
- **Read these instead of this doc for detail:**
  - `docs/tool-inventory.md` — every tool: what it's built from, what works, what's not built,
    the right-click "families" plan. **The main to-do list.**
  - `docs/superpowers/specs/2026-10-23-command-architecture-design.md` — architecture,
    contract, backlog (items 1–18 done; 19 open).
  - `re/SPEC.md` — clean-room behaviour spec of Aspire 12, including the startup sequence,
    the no-job menu (MENU 128) and every right-click popup menu.
  - `git log --oneline` — 43 commits; this session's run from `edde973` to `00de557`.

## Completed this session (commit → what)

- `edde973`, `aa75bd8`, `c8cce76`: Layers. Core support, six layer commands and the Layers tab.
- `181ac8e`: Tool database (`~/.config/cut-craft/tools.json`), picker and editor.
- Preview and 3D view:
  - `11d882f`: material simulation.
  - `71f7261`: Preview / Preview All / Reset.
  - `f48c706`: orbitable lit 3D mesh view.
- 3D model:
  - `477768e`: components and combine modes.
  - `091c5e5`: Create Shape.
  - `4389b3e`: Modeling tab, model in 3D, Multiply mode, STL export.
- Toolpaths:
  - `a2355ac`: tool shapes (end mill, ball nose, V-bit).
  - `1d7e712`: V-Carve.
  - `80dc5a9`, `d5644a2`, `adaee4f`: 3D Finishing and Roughing. Shared zigzag, every move
    checked for overcut.
- `f2b4dcc`: renamed to **Cut Craft**. `.ccjob` job files; old `.ajob` files still open.
- `ce5cb7e`: desktop launcher `scripts/install-launcher` (release build + `.desktop` + icon
  `assets/cut-craft.svg`), app id `cut-craft`.
- Startup and Job Setup:
  - `045885e`: startup sequence (start page, recent files, Close).
  - `777e8d9`: form improvements (titles, radios, sections, colour, inch display).
  - `0bb4778`, `c8c33fa`: Job Setup in Aspire's sections: Z zero, XY datum + offset (moves the
    job's origin), model resolution, material colour, datum corner picker, pinned OK/Cancel.
- `7160202`: 2D viewport controls (wheel zoom, pan, Zoom Box / to Job / to Selection, F6,
  rulers).
- `fa3f2e8`: side tabs and pinnable Design / Toolpaths pages. One page out at a time; F11/F12
  layouts.
- `1cea539`: tool inventory doc, smoke test `every_enabled_control_works` (runs every enabled
  control; floor 49), and wiring of Select, Select All On Current Layer, Job Size and Position,
  Show Toolpaths Tab.
- `4380435`: tool groups are plain sections (user asked: not collapsible).
- `5e88141`: `visible` on toolpaths and components. Subordinate commands Toolpath Properties,
  Component Properties, Delete Toolpath, Delete Component.
- `00de557`: parent families **Show / Hide** and **Delete Items** (layers, toolpaths,
  components), built from those subordinates via `Change::chain_over`.

## To do next, in order

1. **Right-click menus.** Done 2026-10-24 (`1b15d6e`…`52c6f51`): layer, toolpath, component
   and 2D selection menus as `gui::MenuItem` data, with `run:` / `form:` actions; see the
   right-click section of `docs/tool-inventory.md`.
2. **Match Aspire 12, from the user's screenshots.**
   - Side tabs should be Design, Sheets, Layers, Components, Clipart. Design holds the drawing
     and modeling tools; Components is the component tree.
   - The view buttons belong at the **top right** of the 2D view and include Pan.
3. **Work down "Not built yet" in `docs/tool-inventory.md`:**
   - Edit: Cut / Copy / Paste; Select Open / Duplicate; Align L→R and the other distributes.
   - Drawing: text, node editing, fillets, array / circular copy, trim, measure, …
   - Modeling: extrude, sweep, turn, zero plane, from bitmap, smooth, …
   - Toolpath strategies: fluting, inlay, chamfer, prism, …
   - Also STL import as a component (with Aspire's Orientate 3D Model step), view toggles,
     Help windows.
   - Update the inventory as items land.
4. **Small open points:**
   - The menu launcher runs a stale release build. Re-run `scripts/install-launcher` locally.
   - Hover fly-out feel, wheel zoom and the datum picker haven't been tried by hand; they're
     only tested with synthetic events.

## How to work here (non-obvious)

- **The user's working style:**
  - Autonomous TDD cycles: failing test, then code, then the hook green, then a local commit.
  - Commit messages end with
    `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - **Push only when asked.**
  - The user steers with Aspire screenshots; match what they show.
- **Commands:**
  - One crate per command. Scaffold with
    `scripts/new-command <name> "Label" "Group" [icon]`; it also registers the command in
    `crates/app`. The type name breaks for names starting with a digit; rename it, as done for
    `3d-*`.
  - Contract: params in, `Change` out. Every crate runs `contract_tests!(Type[, doc_fn])`;
    test documents live in `cut_craft_core::testing` (`sample_document`, `two_layers`,
    `domed`, `with_toolpath`, …).
  - Parent tools cascade over subordinate crates (path dependencies) with `Change::chain` /
    `chain_over`.
- **GUI code:** no hardcoded values. Derive from the theme; when unavoidable, use a named
  constant with a `ponytail:` comment.
- **Clean room:** Vectric material never goes in git.
  - `re/extracted/` is gitignored. Since 0.2.2 every icon is an original drawing made by
    `scripts/build-icons`, kept in `assets/icons/` in git and built into the binary
    (`crates/app/build.rs`); only the Icons8 GitHub / Thingiverse link PNGs are third-party.
- **Tooling quirks:**
  - The shell is zsh: unquoted variables don't word-split; use `xargs -0` or arrays.
  - `cargo fmt` reflows code, so text patches must match the formatted source.
  - The dev profile builds at `opt-level = 1`, because the geometry is slow unoptimised.
- **Screenshots** (local only): follow the project memory `clean-room-constraints`. Capture with
  spectacle; check the window by size, or ask KWin when the size is unexpected; never `pkill -f`.

## Suggested skills

- `superpowers:test-driven-development`: every feature and fix.
- `superpowers:verification-before-completion`: before claiming anything done.
- `superpowers:brainstorming`: before big new tools (text, node editing, nesting).
- `reverse-engineer-anything` (REA): only on the local machine, where the Aspire install
  exists; static analysis only. REA uses Ghidra 12.1.2 (`/opt/ghidra`). Its `--snapshot` keeps
  only answers already given, not Ghidra's analysis; the analysed `Aspire.exe` is kept in the
  permanent Ghidra project `re/extracted/ghidra/Aspire.gpr` (gitignored).
- `ponytail`: the user's default style.
