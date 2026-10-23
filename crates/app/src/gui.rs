//! Static UI data: Drawing tab tool groups, the main menu, status bar text.

/// Drawing tab tool groups
pub struct Toolbar {
    pub groups: Vec<ToolGroup>,
}

/// A titled group of tools on the Drawing tab
#[derive(Debug, Clone)]
pub struct ToolGroup {
    pub name: String,
    pub tools: Vec<String>,
}

impl Toolbar {
    /// Aspire's Drawing tab. Group names come from `Resources_ENU.dll` (strings 130-945, see `re/SPEC.md`);
    /// which tool sits in which group is inferred from the help topics.
    pub fn new() -> Self {
        let group = |name: &str, tools: &[&str]| ToolGroup {
            name: name.to_string(),
            tools: tools.iter().map(ToString::to_string).collect(),
        };
        Self {
            groups: vec![
                group(
                    "File Operations",
                    &[
                        "New",
                        "Open",
                        "Save",
                        "Import Vectors / Bitmap",
                        "Import Component / 3D Model",
                        "Job Setup",
                    ],
                ),
                group(
                    "2D View Control",
                    &["Pan", "Zoom Box", "Zoom to Job", "Zoom Selected"],
                ),
                group(
                    "Create Vectors",
                    &[
                        "Line / Polyline",
                        "Rectangle",
                        "Circle",
                        "Ellipse",
                        "Polygon",
                        "Star",
                        "Arc",
                        "Freehand",
                        "Text",
                        "Text on a Curve",
                        "Dimensions",
                        "Fillets",
                        "Vector Texture",
                        "Trace Bitmap",
                    ],
                ),
                group(
                    "Edit Objects",
                    &[
                        "Select",
                        "Node Edit",
                        "Transform",
                        "Group",
                        "Ungroup",
                        "Join",
                        "Weld",
                        "Subtract",
                        "Trim",
                        "Overlap",
                        "Extend",
                        "Fit Curves",
                        "Vector Validator",
                        "Measure",
                    ],
                ),
                group(
                    "Transform Objects",
                    &[
                        "Move",
                        "Rotate",
                        "Set Size",
                        "Mirror",
                        "Align",
                        "Distort",
                        "Vector Unwrapper",
                    ],
                ),
                group(
                    "Offset and Layout",
                    &["Offset", "Nest Parts", "Copy Along Vectors"],
                ),
                group("Array Objects", &["Array Copy", "Circular Copy"]),
            ],
        }
    }
}

/// Editable properties of the selected object (replaced by generated command forms)
pub struct PropertyPanel {
    pub properties: Vec<Property>,
}

pub struct Property {
    pub name: String,
    pub value: String,
}

impl PropertyPanel {
    pub fn new() -> Self {
        let prop = |name: &str, value: &str| Property {
            name: name.to_string(),
            value: value.to_string(),
        };
        Self {
            properties: vec![
                prop("Name", "Untitled"),
                prop("Position X", "0.0"),
                prop("Position Y", "0.0"),
            ],
        }
    }
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
                            vec![i("EPS"), i("DXF"), i("AI"), i("SVG"), i("PDF")],
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
                                i("Left"),
                                i("Right"),
                                i("Top"),
                                i("Bottom"),
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
