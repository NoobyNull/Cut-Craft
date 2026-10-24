//! Static UI data: Drawing tab tool groups, the main menu, right-click menus, status bar text.

use cut_craft_core::Command;
use cut_craft_core::document::{Document, Layer};
use cut_craft_core::params::Items;

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
                "turn_and_spin",
                "texture_area",
                "zero_plane",
                "create_from_bitmap",
            ],
        ),
        group(
            "Shape Editing",
            &[
                "sculpt",
                "smooth",
                "emboss",
                "distort",
                "offset_model",
                "slice",
                "add_draft",
                "scale_z_height",
                "replace_below",
                "split",
                "bake",
            ],
        ),
    ]
}

/// The Toolpaths panel, laid out the same way.
fn toolpath_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "Toolpath Operations",
            &[
                "profile_toolpath",
                "pocket_toolpath",
                "drilling_toolpath",
                "quick_engrave",
                "vcarve_engrave_toolpath",
                "vcarve_inlay",
                "chamfer_toolpath",
                "fluting_toolpath",
                "prism_carving",
                "keyhole_toolpath",
                "moulding_toolpath",
                "texture_toolpath",
                "sketch_carving",
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
        group("Laser", &["laser_cut_fill", "laser_picture"]),
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

impl MenuSystem {
    /// Every command item, submenus flattened.
    pub fn items(&self) -> Vec<&MenuItem> {
        fn walk<'a>(items: &'a [MenuItem], out: &mut Vec<&'a MenuItem>) {
            for item in items {
                if item.children.is_empty() {
                    out.push(item);
                } else {
                    walk(&item.children, out);
                }
            }
        }
        let mut out = vec![];
        for menu in &self.menus {
            walk(&menu.items, &mut out);
        }
        out
    }
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

/// Aspire's layer menu (MENU 967) for `layer`.
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
                                i("Left To Right"),
                                i("Right To Left"),
                                i("Top To Bottom"),
                                i("Bottom To Top"),
                            ],
                        ),
                        k("Join Vectors", "J"),
                        i("Curve Fit Vectors"),
                        i("Nest Selected Objects"),
                        sep(),
                        i("Job Size and Position"),
                        i("Swap Sides"),
                        sep(),
                        i("Notes"),
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
                        i("Sculpt Visible Model"),
                        i("Smooth Selected Components"),
                        i("Reset Inside Selected Vectors"),
                        i("Reset Outside Selected Vectors"),
                        i("Split Selected Component"),
                        i("Create Vector Boundary Around Selected Components"),
                        i("Create Cross Section Vector"),
                        sep(),
                        i("Emboss Component"),
                        i("Scale Model Z Height"),
                        i("Distort Selected Component"),
                        i("Replace Below"),
                        sep(),
                        i("Selected Component Properties"),
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
                                i("Standard"),
                                i("High (2x Slower)"),
                                i("Very High (4x Slower)"),
                                i("Extremely High (8x Slower)"),
                                i("Maximum (25x Slower)"),
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
                        i("Merge Visible Toolpaths"),
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
        let same = |name: &str| {
            self.menus
                .iter()
                .find(|m| m.name == name)
                .cloned()
                .unwrap_or_else(|| panic!("there is a {name} menu"))
        };
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
