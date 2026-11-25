//! Every built-in action, the ones with no command crate (New, Save, Undo, Zoom to Job, Load
//! Toolpath Template...), each described once: its id, its name, what it does, its default
//! shortcut, whether it opens a window (its menu item ends in "..."), its icon where the file
//! is not named for its id, when it can run (and why not, when it is greyed out) and what it
//! runs. The menus, the tool pages, the tooltips, the status line, Find Tool, the shortcuts,
//! the help and [`CutCraftApp::invoke`] all read it, so an action is named, greyed and run the
//! same way everywhere. The English here is what `locales/en/tools.ftl` and `tooltips.ftl` say
//! (a test checks): what is shown comes from there, in the language in use.

use super::{CutCraftApp, Dialog, Info, Pages, Screen, Split, View, ViewTool, egui};
use cut_craft_machines::{MachineManager, PostManager};

/// When an action can run.
pub(crate) type When = fn(&CutCraftApp) -> bool;
/// What an action does.
pub(crate) type Run = fn(&mut CutCraftApp, &egui::Context);

/// One built-in action.
#[derive(Clone, Copy)]
pub(crate) struct BuiltIn {
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
    /// Whether it can run now; greyed out when not.
    pub when: When,
    /// Why it is greyed out (a key in `locales/en/actions.ftl`), when [`Self::when`] says no.
    pub why: &'static str,
    /// What it does.
    pub run: Run,
}

impl std::fmt::Debug for BuiltIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltIn")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl PartialEq for BuiltIn {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Runs nothing: an action given no [`BuiltIn::run`] yet, which a test catches.
fn nothing(_: &mut CutCraftApp, _: &egui::Context) {}

