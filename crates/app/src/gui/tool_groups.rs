//! The Drawing, Modeling and Toolpaths tabs' tool groups, in order.

use super::{ToolGroup, group};

/// The Drawing tab. Group names and which tool sits in which group follow the help
/// topics and the trial (see `re/SPEC.md`). Ids match
/// the icon file names, so unimplemented tools still show their icon.
#[expect(clippy::too_many_lines, reason = "the drawing tab's tool list")]
pub(super) fn drawing_groups() -> Vec<ToolGroup> {
    vec![
        group(
            "File Operations",
            &[
                "new",
                "open",
                "save",
                "export_vectors",
                "paste_in_place",
                "open_folder",
                // what a blank job needs; the rest of Job Setup is a link on its form
                "setup_material",
                // the job's notes are kept with its files
                "notes",
            ],
        ),
        // every way to bring something into the job, on the Import sub-tab, rather
        // than spread over the File menu, the Drawing and Modeling tabs and the Clipart tab
        group(
            "Import",
            &[
                "import_vectors_bitmap",
                "import_vectors",
                "import_into_shape",
                "import_component_3d_model",
                "clipart_library",
                "import_toolpaths_file",
                "import_cut_list",
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
                "embellish",
                "trace_bitmap",
                "quick_trace",
                // shown in the Text box (its flyout): text that fills itself in
                "document_variables",
            ],
        ),
        // The Edit Objects section, in two: picking and reshaping, then the tools that
        // make one vector of several or cut them (each a job a beginner names differently)
        group(
            "Select and Edit",
            &[
                "select",
                "node_edit",
                "transform",
                "group",
                "measure_inspect",
                "fit_curves_to_vectors",
                "vector_validator",
                "edit_picture",
                "crop_bitmap",
            ],
        ),
        group(
            "Join, Combine and Trim",
            &[
                "join_vectors",
                "join_open_ends",
                "join_straight_line",
                "join_smooth_curve",
                "join_move_end_points",
                "close_extend_vector",
                "weld_vectors",
                "subtract_vectors",
                "overlap_intersect",
                "interactive_vector_trim",
                "trim_objects",
            ],
        ),
        // The Align Objects section (help: Alignment Tools): the common alignments
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
            &[
                "offset_vectors",
                "nest_parts",
                "name_part",
                "engrave_part_names",
                "plate_production",
            ],
        ),
        group(
            "Array Objects",
            &["array_copy", "circular_copy", "copy_along_vectors"],
        ),
    ]
}

/// The Modeling tab, laid out the same way.
pub(super) fn modeling_groups() -> Vec<ToolGroup> {
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
                "decorative_relief",
                "add_zero_plane",
                "create_component_from_bitmap",
                "relief_from_photo",
                "generate_border",
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
                // The Model Editing row (help: Clearing or Splitting Components,
                // Create Vector Boundary From Selected Components, Create Cross Section
                // Vector): the model cleared inside or outside vectors, vectors drawn from
                // it, and the visible model as one component
                "reset_inside_selected_vectors",
                "reset_outside_selected_vectors",
                "component_boundary",
                "cross_section",
                "create_component_from_visible_model",
            ],
        ),
        // The second row of modeling icons: the model saved for other programs
        group(
            "Export Model",
            &[
                "export_as_stl",
                "export_as_grayscale_bitmap",
                "export_as_3d_clipart",
            ],
        ),
    ]
}

/// The Toolpaths panel, laid out the same way.
pub(super) fn toolpath_groups() -> Vec<ToolGroup> {
    vec![
        // over the sub-tabs: what every toolpath is cut from and with
        group(
            "Job and Tools",
            &[
                "material_setup",
                "materials",
                "tool_database",
                "keep_out_zones",
            ],
        ),
        // Make: grouped by what the cut is for, in a beginner's words
        group(
            "Cut Out and Clear",
            &[
                "profile_toolpath",
                "pocket_toolpath",
                "surfacing_toolpath",
                "drag_knife_toolpath",
            ],
        ),
        group(
            "Holes",
            &[
                "drilling_toolpath",
                "drill_bank_toolpath",
                "thread_milling_toolpath",
                "keyhole_toolpath",
            ],
        ),
        group(
            "Carve and Engrave",
            &[
                "quick_engrave_toolpath",
                "vcarve_engrave_toolpath",
                "fluting_toolpath",
                "prism_toolpath",
                "photo_vcarve_toolpath",
                "sketch_carving_toolpath",
                "texture_toolpath",
            ],
        ),
        group(
            "Edges and Inlays",
            &[
                "chamfer_toolpath",
                "moulding_toolpath",
                "vcarve_inlay_toolpath",
                "inlay_toolpath",
            ],
        ),
        group(
            "3D Toolpaths",
            &[
                "3d_roughing_toolpath",
                "3d_finishing_toolpath",
                "3d_pencil_toolpath",
                "rounding_toolpath",
                "rotary_roughing_toolpath",
                "rotary_finishing_toolpath",
                "rotary_locked_angles_toolpath",
                "rotary_machining_toolpath",
                "rotary_lathe_toolpath",
            ],
        ),
        group(
            "Laser",
            &["laser_cut_fill_toolpath", "laser_picture_toolpath"],
        ),
        // new toolpaths from those already made, and templates to reuse them in other jobs
        group(
            "From Other Toolpaths",
            &["array_copy_toolpath", "merged_toolpath"],
        ),
        group(
            "Templates",
            &[
                "load_template",
                "save_selected_toolpath_as_template",
                "save_all_visible_toolpaths_as_template",
            ],
        ),
        // Analyse
        group("Reports", &["toolpath_summary"]),
        // Output: the Output tab draws this one as its big button
        group("Save for the Machine", &["save_toolpaths"]),
        group(
            "Paperwork for the Job",
            &[
                "create_job_sheet",
                "print_job_sheet",
                "part_labels",
                "cut_list",
            ],
        ),
        group("Big Jobs", &["toolpath_tiling"]),
    ]
}
