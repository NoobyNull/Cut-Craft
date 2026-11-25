//! Every built-in action, the ones with no command crate (New, Save, Undo, Zoom to Job, Load
//! Toolpath Template...), each described once: its id, its name, what it does, its default
//! shortcut, whether it opens a window (its menu item ends in "...") and its icon where the
//! file is not named for its id. The menus, the tool pages, the tooltips, the status line,
//! Find Tool, the shortcuts and the help all read it, so an action is named the same way
//! everywhere. The English here is what `locales/en/tools.ftl` and `tooltips.ftl` say (a test
//! checks): what is shown comes from there, in the language in use.

/// One built-in action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltIn {
    pub id: &'static str,
    /// Its name in English, the same on its button, in the menus and in the help.
    pub name: &'static str,
    /// What it does in English: its tooltip and the status line.
    pub about: &'static str,
    /// Its default shortcut, as `cut_craft_core::shortcut` reads it; empty for none.
    pub keys: &'static str,
    /// It opens a window, so its menu item ends in "...".
    pub window: bool,
    /// Its icon, where the file is not named for its id.
    pub icon: Option<&'static str>,
}

const fn act(id: &'static str, name: &'static str, about: &'static str) -> BuiltIn {
    BuiltIn {
        id,
        name,
        about,
        keys: "",
        window: false,
        icon: None,
    }
}

impl BuiltIn {
    const fn keys(self, keys: &'static str) -> Self {
        Self { keys, ..self }
    }

    const fn window(self) -> Self {
        Self {
            window: true,
            ..self
        }
    }

    const fn icon(self, icon: &'static str) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    /// Its name as a menu item writes it: with "..." when it opens a window.
    pub fn menu_name(&self) -> String {
        if self.window {
            format!("{}...", self.name)
        } else {
            self.name.to_owned()
        }
    }
}

/// Built-in action `id`, if it is one.
pub fn get(id: &str) -> Option<&'static BuiltIn> {
    BUILT_INS.iter().find(|b| b.id == id)
}