const fn act(id: &'static str, name: &'static str, about: &'static str) -> BuiltIn {
    BuiltIn {
        id,
        name,
        about,
        keys: "",
        window: false,
        icon: None,
        when: |_| true,
        why: "",
        run: nothing,
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

    const fn when(self, when: When, why: &'static str) -> Self {
        Self { when, why, ..self }
    }

    const fn run(self, run: Run) -> Self {
        Self { run, ..self }
    }

    /// Whether it has something to run.
    #[cfg(test)]
    pub fn runs(&self) -> bool {
        !std::ptr::fn_addr_eq(self.run, nothing as Run)
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
pub(crate) fn get(id: &str) -> Option<&'static BuiltIn> {
    BUILT_INS.iter().find(|b| b.id == id)
}

/// Every built-in action.
pub(crate) const BUILT_INS: &[BuiltIn] = &[
    act("setup_material", "Setup Material", "Setup Material: sets this job's material: single sided, double sided or rotary, and its size")
        .run(|a, _| a.open_setup_material()),
    act("import_into_shape", "Import Into Selected Shape", "Select a closed shape first (a circle, plaque or heart), then bring in a drawing or \
         picture sized to fit inside it: scaled, stretched or bent to follow its outline. \
         Select an open line instead to bend the drawing along it, like lettering on an arc")
        .run(|a, _| a.import_into_shape()),
    act("try_sample_project", "Try Sample Project", "Try Sample Project: opens a finished sample job to look round, preview and save, with nothing to set up first")
        .run(|a, _| a.open_sample()),
    act("machine_files", "Machine Files", "See the files on the machine's card or shared folder, and upload, rename, delete or \
         make folders there, without walking over to the machine").window()
        .run(|a, _| a.open_active_machine_files()),
    act("first_run_setup", "Run First-Time Setup Again", "First Run Setup: asks about your machine, units and material again, starting from your answers, and corrects My Machine (keeping the posts you added)").window()
        .run(|a, _| a.open_first_run()),
    act("draw_rectangle_from_center", "Draw Rectangle from Center", "Draw Rectangle from Center: drag from the middle out to a corner")
        .when(|a| a.variant_ready("draw_rectangle_from_center"), "actions-why-job-first")
        .run(|a, _| a.start_variant("draw_rectangle_from_center")),
    act("draw_circle_from_edge", "Draw Circle from Edge", "Draw Circle from Edge: drag from one side straight across to the other")
        .when(|a| a.variant_ready("draw_circle_from_edge"), "actions-why-job-first")
        .run(|a, _| a.start_variant("draw_circle_from_edge")),
    act("tool_changer", "Tool Changer", "Tool Changer: which bit is in each slot of your machine's automatic tool changer").window()
        .run(|a, _| a.tool_changer_window = true),
    act("find_tool", "Find Tool", "Find Tool: type part of a tool's name to find it and open it, wherever it is in the menus").keys("Ctrl+K").window()
        .when(CutCraftApp::job_open, "actions-why-job-first")
        .run(|a, _| a.search = Some(String::new())),
    act("activate_layer_of_selection", "Activate Layer of Selection", "Activate Layer of Selection: makes the selected vectors' layer the current one, so what you draw next goes on it")
        .when(|a| !a.doc.selection.is_empty(), "actions-why-select-first")
        .run(CutCraftApp::activate_layer_of_selection),
    act("machine_configuration", "Machine Configuration", "Machine Configuration: your machine's size, its post processor and the folder its files go to. If the saved machines can't be read at start, a copy is kept as machines.json.bad and they are never saved over").window()
        .run(|a, _| a.machine_window = Some(MachineManager::new(&a.machines))),
    act("add_machine", "Add Machine", "Add Machine: sets up another machine, with its size and the post processor that writes its G-code").window()
        .run(|a, _| a.machine_window = Some(MachineManager::adding(&a.machines))),
    act("install_offline_package", "Install Offline Package", "Install Offline Package: adds a machine set-up from a file you downloaded").window()
        .run(|a, _| a.install_package_picked()),
    act("search_for_machine_online", "Search for Machine Online", "Search for Machine Online: opens the list of ready-made machine set-ups in your browser").window()
        .run(|_, ctx| ctx.open_url(egui::OpenUrl::new_tab(super::MACHINES_ONLINE))),
    act("post_processor_management", "Post-Processor Management", "Post Processor Management: the post processors, which write G-code in the form each kind of machine reads").window()
        .run(|a, _| a.post_window = Some(PostManager::for_machine(a.machines.active()))),
    act("paste_in_place", "Paste in Place", "Paste in Place: adds the objects on the clipboard to the job exactly where they were copied from; a copied group or word of text pastes as a group of its own").keys("Ctrl+Shift+V")
        .when(|a| !a.clipboard.is_empty() || !a.clipboard_components.is_empty(), "actions-why-nothing-copied")
        .run(|a, _| a.paste_clipboard()),
    act("edit_text", "Edit Text", "Edit Text: opens the selected text in its form, to change its words, font or size (text \
         in a group too); double-clicking the text does the same")
        .when(CutCraftApp::text_picked, "actions-why-select-text")
        .run(|a, _| a.start_command("draw_text")),
    act("join_straight_line", "Join Straight Line", "Join with a Straight Line: joins the selected open vectors' nearest ends, or closes one, with a straight line")
        .run(|a, _| a.join_ends("straight_line")),
    act("join_smooth_curve", "Join Smooth Curve", "Join with a Smooth Curve: joins the selected open vectors' nearest ends, or closes one, with a curve that carries on smoothly")
        .run(|a, _| a.join_ends("smooth_curve")),
    act("join_move_end_points", "Join Move End Points", "Join by Moving End Points: moves the nearest ends of the selected open vectors to meet halfway, with no new line")
        .run(|a, _| a.join_ends("move_end_points")),
    act("import_vectors", "Import Vectors", "Bring in a drawing made in another program: DXF, SVG or a Vectric job's vectors")
        .run(|a, _| a.import_picked(Dialog::ImportVectors)),
    act("export_vectors", "Export Vectors", "Export Vectors: saves the selected vectors as a drawing file (DXF, SVG and more) for another program")
        .run(|a, _| a.export_vectors_picked()),
    act("open_folder", "Open Folder", "Open Folder: shows the job's folder in the file manager, or the settings folder before the job is saved")
        .run(|a, _| a.open_job_folder()),
    act("open_clipart_folder", "Open Clipart Folder", "Open Clipart Folder: shows the folder your clipart is kept in, in the file manager, to add or tidy files")
        .run(|a, _| a.open_clipart_folder()),
    act("component_levels", "Component Levels", "Component Levels: opens the Components tab, where the 3D model's levels and components are listed. A new component goes on the level of the component picked there, else on the top shown level")
        .run(|a, _| a.open_design_tab("Components")),
    act("clipart_library", "Clipart Library", "Open the Library tab: your folders of ready-made drawings, pictures and 3D clipart, \
         to drag onto the job")
        .run(|a, _| a.open_design_tab("Library")),
    act("edit_toolpath", "Edit Toolpath", "Edit Toolpath: opens the selected toolpath's form, to change its settings and calculate it again")
        .when(|a| a.selected_toolpath.is_some(), "actions-why-pick-toolpath")
        .run(|a, _| a.edit_toolpath()),
    act("create_shape_round", "Create Shape Round", "Create Shape, Round: raises the selected closed vectors into a 3D dome that rises at \
         the angle you set and rounds over the top")
        .run(|a, _| a.create_shape_with("round")),
    act("create_shape_angled", "Create Shape Angled", "Create Shape, Angled: raises the selected closed vectors into a 3D shape with straight \
         slopes, like a roof or a pyramid")
        .run(|a, _| a.create_shape_with("angular")),
    act("create_shape_smooth", "Create Shape Smooth", "Create Shape, Smooth: raises the selected closed vectors into a 3D shape whose sides \
         curve gently up out of the edge and over the top")
        .run(|a, _| a.create_shape_with("smooth")),
    act("create_shape_flat", "Create Shape Flat", "Create Shape, Flat: raises the selected closed vectors into a flat-topped 3D shape, \
         all one height, like a plateau")
        .run(|a, _| a.create_shape_with("flat")),
    act("left_to_right", "Left To Right", "Space Left to Right: spreads the selected objects evenly across, in the order you picked them from the left; the outer ones stay put")
        .run(|a, _| a.space_in_order("left_to_right")),
    act("right_to_left", "Right To Left", "Space Right to Left: spreads the selected objects evenly across, in the order you picked them from the right; the outer ones stay put")
        .run(|a, _| a.space_in_order("right_to_left")),
    act("top_to_bottom", "Top To Bottom", "Space Top to Bottom: spreads the selected objects evenly up and down, in the order you picked them from the top; the outer ones stay put")
        .run(|a, _| a.space_in_order("top_to_bottom")),
    act("bottom_to_top", "Bottom To Top", "Space Bottom to Top: spreads the selected objects evenly up and down, in the order you picked them from the bottom; the outer ones stay put")
        .run(|a, _| a.space_in_order("bottom_to_top")),
    act("delete_all_guides", "Delete All Guides", "Delete All Guides: removes every guide line from the 2D view")
        .run(|a, _| a.set_guides(vec![])),
    act("lock_all_existing_guides", "Lock All Existing Guides", "Lock All Existing Guides: stops the guide lines from being moved by accident")
        .run(|a, _| a.lock_guides(true)),
    act("unlock_all_existing_guides", "Unlock All Existing Guides", "Unlock All Existing Guides: lets the guide lines be dragged again")
        .run(|a, _| a.lock_guides(false)),
    act("options", "Options", "Options: Cut Craft's settings, such as units, colours and where files are kept")
        .run(|a, _| a.options_window = true),
    act("segment_3d_model_file", "Segment 3D Model", "Segment 3D Model File: splits a 3D model file into pieces thin enough for your material, to carve in layers").window()
        .run(|a, _| a.segment_model_file()),
    act("import_toolpaths_file", "Import Toolpaths (G-code)", "Read a G-code file (.nc, .tap, .gcode and the like) back in as toolpaths, to preview \
         and check it before it runs").window()
        .run(|a, _| a.import_toolpaths_picked()),
    act("print_job_sheet", "Print Job Sheet", "Open the job sheet in the web browser to print it: the job, the material, each \
         toolpath's tool and how long it takes").window()
        .run(|a, _| a.print_job_sheet()),
    act("import_cut_list", "Import Cut List", "Draw a part for each row of a spreadsheet's cut list (saved as CSV): name, length, \
         width and how many, ready for Nest Parts. Sizes are in the units after the Length \
         and Width headings, such as Length (in) or Length (mm), else in the job's units. \
         The note after importing names the units used for each, so a heading missing its \
         unit shows up").window()
        .run(|a, _| a.import_cut_list_picked()),
    act("snap_options", "Snap Options", "Snap Options: chooses what the pointer snaps to as you draw: the grid, guides, ends, middles and centres").keys("F4")
        .run(|a, _| a.snap_window = true),
    act("node_edit", "Node Edit", "Node Edit: shows the points (nodes) a vector is made of, to drag them, or right-click to add, delete or change them. Del deletes the nodes picked with a box; picked nodes are let go when you pick another vector, undo or leave Node Edit").keys("N")
        .run(|a, _| a.toggle_node_edit()),
    act("transform", "Transform", "Transform: click an object to move, size or rotate it with handles you drag; click it again to switch between sizing and rotating")
        .run(|a, _| a.toggle_transform()),
    act("measure_inspect", "Measure Inspect", "Measure: drag between two points to read the distance and angle between them (both ends snap to ends, middles and centres as drawing does; hold Shift to place them freely), or click an object to see its size")
        .run(|a, _| a.pick_view_tool(ViewTool::Measure, "measure_inspect")),
    act("print", "Print (opens a PDF)", "Print (opens a PDF): opens the 2D view as a PDF in your PDF viewer, to check and print from there").window()
        .run(|a, _| a.print_drawing()),
    act("save_all_visible_toolpaths_as_template", "Save All Shown Toolpaths as Template", "Save the settings and tools of every toolpath shown in the list, to make the same \
         cuts on another job").window().icon("toolpaths/save_all_as_template.png")
        .when(CutCraftApp::any_toolpath_shown, "actions-why-none-shown")
        .run(|a, _| a.save_toolpath_template(true)),
    act("load_template", "Load Toolpath Template", "Add the toolpaths saved in a template to this job, ready to pick the vectors (or \
         picture) each one cuts and calculate. A Merged or Array Copy toolpath uses the \
         toolpaths saved with it; one saved without them says to tick the ones to use").window().icon("toolpaths/load_toolpath_template.png")
        .run(|a, _| a.load_toolpath_template()),
    act("save_selected_toolpath_as_template", "Save Toolpath as Template", "Save the selected toolpath's settings and tools, to make the same cut on another job").window().icon("toolpaths/save_toolpath_template.png")
        .when(|a| a.selected_toolpath.is_some(), "actions-why-pick-toolpath")
        .run(|a, _| a.save_toolpath_template(false)),
    act("save_as_template", "Save As Template", "Save As Template: saves the job as a template, to start new jobs from it with the same material and toolpaths").window()
        .run(|a, _| a.save_job_template()),
    act("new_from_template", "New from Template", "New from Template: starts a new job from a saved template, with its material, vectors and toolpaths ready").window()
        .run(|a, _| a.new_from_template()),
    act("create_component_from_toolpath_preview", "Create Component from Toolpath Preview", "Create Component from Toolpath Preview: turns the previewed, cut material into a 3D component, to carve on or reuse. It goes on the level of the component picked in the tree, else the top shown level, and the status bar names it")
        .when(|a| a.simulation.is_some(), "actions-why-preview-first")
        .run(|a, _| a.component_from_preview()),
    act("toolpath_summary", "Toolpath Summary", "Toolpath Summary: lists every toolpath with its bit, depths and how long it takes to cut")
        .run(|a, _| a.info = Some(Info::Summary)),
    act("refresh_2d_view", "Refresh 2D View", "Refresh 2D View: draws the 2D view again from scratch").keys("F5")
        .run(|_, ctx| ctx.request_repaint()),
    act("cut", "Cut", "Cut: removes the selected objects and keeps them on the clipboard, to paste somewhere else").keys("Ctrl+X")
        .when(|a| !a.doc.selection.is_empty() || !a.doc.selected_components.is_empty(), "actions-why-select-to-cut")
        .run(|a, ctx| a.copy_selection(ctx, true)),
    act("copy", "Copy", "Copy: puts a copy of the selected objects on the clipboard, to paste").keys("Ctrl+C")
        .when(|a| !a.doc.selection.is_empty() || !a.doc.selected_components.is_empty(), "actions-why-select-to-copy")
        .run(|a, ctx| a.copy_selection(ctx, false)),
    act("paste", "Paste", "Paste: adds the objects on the clipboard to the job exactly where they were copied from, on their own layer (in another job, on the current layer); a copied group or word of text pastes as a group of its own, still text (a dimension stays a dimension). Lines lying on top of their original show red. Ctrl+V of a copy made in another Cut Craft window brings its vectors only: 3D components paste only in the window they were copied in").keys("Ctrl+V")
        .when(|a| !a.clipboard.is_empty() || !a.clipboard_components.is_empty(), "actions-why-nothing-copied")
        .run(|a, _| a.paste_clipboard()),
    act("increment_and_save", "Increment and Save", "Increment and Save: saves the job as a new copy with the next number on its name (sign_1, sign_2), keeping the earlier versions. A year or date on the end of the name stays as it is (Smith-2024 becomes Smith-2024_1, order_2024-11-15 becomes order_2024-11-15_1)").keys("Ctrl+Alt+S")
        .when(CutCraftApp::job_open, "actions-why-job-first")
        .run(|a, _| a.increment_and_save()),
    act("open_application_data_folder", "Open Application Data Folder", "Open Application Data Folder: opens the folder where Cut Craft keeps your settings, tools and machines")
        .run(|a, _| a.open_application_data_folder()),
    act("keyboard_shortcuts", "Keyboard Shortcuts", "Keyboard Shortcuts: lists the keys that run each tool, so you can work without the menus")
        .run(|a, _| a.info = Some(Info::Shortcuts)),
    act("help_contents", "Help Contents", "Help Contents: opens Cut Craft's help, with a page for every tool and a search box")
        .run(|a, _| a.info = Some(Info::Help("contents"))),
    act("did_you_know", "Did You Know?", "Did You Know?: short tips for getting more out of Cut Craft, one at a time").window()
        .run(|a, _| a.open_unseen_tip()),
    act("credits", "Credits", "Credits: the libraries, pictures and icons Cut Craft is built with, and their licences")
        .run(|a, _| a.info = Some(Info::Credits)),
    act("about_cut_craft", "About Cut Craft", "About Cut Craft: its version, its licence and the people who made it")
        .run(|a, _| a.info = Some(Info::About)),
    act("check_for_updates", "Check for Updates", "Check for Updates: asks straight away whether there is a newer version of Cut Craft").window()
        .run(CutCraftApp::check_for_updates),
    act("addons", "Add-ons", "Add-ons: installs, turns on and removes add-ons that bring new tools into Cut Craft. An add-on from GitHub never replaces Cut Craft's own add-ons or one installed from another repository. Remove asks first; a removed gadget leaves the Gadgets menu when Cut Craft restarts").window()
        .run(|a, ctx| a.addons.show(ctx)),
    act("open_gadgets_folder", "Open Gadgets Folder", "Open My Gadgets Folder: opens the folder where your own .lua gadget files go. Each one shows up in Gadgets ▸ My Gadgets when Cut Craft starts")
        .run(|a, _| a.open_gadgets_folder()),
    act("undo", "Undo", "Undo: takes back the last change").keys("Ctrl+Z")
        .when(|a| a.history.can_undo(), "actions-why-nothing-to-undo")
        .run(|a, _| a.undo()),
    act("redo", "Redo", "Redo: puts back the change you just undid").keys("Ctrl+Y")
        .when(|a| a.history.can_redo(), "actions-why-nothing-to-redo")
        .run(|a, _| a.redo()),
    act("exit", "Exit", "Exit: closes Cut Craft, offering to save your work first")
        .run(|_, ctx| ctx.send_viewport_cmd(egui::ViewportCommand::Close)),
    act("new", "New", "New: starts a new job, asking the size and thickness of your material first").keys("Ctrl+N")
        .run(|a, _| a.new_job()),
    act("open", "Open", "Open: opens a saved job to carry on with it").keys("Ctrl+O").window()
        .run(|a, _| a.open_job()),
    act("save", "Save", "Save: saves the job under its own name, or asks for a name the first time").keys("Ctrl+S")
        .run(|a, _| a.save_job()),
    act("save_as", "Save As", "Save As: saves the job under a new name or in another folder, starting from its own name and folder").window()
        .run(|a, _| a.save_as()),
    act("import_vectors_bitmap", "Import Vectors or Picture", "Bring in a drawing (DXF, SVG or a Vectric job's vectors) or a picture (PNG, JPG, \
         BMP, GIF or TIFF) to trace, carve round or draw over. An SVG sized in pixels is \
         read at 96 pixels per inch, and a note says the size it came in at").keys("Ctrl+I").window()
        .run(|a, _| a.import_picked(Dialog::ImportVectorsOrPicture)),
    act("import_component_3d_model", "Import 3D Model", "Bring in a 3D model (STL or OBJ) or saved 3D clipart, to size, place and carve").window()
        .run(|a, _| a.import_picked(Dialog::ImportModel)),
    act("recalculate_toolpath", "Recalculate Toolpath", "Recalculate Toolpath: works the selected toolpath out again, after its vectors, bits or material changed")
        .when(|a| a.selected_toolpath.is_some(), "actions-why-pick-toolpath")
        .run(|a, _| a.recalculate_selected()),
    act("recalculate_visible_toolpaths", "Recalculate Visible Toolpaths", "Recalculate Visible Toolpaths: works out again every toolpath ticked in the Toolpath List")
        .when(CutCraftApp::any_toolpath_shown, "actions-why-none-shown")
        .run(|a, _| a.recalculate_shown()),
    act("recalculate_all_toolpaths", "Recalculate All Toolpaths", "Recalculate All Toolpaths: works out every toolpath again, after you changed the vectors or the material. A merged or copied toolpath is worked out after the toolpaths it copies")
        .when(|a| !a.doc.toolpaths.is_empty(), "actions-why-no-toolpaths")
        .run(|a, _| a.recalculate_all()),
    act("delete_toolpath", "Delete Toolpath", "Delete Toolpath: removes a toolpath from the toolpath list")
        .when(|a| a.selected_toolpath.is_some(), "actions-why-pick-toolpath")
        .run(|a, _| a.delete_selected_toolpath()),
    act("save_toolpaths", "Save Toolpaths", "Save Toolpaths: writes the toolpaths as a G-code file for your machine to run")
        .when(|a| !a.doc.toolpaths.is_empty(), "actions-why-no-toolpaths")
        .run(|a, _| a.save_form = Some(a.default_save_form())),
    act("install_post_processor", "Install Post-Processor", "Install Post Processor: adds a post processor file for a machine Cut Craft does not list yet. One with a name already listed is kept beside it as a copy, named with a copy number and the date and time").window()
        .run(|a, _| a.install_post_picked()),
    act("close", "Close", "Close: closes the job you have open, offering to save it first")
        .when(|a| a.screen != Screen::Start, "actions-why-no-job")
        .run(|a, _| a.close_job()),
    act("select", "Select", "Select: puts away the tool in hand, so clicking picks objects, Shift-click adds to the pick, dragging draws a box round them (a slip of a pixel or two is still a click, not a move), and right-clicking an object picks it and opens its menu")
        .run(CutCraftApp::put_tool_away),
    act("select_all_vectors_on_current_layer", "Select All Vectors On Current Layer", "Select All Vectors on Current Layer: selects every vector on the layer you are working on")
        .run(CutCraftApp::select_current_layer),
    act("job_size_and_position", "Job Size and Position", "Job Size and Position: changes the material's size, thickness and where its origin is")
        .run(|a, _| a.start_command("job_setup")),
    act("show_toolpaths_tab", "Show Toolpaths Tab", "Show Toolpaths Page: opens the Toolpaths page, where toolpaths are made and previewed")
        .run(CutCraftApp::show_toolpaths_tab),
    act("design_layout", "Design Layout", "Design Layout: keeps the Design page open and puts the Toolpaths page away, for drawing").keys("F11")
        .run(|a, _| a.pages = Pages::design_layout()),
    act("machining_layout", "Machining Layout", "Machining Layout: keeps the Toolpaths page open and puts the Design page away, for making toolpaths").keys("F12")
        .run(|a, _| a.pages = Pages::machining_layout()),
    act("interface_layout", "Interface Layout", "Interface Layout: pick how the window is arranged: Ribbon, Rail, Canvas or Classic")
        .run(|a, _| a.open_layout_picker()),
    act("pan", "Pan", "Pan: drag the 2D view to slide the drawing round, without changing how close you are").icon("view/pan.png")
        .run(|a, _| a.pick_view_tool(ViewTool::Pan, "pan")),
    act("zoom_box", "Zoom Box", "Zoom Box: drag a box round the part of the drawing you want to fill the view").icon("view/zoom_box.png")
        .run(|a, _| a.pick_view_tool(ViewTool::ZoomBox, "zoom_box")),
    act("zoom_to_job", "Zoom to Job", "Zoom to Job: fits the whole material into the view").icon("view/zoom_to_job.png")
        .run(|a, _| a.camera = None),
    act("zoom_to_drawing", "Zoom to Drawing", "Zoom to Drawing: fits everything you have drawn into the view, even what is off the material. In the 3D view F does it too, with the pointer over the view and no text box being typed in; F6 always does").keys("F6")
        .run(|a, _| a.camera = None),
    act("zoom_selected", "Zoom to Selection", "Zoom Selected: zooms in or out until the selected objects fill the view").icon("view/zoom_selected.png")
        .when(|a| a.doc.selection_bounds().is_some(), "actions-why-select-first")
        .run(CutCraftApp::zoom_selected),
    act("tool_database", "Tool Database", "Tool Database: the list of your bits, with their sizes, speeds and feeds, to add, change or pick from. Renaming a bit renames it in this job's toolpaths too, and in other jobs when they open, so its tool changer slot still finds it").window()
        .run(CutCraftApp::open_tool_database),
    act("preview_toolpath", "Preview Toolpath", "Preview Toolpath: shows the selected toolpath, or every toolpath in the selected group, cutting the material in the 3D view")
        .when(|a| a.selected_toolpath.is_some() || a.selected_group.is_some(), "actions-why-pick-toolpath-or-group")
        .run(CutCraftApp::preview_selected),
    act("preview_all_toolpaths", "Preview All Toolpaths", "Preview All Toolpaths: shows every toolpath cutting the material in the 3D view, in list order")
        .when(|a| !a.doc.toolpaths.is_empty(), "actions-why-no-toolpaths")
        .run(|a, _| a.preview_all(false)),
    act("preview_visible_toolpaths", "Preview Visible Toolpaths", "Preview Visible Toolpaths: shows the toolpaths ticked in the Toolpath List cutting the material in the 3D view")
        .when(CutCraftApp::any_toolpath_shown, "actions-why-none-shown")
        .run(|a, _| a.preview_all(true)),
    act("undo_last_preview", "Undo Last Preview", "Undo Last Preview: goes back to the preview as it was before the last Preview or Reset \
         Preview, once, turned over to the side the job is on now")
        .when(|a| a.before_preview.is_some() && a.playback.is_none(), "actions-why-no-preview-to-undo")
        .run(CutCraftApp::undo_last_preview),
    act("save_shaded_image", "Save Shaded Image", "Save Shaded Image: saves a picture of the 3D view, to show what the finished piece will look like")
        .when(|a| a.simulation.is_some() || a.playback.is_some() || !a.doc.components.is_empty(), "actions-why-preview-or-model")
        .run(CutCraftApp::save_shaded_image_picked),
    act("edit_object", "Edit Object", "Edit Object: opens the selected object in the tool that made it, with its settings, to change them").keys("E")
        .when(|a| a.object_to_edit().is_some() || a.shape_to_edit(), "actions-why-select-shape")
        .run(CutCraftApp::edit_object),
    act("job_sheet_editor", "Job Setup Sheet Editor", "Job Setup Sheet Editor: chooses what the printed job sheet shows: the job, its tools and its toolpaths").window()
        .run(|a, _| a.job_sheet_window = true),
    act("reset_preview", "Reset Preview", "Reset Preview: clears the preview back to uncut material; Undo Last Preview brings it back")
        .when(|a| a.simulation.is_some() || a.playback.is_some() || a.kept_playback.is_some(), "actions-why-preview-first")
        .run(CutCraftApp::reset_preview),
    act("export_as_stl", "Export Model as STL", "Save the 3D model as an STL file, for a 3D printer or another program")
        .run(|a, _| (a.stl_window, a.stl_result) = (true, None)),
    act("export_as_grayscale_bitmap", "Export Model as Grayscale Picture", "Save the 3D model as a PNG picture where lighter is higher, for another carving \
         program")
        .run(|a, _| a.export("export_as_grayscale_bitmap")),
    act("export_as_3d_clipart", "Export Model as 3D Clipart", "Save the selected 3D parts as clipart, to drop into other jobs from the Library")
        .run(|a, _| a.export("export_as_3d_clipart")),
    act("create_job_sheet", "Save Job Sheet", "Save a page for the machine: the job, the material, each toolpath's tool and how long \
         it takes")
        .run(|a, _| a.create_job_sheet()),
    act("part_labels", "Print Part Labels", "Print a label for each named part, to stick on it once it is cut out").window()
        .run(|a, _| a.part_labels_window = true),
    act("clearing_settings", "Clearing Settings", "How adaptive and trochoidal clearing cut in this job: the light step, the loops, \
         and whether rows and loops keep climb on the way back").window()
        .run(|a, _| a.clearing_window = true),
    act("cut_list", "Save Cut List", "Save a list of the job's parts and their sizes, as a spreadsheet (CSV)").window()
        .run(|a, _| a.export("cut_list")),
    act("show_open_vector_ends", "Show Open Vector Ends", "Put a circle on each end of every open vector, so a gap that stops a shape being \
         pocketed or carved stands out")
        .run(|a, ctx| a.toggle_display(ctx, "show_open_vector_ends")),
    act("show_construction_lines", "Show Construction Lines", "Show or hide the dashed construction lines: lines that only place things and are \
         never cut. Hidden, they cannot be picked either")
        .run(CutCraftApp::show_construction_lines),
    act("tool_help", "Help on the Open Tool", "Help on the Open Tool: opens the help page of the tool whose form is open, else the help contents").keys("F1")
        .run(CutCraftApp::help_on_the_open_tool),
    act("view_2d", "2D View", "2D View: shows the flat view, where vectors are drawn and picked").keys("F2")
        .run(|a, _| a.view = View::TwoD),
    act("view_3d", "3D View", "3D View: shows the material, the 3D model and the toolpath previews in 3D").keys("F3")
        .run(|a, _| a.view = View::ThreeD),
    act("tile_windows_horizontal", "Tile Windows Horizontal", "Tile Windows Horizontal: shows the 2D and 3D views together, one above the other").keys("PgDn")
        .when(CutCraftApp::job_open, "actions-why-job-first")
        .run(|a, _| a.split = Some(Split::Stacked)),
    act("tile_windows_vertical", "Tile Windows Vertical", "Tile Windows Vertical: shows the 2D and 3D views together, side by side").keys("PgUp")
        .when(CutCraftApp::job_open, "actions-why-job-first")
        .run(|a, _| a.split = Some(Split::SideBySide)),
    act("cascade_windows", "Cascade Windows", "Cascade Windows: back to one view at a time, the 2D and 3D views on tabs")
        .when(CutCraftApp::job_open, "actions-why-job-first")
        .run(|a, _| a.split = None),
    act("add_clipart_folder", "Add Clipart Folder", "Add Clipart Folder: picks a folder of drawings, pictures or 3D clipart to show in the Library tab")
        .run(|a, _| a.add_clipart_folder()),
    act("split_stacked", "2D over 3D", "2D over 3D: shows the 2D view above the 3D view; press again for one view at a time")
        .run(|a, _| a.split_toggle(Split::Stacked)),
    act("split_side_by_side", "2D beside 3D", "2D beside 3D: shows the 2D and 3D views side by side; press again for one view at a time")
        .run(|a, _| a.split_toggle(Split::SideBySide)),
    act("split_picture", "One View in a Corner", "One View in a Corner: shows one view small in a corner of the other; press again for one view at a time")
        .run(|a, _| a.split_toggle(Split::Picture)),
    act("toggle_grid_snapping", "Grid Snapping", "Grid Snapping: turns snapping to the grid on or off")
        .run(|a, _| a.toggle_snapping(|s| &mut s.snap_to_grid)),
    act("toggle_smart_snapping", "Smart Snapping", "Smart Snapping: turns snapping to lines through points already drawn on or off")
        .run(|a, _| a.toggle_snapping(|s| &mut s.smart_snapping)),
    act("toggle_geometry_snapping", "Geometry Snapping", "Geometry Snapping: turns snapping to the ends, middles and centres of vectors on or off")
        .run(|a, _| a.toggle_snapping(|s| &mut s.geometry_snapping)),
    act("object_properties", "Object Properties", "Object Properties: opens the form of the selected shape, to change its size, place or settings")
        .when(|a| a.object_properties().is_some(), "actions-why-select-shape")
        .run(CutCraftApp::open_object_properties),
];

#[cfg(test)]
mod tests;
