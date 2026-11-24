//! The right-click menus: toolpaths, tools, levels, groups, models, sheets and layers.

use super::{
    Combine, Command, Component, Document, EntityId, Group, Items, Layer, Level, MenuItem, Mirror,
    Toolpath, delete, show_hide,
};

/// The toolpath menu for toolpath `id`. Right-clicking a toolpath selects it,
/// so Recalculate ▸ This and Preview act on it.
pub fn toolpath_menu(doc: &Document, id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_delete_items::Which;
    let [show, hide] = show_hide(Items::Toolpaths, id.0);
    let properties = cmd_toolpath_properties::Params {
        toolpath: id,
        name: toolpath.name.clone(),
        visible: toolpath.visible,
        preview: cmd_toolpath_properties::PreviewFill::Unchanged,
        notes: None,
    };
    let no_fill = cmd_toolpath_properties::Params {
        preview: cmd_toolpath_properties::PreviewFill::NoFill,
        ..properties.clone()
    };
    vec![
        show,
        hide,
        MenuItem::form::<cmd_toolpath_properties::ToolpathProperties>("Rename", &properties),
        MenuItem::run::<cmd_toolpath_properties::ToolpathProperties>(
            "Preview Color: No Fill",
            &no_fill,
        ),
        MenuItem::run::<cmd_duplicate_toolpath::DuplicateToolpath>(
            "Duplicate",
            &cmd_duplicate_toolpath::Params { toolpath: id },
        ),
        MenuItem::submenu("Group", group_items(doc, id, toolpath)),
        MenuItem::run::<cmd_merged_toolpath::UngroupMergedToolpath>(
            "Ungroup",
            &cmd_merged_toolpath::UngroupParams { toolpath: id },
        )
        .enabled(toolpath.strategy == "merged_toolpath"),
        MenuItem::separator(),
        MenuItem::submenu(
            "Recalculate",
            vec![
                MenuItem::with_action("This", "recalculate_toolpath"),
                MenuItem::with_action("Visible", "recalculate_visible_toolpaths"),
                MenuItem::with_action("All", "recalculate_all_toolpaths"),
            ],
        ),
        MenuItem::with_action("Preview", "preview_toolpath"),
        MenuItem::submenu(
            "Copy to Sheet",
            other_sheets(doc, |sheet| {
                MenuItem::run::<cmd_copy_toolpath_to_sheet::CopyToolpathToSheet>(
                    &doc.sheets[sheet].name,
                    &cmd_copy_toolpath_to_sheet::Params {
                        toolpath: id,
                        sheet,
                    },
                )
            }),
        )
        .enabled(doc.sheets.len() > 1),
        delete(
            Items::Toolpaths,
            id.0,
            &[
                ("This", Which::This),
                ("Invisible", Which::Invisible),
                ("Visible", Which::Visible),
                ("All", Which::All),
            ],
        ),
    ]
}

/// A toolpath's Group submenu: this toolpath into a new group (inside the group it is in),
/// the shown ones into a new group, a new empty group, this toolpath into another group, or
/// out of its group one level.
pub(super) fn group_items(doc: &Document, id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_toolpath_group::{GroupParams, GroupToolpaths};
    let at = toolpath.group.clone().unwrap_or_default();
    let into = |name: &str, toolpaths: Vec<EntityId>, group: &str, inside: &str| {
        MenuItem::run::<GroupToolpaths>(
            name,
            &GroupParams {
                toolpaths,
                group: group.to_owned(),
                inside: inside.to_owned(),
            },
        )
    };
    let new = doc.next_toolpath_group_name();
    let shown = doc
        .toolpaths
        .iter()
        .filter(|(_, t)| t.visible)
        .map(|(&id, _)| id)
        .collect();
    let others: Vec<MenuItem> = doc
        .toolpath_groups_in_use()
        .iter()
        .filter(|g| toolpath.group.as_ref() != Some(g))
        .map(|g| into(&doc.toolpath_group_path(g).join(" ▸ "), vec![id], g, ""))
        .collect();
    // out of its group: into the group that one sits in, if any
    let up = toolpath
        .group
        .as_deref()
        .and_then(|g| doc.toolpath_group_parent(g))
        .unwrap_or_default();
    vec![
        into("Group This", vec![id], &new, &at),
        into("Group Visible", shown, &new, ""),
        into("Create Empty Group", vec![], &new, ""),
        MenuItem::submenu("Move to Group", others.clone()).enabled(!others.is_empty()),
        into("Remove from Group", vec![id], &up, "").enabled(toolpath.group.is_some()),
    ]
}

