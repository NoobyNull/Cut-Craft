# Window layout setting

Cut Craft's window is laid out as Aspire's: a Design page of icon grids on the left, a
Toolpaths page on the right, side tabs for the rest. Matthew asked for a modern layout, then
for the choice to be the user's. This spec adds one setting with three layouts. Classic stays
the default, so nothing changes for anyone who does not pick another.

## The setting

- `Options ▸ General ▸ Window layout`: Classic, Rail, or Ribbon. Kept in `options.json`
  with the rest of the options (`General.layout`, serialised `classic` / `rail` / `ribbon`,
  missing means Classic so old files still load).
- `View ▸ Window Layout` submenu with the same three choices as radio items
  (`layout_classic`, `layout_rail`, `layout_ribbon`). Picking one saves the options at once.
- The layout changes on the next frame; no restart.

## What every layout shares

- **The views**: the top bar (2D / 3D, split, sheet, layer, level, snapping) and the 2D and
  3D views are the same panel in every layout.
- **Tool search** (`Ctrl+K`, and `Edit ▸ Find Tool…`): a small window with a text field over
  the view. Every tool in the drawing, modeling and toolpath groups whose label contains the
  text (any case) is listed, unavailable ones left out, in group order. A click runs that tool;
  `Enter` runs the first; `Escape` closes. Works in Classic too.
- **The inspector**: a right panel. While a tool's form is open it shows that form with its
  Apply / Close buttons, whichever page the tool belongs to. Otherwise it shows the browser.
- **The browser**: the Sheets, Layers, Components, Clipart and Toolpath List panels that exist
  today, under a row of tabs instead of Aspire's vertical side tabs.

## Classic

Unchanged: the pages, side tabs, flyouts, pins and F11 / F12 as today. The inspector and
browser are not drawn; the tool search is the one addition.

## Rail

- **Rail**: a narrow left panel with one icon per tool group (drawing groups, then modeling,
  then toolpath groups), the first of the group's tools that has an icon standing for it
  (a group with none shows its name, cut to fit), the group name as the tooltip. Scrolls if the window is short. The open group's icon is highlighted.
- **Flyout**: clicking a rail icon opens a panel beside the rail listing that group's tools
  by icon and name, one per row, with the group name as its heading and a pin. Clicking the
  same icon again closes it. Picking a tool closes the flyout unless it is pinned.
  `Escape` closes it along with the form, as today.
- **Inspector** on the right, always drawn (browser when no form is open).
- The pages and side tabs are not drawn; F11 / F12 and the Switch Layout button do nothing
  visible here.

## Ribbon

- **Ribbon**: a top panel under the menu bar. First a row of tabs, Design, Model and
  Toolpaths, then the chosen tab's groups side by side, each its name above its tool icons
  wrapped to two rows (never narrower than its name). The strip scrolls sideways when the window is narrow. The tab is
  remembered for the session (`ribbon_tab`), starting on Design.
- **Browser** on the left, as a panel of its own.
- **Inspector** on the right, drawn only while a form is open (the browser is already on
  the left).

## Code

- `options.rs`: `WindowLayout` enum, `General.layout`.
- `app.rs`: fields `search: Option<String>`, `inspector_tab: usize`, `rail_group:
  Option<usize>`, `rail_pinned: bool`, `ribbon_tab: usize`; `INSPECTOR_TABS`,
  `RIBBON_TABS`.
- `app/layouts.rs` (new): `tool_search`, `search_matches`, `inspector`, `browser`,
  `tool_rows`, `rail_layout`, `ribbon_layout`, `all_groups`, `layout_checked`, `set_layout`.
- `views.rs` `draw_window`: `Screen::Job` matches on the layout; `Screen::NewJob` keeps its
  left panel in every layout.
- `gui.rs`: the View submenu and the Edit item.
- `actions.rs`: `available` and `invoke` arms for the three layout ids and `find_tool`;
  `Ctrl+K` in `shortcuts`.

## Tests (headless harness)

- The default layout is Classic; `layout: "rail"` round-trips; a file without `layout` loads.
- `set_layout("layout_rail")` makes `layout_checked` tick Rail only.
- `search_matches("circ")` lists Draw Circle and nothing from the toolpath groups that does
  not match; an unavailable tool is left out.
- Rail: a frame with `rail_group = Some(index of "Create Vectors")` draws "Draw Circle";
  picking a tool from an unpinned flyout closes it.
- Ribbon: a frame draws the three tab names and the Design groups' names; the Toolpaths tab
  draws "Toolpath Operations".
- A form opened in Rail or Ribbon is drawn (its label) without the Design page.

## Out of scope

A selection-properties tab, the view toolbar as pills over the view, drawn group icons of
their own, remembering the flyout and inspector widths beyond what egui keeps, and the
Modeling tab Aspire has (its groups stay in Design's list, as today).
