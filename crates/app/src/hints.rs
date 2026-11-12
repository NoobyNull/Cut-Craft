//! What the app's own tools and menu items do, for those with no command crate to say it:
//! their tooltip and status bar text, and their help page when they have no form.

/// Each built-in action's id and what it does, in the person's words.
pub const HINTS: &[(&str, &str)] = &[
    (
        "create_shape_round",
        "Create Shape, Round: raises the selected closed vectors into a 3D dome that rises at \
         the angle you set and rounds over the top",
    ),
    (
        "create_shape_angled",
        "Create Shape, Angled: raises the selected closed vectors into a 3D shape with straight \
         slopes, like a roof or a pyramid",
    ),
    (
        "create_shape_smooth",
        "Create Shape, Smooth: raises the selected closed vectors into a 3D shape whose sides \
         curve gently up out of the edge and over the top",
    ),
    (
        "create_shape_flat",
        "Create Shape, Flat: raises the selected closed vectors into a flat-topped 3D shape, \
         all one height, like a plateau",
    ),
    (
        "edit_text",
        "Edit Text: opens the selected text in its form, to change its words, font or size",
    ),
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
        "Center Horizontally: moves the selected objects sideways so their centres line up on one vertical line",
    ),
    (
        "center_vertically",
        "Center Vertically: moves the selected objects up or down so their centres line up on one horizontal line",
    ),
    (
        "center",
        "Center: moves the selected objects so their centres all sit on one point, the middle of the selection",
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
        "unselect_all",
        "Unselect All: clears the selection, so nothing is selected",
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
        "ungroup_onto_current_layer",
        "Ungroup onto Current Layer: takes the selected groups apart and moves their vectors onto the layer you are working on",
    ),
    (
        "group_toolpaths",
        "Group Toolpaths: puts the selected toolpaths together in a folder in the toolpath list, to keep a long list tidy",
    ),
    (
        "rename_toolpath_group",
        "Rename Toolpath Group: gives a folder of toolpaths a new name",
    ),
    (
        "ungroup_toolpaths",
        "Ungroup Toolpaths: takes the toolpaths out of their folder, keeping them, or deletes them with it",
    ),
    (
        "ungroup_merged_toolpath",
        "Ungroup Merged Toolpath: splits a merged toolpath back into the toolpaths it was made from",
    ),
    (
        "break_text_into_lines",
        "Break Block into Lines: splits a block of text into one text object per line, so each line can be moved on its own",
    ),
];

/// What built-in action `id` does, if it is listed.
pub fn hint(id: &str) -> Option<&'static str> {
    HINTS.iter().find(|(i, _)| *i == id).map(|(_, h)| *h)
}