/// The toolpath list's menu on a group: rename it, take it apart, or delete it with its
/// toolpaths.
pub fn toolpath_group_menu(group: &str) -> Vec<MenuItem> {
    use cmd_toolpath_group::{RenameParams, RenameToolpathGroup, UngroupParams, UngroupToolpaths};
    let rename = RenameParams {
        group: group.to_owned(),
        name: group.to_owned(),
    };
    let ungroup = |delete_toolpaths| UngroupParams {
        group: group.to_owned(),
        delete_toolpaths,
    };
    vec![
        MenuItem::form::<RenameToolpathGroup>("Rename", &rename),
        MenuItem::run::<UngroupToolpaths>("Ungroup", &ungroup(false)),
        MenuItem::run::<UngroupToolpaths>("Delete Group and Its Toolpaths", &ungroup(true)),
    ]
}

/// One item for each sheet but the active one.
pub(super) fn other_sheets(doc: &Document, item: impl Fn(usize) -> MenuItem) -> Vec<MenuItem> {
    (0..doc.sheets.len())
        .filter(|&i| i != doc.active_sheet)
        .map(item)
        .collect()
}

/// The level menu for level `id`: combine mode ticked, clipping, mirror mode
/// and (rotary jobs) wrapping.
/// A level's Combine Mode submenu, in the standard order.
/// A favourites action: then the page (`design` or `toolpaths`), what to do (`add`,
/// `remove`, `earlier`, `later`) and the tool's id.
pub const FAVOURITE: &str = "favourite:";
/// Showing a page's favourites or all its tools: then the page and `true` or `false`.
pub const FAVOURITES_ONLY: &str = "favourites_only:";

/// A page as favourites actions name it.
pub fn page_key(page: crate::pages::Page) -> &'static str {
    match page {
        crate::pages::Page::Design => "design",
        crate::pages::Page::Toolpaths => "toolpaths",
    }
}

/// The page a favourites action names.
pub fn page_of(key: &str) -> Option<crate::pages::Page> {
    match key {
        "design" => Some(crate::pages::Page::Design),
        "toolpaths" => Some(crate::pages::Page::Toolpaths),
        _ => None,
    }
}

/// A tool button's menu: add it to its page's favourites or take it off; shown among the
/// favourites, move it along them too.
pub fn tool_menu(
    page: crate::pages::Page,
    id: &str,
    favourite: bool,
    among: bool,
    folded: bool,
) -> Vec<MenuItem> {
    let action = |what: &str| format!("{FAVOURITE}{}:{what}:{id}", page_key(page));
    let mut menu = vec![favourite_item(page, id, favourite)];
    if among {
        menu.push(MenuItem::with_action("Move Earlier", &action("earlier")));
        menu.push(MenuItem::with_action("Move Later", &action("later")));
    } else {
        menu.push(MenuItem::separator());
        menu.push(MenuItem {
            is_checked: folded,
            ..MenuItem::with_action(
                "Fold Tool Groups into One Button",
                &format!("{FOLD_TOOL_GROUPS}{}", !folded),
            )
        });
    }
    menu
}

