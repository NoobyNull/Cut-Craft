# Shared tool model and the switch to a new layout

Agreed with Matthew on 2026-11-24 (road to 1.0, item 2.3a). The window layouts (Ribbon, Rail,
Classic) stop wiring their tools up separately. One **tool model** says what the tools are;
each layout is a pair of **templates** that only decide where those tools sit on screen.
This is built in two pull requests.

## The tool model (`crates/app/src/tool_model.rs`)

Defined once, read by every layout:

- **Two kinds of tools**, named by `Page`: the **Create/Edit** tools (`Page::Design`:
  drawing, editing, arranging, modeling, importing) and the **Process** tools
  (`Page::Toolpaths`: toolpaths and the files the machine runs). The kinds say what the
  tools are, not where they sit: Ribbon puts Create/Edit across the top, Classic down the
  left. "Left for shapes, right for process" holds for Classic only.
- **Groups and their order**: `gui::Toolbar`, read through `ToolModel::groups(kind)` and
  `all_groups()` (Create/Edit then Process).
- **Sections**: each kind's sub-tabs and the groups always shown over them
  (`gui::DESIGN`, `gui::TOOLPATHS`), through `ToolModel::sections(kind)`.
- **A tool's kind**: `kind_of(id)`. Gadgets and add-on tools count as Create/Edit and are
  always shown last.
- **Favourites**: `favourites(kind)` and `every_favourite()`, in the order picked, leaving out
  anything that is no longer a tool of that kind.
- **Right-click menu**: `menu(id, place)`. Every layout puts Add to (or Remove from)
  Favourites first. What follows depends on where the button is: a grid of icons can fold
  tool groups, a tool among the favourites moves along them, and a row in a list has
  nothing more.

## Templates and layouts

A layout is a fixed pair of templates, one for each kind. People pick whole layouts only;
picking a template for each kind separately can come after 1.0, because the model already
allows it. There are no layout files from themes or add-ons before 1.0.

| Layout | Create/Edit template | Process template |
| --- | --- | --- |
| Ribbon | Tabs across the top, each group's icons in two rows | The same tabs |
| Rail | A column of group icons opening a list of tools by name | The same column |
| Classic | A page of icon grids with sub-tabs, down the left | A page with sub-tabs, on the right |

## PR 1: the shared model (no visible change)

Rail, Ribbon and Classic read groups, kinds, favourites, menus and use counts from
`ToolModel`. The parallel code they had (`all_groups`, `page_of`, `favourite_tools`,
`rail_menu`, `group_uses`, `holds_a_favourite`, the favourites filter in
`tools_or_favourites`, the page passed to `tool_menu_on`) is gone. Tests live with the module
(`tool_model/tests.rs`) and check what it gives, not how.

## PR 2: switch to the new layout

- **Ribbon is the default** for new installs. Everywhere layouts are listed, the order is
  Ribbon, Rail, Classic.
- **The Interface Layout button becomes the layout picker**: Ribbon, Rail, Classic, and
  Classic with the Design and Toolpaths sides swapped.
- **First-run setup** asks which layout to use, with a picture of each and Ribbon picked.
- **Existing installs** get the same question once after updating. Closing it keeps the
  layout they have.
- **A Did You Know tip** shows the layouts and where the picker is.
- **Help**: the Interface Layout page becomes an overview with the three pictures, and
  Ribbon, Rail and Classic each get a page of their own.
- `docs/ROADMAP.md` gains item 2.3a.

## Look and feel

The layouts borrow ways of working (a ribbon of tabs, a rail with an inspector), not another
program's screens, menus or icons. No layout, menu or help page names another program.

## Narrow windows (agreed 2026-11-24)

The Ribbon never folds a group away. When the window is too narrow for every group, the
groups go on to a second row (or more), in order, each group whole. Under each group's icons
is its name with a ▾: a click lists every tool in the group by icon and name, each with a
pin. Every tool starts pinned; unpinning one takes its icon off the Ribbon, and its group's
list keeps it (`options.ribbon_hidden`). The use counts that chose which group to fold are
gone.
