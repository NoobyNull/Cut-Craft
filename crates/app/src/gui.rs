//! Static UI data: Drawing tab tool groups, the main menu, right-click menus, status bar text.

use cmd_space_horizontally::Order as SpaceH;
use cmd_space_vertically::Order as SpaceV;
use cut_craft_core::Command;
use cut_craft_core::component::{Combine, Component, Group, Level, Mirror};
use cut_craft_core::document::{Document, EntityId, Layer};
use cut_craft_core::params::Items;
use cut_craft_core::toolpath::Toolpath;

/// Drawing tab layout: groups of command ids. Implemented commands supply their own label and
/// icon; the rest show their humanized id, disabled.
pub struct Toolbar {
    pub groups: Vec<ToolGroup>,
    /// The Toolpaths panel, laid out the same way.
    pub toolpath_groups: Vec<ToolGroup>,
    /// The Modeling tab.
    pub modeling_groups: Vec<ToolGroup>,
}

#[derive(Clone, Copy)]
pub struct ToolGroup {
    pub name: &'static str,
    pub tools: &'static [&'static str],
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            groups: drawing_groups(),
            toolpath_groups: toolpath_groups(),
            modeling_groups: modeling_groups(),
        }
    }
}

fn group(name: &'static str, tools: &'static [&'static str]) -> ToolGroup {
    ToolGroup { name, tools }
}

/// Aspire's Drawing tab. Group names come from `Resources_ENU.dll` (strings 130-945, see
/// `re/SPEC.md`); which tool sits in which group is inferred from the help topics. Ids match
/// the icon file names, so unimplemented tools still show their icon.
#[expect(clippy::too_many_lines, reason = "the drawing tab's tool list")]
fn drawing_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "File Operations",
            &[
                "new",
                "open",
                "save",
                "import_vectors_bitmap",
                "import_vectors",
                "export_vectors",
                "paste_in_place",
                "import_component_3d_model",
                "open_folder",
                "job_setup",
            ],
        ),
        group(
            "2D View Control",
            &[
                "pan",
                "zoom_box",
                "zoom_to_job",
                "zoom_selected",
                "interface_layout",
            ],
        ),
        group(
            "Create Vectors",
            &[
                "draw_polyline",
                "draw_rectangle",
                "draw_rectangle_from_center",
                "draw_circle",
                "draw_circle_from_edge",
                "draw_ellipse",
                "draw_polygon",
                "draw_star",
                "draw_arc",
                "draw_curve",
                "draw_freehand",
                "draw_text",
                "edit_text",
                "convert_text_to_curves",
                "text_on_a_curve",
                "auto_layout_text",
                "dimensions",
                "create_fillets",
                "vector_boundary",
                "vector_texture",
                "trace_bitmap",
            ],
        ),
        group(
            "Edit Objects",
            &[
                "select",
                "node_edit",
                "transform",
                "group",
                "join_vectors",
                "join_straight_line",
                "join_smooth_curve",
                "join_move_end_points",
                "weld_vectors",
                "subtract_vectors",
                "overlap_intersect",
                "interactive_vector_trim",
                "trim_objects",
                "close_extend_vector",
                "fit_curves_to_vectors",
                "vector_validator",
                "measure_inspect",
                "edit_picture",
                "crop_bitmap",
            ],
        ),
        // Aspire's Align Objects section (help: Alignment Tools): the common alignments
        group(
            "Align Objects",
            &[
                "center_in_material",
                "center_horizontally_in_material",
                "center_vertically_in_material",
                "center",
                "align_left",
                "align_right",
                "align_top",
                "align_bottom",
                "center_horizontally",
                "center_vertically",
                "left_to_right",
                "right_to_left",
                "top_to_bottom",
                "bottom_to_top",
            ],
        ),
        group(
            "Transform Objects",
            &["move", "rotate", "set_size", "mirror", "distort_object"],
        ),
        group(
            "Offset and Layout",
            &["offset_vectors", "nest_parts", "plate_production"],
        ),
        group(
            "Array Objects",
            &["array_copy", "circular_copy", "copy_along_vectors"],
        ),
    ]
}

/// The Modeling tab, laid out the same way.
fn modeling_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "Shape Creation",
            &[
                "create_shape_round",
                "create_shape_angled",
                "create_shape_smooth",
                "create_shape_flat",
                "extrude_and_weave",
                "two_rail_sweep",
                "vector_unwrapper",
                "turn_and_spin",
                "texture_area",
                "add_zero_plane",
                "create_component_from_bitmap",
            ],
        ),
        group(
            "Shape Editing",
            &[
                "sculpt",
                "smooth_selected_components",
                "emboss",
                "distort_object",
                "offset_model",
                "slice",
                "add_draft",
                "scale_model_z_height",
                "replace_below",
                "split",
                "bake_components",
                "component_levels",
            ],
        ),
    ]
}