/// A tool's Add to Favourites (or Remove from Favourites, when it is one) for `page`.
pub fn favourite_item(page: crate::pages::Page, id: &str, favourite: bool) -> MenuItem {
    let action = |what: &str| format!("{FAVOURITE}{}:{what}:{id}", page_key(page));
    if favourite {
        MenuItem::with_action("Remove from Favourites", &action("remove"))
    } else {
        MenuItem::with_action("Add to Favourites", &action("add"))
    }
}

/// Folding each group of tools that do one job different ways into one button, or spreading
/// them out: then `true` or `false` (Options ▸ General ▸ Fold tool groups into one button).
pub const FOLD_TOOL_GROUPS: &str = "fold_tool_groups:";

/// A tool page's tab menu: its favourites or all its tools, the one shown ticked.
pub fn favourites_menu(page: crate::pages::Page, only: bool) -> Vec<MenuItem> {
    [("Show Favourites", true), ("Show All Tools", false)]
        .into_iter()
        .map(|(name, on)| MenuItem {
            is_checked: on == only,
            ..MenuItem::with_action(name, &format!("{FAVOURITES_ONLY}{}:{on}", page_key(page)))
        })
        .collect()
}

/// The action that imports a model again from its file, followed by the component's id.
pub const REIMPORT_MODEL: &str = "reimport_model:";
/// `select_component:<id>`: the 3D view's menu lists the components under the click.
pub const SELECT_COMPONENT: &str = "select_component:";
/// A guide action: then `props:<index>`, `lock:<index>`, `delete:<index>`, or `insert:h`,
/// `insert:v` or `insert:a` (a horizontal, vertical or angled guide where the menu opened).
pub const GUIDE: &str = "guide:";
/// Save as Sculpting Brush on a component: then its id.
pub const SAVE_BRUSH: &str = "save_brush:";

/// The combine modes in the standard order, each with its icon.
pub const COMBINE_MODES: [(&str, Combine, &str); 5] = [
    ("Add", Combine::Add, "modeling/combine_add.png"),
    (
        "Subtract",
        Combine::Subtract,
        "modeling/combine_subtract.png",
    ),
    ("Merge", Combine::Merge, "modeling/combine_merge.png"),
    ("Low", Combine::Low, "modeling/combine_low.png"),
    (
        "Multiply",
        Combine::Multiply,
        "modeling/combine_multiply.png",
    ),
];

/// A level's Mirror Mode submenu, in the standard order.
pub(super) const MIRROR_MODES: [(&str, Mirror); 9] = [
    ("Off", Mirror::Off),
    ("Left to Right", Mirror::LeftToRight),
    ("Right to Left", Mirror::RightToLeft),
    ("Top to Bottom", Mirror::TopToBottom),
    ("Bottom to Top", Mirror::BottomToTop),
    ("Top Left Quadrant", Mirror::TopLeft),
    ("Top Right Quadrant", Mirror::TopRight),
    ("Bottom Left Quadrant", Mirror::BottomLeft),
    ("Bottom Right Quadrant", Mirror::BottomRight),
];

