//! What the tools and menu items below do, where their command crate's own words will not do
//! (a crate holding several commands says one thing for all of them) or there is no crate: the
//! view and preview switches, the layouts and a gadget. The built-in actions say it in
//! [`crate::app::built_ins`].

/// Each id and what it does, in the person's words.
pub const HINTS: &[(&str, &str)] = &[
    (
        "align_left",
        "Align Left: moves the selected objects sideways so their left edges line up with the selection's left edge",
    ),
    (
        "align_right",
        "Align Right: moves the selected objects sideways so their right edges line up with the selection's right edge",
    ),
    (
        "align_top",
        "Align Top: moves the selected objects up or down so their top edges line up with the selection's top",
    ),
    (
        "align_bottom",
        "Align Bottom: moves the selected objects up or down so their bottom edges line up with the selection's bottom",
    ),
    (
        "center_horizontally",
        "Center Horizontally: moves the selected objects sideways so their centres line up on one vertical line. A group, like a word of text, moves as one piece",
    ),
    (
        "center_vertically",
        "Center Vertically: moves the selected objects up or down so their centres line up on one horizontal line. A group, like a word of text, moves as one piece",
    ),
    (
        "center",
        "Center: moves the selected objects so their centres all sit on one point, the middle of the selection. A group, like a word of text, moves as one piece",
    ),
    (
        "center_horizontally_in_material",
        "Center Horizontally in Material: moves the selection sideways to the middle of the material's width",
    ),
    (
        "center_vertically_in_material",
        "Center Vertically in Material: moves the selection up or down to the middle of the material's height",
    ),
    (
        "center_in_material",
        "Center in Material: moves the selection to the very middle of the material",
    ),
    (
        "select_all_vectors",
        "Select All Vectors: selects every vector on the layers that are shown and not locked",
    ),
    (
        "select_all_open_vectors",
        "Select All Open Vectors: selects every vector whose ends do not meet, often the ones a toolpath cannot use",
    ),
    (
        "select_all_duplicate_vectors",
        "Select All Duplicate Vectors: selects the extra copies of vectors drawn exactly on top of others, so you can delete them and keep one of each",
    ),
    (
        "select_vectors_inside",
        "Select Vectors Inside: adds every vector that lies wholly inside a selected closed shape, so a part with its holes is picked in one go",
    ),
    (
        "span_to_line",
        "Convert Span to Line: makes the span under the pointer a straight line between its two nodes",
    ),
    (
        "span_to_arc",
        "Convert Span to Arc: makes the span under the pointer a circular arc between its two nodes",
    ),
    (
        "span_to_bezier",
        "Convert Span to Bezier: makes the span under the pointer a curve with two handles you can drag to shape it",
    ),
    (
        "move_node",
        "Move Node: drags the node (a square) under the pointer to a new place; a curve's handles move with it",
    ),
    (
        "move_handle",
        "Move Handle: drags a curve's round handle in Node Edit to bend that span; its nodes stay put",
    ),
    (
        "ungroup_onto_current_layer",
        "Ungroup onto Current Layer: takes the selected groups apart one level (groups and text in them come out as they were) and moves their vectors onto the layer you are working on",
    ),
    (
        "merge_visible_layers",
        "Merge Visible: moves the vectors of every shown, unlocked layer onto this one. This layer is shown and unlocked so the merged work can be edited; locked layers are left as they are",
    ),
    (
        "group_toolpaths",
        "Group Toolpaths: puts the selected toolpaths together in a folder in the toolpath list, to keep a long list tidy. A folder can hold folders of its own",
    ),
    (
        "arrange_toolpaths",
        "Arrange Toolpaths: puts the toolpaths in a new cutting order, and in or out of groups, in one step, as dragging them (or whole groups) in the toolpath list does",
    ),
    (
        "show_toolpath_group",
        "Show or Hide Toolpath Group: the tick beside a group's name shows or hides every toolpath in it, its sub-groups' too, in one step",
    ),
    (
        "rename_toolpath_group",
        "Rename Toolpath Group: gives a folder of toolpaths a new name",
    ),
    (
        "ungroup_toolpaths",
        "Ungroup Toolpaths: takes a folder apart one level, its toolpaths and folders moving up into the folder it sat in, or deletes them with it",
    ),
    (
        "ungroup_merged_toolpath",
        "Ungroup Merged Toolpath: splits a merged toolpath back into the toolpaths it was made from",
    ),
    (
        "break_text_into_lines",
        "Break Block into Lines: splits a block of text into one text object per line, so each line can be moved on its own",
    ),
    (
        "make_real_line",
        "Make Real Line: turns the selected dashed construction lines back into normal vectors, so toolpaths can cut them and exported files include them",
    ),
    (
        "unselect_all",
        "Unselect All: clears the selection, so nothing is picked and no tool will change anything until you pick again",
    ),
    ("open", "Open: opens a saved job to carry on with it"),
    (
        "recent_files",
        "Recent Files: the jobs you opened last, to open one again",
    ),
    (
        "export_dxf",
        "Export as DXF: saves the selected vectors as a DXF file, which most drawing and CNC programs read",
    ),
    (
        "export_svg",
        "Export as SVG: saves the selected vectors as an SVG file, for laser cutters, vinyl cutters and drawing programs",
    ),
    (
        "export_pdf",
        "Export as PDF: saves the selected vectors as a PDF, to print or send",
    ),
    (
        "export_eps",
        "Export as EPS: saves the selected vectors as an EPS file, for older drawing and sign programs",
    ),
    (
        "export_ai",
        "Export as AI: saves the selected vectors as an Illustrator-style file, for drawing programs",
    ),
    ("undo", "Undo: takes back the last change"),
    ("redo", "Redo: puts back the change you just undid"),
    (
        "run:space_horizontally:{\"order\":\"left_to_right\",\"inside_last_vector\":false}",
        "Space Left to Right: spreads the selected objects evenly across, in the order you picked them from the left. A group, like a word of text, counts as one object",
    ),
    (
        "run:space_horizontally:{\"order\":\"right_to_left\",\"inside_last_vector\":false}",
        "Space Right to Left: spreads the selected objects evenly across, in the order you picked them from the right. A group, like a word of text, counts as one object",
    ),
    (
        "run:space_horizontally:{\"order\":\"as_placed\",\"inside_last_vector\":true}",
        "Space Horizontally Inside Last Vector: spreads the selected objects evenly across inside the vector you picked last",
    ),
    (
        "run:space_vertically:{\"order\":\"top_to_bottom\",\"inside_last_vector\":false}",
        "Space Top to Bottom: spreads the selected objects evenly up and down, in the order you picked them from the top",
    ),
    (
        "run:space_vertically:{\"order\":\"bottom_to_top\",\"inside_last_vector\":false}",
        "Space Bottom to Top: spreads the selected objects evenly up and down, in the order you picked them from the bottom",
    ),
    (
        "run:space_vertically:{\"order\":\"as_placed\",\"inside_last_vector\":true}",
        "Space Vertically Inside Last Vector: spreads the selected objects evenly up and down inside the vector you picked last",
    ),
    (
        "guides_visible",
        "Guides Visible: shows or hides the guide lines you dragged out of the rulers; drag a guide where no shape lies on it to move it, or right-click it for its properties, to lock it or to delete it",
    ),
    (
        "units_mm",
        "Millimetres: shows and types sizes in millimetres",
    ),
    ("units_inch", "Inches: shows and types sizes in inches"),
    (
        "layout_classic",
        "Classic Layout: the Design page on the left and the Toolpaths page on the right, opened from tabs at the sides",
    ),
    (
        "layout_canvas",
        "Canvas Layout: the whole window is the drawing; the tools, the settings and the job's lists float over it",
    ),
    (
        "layout_rail",
        "Rail Layout: a column of tool groups on the left, each opening its list of tools, and the settings on the right",
    ),
    (
        "layout_ribbon",
        "Ribbon Layout: tabs of tool groups across the top, the job's lists on the left and the settings on the right",
    ),
    (
        "auto_open_3d_view",
        "Open 3D View Automatically: switches to the 3D view on its own each time a toolpath is calculated",
    ),
    (
        "auto_wrapping_view",
        "Wrapped View: shows a rotary job wrapped round its cylinder in the 3D view, as it will be cut",
    ),
    (
        "animate_preview",
        "Animate Preview: shows a toolpath preview being cut step by step, instead of all at once",
    ),
    (
        "draw_2d_previews",
        "Show Toolpaths in 2D: draws the selected toolpaths' cuts over the drawing in the 2D view",
    ),
    (
        "make_2d_previews_solid",
        "Solid 2D Previews: fills in the toolpaths drawn in the 2D view, to show how much material each one removes",
    ),
    (
        "draw_rapid_moves",
        "Show Rapid Moves: draws the fast moves the bit makes above the material between cuts",
    ),
    (
        "draw_plunge_moves",
        "Show Plunge Moves: draws the moves where the bit goes down into the material",
    ),
    (
        "draw_retract_moves",
        "Show Retract Moves: draws the moves where the bit lifts out of the material",
    ),
    (
        "draw_join_moves",
        "Show Join Moves: draws the short moves that link one cut to the next without lifting",
    ),
    (
        "draw_material_block",
        "Show Material Block: draws the block of material in the 3D view",
    ),
    (
        "draw_modeling_plane",
        "Show Modeling Plane: draws the flat plane the 3D model is built on in the 3D view",
    ),
    (
        "draw_origin",
        "Draw Origin: marks the job's zero point, where the machine's X, Y and Z start",
    ),
    (
        "draw_vectors_in_3d_view",
        "Show Vectors in 3D: draws the vectors over the model in the 3D view",
    ),
    (
        "draw_keep_out_zones_in_3d_view",
        "Show Keep-Out Zones in 3D: draws the areas the bit must stay out of, such as clamps, in the 3D view",
    ),
    (
        "show_keep_out_zone_labels",
        "Show Keep-Out Zone Labels: writes each keep-out zone's name on it in the views",
    ),
    (
        "use_shaded_background",
        "Shaded Background: fades the 3D view's background from light to dark instead of a plain colour",
    ),
    (
        "use_shadow_shading",
        "Shadows: shades the 3D view with shadows, so shallow carving is easier to see",
    ),
    (
        "color_shaded_view",
        "Colour Shaded View: shows the 3D model in its material's colour, wood with its grain, instead of plain grey",
    ),
    (
        "light_follows_viewer",
        "Light Follows Viewer: keeps the light behind you as you turn the 3D view, so the side facing you stays lit",
    ),
    (
        "model_resolution_standard",
        "Model Resolution Standard: the 3D model in normal detail, quickest to work with",
    ),
    (
        "model_resolution_high",
        "Model Resolution High: the 3D model in finer detail, about three times slower to work with",
    ),
    (
        "model_resolution_highest",
        "Model Resolution Highest: the 3D model in the finest detail, for small, detailed carvings; much slower",
    ),
    (
        "preview_quality_1",
        "Preview Quality Standard: moves the Preview detail slider to 20%, quick to work out; the preview showing is worked out again",
    ),
    (
        "preview_quality_2",
        "Preview Quality High: moves the Preview detail slider to 40%; the preview showing is worked out again",
    ),
    (
        "preview_quality_4",
        "Preview Quality Very High: moves the Preview detail slider to 60%; the preview showing is worked out again",
    ),
    (
        "preview_quality_8",
        "Preview Quality Extremely High: moves the Preview detail slider to 80%; the preview showing is worked out again",
    ),
    (
        "preview_quality_25",
        "Preview Quality Maximum: moves the Preview detail slider to 100%, the finest detail, for small bits and fine carving; the preview showing is worked out again",
    ),
    (
        "community:0",
        "GitHub: opens PowerWhittle's page on GitHub, where you can report problems and see what is coming",
    ),
    (
        "community:1",
        "Facebook: opens Facebook's home page in your browser",
    ),
    ("community:2", "X: opens the X home page in your browser"),
    (
        "community:3",
        "YouTube: opens YouTube's home page in your browser",
    ),
    (
        "community:4",
        "Instagram: opens Instagram's home page in your browser",
    ),
    (
        "community:5",
        "Pinterest: opens Pinterest's home page in your browser, for project ideas",
    ),
    (
        "community:6",
        "Thingiverse: opens Thingiverse's home page in your browser, for designs to download",
    ),
    (
        "gadget_drill_grid",
        "Drill Grid: draws rows and columns of circles in the middle of the job, for pegboards, shelf-pin holes and speaker grilles",
    ),
    (
        "gadget_hole_pattern",
        "Hole Pattern: draws circles spaced evenly round a circle in the middle of the job, for bolt circles and clock faces",
    ),
];

/// What `id` does, if it is listed.
pub fn hint(id: &str) -> Option<&'static str> {
    HINTS.iter().find(|(i, _)| *i == id).map(|(_, h)| *h)
}

#[cfg(test)]
mod tests {
    #[test]
    fn each_action_is_listed_once() {
        let mut ids: Vec<&str> = super::HINTS.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(
            ids.len(),
            before,
            "an id listed twice: only the first is read"
        );
    }
}
