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
        "Edit Text: opens the selected text in its form, to change its words, font or size (text \
         in a group too); double-clicking the text does the same",
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
    (
        "select",
        "Select: puts away the tool in hand, so clicking picks objects, Shift-click adds to the pick, dragging draws a box round them (a slip of a pixel or two is still a click, not a move), and right-clicking an object picks it and opens its menu",
    ),
    (
        "transform",
        "Transform: click an object to move, size or rotate it with handles you drag; click it again to switch between sizing and rotating",
    ),
    (
        "node_edit",
        "Node Edit: shows the points (nodes) a vector is made of, to drag them, or right-click to add, delete or change them. Del deletes the nodes picked with a box; picked nodes are let go when you pick another vector, undo or leave Node Edit",
    ),
    (
        "measure_inspect",
        "Measure: drag between two points to read the distance and angle between them (both ends snap to ends, middles and centres as drawing does; hold Shift to place them freely), or click an object to see its size",
    ),
    (
        "pan",
        "Pan: drag the 2D view to slide the drawing round, without changing how close you are",
    ),
    (
        "zoom_box",
        "Zoom Box: drag a box round the part of the drawing you want to fill the view",
    ),
    (
        "zoom_selected",
        "Zoom Selected: zooms in or out until the selected objects fill the view",
    ),
    (
        "zoom_to_drawing",
        "Zoom to Drawing: fits everything you have drawn into the view, even what is off the material. In the 3D view F does it too, with the pointer over the view and no text box being typed in; F6 always does",
    ),
    (
        "zoom_to_job",
        "Zoom to Job: fits the whole material into the view",
    ),
    (
        "refresh_2d_view",
        "Refresh 2D View: draws the 2D view again from scratch",
    ),
    (
        "new",
        "New: starts a new job, asking the size and thickness of your material first",
    ),
    (
        "new_from_template",
        "New from Template: starts a new job from a saved template, with its material, vectors and toolpaths ready",
    ),
    ("open", "Open: opens a saved job to carry on with it"),
    (
        "close",
        "Close: closes the job you have open, offering to save it first",
    ),
    (
        "save",
        "Save: saves the job under its own name, or asks for a name the first time",
    ),
    (
        "save_as",
        "Save As: saves the job under a new name or in another folder, starting from its own name and folder",
    ),
    (
        "save_as_template",
        "Save As Template: saves the job as a template, to start new jobs from it with the same material and toolpaths",
    ),
    (
        "increment_and_save",
        "Increment and Save: saves the job as a new copy with the next number on its name (sign_1, sign_2), keeping the earlier versions. A year or date on the end of the name stays as it is (Smith-2024 becomes Smith-2024_1, order_2024-11-15 becomes order_2024-11-15_1)",
    ),
    (
        "recent_files",
        "Recent Files: the jobs you opened last, to open one again",
    ),
    (
        "exit",
        "Exit: closes Cut Craft, offering to save your work first",
    ),
    (
        "setup_material",
        "Setup Material: sets this job's material: single sided, double sided or rotary, and its size",
    ),
    (
        "open_folder",
        "Open: opens a saved job, or a drawing or picture to bring in",
    ),
    (
        "export_vectors",
        "Export Vectors: saves the selected vectors as a drawing file (DXF, SVG and more) for another program",
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
    (
        "print",
        "Print (opens a PDF): opens the 2D view as a PDF in your PDF viewer, to check and print from there",
    ),
    (
        "save_shaded_image",
        "Save Shaded Image: saves a picture of the 3D view, to show what the finished piece will look like",
    ),
    (
        "save_toolpaths",
        "Save Toolpaths: writes the toolpaths as a G-code file for your machine to run",
    ),
    (
        "toolpath_summary",
        "Toolpath Summary: lists every toolpath with its bit, depths and how long it takes to cut",
    ),
    (
        "recalculate_all_toolpaths",
        "Recalculate All Toolpaths: works out every toolpath again, after you changed the vectors or the material. A merged or copied toolpath is worked out after the toolpaths it copies",
    ),
    (
        "tool_database",
        "Bit Library: the list of your bits, with their sizes, speeds and feeds, to add, change or pick from. Renaming a bit renames it in this job's toolpaths too, and in other jobs when they open, so its tool changer slot still finds it",
    ),
    (
        "tool_changer",
        "Tool Changer (ATC): which bit is in each slot of your machine's automatic tool changer",
    ),
    (
        "find_tool",
        "Find Tool: type part of a tool's name to find it and open it, wherever it is in the menus",
    ),
    ("undo", "Undo: takes back the last change"),
    ("redo", "Redo: puts back the change you just undid"),
    (
        "cut",
        "Cut: removes the selected objects and keeps them on the clipboard, to paste somewhere else",
    ),
    (
        "copy",
        "Copy: puts a copy of the selected objects on the clipboard, to paste",
    ),
    (
        "paste",
        "Paste: adds the objects on the clipboard to the job exactly where they were copied from, on their own layer (in another job, on the current layer); a copied group or word of text pastes as a group of its own, still text (a dimension stays a dimension). Lines lying on top of their original show red. Ctrl+V of a copy made in another Cut Craft window brings its vectors only: 3D components paste only in the window they were copied in",
    ),
    (
        "paste_in_place",
        "Paste in Place: adds the objects on the clipboard to the job exactly where they were copied from; a copied group or word of text pastes as a group of its own",
    ),
    (
        "edit_object",
        "Edit Object: opens the selected object in the tool that made it, with its settings, to change them",
    ),
    (
        "select_all_vectors_on_current_layer",
        "Select All Vectors on Current Layer: selects every vector on the layer you are working on",
    ),
    (
        "component_levels",
        "Component Levels: opens the Components tab, where the 3D model's levels and components are listed. A new component goes on the level of the component picked there, else on the top shown level",
    ),
    (
        "join_straight_line",
        "Join with a Straight Line: joins the selected open vectors' nearest ends, or closes one, with a straight line",
    ),
    (
        "join_smooth_curve",
        "Join with a Smooth Curve: joins the selected open vectors' nearest ends, or closes one, with a curve that carries on smoothly",
    ),
    (
        "join_move_end_points",
        "Join by Moving End Points: moves the nearest ends of the selected open vectors to meet halfway, with no new line",
    ),
    (
        "left_to_right",
        "Space Left to Right: spreads the selected objects evenly across, in the order you picked them from the left; the outer ones stay put",
    ),
    (
        "right_to_left",
        "Space Right to Left: spreads the selected objects evenly across, in the order you picked them from the right; the outer ones stay put",
    ),
    (
        "top_to_bottom",
        "Space Top to Bottom: spreads the selected objects evenly up and down, in the order you picked them from the top; the outer ones stay put",
    ),
    (
        "bottom_to_top",
        "Space Bottom to Top: spreads the selected objects evenly up and down, in the order you picked them from the bottom; the outer ones stay put",
    ),
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
        "lock_all_existing_guides",
        "Lock All Existing Guides: stops the guide lines from being moved by accident",
    ),
    (
        "unlock_all_existing_guides",
        "Unlock All Existing Guides: lets the guide lines be dragged again",
    ),
    (
        "delete_all_guides",
        "Delete All Guides: removes every guide line from the 2D view",
    ),
    (
        "units_mm",
        "Millimetres: shows and types sizes in millimetres",
    ),
    ("units_inch", "Inches: shows and types sizes in inches"),
    (
        "job_size_and_position",
        "Job Size and Position: changes the material's size, thickness and where its origin is",
    ),
    (
        "show_toolpaths_tab",
        "Show Toolpaths Page: opens the Toolpaths page, where toolpaths are made and previewed",
    ),
    (
        "interface_layout",
        "Interface Layout: pick how the window is arranged: Ribbon, Rail, Canvas or Classic",
    ),
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
        "tile_windows_horizontal",
        "Tile Horizontally: shows the 2D and 3D views together, one above the other",
    ),
    (
        "tile_windows_vertical",
        "Tile Vertically: shows the 2D and 3D views together, side by side",
    ),
    (
        "cascade_windows",
        "Cascade Windows: back to one view at a time, the 2D and 3D views on tabs",
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
        "reset_preview",
        "Reset Preview: clears the preview back to uncut material; Undo Last Preview brings it back",
    ),
    (
        "undo_last_preview",
        "Undo Last Preview: goes back to the preview as it was before the last Preview or Reset \
         Preview, once, turned over to the side the job is on now",
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
        "create_component_from_toolpath_preview",
        "Create Component from Toolpath Preview: turns the previewed, cut material into a 3D component, to carve on or reuse. It goes on the level of the component picked in the tree, else the top shown level, and the status bar names it",
    ),
    (
        "segment_3d_model_file",
        "Segment 3D Model File: splits a 3D model file into pieces thin enough for your material, to carve in layers",
    ),
    (
        "machine_configuration",
        "Machine Configuration: your machine's size, its post processor and the folder its files go to. If the saved machines can't be read at start, a copy is kept as machines.json.bad and they are never saved over",
    ),
    (
        "add_machine",
        "Add Machine: sets up another machine, with its size and the post processor that writes its G-code",
    ),
    (
        "search_for_machine_online",
        "Search for Machine Online: opens the list of ready-made machine set-ups in your browser",
    ),
    (
        "install_offline_package",
        "Install Offline Package: adds a machine set-up from a file you downloaded",
    ),
    (
        "post_processor_management",
        "Post Processor Management: the post processors, which write G-code in the form each kind of machine reads",
    ),
    (
        "install_post_processor",
        "Install Post Processor: adds a post processor file for a machine Cut Craft does not list yet. One with a name already listed is kept beside it as a copy, named with a copy number and the date and time",
    ),
    (
        "job_sheet_editor",
        "Job Setup Sheet Editor: chooses what the printed job sheet shows: the job, its bits and its toolpaths",
    ),
    (
        "first_run_setup",
        "First Run Setup: asks about your machine, units and material again, starting from your answers, and corrects My Machine (keeping the posts you added)",
    ),
    (
        "options",
        "Options: Cut Craft's settings, such as units, colours and where files are kept",
    ),
    (
        "snap_options",
        "Snap Options: chooses what the pointer snaps to as you draw: the grid, guides, ends, middles and centres",
    ),
    (
        "keyboard_shortcuts",
        "Keyboard Shortcuts: lists the keys that run each tool, so you can work without the menus",
    ),
    (
        "help_contents",
        "Help Contents: opens Cut Craft's help, with a page for every tool and a search box",
    ),
    (
        "did_you_know",
        "Did You Know?: short tips for getting more out of Cut Craft, one at a time",
    ),
    (
        "check_for_updates",
        "Check for Updates: asks straight away whether there is a newer version of Cut Craft",
    ),
    (
        "about_cut_craft",
        "About Cut Craft: its version, its licence and the people who made it",
    ),
    (
        "credits",
        "Credits: the libraries, pictures and icons Cut Craft is built with, and their licences",
    ),
    (
        "open_application_data_folder",
        "Open Application Data Folder: opens the folder where Cut Craft keeps your settings, bits and machines",
    ),
    (
        "addons",
        "Add-ons: installs, turns on and removes add-ons that bring new tools into Cut Craft. An add-on from GitHub never replaces Cut Craft's own add-ons or one installed from another repository. Remove asks first; a removed gadget leaves the Gadgets menu when Cut Craft restarts",
    ),
    (
        "open_gadgets_folder",
        "Open My Gadgets Folder: opens the folder where your own .lua gadget files go. Each one shows up in Gadgets ▸ My Gadgets when Cut Craft starts",
    ),
    (
        "community:0",
        "GitHub: opens Cut Craft's page on GitHub, where you can report problems and see what is coming",
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

/// What built-in action `id` does, if it is listed.
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
