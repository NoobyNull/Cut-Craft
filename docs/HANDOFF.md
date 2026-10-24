---
status: waiting
created: 2026-10-24
focus: Keep building Cut Craft — next the right-click menus (designed, not coded), then down the tool inventory
---

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

1. **Right-click menus.** Designed, no code yet. User's emphasis: reuse parent/subordinate
   tools, no new logic in surfaces.
   - **Menus as data:** build `gui::MenuItem` trees per click and render them with the existing
     `menu_items()` in `crates/app/src/app.rs`. Make `MenuItem::submenu` / `separator` `pub`.
   - **New actions:** `run:<command>:<json params>` applies a command with those params;
     `form:<command>:<json>` opens its form prefilled. Route both in `invoke`; in `available`
     they are enabled when the command is registered.
   - **Layer menu** (Aspire MENU 967), replacing the current one in `layers_panel`: Activate,
     Show ▸ This / Only This / All But This / All, Hide ▸ This / All, Lock / Unlock (disable
     whichever doesn't apply), Insert New Layer, Delete ▸ This / Visible / Invisible / Empty
     (`delete_items`), Rename (form `layer_properties`), Merge Visible (greyed), Select Layer
     Vectors, Move Selection Here.
   - **Toolpath menu** (968), on the toolpath list rows in `toolpaths_page`. Right-click
     selects the row.
     - Show / Hide (`show_hide`) and Rename (form `toolpath_properties`).
     - Recalculate ▸ This / Visible / All. Visible is a new built-in: `Change::chain_over` with
       `Registry::recalculate`.
     - Preview, and Delete ▸ This / Invisible / Visible / All.
   - **Component menu** (613), on `component_list` rows:
     - Combine Mode ▸ Add / Subtract / Merge / Low / Multiply (`component_properties`, tick
       the current one).
     - Show / Hide, Delete, Rename, Properties.
   - **2D selection menu** (318 plus ours), via a new `Canvas::context_menu`:
     - Undo, Redo, Delete; Cut / Copy / Paste greyed for now.
     - Move to Layer ▸ one entry per layer.
     - Weld, Subtract, Overlap, Join; Align ▸.
     - Object Properties greyed.
   - **Smoke test:** extend `every_enabled_control_works` to run every leaf of these menus.
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
  exists; static analysis only.
- `ponytail`: the user's default style.