pub fn level_menu(doc: &Document, id: u64, level: &Level) -> Vec<MenuItem> {
    use cmd_level_clipping::{Action, LevelClipping};
    use cmd_level_properties::{LevelProperties, Params};
    let properties = Params::of(id, level);
    let set = |name: &str, params: Params| MenuItem::run::<LevelProperties>(name, &params);
    let mode = |name: &str, combine| {
        set(
            name,
            Params {
                combine,
                ..properties.clone()
            },
        )
        .checked(level.combine == combine)
    };
    let mirror = |name: &str, mirror| {
        set(
            name,
            Params {
                mirror,
                ..properties.clone()
            },
        )
        .checked(level.mirror == mirror)
    };
    let clip = |name: &str, action| {
        MenuItem::run::<LevelClipping>(name, &cmd_level_clipping::Params { level: id, action })
    };
    let reorder = |name: &str, direction| {
        MenuItem::run::<cmd_reorder_level::ReorderLevel>(
            name,
            &cmd_reorder_level::Params {
                level: id,
                direction,
            },
        )
    };
    let clipped = !level.clip.is_empty();
    let [show, hide] = show_hide(Items::Levels, id);
    vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        show,
        hide,
        MenuItem::with_action(
            "Select Components",
            &format!("{}{id}", crate::app::SELECT_LEVEL),
        ),
        MenuItem::submenu(
            "Clipping",
            vec![
                clip("Apply", Action::Apply).checked(clipped),
                clip("Update", Action::Update).enabled(clipped),
                clip("Remove", Action::Remove).enabled(clipped),
            ],
        ),
        MenuItem::submenu(
            "Mirror Mode",
            MIRROR_MODES
                .iter()
                .map(|&(name, m)| mirror(name, m))
                .collect(),
        ),
        set(
            "Wrapping",
            Params {
                wrapping: !level.wrapping,
                ..properties.clone()
            },
        )
        .checked(level.wrapping)
        .enabled(doc.job.rotary.is_some()),
        MenuItem::separator(),
        reorder("Move Level Up", cmd_reorder_level::Direction::Up)
            .enabled(doc.levels.range(id + 1..).next().is_some()),
        reorder("Move Level Down", cmd_reorder_level::Direction::Down)
            .enabled(doc.levels.range(..id).next().is_some()),
        MenuItem::form::<cmd_insert_level::InsertLevel>(
            "Insert New Level",
            &cmd_insert_level::InsertLevel::initial_params(doc),
        ),
        MenuItem::run::<cmd_delete_level::DeleteLevel>(
            "Delete Level",
            &cmd_delete_level::Params { level: id },
        )
        .enabled(doc.levels.len() > 1),
        MenuItem::form::<LevelProperties>("Rename Level", &properties),
        MenuItem::with_action(
            "Export as 3D Clipart",
            &format!("{}{id}", crate::app::EXPORT_LEVEL),
        )
        .enabled(!doc.level_components(id).is_empty()),
        MenuItem::separator(),
        MenuItem::form::<LevelProperties>("Properties", &properties),
    ]
}

/// A component group's menu in the tree: its combine mode ticked, Ungroup, Rename and
/// Properties.
pub fn group_menu(group: &Group) -> Vec<MenuItem> {
    use cmd_group_properties::{GroupProperties, Params};
    let properties = Params::of(group);
    let mode = |name: &str, combine| {
        let params = Params {
            combine,
            ..properties.clone()
        };
        MenuItem::run::<GroupProperties>(name, &params).checked(group.combine == combine)
    };
    vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
            "Ungroup",
            &cmd_ungroup_components::Params { group: group.id },
        ),
        MenuItem::form::<GroupProperties>("Rename", &properties),
        MenuItem::separator(),
        MenuItem::form::<GroupProperties>("Properties", &properties),
    ]
}

/// Move to Level ▸ each level, for the selected components; their level ticked.
pub(super) fn move_to_level(doc: &Document, component: &Component) -> MenuItem {
    MenuItem::submenu(
        "Move to Level",
        doc.levels
            .iter()
            .map(|(&id, l)| {
                MenuItem::run::<cmd_move_to_level::MoveToLevel>(
                    &l.name,
                    &cmd_move_to_level::Params { level: id },
                )
                .checked(component.level == id)
            })
            .collect(),
    )
}

