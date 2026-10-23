//! Static UI data: Drawing tab tool groups, the main menu, status bar text.

/// Drawing tab layout: groups of command ids. Implemented commands supply their own label and
/// icon; the rest show their humanized id, disabled.
pub struct Toolbar {
    pub groups: Vec<ToolGroup>,
}

pub struct ToolGroup {
    pub name: &'static str,
    pub tools: &'static [&'static str],
}

impl Toolbar {
    /// Aspire's Drawing tab. Group names come from `Resources_ENU.dll` (strings 130-945, see
    /// `re/SPEC.md`); which tool sits in which group is inferred from the help topics. Ids match
    /// the icon file names, so unimplemented tools still show their icon.
    pub fn new() -> Self {
        let group = |name, tools| ToolGroup { name, tools };
        Self {
            groups: vec![
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
                    &["pan", "zoom_box", "zoom_to_job", "zoom_selected"],
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
            ],
        }
    }
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

    fn submenu(name: &str, children: Vec<MenuItem>) -> Self {
        Self {
            children,
            ..Self::new(name, "")
        }
    }

    /// A separator is an item with an empty name.
    fn separator() -> Self {
        Self::new("", "")
    }

    pub fn is_separator(&self) -> bool {
        self.name.is_empty()
    }
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
        // an item whose action is not the slug of its label
        let a = |name: &str, action: &str| MenuItem {
            action: action.to_string(),
            ..MenuItem::new(name, "")
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
                        i("About Aspire Clone"),
                    ],
                ),
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
