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
fn drawing_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "File Operations",
            &[
                "new",
                "open",
                "save",
                "import_vectors_bitmap",
                "import_component_3d_model",
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
                "draw_circle",
                "draw_ellipse",
                "draw_polygon",
                "draw_star",
                "draw_arc",
                "draw_curve",
                "draw_freehand",
                "draw_text",
                "text_on_a_curve",
                "auto_layout_text",
                "dimensions",
                "create_fillets",
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
                "weld_vectors",
                "subtract_vectors",
                "overlap_intersect",
                "interactive_vector_trim",
                "close_extend_vector",
                "fit_curves_to_vectors",
                "vector_validator",
                "measure_inspect",
            ],
        ),
        group(
            "Transform Objects",
            &["move", "rotate", "set_size", "mirror", "distort_object"],
        ),
        group(
            "Offset and Layout",
            &["offset_vectors", "nest_parts", "copy_along_vectors"],
        ),
        group("Array Objects", &["array_copy", "circular_copy"]),
    ]
}

/// The Modeling tab, laid out the same way.
fn modeling_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "Shape Creation",
            &[
                "create_shape",
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
            ],
        ),
    ]
}

/// The Toolpaths panel, laid out the same way.
fn toolpath_groups() -> Vec<ToolGroup> {
    vec![
        group("Material", &["material_setup"]),
        group(
            "Toolpath Operations",
            &[
                "profile_toolpath",
                "pocket_toolpath",
                "drilling_toolpath",
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
            &["save_toolpaths", "toolpath_summary", "tool_database"],
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
pub fn node_menu(at: &cut_craft_core::nodes::NodeAt) -> Vec<MenuItem> {
    vec![
        MenuItem::run::<cmd_insert_point::InsertPoint>("Insert Point", at),
        MenuItem::run::<cmd_delete_point::DeletePoint>("Delete Point", at),
        MenuItem::separator(),
        MenuItem::run::<cmd_span_to_line::SpanToLine>("Convert Span to Line", at),
        MenuItem::run::<cmd_span_to_bezier::SpanToBezier>("Convert Span to Bezier", at),
        MenuItem::run::<cmd_span_to_arc::SpanToArc>("Convert Span to Arc", at),
        MenuItem::separator(),
        MenuItem::run::<cmd_cut_vector::CutVector>("Cut Vector", at),
        MenuItem::run::<cmd_make_start_point::MakeStartPoint>("Make Start Point", at),
        MenuItem::run::<cmd_reverse_direction::ReverseDirection>("Reverse Direction", at),
        MenuItem::run::<cmd_close_with_line::CloseWithLine>("Close With a Line", at),
        MenuItem::separator(),
        MenuItem::with_action("Exit Node Editing", "node_edit"),
    ]
}

impl MenuSystem {
    fn menu(&self, name: &str) -> &Menu {
        self.menus
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("there is a {name} menu"))
    }

    /// The 2D view's right-click menu: Aspire's selection menu (MENU 318), plus Move to Layer,
    /// the vector booleans and Align. Edit menu items are reused as they are.
    pub fn selection_menu(&self, doc: &Document) -> Vec<MenuItem> {
        let edit = |name: &str| {
            let items = &self.menu("Edit").items;
            let item = items.iter().find(|i| i.name == name);
            item.cloned()
                .unwrap_or_else(|| panic!("there is an Edit ▸ {name}"))
        };
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
            MenuItem::separator(),
            MenuItem::with_action("Weld", "weld_vectors"),
            MenuItem::with_action("Subtract", "subtract_vectors"),
            MenuItem::with_action("Overlap", "overlap_intersect"),
            edit("Join Vectors"),
            MenuItem::with_action("Group", "group"),
            MenuItem::with_action("Ungroup", "ungroup"),
            MenuItem {
                name: "Align".into(),
                ..edit("Align Selected Objects")
            },
            MenuItem::separator(),
            MenuItem::with_action("Object Properties", "object_properties"),
            MenuItem::with_action("Bitmap Properties", "bitmap_properties")
                .enabled(!doc.selected_bitmaps.is_empty()),
        ]
    }

    /// Every command item, submenus flattened.
    pub fn items(&self) -> Vec<&MenuItem> {
        self.menus.iter().flat_map(|m| leaves(&m.items)).collect()
    }
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
pub fn toolpath_menu(id: EntityId, toolpath: &Toolpath) -> Vec<MenuItem> {
    use cmd_delete_items::Which;
    let [show, hide] = show_hide(Items::Toolpaths, id.0);
    let properties = cmd_toolpath_properties::Params {
        toolpath: id,
        name: toolpath.name.clone(),
        visible: toolpath.visible,
    };
    vec![
        show,
        hide,
        MenuItem::form::<cmd_toolpath_properties::ToolpathProperties>("Rename", &properties),
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

/// Aspire's level menu (MENU 1046) for level `id`: combine mode ticked, clipping, mirror mode
/// and (rotary jobs) wrapping.
/// A level's Combine Mode submenu, in Aspire's order.
const COMBINE_MODES: [(&str, Combine); 5] = [
    ("Add", Combine::Add),
    ("Subtract", Combine::Subtract),
    ("Merge", Combine::Merge),
    ("Low", Combine::Low),
    ("Multiply", Combine::Multiply),
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
                .map(|&(name, c)| mode(name, c))
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
            vec![
                mode("Add", Combine::Add),
                mode("Subtract", Combine::Subtract),
                mode("Merge", Combine::Merge),
                mode("Low", Combine::Low),
                mode("Multiply", Combine::Multiply),
            ],
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
    vec![
        MenuItem::submenu(
            "Combine Mode",
            vec![
                mode("Add", Combine::Add),
                mode("Subtract", Combine::Subtract),
                mode("Merge", Combine::Merge),
                mode("Low", Combine::Low),
                mode("Multiply", Combine::Multiply),
            ],
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
    ]
}

/// Aspire's layer menu (MENU 967) for `layer`.
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
    vec![
        MenuItem::run::<cmd_activate_sheet::ActivateSheet>(
            "Activate",
            &cmd_activate_sheet::Params { sheet },
        )
        .enabled(sheet != doc.active_sheet),
        shown("Show", true),
        shown("Hide", false),
        MenuItem::separator(),
        MenuItem::with_action("Insert New Sheet", "add_sheet"),
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
        MenuItem::with_action("Merge Visible", "merge_visible_layers").enabled(false),
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
        let space_h = |name: &str, order| {
            MenuItem::run::<cmd_space_horizontally::SpaceHorizontally>(
                name,
                &cmd_space_horizontally::Params { order },
            )
        };
        let space_v = |name: &str, order| {
            MenuItem::run::<cmd_space_vertically::SpaceVertically>(
                name,
                &cmd_space_vertically::Params { order },
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
                        k("Delete", "Del"),
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
                                space_h("Left To Right", SpaceH::LeftToRight),
                                space_h("Right To Left", SpaceH::RightToLeft),
                                space_v("Top To Bottom", SpaceV::TopToBottom),
                                space_v("Bottom To Top", SpaceV::BottomToTop),
                            ],
                        ),
                        k("Join Vectors", "J"),
                        a("Curve Fit Vectors", "fit_curves_to_vectors"),
                        a("Nest Selected Objects", "nest_parts"),
                        sep(),
                        i("Job Size and Position"),
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
                                i("Create Textured Area Component"),
                                i("Add Zero Plane"),
                            ],
                        ),
                        sub(
                            "Create Component from",
                            vec![
                                i("Bake Components"),
                                i("Create Component from Visible Model"),
                                i("Create Component with Draft from Visible Model"),
                                i("Create an Offset Model"),
                                i("Slice Model"),
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
                        a("Sculpt Visible Model", "sculpt"),
                        i("Smooth Selected Components"),
                        i("Reset Inside Selected Vectors"),
                        i("Reset Outside Selected Vectors"),
                        i("Split Selected Component"),
                        a(
                            "Create Vector Boundary Around Selected Components",
                            "component_boundary",
                        ),
                        a("Create Cross Section Vector", "cross_section"),
                        sep(),
                        i("Emboss Component"),
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
                        sub(
                            "Toolpath Drawing",
                            vec![
                                i("Draw 2D Previews"),
                                i("Make 2D Previews Solid"),
                                i("Auto Open 3D View"),
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
                    ],
                ),
                menu(
                    "Gadgets",
                    vec![MenuItem {
                        is_enabled: false,
                        ..i("No Gadgets Installed")
                    }],
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
                menu(
                    "Toolpaths",
                    vec![i("Materials..."), sep(), i("Tool Database...")],
                ),
                same("Gadgets"),
                same("Help"),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
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
            .filter(|i| !i.is_separator() && !i.name.contains(" To "))
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