/// The Toolpaths panel, laid out the same way.
fn toolpath_groups() -> Vec<ToolGroup> {
    vec![
        group("Material", &["material_setup", "materials"]),
        group(
            "Toolpath Operations",
            &[
                "profile_toolpath",
                "pocket_toolpath",
                "drilling_toolpath",
                "drill_bank_toolpath",
                "quick_engrave_toolpath",
                "vcarve_engrave_toolpath",
                "vcarve_inlay_toolpath",
                "inlay_toolpath",
                "chamfer_toolpath",
                "fluting_toolpath",
                "prism_toolpath",
                "thread_milling_toolpath",
                "keyhole_toolpath",
                "drag_knife_toolpath",
                "moulding_toolpath",
                "texture_toolpath",
                "photo_vcarve_toolpath",
                "sketch_carving_toolpath",
            ],
        ),
        group(
            "3D Toolpaths",
            &[
                "3d_roughing_toolpath",
                "3d_finishing_toolpath",
                "rounding_toolpath",
            ],
        ),
        group(
            "Laser",
            &["laser_cut_fill_toolpath", "laser_picture_toolpath"],
        ),
        group(
            "Toolpath Templates",
            &["array_copy_toolpath", "merged_toolpath"],
        ),
        group(
            "Output",
            &[
                "save_toolpaths",
                "toolpath_summary",
                "tool_database",
                "keep_out_zones",
                "toolpath_tiling",
            ],
        ),
    ]
}

/// `"join_vectors"` → `"Join Vectors"`: the label of a tool that is not implemented yet.
pub fn title_case(id: &str) -> String {
    id.split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Status bar text
mod abouts {
    include!(concat!(env!("OUT_DIR"), "/abouts.rs"));
}

/// What command `id` does (its crate's first sentence), for its tooltip and the status bar.
pub fn about(id: &str) -> Option<&'static str> {
    abouts::ABOUTS
        .iter()
        .find(|(command, _)| *command == id)
        .map(|(_, about)| *about)
}

pub struct StatusBar {
    pub message: String,
    pub current_tool: String,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            message: "Ready".to_string(),
            current_tool: "Select".to_string(),
        }
    }

    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }
}

/// Menu system for application controls
pub struct MenuSystem {
    pub menus: Vec<Menu>,
}

/// The node-editing popup for the node or span at `at` (Aspire's popup menus 142, 317, 635).
/// Edit Polyline's action: drawing on from an open vector's end node, `edit_polyline:` then
/// the vector's id and `start` or `end`.
pub const EDIT_POLYLINE: &str = "edit_polyline:";

pub fn node_menu(doc: &Document, at: &cut_craft_core::nodes::NodeAt) -> Vec<MenuItem> {
    // Node Position starts at the node itself, not at the click near it
    let node = at
        .path(doc)
        .ok()
        .and_then(|path| Some(cut_craft_core::nodes::nodes(path)[at.node(path).ok()?]))
        .map_or(at.at, |p| cut_craft_core::params::Xy::new(p.x, p.y));
    let mut items = vec![
        MenuItem::run::<cmd_insert_point::InsertPoint>("Insert Point", at),
        MenuItem::run::<cmd_delete_point::DeletePoint>("Delete Point", at),
        MenuItem::run::<cmd_smooth_point::SmoothPoint>("Smooth Point", at),
        MenuItem::form::<cmd_move_node::MoveNode>(
            "Node Position...",
            &cmd_move_node::Params {
                from: at.clone(),
                to: node,
            },
        ),
        MenuItem::separator(),
        MenuItem::run::<cmd_convert_span::SpanToLine>("Convert Span to Line", at),
        MenuItem::run::<cmd_convert_span::SpanToBezier>("Convert Span to Bezier", at),
        MenuItem::run::<cmd_convert_span::SpanToArc>("Convert Span to Arc", at),
        MenuItem::separator(),
        MenuItem::run::<cmd_cut_vector::CutVector>("Cut Vector", at),
        MenuItem::run::<cmd_make_start_point::MakeStartPoint>("Make Start Point", at),
        MenuItem::run::<cmd_reverse_direction::ReverseDirection>("Reverse Direction", at),
        MenuItem::run::<cmd_close_with_line::CloseWithLine>("Close With a Line", at),
    ];
    // an open vector's end node draws on from there
    let end = at.path(doc).ok().filter(|p| !p.closed).and_then(|path| {
        let last = cut_craft_core::nodes::nodes(path).len().checked_sub(1)?;
        match at.node(path).ok()? {
            0 => Some("start"),
            n if n == last => Some("end"),
            _ => None,
        }
    });
    if let Some(end) = end {
        items.push(MenuItem::with_action(
            "Edit Polyline",
            &format!("{EDIT_POLYLINE}{}:{end}", at.vector.0),
        ));
    }
    items.extend([
        MenuItem::separator(),
        MenuItem::with_action("Exit Node Editing", "node_edit"),
    ]);
    items
}

