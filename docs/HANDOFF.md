---
status: read
created: 2026-10-24
updated: 2026-10-25
focus: Keep building Cut Craft down docs/tool-inventory.md's "Not built yet"
---

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

## Where things are

- **Repo:** `/home/matthew/aspire-clone` (local folder still has the old name), also the private
  GitHub repo https://github.com/NoobyNull/cut-craft (`origin`, branch `main` at `00de557`,
  everything pushed, working tree clean). No Gitea remote.
- **Health:** 643 tests pass; `.githooks/pre-commit` (fmt, clippy pedantic `-D warnings`,
  nextest) is green.
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
  - `re/extracted/` and `assets/icons/` are gitignored. Icons are made locally from
    `/home/matthew/Desktop/Aspire 12.0` by `re/make_icons.py`.
  - **In the cloud the icons are missing**, so the app shows text buttons. Tests rely only on
    the tracked manifest `re/icon_set.json`.
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