/// The view menu's items that act on the chosen component (Duplicate, Group, Ungroup,
/// Component ▸, Move To ▸), or for vectors when there is none.
pub(super) fn component_items(
    doc: &Document,
    chosen: Option<(EntityId, &Component)>,
    layers: Vec<MenuItem>,
) -> [MenuItem; 5] {
    let vectors = !doc.selection.is_empty();
    match chosen {
        Some((id, k)) => [
            MenuItem::run::<cmd_duplicate_component::DuplicateComponent>(
                "Duplicate",
                &cmd_duplicate_component::Params { component: id },
            ),
            MenuItem::form::<cmd_group_components::GroupComponents>(
                "Group",
                &cmd_group_components::Params::default(),
            )
            .enabled(doc.selected_components.len() > 1),
            MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
                "Ungroup",
                &cmd_ungroup_components::Params {
                    group: k.group.as_ref().map_or(0, |g| g.id),
                },
            )
            .enabled(k.group.is_some()),
            {
                // shape to vector outline, with the component's own menu
                let mut items = component_menu(doc, id, k);
                items.push(MenuItem::separator());
                items.push(MenuItem::with_action(
                    "Create Vector Boundary",
                    "component_boundary",
                ));
                MenuItem::submenu("Component", items)
            },
            MenuItem {
                name: "Move To".into(),
                ..move_to_level(doc, k)
            },
        ],
        None => [
            MenuItem::with_action("Duplicate", "duplicate_component").enabled(false),
            MenuItem::with_action("Group", "group").enabled(vectors),
            MenuItem::with_action("Ungroup", "ungroup").enabled(vectors),
            MenuItem::submenu("Component", vec![]).enabled(false),
            MenuItem::submenu("Move To", layers).enabled(vectors),
        ],
    }
}

/// The component menu for component `id`, its combine mode ticked.
pub fn component_menu(doc: &Document, id: EntityId, component: &Component) -> Vec<MenuItem> {
    use cmd_component_properties::{ComponentProperties, Params};
    let properties = Params::of(id, component);
    let mode = |name: &str, combine| {
        let params = Params {
            combine,
            ..properties.clone()
        };
        MenuItem::run::<ComponentProperties>(name, &params).checked(component.combine == combine)
    };
    let [show, hide] = show_hide(Items::Components, id.0);
    let mut menu = vec![
        MenuItem::submenu(
            "Combine Mode",
            COMBINE_MODES
                .iter()
                .map(|&(name, c, icon)| mode(name, c).icon(icon))
                .collect(),
        ),
        MenuItem::separator(),
        MenuItem::run::<cmd_duplicate_component::DuplicateComponent>(
            "Duplicate",
            &cmd_duplicate_component::Params { component: id },
        ),
        MenuItem::run::<cmd_delete_component::DeleteComponent>(
            "Delete",
            &cmd_delete_component::Params { component: id },
        ),
        MenuItem::with_action("Save as Sculpting Brush…", &format!("{SAVE_BRUSH}{}", id.0)),
        move_to_level(doc, component),
        MenuItem::form::<cmd_group_components::GroupComponents>(
            "Group",
            &cmd_group_components::Params::default(),
        )
        .enabled(doc.selected_components.len() > 1),
        MenuItem::run::<cmd_ungroup_components::UngroupComponents>(
            "Ungroup",
            &cmd_ungroup_components::Params {
                group: component.group.as_ref().map_or(0, |g| g.id),
            },
        )
        .enabled(component.group.is_some()),
        MenuItem::form::<ComponentProperties>("Rename", &properties),
        show,
        hide,
        MenuItem::separator(),
        MenuItem::form::<ComponentProperties>("Properties", &properties),
    ];
    // an imported model: again from its file, to orientate or size it afresh
    if component.source.is_some() {
        let again = format!("{REIMPORT_MODEL}{}", id.0);
        menu.splice(
            0..0,
            [
                MenuItem::with_action("Re-import Model…", &again),
                MenuItem::separator(),
            ],
        );
    }
    menu
}

/// The layer menu for `layer`.
/// A menu's categories: the runs of items between separators, each drawn in a border.
pub fn categories(items: &[MenuItem]) -> Vec<&[MenuItem]> {
    items
        .split(MenuItem::is_separator)
        .filter(|run| !run.is_empty())
        .collect()
}

