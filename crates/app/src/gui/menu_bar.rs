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
        let b = MenuItem::built_in;
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
                        b("new"),
                        b("new_from_template"),
                        b("open"),
                        b("close"),
                        sep(),
                        b("save"),
                        b("save_as"),
                        b("save_as_template"),
                        b("increment_and_save"),
                        sep(),
                        b("print"),
                        sep(),
                        sub(
                            "Import",
                            vec![
                                b("import_vectors_bitmap"),
                                b("import_component_3d_model"),
                                b("segment_3d_model_file"),
                                b("import_toolpaths_file"),
                                b("import_cut_list"),
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
                        b("open_application_data_folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        b("exit"),
                    ],
                ),
                menu(
                    "Edit",
                    vec![
                        b("undo"),
                        b("redo"),
                        sep(),
                        b("cut"),
                        b("copy"),
                        b("paste"),
                        b("paste_in_place"),
                        k("Delete", "Del"),
                        b("edit_object"),
                        sep(),
                        b("find_tool"),
                        sep(),
                        sub(
                            "Selection",
                            vec![
                                k("Select All Vectors", "Ctrl+A"),
                                i("Select All Open Vectors"),
                                i("Select All Duplicate Vectors"),
                                b("select_all_vectors_on_current_layer"),
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
                        b("job_size_and_position"),
                        sub(
                            "Units",
                            vec![a("Millimetres", "units_mm"), a("Inches", "units_inch")],
                        ),
                        i("Swap Sides"),
                        sep(),
                        a("Notes", "notes"),
                        i("Document Variables"),
                        b("snap_options"),
                        b("options"),
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
                                b("create_component_from_toolpath_preview"),
                            ],
                        ),
                        sep(),
                        i("Create Component from Bitmap"),
                        b("import_component_3d_model"),
                        b("export_as_stl"),
                        b("export_as_grayscale_bitmap"),
                        b("export_as_3d_clipart"),
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
                        b("machine_configuration"),
                        b("tool_changer"),
                        b("machine_files"),
                        sep(),
                        b("add_machine"),
                        b("install_offline_package"),
                        b("search_for_machine_online"),
                        sep(),
                        b("post_processor_management"),
                        sep(),
                        b("install_post_processor"),
                    ],
                ),
                menu(
                    "Toolpaths",
                    vec![
                        b("show_toolpaths_tab"),
                        b("recalculate_all_toolpaths"),
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
                                b("save_selected_toolpath_as_template"),
                                b("save_all_visible_toolpaths_as_template"),
                                b("load_template"),
                            ],
                        ),
                        MenuItem::with_action("Merge Visible Toolpaths", "merged_toolpath"),
                        b("create_job_sheet"),
                        b("print_job_sheet"),
                        b("part_labels"),
                        b("cut_list"),
                        sep(),
                        b("clearing_settings"),
                        b("advanced_rotary_mode"),
                        a("Preview Colors...", "materials"),
                        sep(),
                        b("tool_database"),
                    ],
                ),
                menu(
                    "View",
                    vec![
                        b("zoom_to_drawing"),
                        b("refresh_2d_view"),
                        sub(
                            "Guide Lines",
                            vec![
                                i("Guides Visible"),
                                b("delete_all_guides"),
                                b("lock_all_existing_guides"),
                                b("unlock_all_existing_guides"),
                            ],
                        ),
                        sep(),
                        i("Draw Origin"),
                        i("Draw Modeling Plane"),
                        i("Draw Material Block"),
                        i("Draw Vectors in 3D View"),
                        i("Draw Keep-Out Zones in 3D View"),
                        i("Show Keep-Out Zone Labels"),
                        b("show_open_vector_ends"),
                        b("show_construction_lines"),
                        sep(),
                        i("Auto-Wrapping View"),
                        i("Color Shaded View"),
                        i("Use Shaded Background"),
                        i("Light Follows Viewer"),
                        i("Use Shadow Shading"),
                        sep(),
                        b("save_shaded_image"),
                        sep(),
                        b("tile_windows_horizontal"),
                        b("tile_windows_vertical"),
                        b("cascade_windows"),
                        sep(),
                        sub(
                            "Window Layout",
                            vec![
                                MenuItem::with_action("Ribbon", "layout_ribbon"),
                                MenuItem::with_action("Rail", "layout_rail"),
                                MenuItem::with_action("Canvas", "layout_canvas"),
                                MenuItem::with_action(
                                    "Classic (Design and Toolpaths pages)",
                                    "layout_classic",
                                ),
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
                        b("job_sheet_editor"),
                        sep(),
                        b("addons"),
                    ],
                ),
                menu(
                    "Help",
                    vec![
                        b("help_contents"),
                        b("did_you_know"),
                        b("keyboard_shortcuts"),
                        b("credits"),
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
                        b("check_for_updates"),
                        b("first_run_setup"),
                        b("about_cut_craft"),
                    ],
                ),
            ],
        }
    }

    /// The menus with no job open (see `re/SPEC.md`, Startup sequence): files, options, machines,
    /// tools and help. Machine, Gadgets and Help are the same as with a job.
    pub fn without_job(&self) -> Self {
        let i = |name: &str| MenuItem::new(name, "");
        let sep = MenuItem::separator;
        let b = MenuItem::built_in;
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
                        b("new"),
                        b("new_from_template"),
                        b("open"),
                        b("open_application_data_folder"),
                        sep(),
                        i("Recent Files"),
                        sep(),
                        b("close"),
                        b("exit"),
                    ],
                ),
                menu("Edit", vec![b("options")]),
                same("Machine"),
                // Materials colours a job, so it is in the job's Toolpaths menu only
                menu("Toolpaths", vec![b("tool_database")]),
                same("Gadgets"),
                same("Help"),
            ],
        }
    }
}