impl MenuSystem {
    fn menu(&self, name: &str) -> &Menu {
        self.menus
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("there is a {name} menu"))
    }

    /// Edit ▸ `name`, reused as it is.
    fn edit(&self, name: &str) -> MenuItem {
        let items = &self.menu("Edit").items;
        let item = items.iter().find(|i| i.name == name);
        item.cloned()
            .unwrap_or_else(|| panic!("there is an Edit ▸ {name}"))
    }

    /// The 3D view's right-click menu: Aspire 12's 2D/3D view menu (help: Right Mouse Click
    /// Menu) headed by the components `under` the click, topmost first and the selected
    /// ticked (a click picks that one, so a buried component can be picked); its vector
    /// items greyed while no vector is selected, and Component ▸ the chosen component's own
    /// menu.
    pub fn view_3d_menu(&self, doc: &Document, under: &[EntityId]) -> Vec<MenuItem> {
        let edit = |name: &str| self.edit(name);
        let chosen = doc
            .selected_components
            .iter()
            .rev()
            .find(|id| doc.components.contains_key(id))
            .map(|&id| (id, &doc.components[&id]));
        let vectors = !doc.selection.is_empty();
        let text = doc.selection.iter().any(|id| {
            doc.vector_groups
                .get(id)
                .is_some_and(|g| doc.texts.contains_key(g))
        });
        let layers = doc.layers.iter().map(|l| {
            MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
                &l.name,
                &cmd_move_to_layer::Params { layer: l.id },
            )
        });
        let mut menu: Vec<MenuItem> = under
            .iter()
            .filter_map(|id| {
                let name = &doc.components.get(id)?.name;
                let pick = format!("{SELECT_COMPONENT}{}", id.0);
                Some(
                    MenuItem::with_action(name, &pick)
                        .checked(doc.selected_components.contains(id)),
                )
            })
            .collect();
        if !menu.is_empty() {
            menu.push(MenuItem::separator());
        }
        let [duplicate, group, ungroup, component, move_to] =
            component_items(doc, chosen, layers.collect());
        menu.extend([
            edit("Cut"),
            edit("Copy"),
            edit("Paste"),
            duplicate,
            MenuItem::separator(),
            edit("Delete"),
            MenuItem::separator(),
            group,
            ungroup,
            MenuItem::separator(),
            MenuItem {
                name: "Select".into(),
                ..edit("Selection")
            },
            MenuItem::with_action("Activate Layer of Selection", "activate_layer_of_selection")
                .enabled(vectors),
            MenuItem::separator(),
            // ponytail: Aspire's Text ▸ also removes text from a curve and breaks a block
            // into lines; neither exists yet
            MenuItem::submenu(
                "Text",
                vec![MenuItem::with_action(
                    "Convert to Vectors",
                    "convert_text_to_curves",
                )],
            )
            .enabled(text),
            MenuItem::with_action("Convert To Vectors", "convert_text_to_curves").enabled(text),
            MenuItem::separator(),
            MenuItem::form::<cmd_join_close_vectors::JoinCloseVectors>(
                "Join / Close Vectors...",
                &cmd_join_close_vectors::Params::default(),
            )
            .enabled(vectors),
            MenuItem::with_action("Node Edit Mode", "node_edit").enabled(vectors),
            MenuItem::separator(),
            // Appearance fades a picture's greyscale in the 2D view: Bitmap Properties
            MenuItem::with_action("Appearance", "bitmap_properties")
                .enabled(!doc.selected_bitmaps.is_empty()),
            component,
            MenuItem::submenu(
                "Copy To",
                vec![
                    MenuItem::with_action("Other Side", "copy_to_other_side")
                        .enabled(doc.job.double_sided.is_some()),
                ],
            ),
            move_to,
        ]);
        menu
    }

    /// The 2D view's right-click menu: Aspire's selection menu (MENU 318), plus Move to Layer,
    /// the vector booleans and Align. Edit menu items are reused as they are.
    pub fn selection_menu(&self, doc: &Document) -> Vec<MenuItem> {
        let edit = |name: &str| self.edit(name);
        let layers = doc.layers.iter().map(|l| {
            MenuItem::run::<cmd_move_to_layer::MoveToLayer>(
                &l.name,
                &cmd_move_to_layer::Params { layer: l.id },
            )
        });
        vec![
            edit("Undo"),
            edit("Redo"),
            MenuItem::separator(),
            edit("Cut"),
            edit("Copy"),
            edit("Paste"),
            edit("Delete"),
            MenuItem::separator(),
            MenuItem::submenu("Move to Layer", layers.collect()),
            MenuItem::with_action("Copy to Other Side", "copy_to_other_side")
                .enabled(doc.job.double_sided.is_some()),
            MenuItem::separator(),
            MenuItem::with_action("Weld", "weld_vectors"),
            MenuItem::with_action("Subtract", "subtract_vectors"),
            MenuItem::with_action("Overlap", "overlap_intersect"),
            edit("Join Vectors"),
            MenuItem::with_action("Group", "group"),
            MenuItem::submenu(
                "Ungroup",
                vec![
                    MenuItem::with_action("Ungroup onto Original Object Layers", "ungroup"),
                    MenuItem::with_action(
                        "Ungroup onto Current Layer",
                        "ungroup_onto_current_layer",
                    ),
                ],
            ),
            MenuItem {
                name: "Align".into(),
                ..edit("Align Selected Objects")
            },
            MenuItem::submenu("Text", text_items(doc)),
            MenuItem::separator(),
            MenuItem::with_action("Object Properties", "object_properties"),
            MenuItem::with_action("Bitmap Properties", "bitmap_properties")
                .enabled(!doc.selected_bitmaps.is_empty()),
            MenuItem::separator(),
            guides_submenu(),
        ]
    }

    /// Every command item, submenus flattened.
    pub fn items(&self) -> Vec<&MenuItem> {
        self.menus.iter().flat_map(|m| leaves(&m.items)).collect()
    }
}