/// The right-click menu of sheet `sheet` in the Sheets tab.
pub fn sheet_menu(doc: &Document, sheet: usize) -> Vec<MenuItem> {
    use cmd_sheet_properties::SheetProperties;
    let Some(s) = doc.sheets.get(sheet) else {
        return vec![];
    };
    let props = cmd_sheet_properties::Params {
        sheet,
        name: s.name.clone(),
    };
    let send = |name: &str, move_vectors: bool| {
        let params = cmd_send_to_sheet::Params {
            sheet,
            move_vectors,
        };
        MenuItem::run::<cmd_send_to_sheet::SendToSheet>(name, &params)
            .enabled(sheet != doc.active_sheet && !doc.selection.is_empty())
    };
    vec![
        MenuItem::run::<cmd_activate_sheet::ActivateSheet>(
            "Activate",
            &cmd_activate_sheet::Params { sheet },
        )
        .enabled(sheet != doc.active_sheet),
        MenuItem::separator(),
        send("Copy Selected Vectors Here", false),
        send("Move Selected Vectors Here", true),
        MenuItem::separator(),
        MenuItem::form::<cmd_add_sheet::AddSheet>(
            "Insert New Sheet",
            &cmd_add_sheet::Params {
                like: Some(sheet),
                ..cmd_add_sheet::AddSheet::initial_params(doc)
            },
        ),
        // ponytail: a sheet is edited while active, in Job Setup; the Sheet Editor is that form
        MenuItem::with_action("Edit Sheet...", "job_setup").enabled(sheet == doc.active_sheet),
        MenuItem::form::<SheetProperties>("Rename", &props),
        MenuItem::run::<cmd_delete_sheet::DeleteSheet>(
            "Delete",
            &cmd_delete_sheet::Params { sheet },
        )
        .enabled(doc.sheets.len() > 1),
    ]
}

pub fn layer_menu(doc: &Document, layer: &Layer) -> Vec<MenuItem> {
    use cmd_delete_items::Which;
    use cmd_layer_properties::LayerProperties;
    let id = layer.id;
    let lock = |name: &str, locked: bool| {
        let params = cmd_layer_properties::Params {
            locked,
            ..layer.clone().into()
        };
        MenuItem::run::<LayerProperties>(name, &params).enabled(layer.locked != locked)
    };
    let [show, hide] = show_hide(Items::Layers, u64::from(id.0));
    vec![
        MenuItem::run::<cmd_set_current_layer::SetCurrentLayer>(
            "Activate",
            &cmd_set_current_layer::Params { layer: id },
        ),
        show,
        hide,
        lock("Lock", true),
        lock("Unlock", false),
        MenuItem::separator(),
        MenuItem::with_action("Insert New Layer", "add_layer"),
        delete(
            Items::Layers,
            u64::from(id.0),
            &[
                ("This", Which::This),
                ("Visible", Which::Visible),
                ("Invisible", Which::Invisible),
                ("Empty", Which::Empty),
            ],
        ),
        MenuItem::run::<cmd_delete_layer::DeleteLayer>(
            "Delete Layer",
            &cmd_delete_layer::Params { layer: id },
        )
        .enabled(doc.layers.len() > 1),
        MenuItem::form::<LayerProperties>("Rename", &layer.clone().into()),
        MenuItem::run::<cmd_merge_visible_layers::MergeVisibleLayers>(
            "Merge Visible",
            &cmd_merge_visible_layers::Params { layer: id },
        )
        .enabled(doc.layers.iter().any(|l| l.visible && l.id != id)),
        MenuItem::separator(),
        MenuItem::run::<cmd_select_layer_vectors::SelectLayerVectors>(
            "Select Layer Vectors",
            &cmd_select_layer_vectors::Params { layer: id },
        ),
        MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
            "Move Selection Here",
            &cmd_move_to_layer::Params { layer: id },
        )
        .enabled(!doc.selection.is_empty()),
    ]
}

/// The version's menu, at the far right of the status bar: Help ▸ Check for Updates and
/// About, where the version is.
pub fn version_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::with_action("Check for Updates...", "check_for_updates"),
        MenuItem::new("About Cut Craft", ""),
    ]
}