/// Every built-in action.
pub const BUILT_INS: &[BuiltIn] = &[
    act("setup_material", "Setup Material", "Setup Material: sets this job's material: single sided, double sided or rotary, and its size"),
    act("import_into_shape", "Import Into Selected Shape", "Select a closed shape first (a circle, plaque or heart), then bring in a drawing or \
         picture sized to fit inside it: scaled, stretched or bent to follow its outline. \
         Select an open line instead to bend the drawing along it, like lettering on an arc"),
    act("try_sample_project", "Try Sample Project", "Try Sample Project: opens a finished sample job to look round, preview and save, with nothing to set up first"),
    act("machine_files", "Machine Files", "See the files on the machine's card or shared folder, and upload, rename, delete or \
         make folders there, without walking over to the machine").window(),
    act("first_run_setup", "Run First-Time Setup Again", "First Run Setup: asks about your machine, units and material again, starting from your answers, and corrects My Machine (keeping the posts you added)").window(),
    act("draw_rectangle_from_center", "Draw Rectangle from Center", "Draw Rectangle from Center: drag from the middle out to a corner"),
    act("draw_circle_from_edge", "Draw Circle from Edge", "Draw Circle from Edge: drag from one side straight across to the other"),
    act("tool_changer", "Tool Changer", "Tool Changer: which bit is in each slot of your machine's automatic tool changer").window(),
    act("find_tool", "Find Tool", "Find Tool: type part of a tool's name to find it and open it, wherever it is in the menus").keys("Ctrl+K").window(),
    act("activate_layer_of_selection", "Activate Layer of Selection", "Activate Layer of Selection: makes the selected vectors' layer the current one, so what you draw next goes on it"),
    act("machine_configuration", "Machine Configuration", "Machine Configuration: your machine's size, its post processor and the folder its files go to. If the saved machines can't be read at start, a copy is kept as machines.json.bad and they are never saved over").window(),
    act("add_machine", "Add Machine", "Add Machine: sets up another machine, with its size and the post processor that writes its G-code").window(),
    act("install_offline_package", "Install Offline Package", "Install Offline Package: adds a machine set-up from a file you downloaded").window(),
    act("search_for_machine_online", "Search for Machine Online", "Search for Machine Online: opens the list of ready-made machine set-ups in your browser").window(),
    act("post_processor_management", "Post-Processor Management", "Post Processor Management: the post processors, which write G-code in the form each kind of machine reads").window(),
    act("paste_in_place", "Paste in Place", "Paste in Place: adds the objects on the clipboard to the job exactly where they were copied from; a copied group or word of text pastes as a group of its own").keys("Ctrl+Shift+V"),
    act("edit_text", "Edit Text", "Edit Text: opens the selected text in its form, to change its words, font or size (text \
         in a group too); double-clicking the text does the same"),
    act("join_straight_line", "Join Straight Line", "Join with a Straight Line: joins the selected open vectors' nearest ends, or closes one, with a straight line"),
    act("join_smooth_curve", "Join Smooth Curve", "Join with a Smooth Curve: joins the selected open vectors' nearest ends, or closes one, with a curve that carries on smoothly"),
    act("join_move_end_points", "Join Move End Points", "Join by Moving End Points: moves the nearest ends of the selected open vectors to meet halfway, with no new line"),
    act("import_vectors", "Import Vectors", "Bring in a drawing made in another program: DXF, SVG or a Vectric job's vectors"),
    act("export_vectors", "Export Vectors", "Export Vectors: saves the selected vectors as a drawing file (DXF, SVG and more) for another program"),
    act("open_folder", "Open Folder", "Open: opens a saved job, or a drawing or picture to bring in"),
    act("open_clipart_folder", "Open Clipart Folder", "Open Clipart Folder: shows the folder your clipart is kept in, in the file manager, to add or tidy files"),
    act("component_levels", "Component Levels", "Component Levels: opens the Components tab, where the 3D model's levels and components are listed. A new component goes on the level of the component picked there, else on the top shown level"),
    act("clipart_library", "Clipart Library", "Open the Library tab: your folders of ready-made drawings, pictures and 3D clipart, \
         to drag onto the job"),
    act("edit_toolpath", "Edit Toolpath", "Edit Toolpath: opens the selected toolpath's form, to change its settings and calculate it again"),
    act("create_shape_round", "Create Shape Round", "Create Shape, Round: raises the selected closed vectors into a 3D dome that rises at \
         the angle you set and rounds over the top"),
    act("create_shape_angled", "Create Shape Angled", "Create Shape, Angled: raises the selected closed vectors into a 3D shape with straight \
         slopes, like a roof or a pyramid"),
    act("create_shape_smooth", "Create Shape Smooth", "Create Shape, Smooth: raises the selected closed vectors into a 3D shape whose sides \
         curve gently up out of the edge and over the top"),
    act("create_shape_flat", "Create Shape Flat", "Create Shape, Flat: raises the selected closed vectors into a flat-topped 3D shape, \
         all one height, like a plateau"),
    act("left_to_right", "Left To Right", "Space Left to Right: spreads the selected objects evenly across, in the order you picked them from the left; the outer ones stay put"),
    act("right_to_left", "Right To Left", "Space Right to Left: spreads the selected objects evenly across, in the order you picked them from the right; the outer ones stay put"),
    act("top_to_bottom", "Top To Bottom", "Space Top to Bottom: spreads the selected objects evenly up and down, in the order you picked them from the top; the outer ones stay put"),
    act("bottom_to_top", "Bottom To Top", "Space Bottom to Top: spreads the selected objects evenly up and down, in the order you picked them from the bottom; the outer ones stay put"),
    act("delete_all_guides", "Delete All Guides", "Delete All Guides: removes every guide line from the 2D view"),
    act("lock_all_existing_guides", "Lock All Existing Guides", "Lock All Existing Guides: stops the guide lines from being moved by accident"),
    act("unlock_all_existing_guides", "Unlock All Existing Guides", "Unlock All Existing Guides: lets the guide lines be dragged again"),
    act("options", "Options", "Options: Cut Craft's settings, such as units, colours and where files are kept"),
    act("segment_3d_model_file", "Segment 3D Model", "Segment 3D Model File: splits a 3D model file into pieces thin enough for your material, to carve in layers").window(),
    act("import_toolpaths_file", "Import Toolpaths (G-code)", "Read a G-code file (.nc, .tap, .gcode and the like) back in as toolpaths, to preview \
         and check it before it runs").window(),
    act("print_job_sheet", "Print Job Sheet", "Open the job sheet in the web browser to print it: the job, the material, each \
         toolpath's tool and how long it takes").window(),
    act("import_cut_list", "Import Cut List", "Draw a part for each row of a spreadsheet's cut list (saved as CSV): name, length, \
         width and how many, ready for Nest Parts. Sizes are in the units after the Length \
         and Width headings, such as Length (in) or Length (mm), else in the job's units. \
         The note after importing names the units used for each, so a heading missing its \
         unit shows up").window(),
    act("snap_options", "Snap Options", "Snap Options: chooses what the pointer snaps to as you draw: the grid, guides, ends, middles and centres").keys("F4"),
    act("node_edit", "Node Edit", "Node Edit: shows the points (nodes) a vector is made of, to drag them, or right-click to add, delete or change them. Del deletes the nodes picked with a box; picked nodes are let go when you pick another vector, undo or leave Node Edit").keys("N"),
    act("transform", "Transform", "Transform: click an object to move, size or rotate it with handles you drag; click it again to switch between sizing and rotating"),
    act("measure_inspect", "Measure Inspect", "Measure: drag between two points to read the distance and angle between them (both ends snap to ends, middles and centres as drawing does; hold Shift to place them freely), or click an object to see its size"),
    act("print", "Print (opens a PDF)", "Print (opens a PDF): opens the 2D view as a PDF in your PDF viewer, to check and print from there").window(),
    act("save_all_visible_toolpaths_as_template", "Save All Shown Toolpaths as Template", "Save the settings and tools of every toolpath shown in the list, to make the same \
         cuts on another job").window().icon("toolpaths/save_all_as_template.png"),
    act("load_template", "Load Toolpath Template", "Add the toolpaths saved in a template to this job, ready to pick the vectors (or \
         picture) each one cuts and calculate. A Merged or Array Copy toolpath uses the \
         toolpaths saved with it; one saved without them says to tick the ones to use").window().icon("toolpaths/load_toolpath_template.png"),
    act("save_selected_toolpath_as_template", "Save Toolpath as Template", "Save the selected toolpath's settings and tools, to make the same cut on another job").window().icon("toolpaths/save_toolpath_template.png"),
    act("save_as_template", "Save As Template", "Save As Template: saves the job as a template, to start new jobs from it with the same material and toolpaths").window(),
    act("new_from_template", "New from Template", "New from Template: starts a new job from a saved template, with its material, vectors and toolpaths ready").window(),
    act("create_component_from_toolpath_preview", "Create Component from Toolpath Preview", "Create Component from Toolpath Preview: turns the previewed, cut material into a 3D component, to carve on or reuse. It goes on the level of the component picked in the tree, else the top shown level, and the status bar names it"),
    act("toolpath_summary", "Toolpath Summary", "Toolpath Summary: lists every toolpath with its bit, depths and how long it takes to cut"),
    act("refresh_2d_view", "Refresh 2D View", "Refresh 2D View: draws the 2D view again from scratch").keys("F5"),
    act("cut", "Cut", "Cut: removes the selected objects and keeps them on the clipboard, to paste somewhere else").keys("Ctrl+X"),
    act("copy", "Copy", "Copy: puts a copy of the selected objects on the clipboard, to paste").keys("Ctrl+C"),
    act("paste", "Paste", "Paste: adds the objects on the clipboard to the job exactly where they were copied from, on their own layer (in another job, on the current layer); a copied group or word of text pastes as a group of its own, still text (a dimension stays a dimension). Lines lying on top of their original show red. Ctrl+V of a copy made in another Cut Craft window brings its vectors only: 3D components paste only in the window they were copied in").keys("Ctrl+V"),
    act("increment_and_save", "Increment and Save", "Increment and Save: saves the job as a new copy with the next number on its name (sign_1, sign_2), keeping the earlier versions. A year or date on the end of the name stays as it is (Smith-2024 becomes Smith-2024_1, order_2024-11-15 becomes order_2024-11-15_1)").keys("Ctrl+Alt+S"),
    act("open_application_data_folder", "Open Application Data Folder", "Open Application Data Folder: opens the folder where Cut Craft keeps your settings, tools and machines"),
    act("keyboard_shortcuts", "Keyboard Shortcuts", "Keyboard Shortcuts: lists the keys that run each tool, so you can work without the menus"),
    act("help_contents", "Help Contents", "Help Contents: opens Cut Craft's help, with a page for every tool and a search box"),
    act("did_you_know", "Did You Know?", "Did You Know?: short tips for getting more out of Cut Craft, one at a time").window(),
    act("credits", "Credits", "Credits: the libraries, pictures and icons Cut Craft is built with, and their licences"),
    act("about_cut_craft", "About Cut Craft", "About Cut Craft: its version, its licence and the people who made it"),
    act("check_for_updates", "Check for Updates", "Check for Updates: asks straight away whether there is a newer version of Cut Craft").window(),
    act("addons", "Add-ons", "Add-ons: installs, turns on and removes add-ons that bring new tools into Cut Craft. An add-on from GitHub never replaces Cut Craft's own add-ons or one installed from another repository. Remove asks first; a removed gadget leaves the Gadgets menu when Cut Craft restarts").window(),
    act("open_gadgets_folder", "Open Gadgets Folder", "Open My Gadgets Folder: opens the folder where your own .lua gadget files go. Each one shows up in Gadgets ▸ My Gadgets when Cut Craft starts"),
    act("undo", "Undo", "Undo: takes back the last change").keys("Ctrl+Z"),
    act("redo", "Redo", "Redo: puts back the change you just undid").keys("Ctrl+Y"),
    act("exit", "Exit", "Exit: closes Cut Craft, offering to save your work first"),
    act("new", "New", "New: starts a new job, asking the size and thickness of your material first").keys("Ctrl+N"),
    act("open", "Open", "Open: opens a saved job to carry on with it").keys("Ctrl+O").window(),
    act("save", "Save", "Save: saves the job under its own name, or asks for a name the first time").keys("Ctrl+S"),
    act("save_as", "Save As", "Save As: saves the job under a new name or in another folder, starting from its own name and folder").window(),
    act("import_vectors_bitmap", "Import Vectors or Picture", "Bring in a drawing (DXF, SVG or a Vectric job's vectors) or a picture (PNG, JPG, \
         BMP, GIF or TIFF) to trace, carve round or draw over. An SVG sized in pixels is \
         read at 96 pixels per inch, and a note says the size it came in at").keys("Ctrl+I").window(),
    act("import_component_3d_model", "Import 3D Model", "Bring in a 3D model (STL or OBJ) or saved 3D clipart, to size, place and carve").window(),
    act("recalculate_toolpath", "Recalculate Toolpath", "Recalculate Toolpath: works the selected toolpath out again, after its vectors, bits or material changed"),
    act("recalculate_visible_toolpaths", "Recalculate Visible Toolpaths", "Recalculate Visible Toolpaths: works out again every toolpath ticked in the Toolpath List"),
    act("recalculate_all_toolpaths", "Recalculate All Toolpaths", "Recalculate All Toolpaths: works out every toolpath again, after you changed the vectors or the material. A merged or copied toolpath is worked out after the toolpaths it copies"),
    act("delete_toolpath", "Delete Toolpath", "Delete Toolpath: removes a toolpath from the toolpath list"),
    act("save_toolpaths", "Save Toolpaths", "Save Toolpaths: writes the toolpaths as a G-code file for your machine to run"),
    act("install_post_processor", "Install Post-Processor", "Install Post Processor: adds a post processor file for a machine Cut Craft does not list yet. One with a name already listed is kept beside it as a copy, named with a copy number and the date and time").window(),
    act("close", "Close", "Close: closes the job you have open, offering to save it first"),
    act("select", "Select", "Select: puts away the tool in hand, so clicking picks objects, Shift-click adds to the pick, dragging draws a box round them (a slip of a pixel or two is still a click, not a move), and right-clicking an object picks it and opens its menu"),
    act("select_all_vectors_on_current_layer", "Select All Vectors On Current Layer", "Select All Vectors on Current Layer: selects every vector on the layer you are working on"),
    act("job_size_and_position", "Job Size and Position", "Job Size and Position: changes the material's size, thickness and where its origin is"),
    act("show_toolpaths_tab", "Show Toolpaths Tab", "Show Toolpaths Page: opens the Toolpaths page, where toolpaths are made and previewed"),
    act("design_layout", "Design Layout", "Design Layout: keeps the Design page open and puts the Toolpaths page away, for drawing").keys("F11"),
    act("machining_layout", "Machining Layout", "Machining Layout: keeps the Toolpaths page open and puts the Design page away, for making toolpaths").keys("F12"),
    act("interface_layout", "Interface Layout", "Interface Layout: pick how the window is arranged: Ribbon, Rail, Canvas or Classic"),
    act("pan", "Pan", "Pan: drag the 2D view to slide the drawing round, without changing how close you are").icon("view/pan.png"),
    act("zoom_box", "Zoom Box", "Zoom Box: drag a box round the part of the drawing you want to fill the view").icon("view/zoom_box.png"),
    act("zoom_to_job", "Zoom to Job", "Zoom to Job: fits the whole material into the view, however far you zoomed").icon("view/zoom_to_job.png"),
    act("zoom_to_drawing", "Zoom to Drawing", "Zoom to Drawing: fits everything you have drawn into the view, even what is off the material. In the 3D view F does it too, with the pointer over the view and no text box being typed in; F6 always does").keys("F6"),
    act("zoom_selected", "Zoom to Selection", "Zoom Selected: zooms in or out until the selected objects fill the view").icon("view/zoom_selected.png"),
    act("tool_database", "Tool Database", "Tool Database: the list of your bits, with their sizes, speeds and feeds, to add, change or pick from. Renaming a bit renames it in this job's toolpaths too, and in other jobs when they open, so its tool changer slot still finds it").window(),
    act("preview_toolpath", "Preview Toolpath", "Preview Toolpath: shows the selected toolpath, or every toolpath in the selected group, cutting the material in the 3D view"),
    act("preview_all_toolpaths", "Preview All Toolpaths", "Preview All Toolpaths: shows every toolpath cutting the material in the 3D view, in list order"),
    act("preview_visible_toolpaths", "Preview Visible Toolpaths", "Preview Visible Toolpaths: shows the toolpaths ticked in the Toolpath List cutting the material in the 3D view"),
    act("undo_last_preview", "Undo Last Preview", "Undo Last Preview: goes back to the preview as it was before the last Preview or Reset \
         Preview, once, turned over to the side the job is on now"),
    act("save_shaded_image", "Save Shaded Image", "Save Shaded Image: saves a picture of the 3D view, to show what the finished piece will look like"),
    act("edit_object", "Edit Object", "Edit Object: opens the selected object in the tool that made it, with its settings, to change them").keys("E"),
    act("job_sheet_editor", "Job Setup Sheet Editor", "Job Setup Sheet Editor: chooses what the printed job sheet shows: the job, its tools and its toolpaths").window(),
    act("reset_preview", "Reset Preview", "Reset Preview: clears the preview back to uncut material; Undo Last Preview brings it back"),
    act("export_as_stl", "Export Model as STL", "Save the 3D model as an STL file, for a 3D printer or another program"),
    act("export_as_grayscale_bitmap", "Export Model as Grayscale Picture", "Save the 3D model as a PNG picture where lighter is higher, for another carving \
         program"),
    act("export_as_3d_clipart", "Export Model as 3D Clipart", "Save the selected 3D parts as clipart, to drop into other jobs from the Library"),
    act("create_job_sheet", "Save Job Sheet", "Save a page for the machine: the job, the material, each toolpath's tool and how long \
         it takes"),
    act("part_labels", "Print Part Labels", "Print a label for each named part, to stick on it once it is cut out").window(),
    act("clearing_settings", "Clearing Settings", "How adaptive and trochoidal clearing cut in this job: the light step, the loops, \
         and whether rows and loops keep climb on the way back").window(),
    act("advanced_rotary_mode", "Turn On Advanced Rotary Mode", "Open Options at Advanced rotary mode: each rotary toolpath then cuts the whole blank, \
         not only what the toolpaths above it leave").window(),
    act("cut_list", "Save Cut List", "Save a list of the job's parts and their sizes, as a spreadsheet (CSV)").window(),
    act("show_open_vector_ends", "Show Open Vector Ends", "Put a circle on each end of every open vector, so a gap that stops a shape being \
         pocketed or carved stands out"),
    act("show_construction_lines", "Show Construction Lines", "Show or hide the dashed construction lines: lines that only place things and are \
         never cut. Hidden, they cannot be picked either"),
    act("tool_help", "Help on the Open Tool", "Help on the Open Tool: opens the help page of the tool whose form is open, else the help contents").keys("F1"),
    act("view_2d", "2D View", "2D View: shows the flat view, where vectors are drawn and picked").keys("F2"),
    act("view_3d", "3D View", "3D View: shows the material, the 3D model and the toolpath previews in 3D").keys("F3"),
    act("tile_windows_horizontal", "Tile Windows Horizontal", "Tile Windows Horizontal: shows the 2D and 3D views together, one above the other").keys("PgDn"),
    act("tile_windows_vertical", "Tile Windows Vertical", "Tile Windows Vertical: shows the 2D and 3D views together, side by side").keys("PgUp"),
    act("cascade_windows", "Cascade Windows", "Cascade Windows: back to one view at a time, the 2D and 3D views on tabs"),
];

#[cfg(test)]
mod tests;