/// The 2D view's menu on a guide: its properties, locking it, deleting it.
pub fn guide_menu(index: usize, locked: bool) -> Vec<MenuItem> {
    vec![
        MenuItem::with_action("Guide Properties…", &format!("{GUIDE}props:{index}")),
        MenuItem::with_action(
            if locked { "Unlock Guide" } else { "Lock Guide" },
            &format!("{GUIDE}lock:{index}"),
        ),
        MenuItem::with_action("Delete Guide", &format!("{GUIDE}delete:{index}")),
    ]
}

/// The 2D view's Guidelines submenu: a guide inserted where the menu opened.
pub fn guides_submenu() -> MenuItem {
    MenuItem::submenu(
        "Guidelines",
        vec![
            MenuItem::with_action("Insert Horizontal Guide", &format!("{GUIDE}insert:h")),
            MenuItem::with_action("Insert Vertical Guide", &format!("{GUIDE}insert:v")),
            MenuItem::with_action("Insert Angled Guide…", &format!("{GUIDE}insert:a")),
        ],
    )
}

/// The selection's Text submenu: editing the selected text, and breaking a block of lines
/// into a text per line.
fn text_items(doc: &Document) -> Vec<MenuItem> {
    let group = cmd_draw_text::selected_text_group(doc);
    let lines = group
        .and_then(|g| doc.texts.get(&g))
        .and_then(|t| t["text"].as_str())
        .map_or(0, |t| t.lines().count());
    vec![
        MenuItem::with_action("Edit Text", "edit_text"),
        MenuItem::run::<cmd_draw_text::BreakTextIntoLines>(
            "Break Block into Lines",
            &cmd_draw_text::BreakParams {
                group: group.unwrap_or(0),
            },
        )
        .enabled(lines > 1),
    ]
}

