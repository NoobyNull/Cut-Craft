//! The menu bar: every menu and its items, and the menus without a job open.

use super::{COMMUNITY, Menu, MenuItem, MenuSystem, SpaceH, SpaceV};

impl MenuSystem {
    /// The main menu (see `re/SPEC.md`, Menus), minus vendor web links and licensing.
    /// Shortcuts follow the keyboard shortcut table there where the two disagree.
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
                        a("Print (opens a PDF)...", "print"),
                        sep(),
                        sub(
                            "Import",
                            vec![
                                k("Import Vectors / Bitmap...", "Ctrl+I"),
                                i("Import Component / 3D Model..."),
                                a("Segment 3D Model...", "segment_3d_model_file"),
                                a("Import Toolpaths...", "import_toolpaths_file"),
                                a("Import Cut List (CSV)...", "import_cut_list"),
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
                                i("Select Vectors Inside"),
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
                        a("Join Open Ends...", "join_open_ends"),
                        a("Curve Fit Vectors", "fit_curves_to_vectors"),
                        a("Nest Selected Objects", "nest_parts"),
                        a("Name Part...", "name_part"),
                        a("Engrave Part Names...", "engrave_part_names"),
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
                                a("Create Decorative Relief", "decorative_relief"),
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
                        // Job Setup's Resolution (level of detail), here too
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
                        i("Machine Files..."),
                        sep(),
                        i("Add Machine..."),
                        i("Install Offline Package..."),
                        i("Search for Machine Online..."),
                        sep(),
                        i("Post-Processor Management..."),
                        sep(),
                        i("Install Post-Processor..."),
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
                                // quick picks on the Preview tab's detail slider
                                MenuItem::with_action("Standard (20%)", "preview_quality_1"),
                                MenuItem::with_action("High (40%)", "preview_quality_2"),
                                MenuItem::with_action("Very High (60%)", "preview_quality_4"),
                                MenuItem::with_action("Extremely High (80%)", "preview_quality_8"),
                                MenuItem::with_action("Maximum (100%)", "preview_quality_25"),
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
                        a("Print Job Sheet...", "print_job_sheet"),
                        a("Print Part Labels...", "part_labels"),
                        a("Save Cut List...", "cut_list"),
                        sep(),
                        a("Clearing Settings...", "clearing_settings"),
                        a("Preview Colors...", "materials"),
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
                        i("Show Keep-Out Zone Labels"),
                        i("Show Open Vector Ends"),
                        i("Show Construction Lines"),
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
                                MenuItem::with_action(
                                    "Classic (Design and Toolpaths pages)",
                                    "layout_classic",
                                ),
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
                        sep(),
                        MenuItem::with_action("Add-ons...", "addons"),
                    ],
                ),
                menu(
                    "Help",
                    vec![
                        i("Help Contents"),
                        MenuItem::with_action("Did You Know?...", "did_you_know"),
                        i("Keyboard Shortcuts"),
                        i("Credits"),
                        sub(
                            "Community",
                            crate::app::LINKS
                                .iter()
                                .enumerate()
                                .map(|(n, (label, _, icon))| {
                                    MenuItem::with_action(label, &format!("{COMMUNITY}{n}"))
                                        .icon(icon)
                                })
                                .collect(),
                        ),
                        sep(),
                        MenuItem::with_action("Check for Updates...", "check_for_updates"),
                        MenuItem::with_action("Run First-Time Setup Again...", "first_run_setup"),
                        i("About Cut Craft"),
                    ],
                ),
            ],
        }
    }

    /// The menus with no job open (see `re/SPEC.md`, Startup sequence): files, options, machines,
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