/// Every command item of `items`, submenus flattened.
pub fn leaves(items: &[MenuItem]) -> Vec<&MenuItem> {
    items
        .iter()
        .flat_map(|item| {
            if item.children.is_empty() {
                vec![item]
            } else {
                leaves(&item.children)
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct Menu {
    pub name: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub name: String,
    pub action: String,
    pub shortcut: String,
    pub is_enabled: bool,
    pub is_checked: bool,
    /// Non-empty for a submenu
    pub children: Vec<MenuItem>,
    /// Its picture, an icon path, where Aspire's menu has one (the combine modes).
    pub icon: Option<String>,
}

impl MenuItem {
    /// Command item; the action id is derived from the name (`"Join Vectors"` -> `"join_vectors"`).
    fn new(name: &str, shortcut: &str) -> Self {
        let words: Vec<String> = name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(String::from)
            .collect();
        Self {
            name: name.to_string(),
            action: words.join("_"),
            shortcut: shortcut.to_string(),
            is_enabled: true,
            is_checked: false,
            children: vec![],
            icon: None,
        }
    }

    /// With a picture.
    #[must_use]
    pub fn icon(self, path: &str) -> Self {
        Self {
            icon: Some(path.to_owned()),
            ..self
        }
    }

    /// An item whose action is not the slug of its name.
    pub fn with_action(name: &str, action: &str) -> Self {
        Self {
            action: action.to_string(),
            ..Self::new(name, "")
        }
    }

    /// Runs command `C` with `params`: `run:<id>:<json>`.
    pub fn run<C: Command>(name: &str, params: &C::Params) -> Self {
        Self::command::<C>(name, "run", params)
    }

    /// Opens command `C`'s form filled in with `params`: `form:<id>:<json>`.
    pub fn form<C: Command>(name: &str, params: &C::Params) -> Self {
        Self::command::<C>(name, "form", params)
    }

    fn command<C: Command>(name: &str, how: &str, params: &C::Params) -> Self {
        let params = serde_json::to_string(params).expect("command params serialize");
        Self::with_action(name, &format!("{how}:{}:{params}", C::ID))
    }

    pub fn submenu(name: &str, children: Vec<MenuItem>) -> Self {
        Self {
            children,
            ..Self::new(name, "")
        }
    }

    /// A separator is an item with an empty name.
    pub fn separator() -> Self {
        Self::new("", "")
    }

    pub fn is_separator(&self) -> bool {
        self.name.is_empty()
    }

    /// Greyed out unless `on`.
    #[must_use]
    pub fn enabled(self, on: bool) -> Self {
        Self {
            is_enabled: on,
            ..self
        }
    }

    /// Ticked when `on`.
    #[must_use]
    pub fn checked(self, on: bool) -> Self {
        Self {
            is_checked: on,
            ..self
        }
    }
}

/// Show ▸ and Hide ▸ for one layer, toolpath or component: Aspire's Show / Hide family.
fn show_hide(items: Items, item: u64) -> [MenuItem; 2] {
    use cmd_show_hide::{Action, Params, ShowHide};
    let run = |name: &str, action| {
        MenuItem::run::<ShowHide>(
            name,
            &Params {
                items,
                item,
                action,
            },
        )
    };
    [
        MenuItem::submenu(
            "Show",
            vec![
                run("This", Action::ShowThis),
                run("Only This", Action::ShowOnlyThis),
                run("All But This", Action::ShowAllButThis),
                run("All", Action::ShowAll),
            ],
        ),
        MenuItem::submenu(
            "Hide",
            vec![run("This", Action::HideThis), run("All", Action::HideAll)],
        ),
    ]
}

/// Delete ▸ for one layer, toolpath or component: Aspire's Delete family.
fn delete(items: Items, item: u64, which: &[(&str, cmd_delete_items::Which)]) -> MenuItem {
    use cmd_delete_items::{DeleteItems, Params};
    let run = |&(name, which): &(&str, _)| {
        MenuItem::run::<DeleteItems>(name, &Params { items, item, which })
    };
    MenuItem::submenu("Delete", which.iter().map(run).collect())
}

/// Aspire's toolpath menu (MENU 968) for toolpath `id`. Right-clicking a toolpath selects it,
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

/// A toolpath's Group submenu: this toolpath or the shown ones into a new group, a new empty
/// group, this toolpath into another group, or out of its group.
fn group_items(doc: &Document, id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_toolpath_group::{GroupParams, GroupToolpaths};
    let into = |name: &str, toolpaths: Vec<EntityId>, group: &str| {
        MenuItem::run::<GroupToolpaths>(
            name,
            &GroupParams {
                toolpaths,
                group: group.to_owned(),
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
        .map(|g| into(g, vec![id], g))
        .collect();
    vec![
        into("Group This", vec![id], &new),
        into("Group Visible", shown, &new),
        into("Create Empty Group", vec![], &new),
        MenuItem::submenu("Move to Group", others.clone()).enabled(!others.is_empty()),
        into("Remove from Group", vec![id], "").enabled(toolpath.group.is_some()),
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
fn other_sheets(doc: &Document, item: impl Fn(usize) -> MenuItem) -> Vec<MenuItem> {
    (0..doc.sheets.len())
        .filter(|&i| i != doc.active_sheet)
        .map(item)
        .collect()
}

/// Aspire's level menu (MENU 1046) for level `id`: combine mode ticked, clipping, mirror mode
/// and (rotary jobs) wrapping.
/// A level's Combine Mode submenu, in Aspire's order.
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
) -> Vec<MenuItem> {
    let action = |what: &str| format!("{FAVOURITE}{}:{what}:{id}", page_key(page));
    let mut menu = vec![if favourite {
        MenuItem::with_action("Remove from Favourites", &action("remove"))
    } else {
        MenuItem::with_action("Add to Favourites", &action("add"))
    }];
    if among {
        menu.push(MenuItem::with_action("Move Earlier", &action("earlier")));
        menu.push(MenuItem::with_action("Move Later", &action("later")));
    }
    menu
}

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

/// The combine modes in Aspire's order, each with its icon.
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

/// A level's Mirror Mode submenu, in Aspire's order.
const MIRROR_MODES: [(&str, Mirror); 9] = [
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
fn move_to_level(doc: &Document, component: &Component) -> MenuItem {
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
fn component_items(
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

/// Aspire's component menu (MENU 613) for component `id`, its combine mode ticked.
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

/// Aspire's layer menu (MENU 967) for `layer`.
/// A menu's categories: the runs of items between separators, each drawn in a border.
pub fn categories(items: &[MenuItem]) -> Vec<&[MenuItem]> {
    items
        .split(MenuItem::is_separator)
        .filter(|run| !run.is_empty())
        .collect()
}

/// The right-click menu of sheet `sheet` in the Sheets tab (MENU 5030).
pub fn sheet_menu(doc: &Document, sheet: usize) -> Vec<MenuItem> {
    use cmd_sheet_properties::SheetProperties;
    let Some(s) = doc.sheets.get(sheet) else {
        return vec![];
    };
    let props = cmd_sheet_properties::Params {
        sheet,
        name: s.name.clone(),
        visible: s.visible,
    };
    let shown = |name: &str, visible: bool| {
        let params = cmd_sheet_properties::Params {
            visible,
            ..props.clone()
        };
        MenuItem::run::<SheetProperties>(name, &params).enabled(s.visible != visible)
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
        shown("Show", true),
        shown("Hide", false),
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
        // ponytail: a sheet is edited while active, in Job Setup; Aspire's Sheet Editor is that form
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

/// A `run:` or `form:` action taken apart: whether it opens the form, the command id, its params.
pub fn command_action(action: &str) -> Option<(bool, &str, serde_json::Value)> {
    let (form, rest) = match action.split_once(':')? {
        ("run", rest) => (false, rest),
        ("form", rest) => (true, rest),
        _ => return None,
    };
    let (id, params) = rest.split_once(':')?;
    Some((form, id, serde_json::from_str(params).ok()?))
}

impl MenuSystem {
    /// Aspire's main menu (`Resources_ENU.dll` MENU 129, see `re/SPEC.md`), minus vendor web links,
    /// licensing and the debug menu. Shortcuts follow the accelerator table where the two disagree.
    #[expect(clippy::too_many_lines, reason = "the menu table is data, not logic")]
    pub fn new() -> Self {
        let i = |name: &str| MenuItem::new(name, "");
        let k = MenuItem::new;
        let sub = MenuItem::submenu;
        let sep = MenuItem::separator;
        let a = MenuItem::with_action;
        let space_h = |name: &str, order, inside_last_vector| {
            MenuItem::run::<cmd_space_horizontally::SpaceHorizontally>(
                name,
                &cmd_space_horizontally::Params {
                    order,
                    inside_last_vector,
                },
            )
        };
        let space_v = |name: &str, order, inside_last_vector| {
            MenuItem::run::<cmd_space_vertically::SpaceVertically>(
                name,
                &cmd_space_vertically::Params {
                    order,
                    inside_last_vector,
                },
            )
        };
        let menu = |name: &str, items: Vec<MenuItem>| Menu {
            name: name.to_string(),
            items,
        };

        Self {
            menus: vec![
                menu(
                    "File",
                    vec![
                        k("New", "Ctrl+N"),
                        i("New from Template..."),
                        k("Open...", "Ctrl+O"),
                        i("Close"),
                        sep(),
                        k("Save", "Ctrl+S"),
                        i("Save As..."),
                        i("Save As Template..."),
                        k("Increment and Save", "Ctrl+Alt+S"),
                        sep(),
                        i("Print..."),
                        i("Print Preview"),
                        sep(),
                        sub(
                            "Import",
                            vec![
                                k("Import Vectors / Bitmap...", "Ctrl+I"),
                                i("Import Component / 3D Model..."),
                                a("Segment 3D Model...", "segment_3d_model_file"),
                                i("Import Toolpaths..."),
                            ],
                        ),
                        sub(
                            "Export",
                            vec![
                                a("EPS", "export_eps"),
                                a("DXF", "export_dxf"),
                                a("AI", "export_ai"),
                                a("SVG", "export_svg"),
                                a("PDF", "export_pdf"),
                            ],
                        ),
                        sep(),
                        i("Open Application Data Folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        i("Exit"),
                    ],
                ),
                menu(
                    "Edit",
                    vec![
                        k("Undo", "Ctrl+Z"),
                        k("Redo", "Ctrl+Y"),
                        sep(),
                        k("Cut", "Ctrl+X"),
                        k("Copy", "Ctrl+C"),
                        k("Paste", "Ctrl+V"),
                        k("Paste in Place", "Ctrl+Shift+V"),
                        k("Delete", "Del"),
                        k("Edit Object", "E"),
                        sep(),
                        k("Find Tool...", "Ctrl+K"),
                        sep(),
                        sub(
                            "Selection",
                            vec![
                                k("Select All Vectors", "Ctrl+A"),
                                i("Select All Open Vectors"),
                                i("Select All Duplicate Vectors"),
                                i("Select All Vectors On Current Layer"),
                                i("Unselect All"),
                                i("Vector Selector..."),
                            ],
                        ),
                        sub(
                            "Align Selected Objects",
                            vec![
                                k("Center In Material", "F9"),
                                i("Center Horizontally In Material"),
                                i("Center Vertically In Material"),
                                sep(),
                                i("Center"),
                                a("Left", "align_left"),
                                a("Right", "align_right"),
                                a("Top", "align_top"),
                                a("Bottom", "align_bottom"),
                                i("Center Horizontally"),
                                i("Center Vertically"),
                                sep(),
                                space_h("Left To Right", SpaceH::LeftToRight, false),
                                space_h("Right To Left", SpaceH::RightToLeft, false),
                                space_v("Top To Bottom", SpaceV::TopToBottom, false),
                                space_v("Bottom To Top", SpaceV::BottomToTop, false),
                                space_h("Horizontally Inside Last Vector", SpaceH::AsPlaced, true),
                                space_v("Vertically Inside Last Vector", SpaceV::AsPlaced, true),
                            ],
                        ),
                        k("Join Vectors", "J"),
                        a("Curve Fit Vectors", "fit_curves_to_vectors"),
                        a("Nest Selected Objects", "nest_parts"),
                        sep(),
                        i("Job Size and Position"),
                        sub(
                            "Units",
                            vec![a("Millimetres", "units_mm"), a("Inches", "units_inch")],
                        ),
                        i("Swap Sides"),
                        sep(),
                        a("Notes", "notes"),
                        i("Document Variables"),
                        k("Snap Options", "F4"),
                        i("Options"),
                    ],
                ),
                menu(
                    "Model",
                    vec![
                        sub(
                            "Create Component",
                            vec![
                                i("Two Rail Sweep"),
                                i("Extrude and Weave"),
                                i("Turn and Spin"),
                                a("Create Textured Area Component", "texture_area"),
                                i("Add Zero Plane"),
                            ],
                        ),
                        sub(
                            "Create Component from",
                            vec![
                                i("Bake Components"),
                                i("Create Component from Visible Model"),
                                a(
                                    "Create Component with Draft from Visible Model",
                                    "add_draft",
                                ),
                                a("Create an Offset Model", "offset_model"),
                                a("Slice Model", "slice"),
                                i("Create Component from Toolpath Preview"),
                            ],
                        ),
                        sep(),
                        i("Create Component from Bitmap"),
                        i("Import Component / 3D Model..."),
                        i("Export as STL"),
                        i("Export as Grayscale Bitmap"),
                        i("Export as 3D Clipart"),
                        sep(),
                        // Job Setup's Resolution (Aspire's level of detail), here too
                        sub(
                            "Model Resolution",
                            vec![
                                MenuItem::with_action(
                                    "Standard (fastest)",
                                    "model_resolution_standard",
                                ),
                                MenuItem::with_action("High (3x slower)", "model_resolution_high"),
                                MenuItem::with_action(
                                    "Highest (7x slower)",
                                    "model_resolution_highest",
                                ),
                            ],
                        ),
                        sep(),
                        a("Sculpt Visible Model", "sculpt"),
                        i("Smooth Selected Components"),
                        i("Reset Inside Selected Vectors"),
                        i("Reset Outside Selected Vectors"),
                        a("Split Selected Component", "split"),
                        a(
                            "Create Vector Boundary Around Selected Components",
                            "component_boundary",
                        ),
                        a("Create Cross Section Vector", "cross_section"),
                        sep(),
                        a("Emboss Component", "emboss"),
                        i("Scale Model Z Height"),
                        a("Distort Selected Component", "distort_object"),
                        i("Replace Below"),
                        sep(),
                        MenuItem::with_action(
                            "Selected Component Properties",
                            "component_properties",
                        ),
                    ],
                ),
                menu(
                    "Machine",
                    vec![
                        i("Machine Configuration..."),
                        i("Tool Changer..."),
                        sep(),
                        i("Add Machine..."),
                        i("Install Offline Package..."),
                        i("Search for Machine Online..."),
                        sep(),
                        i("Post-Processor Management..."),
                        sep(),
                        i("Install Post-Processor..."),
                        i("Update Post-Processor Database"),
                    ],
                ),
                menu(
                    "Toolpaths",
                    vec![
                        i("Show Toolpaths Tab"),
                        i("Recalculate All Toolpaths"),
                        MenuItem::with_action("Keep-Out Zones...", "keep_out_zones"),
                        MenuItem::with_action("Toolpath Tiling...", "toolpath_tiling"),
                        sub(
                            "Toolpath Drawing",
                            vec![
                                i("Draw 2D Previews"),
                                i("Make 2D Previews Solid"),
                                i("Auto Open 3D View"),
                                i("Animate Preview"),
                                i("Draw Rapid Moves"),
                                i("Draw Plunge Moves"),
                                i("Draw Retract Moves"),
                                i("Draw Join Moves"),
                            ],
                        ),
                        sub(
                            "Preview Simulation Quality",
                            vec![
                                MenuItem::with_action("Standard", "preview_quality_1"),
                                MenuItem::with_action("High (2x Slower)", "preview_quality_2"),
                                MenuItem::with_action("Very High (4x Slower)", "preview_quality_4"),
                                MenuItem::with_action(
                                    "Extremely High (8x Slower)",
                                    "preview_quality_8",
                                ),
                                MenuItem::with_action("Maximum (25x Slower)", "preview_quality_25"),
                            ],
                        ),
                        sep(),
                        sub(
                            "Templates",
                            vec![
                                i("Save Selected Toolpath as Template..."),
                                i("Save All Visible Toolpaths as Template..."),
                                i("Load Template..."),
                            ],
                        ),
                        MenuItem::with_action("Merge Visible Toolpaths", "merged_toolpath"),
                        i("Create Job Sheet"),
                        sep(),
                        i("Materials..."),
                        sep(),
                        i("Tool Database..."),
                    ],
                ),
                menu(
                    "View",
                    vec![
                        k("Zoom to Drawing", "F6"),
                        k("Refresh 2D View", "F5"),
                        sub(
                            "Guide Lines",
                            vec![
                                i("Guides Visible"),
                                i("Delete All Guides"),
                                i("Lock All Existing Guides"),
                                i("Unlock All Existing Guides"),
                            ],
                        ),
                        sep(),
                        i("Draw Origin"),
                        i("Draw Modeling Plane"),
                        i("Draw Material Block"),
                        i("Draw Vectors in 3D View"),
                        i("Draw Keep-Out Zones in 3D View"),
                        sep(),
                        i("Auto-Wrapping View"),
                        i("Color Shaded View"),
                        i("Use Shaded Background"),
                        i("Light Follows Viewer"),
                        i("Use Shadow Shading"),
                        sep(),
                        i("Save Shaded Image"),
                        sep(),
                        k("Tile Windows Horizontal", "PgDn"),
                        k("Tile Windows Vertical", "PgUp"),
                        i("Cascade Windows"),
                        sep(),
                        sub(
                            "Window Layout",
                            vec![
                                MenuItem::with_action("Classic (Aspire's pages)", "layout_classic"),
                                MenuItem::with_action("Rail and Inspector", "layout_rail"),
                                MenuItem::with_action("Ribbon", "layout_ribbon"),
                            ],
                        ),
                    ],
                ),
                menu(
                    "Gadgets",
                    vec![
                        MenuItem::with_action("Wrapped Fluting Layout", "wrapped_fluting_layout"),
                        MenuItem::with_action("Wrapped Spiral Layout", "wrapped_spiral_layout"),
                        MenuItem::with_action("Celtic Weave Creator", "celtic_weave"),
                        MenuItem::with_action("Job Setup Sheet Editor...", "job_sheet_editor"),
                    ],
                ),
                menu(
                    "Help",
                    vec![
                        i("Help Contents"),
                        i("Keyboard Shortcuts"),
                        i("Third Party Licenses"),
                        sep(),
                        i("About Cut Craft"),
                    ],
                ),
            ],
        }
    }

    /// The menus with no job open (`Resources_ENU.dll` MENU 128): files, options, machines,
    /// tools and help. Machine, Gadgets and Help are the same as with a job.
    pub fn without_job(&self) -> Self {
        let i = |name: &str| MenuItem::new(name, "");
        let k = MenuItem::new;
        let sep = MenuItem::separator;
        let menu = |name: &str, items: Vec<MenuItem>| Menu {
            name: name.to_string(),
            items,
        };
        let same = |name: &str| self.menu(name).clone();
        Self {
            menus: vec![
                menu(
                    "File",
                    vec![
                        k("New", "Ctrl+N"),
                        i("New from Template..."),
                        k("Open...", "Ctrl+O"),
                        i("Open Application Data Folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        i("Close"),
                        i("Exit"),
                    ],
                ),
                menu("Edit", vec![i("Options")]),
                same("Machine"),
                // Materials colours a job, so it is in the job's Toolpaths menu only
                menu("Toolpaths", vec![i("Tool Database...")]),
                same("Gadgets"),
                same("Help"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MenuItem;

    #[test]
    fn separators_split_a_menu_into_categories() {
        let items = [
            MenuItem::separator(),
            MenuItem::with_action("Undo", "undo"),
            MenuItem::with_action("Redo", "redo"),
            MenuItem::separator(),
            MenuItem::separator(),
            MenuItem::with_action("Delete", "delete"),
        ];
        let names: Vec<Vec<&str>> = super::categories(&items)
            .iter()
            .map(|c| c.iter().map(|i| i.name.as_str()).collect())
            .collect();
        assert_eq!(names, [vec!["Undo", "Redo"], vec!["Delete"]]);
    }

    #[test]
    fn title_case_capitalizes_each_word() {
        assert_eq!(super::title_case("join_vectors"), "Join Vectors");
        assert_eq!(super::title_case("2d_view"), "2d View");
    }
}

#[cfg(test)]
mod menu_tests {
    use super::MenuSystem;

    #[test]
    fn every_strategy_in_the_toolpaths_tab_is_a_command() {
        let registry = crate::commands::registry();
        for group in super::toolpath_groups() {
            if group.name == "Output" {
                continue; // app actions, not commands
            }
            for id in group.tools {
                assert!(
                    registry.get(id).is_some(),
                    "{} lists `{id}`, which is not a command",
                    group.name
                );
            }
        }
    }

    #[test]
    fn align_submenu_items_run_the_align_commands() {
        let registry = crate::commands::registry();
        let menus = MenuSystem::new();
        let edit = menus.menus.iter().find(|m| m.name == "Edit").unwrap();
        let align = edit
            .items
            .iter()
            .find(|i| i.name == "Align Selected Objects")
            .unwrap();
        for item in align
            .children
            .iter()
            // the spacing items run their command with its settings
            .filter(|i| !i.is_separator() && !i.action.starts_with("run:"))
        {
            assert!(
                registry.get(&item.action).is_some(),
                "Align > {} runs `{}`, which is not a command",
                item.name,
                item.action
            );
        }
    }
}
