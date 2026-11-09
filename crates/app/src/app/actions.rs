//! What the window does: every action's dispatch, files and templates, forms, shortcuts,
//! menus, the start page and the Design and Toolpaths pages.

use super::{
    APP_NAME, Align, BUILT_INS, CONTRIBUTORS, Change, CutCraftApp, DESIGN_PANEL_WIDTHS,
    DESIGN_TABS, Document, EXPORT_LEVEL, EntityId, History, Icons, Info, JOB_EXTENSION, Key,
    KeyboardShortcut, LAYOUT_KEYS, LINKS, LIVE_FORMS, LOGO, Layout, MODEL_IMPORTERS, Menus,
    Modifiers, OpenForm, PLAYBACK_SPEED, Page, Pages, Point, RECENT_FILE_ICON,
    SECTION_PICTURE_ROWS, SELECT_LEVEL, SPLASH_SHARE, STARTUP_TASKS, SaveChecks, SaveForm, Screen,
    Session, Shown, TEMPLATE, THIRD_PARTY, TOOL_ICON_ROWS, TOP_BAR_TOGGLES, ToolDatabase,
    VECTOR_EXPORTERS, VECTOR_IMPORTERS, Value, View, WHATS_NEW, brush_dialog, commands,
    dialog_size, egui, file_dialog, file_name, files, form, form_settings_dialog, gui, job_dialog,
    menu_items, message_dialog, on_the_desktop, rest_section, selector_section, shortcut,
    side_panel, side_tabs, summary_table, template_dialog, texture_dialog, tool_list, tool_picker,
    toolpath_template_dialog, view3d,
};
use crate::options::{CuttingChecks, IntersectionCheck};
use cut_craft_bug_report::hint::HoverHint;

impl CutCraftApp {
    #[expect(
        clippy::too_many_lines,
        reason = "one line per field of the window's state"
    )]
    pub(super) fn new() -> Self {
        let mut registry = commands::registry();
        let my_gadgets = crate::gadgets::my_gadgets_dir();
        let addon_gadgets = crate::depth::addons_dir()
            .map(|dir| crate::gadgets::from_addons(&dir))
            .unwrap_or_default();
        let gadgets = crate::gadgets::load(&mut registry, my_gadgets.as_deref(), &addon_gadgets);
        let mut menus = gui::MenuSystem::new();
        menus.add_gadgets(&gadgets, my_gadgets.as_deref());
        let mut status = gui::StatusBar::new();
        if !gadgets.problems.is_empty() {
            status.set_message(&format!(
                "Some gadgets could not be loaded: {}",
                gadgets.problems.join("; ")
            ));
        }
        Self {
            screen: Screen::Start,
            start_menus: menus.without_job(),
            doc: Document::default(),
            history: History::default(),
            registry: std::sync::Arc::new(registry),
            icons: Icons::default_set(),
            menus,
            toolbar: gui::Toolbar::new(),
            status,
            design_tab: 0,
            design_subtab: 0,
            first_steps: std::cell::RefCell::default(),
            view: View::TwoD,
            cursor: None,
            form: None,
            view_transform: None,
            view_area: None,
            view_3d_area: None,
            shaded_image: None,
            shaded_saved: std::sync::Arc::default(),
            camera: None,
            view_tool: None,
            pages: Pages::default(),
            search: None,
            inspector_tab: 0,
            rail_group: None,
            rail_pinned: false,
            ribbon_tab: 0,
            options_path: None,
            words_path: None,
            speller: None,
            speller_looked: false,
            session: Session::default(),
            selected_toolpath: None,
            selected_group: None,
            immediate_output: SaveForm {
                mode: files::SaveMode::Selected,
                group: false,
                post: None,
                add_side: false,
                changes: files::ToolChanges::SeparateFiles,
            },
            depth_estimate: None,
            busy: None,
            wait_before_showing: crate::busy::SHOW_AFTER,
            addons: crate::addons::Addons::new(None),
            machines: cut_craft_machines::Machines::default(),
            machines_path: None,
            posts: cut_craft_machines::Library::new(None),
            machine_window: None,
            post_window: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tool_manager: None,
            shots: None,
            space_mouse: None,
            unsigned_gadgets: gadgets
                .mine
                .iter()
                .map(|g| (g.id.clone(), g.label.clone()))
                .collect(),
            unsigned_gadgets_allowed: std::collections::HashSet::new(),
            unsigned_gadget_ok: ask_to_run_unsigned,
            reporter: cut_craft_bug_report::Reporter::new(
                super::BUG_REPO,
                files::config_dir().map(|d| d.join("bug-reports")),
            ),
            updates: crate::update::Updates::default(),
            restart_after_close: None,
            clipboard: vec![],
            clipboard_components: vec![],
            fit_3d: None,
            handles_3d: vec![],
            under_3d: vec![],
            drag_3d: None,
            drag_base: None,
            settings_3d: None,
            rings_3d: (vec![], None),
            turn_ring: None,
            info: None,
            help_cache: egui_commonmark::CommonMarkCache::default(),
            sketch_areas: None,
            summary: super::Summary::default(),
            stroke: Vec::new(),
            snap: cut_craft_core::snap::SnapOptions::default(),
            snap_window: false,
            job_sheet_window: false,
            part_labels_window: false,
            stl_window: false,
            stl_result: None,
            layers_on_sheet: false,
            save_form: None,
            save_checks: None,
            before_preview: None,
            picked_nodes: Vec::new(),
            node_mirror: [false, false],
            menu_at: None,
            guide_props: None,
            tool_changer_window: false,
            list_height: None,
            measure_rows: Vec::new(),
            display: crate::display::Display::default(),
            simulation: None,
            simulation_coarse: None,
            import_level: None,
            polyline: crate::polyline::Draft::default(),
            fine_3d: None,
            drawn_on: None,
            model_3d: None,
            gpu_3d: None,
            surface_gpu: None,
            solid_gpu: None,
            model_gpu: None,
            moved_3d: 0.0,
            orbit: view3d::Orbit::default(),
            framing: view3d::Framing::default(),
            view_cube: None,
            recent: vec![],
            playback: None,
            playing: false,
            playback_speed: PLAYBACK_SPEED,
            collapsed_levels: std::collections::HashSet::new(),
            guide_drag: None,
            rotating: false,
            options: crate::options::Options::default(),
            options_window: false,
            options_page: 0,
            show_advanced: false,
            split: None,
            clipart: crate::clipart::Clipart::default(),
            import_queue: std::collections::VecDeque::new(),
            recent_path: None,
            recovery_path: None,
            autosaved: (0, 0.0),
            autosave: None,
            title: String::new(),
            title_bar_at_start: false,
        }
    }

    pub(super) fn available(&self, id: &str) -> bool {
        if let Some((_, command, _)) = gui::command_action(id) {
            return self.registry.get(command).is_some();
        }
        match id {
            "undo" => self.history.can_undo(),
            "close" => self.screen != Screen::Start,
            "zoom_selected" => self.doc.selection_bounds().is_some(),
            id if id.starts_with("open_recent:") || id.starts_with(gui::EDIT_POLYLINE) => true,
            id if id.starts_with(gui::REIMPORT_MODEL) => self
                .model_source(id)
                .is_some_and(|s| s.find(self.job_folder()).is_some()),
            id if id.starts_with(SELECT_LEVEL) || id.starts_with(gui::SELECT_COMPONENT) => true,
            id if id.starts_with(gui::FAVOURITE) || id.starts_with(gui::FAVOURITES_ONLY) => true,
            "add_clipart_folder" => true,
            id if TOP_BAR_TOGGLES.contains(&id) => true,
            id if id.starts_with(gui::GUIDE) => true,
            id if id.starts_with(gui::SAVE_BRUSH) => self.brush_component(id).is_some(),
            id if id.starts_with(EXPORT_LEVEL) => {
                self.registry.exporter("export_as_3d_clipart").is_some()
            }
            "redo" => self.history.can_redo(),
            "recalculate_toolpath"
            | "delete_toolpath"
            | "save_selected_toolpath_as_template"
            | "edit_toolpath" => self.selected_toolpath.is_some(),
            "preview_toolpath" => self.selected_toolpath.is_some() || self.selected_group.is_some(),
            "edit_text" => self.doc.selection.iter().any(|id| {
                self.doc.vector_groups.get(id).is_some_and(|g| {
                    self.doc
                        .texts
                        .get(g)
                        .is_some_and(|t| t.get("text").is_some())
                })
            }),
            "save_all_visible_toolpaths_as_template" => {
                self.doc.toolpaths.values().any(|t| t.visible)
            }
            "recalculate_all_toolpaths" | "preview_all_toolpaths" | "save_toolpaths" => {
                !self.doc.toolpaths.is_empty()
            }
            "recalculate_visible_toolpaths" | "preview_visible_toolpaths" => {
                self.doc.toolpaths.values().any(|t| t.visible)
            }
            "undo_last_preview" => self.before_preview.is_some() && self.playback.is_none(),
            "save_shaded_image" => {
                self.simulation.is_some()
                    || self.playback.is_some()
                    || !self.doc.components.is_empty()
            }
            "edit_object" => self.object_to_edit().is_some() || self.shape_to_edit(),
            "object_properties" => self.object_properties().is_some(),
            "tile_windows_horizontal" | "tile_windows_vertical" | "cascade_windows" => {
                self.screen == Screen::Job
            }
            "update_post_processor_database" | "job_sheet_editor" => true,
            "reset_preview" => self.simulation.is_some() || self.playback.is_some(),
            id if self.display.checked(id).is_some() => true,
            id if self.resolution_checked(id).is_some() => self.screen == Screen::Job,
            id if self.units_checked(id).is_some() => self.screen == Screen::Job,
            id if self.layout_checked(id).is_some() => true,
            "cut" | "copy" => {
                !self.doc.selection.is_empty() || !self.doc.selected_components.is_empty()
            }
            "create_component_from_toolpath_preview" => self.simulation.is_some(),
            "paste" | "paste_in_place" => {
                !self.clipboard.is_empty() || !self.clipboard_components.is_empty()
            }
            "activate_layer_of_selection" => !self.doc.selection.is_empty(),
            "increment_and_save" | "find_tool" => self.screen == Screen::Job,
            id if crate::flyout::variant(id).is_some() => {
                crate::flyout::variant(id).is_some_and(|v| self.registry.get(v.command).is_some())
                    && self.screen != Screen::Start
            }
            _ => {
                BUILT_INS.contains(&id)
                    // a command's form shows in a job's pages: with no job it could not be seen
                    || (self.registry.get(id).is_some() && self.screen != Screen::Start)
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Menu items, tool buttons and shortcuts all end up here: built-in actions first, then
    /// the command registry.
    #[expect(clippy::too_many_lines, reason = "the one dispatch of every action")]
    pub(super) fn invoke(&mut self, ctx: &egui::Context, id: &str) {
        self.show_subtab_of(id);
        // right-click menu items: a command with params chosen by the menu
        if let Some((form, command, params)) = gui::command_action(id) {
            if form {
                self.start_command(command);
                if let Some(open) = self.form.as_mut().filter(|f| f.id == command) {
                    open.params = params;
                }
            } else if let Err(e) = self.apply(command, &params) {
                self.status.set_message(&e);
            }
            return;
        }
        match id {
            "undo" => {
                if self.history.undo(&mut self.doc) {
                    self.status.set_message("Undone");
                }
            }
            "redo" => {
                if self.history.redo(&mut self.doc) {
                    self.status.set_message("Redone");
                }
            }
            // closing goes through the close request, which checks for unsaved work
            "exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            "new"
            | "open"
            | "close"
            | "save"
            | "save_as"
            | "import_vectors_bitmap"
            | "import_component_3d_model"
            | "segment_3d_model_file"
            | "import_toolpaths_file"
            | "import_cut_list"
            | "install_post_processor"
            | "add_clipart_folder" => self.file_action(id),
            "save_as_template"
            | "new_from_template"
            | "save_selected_toolpath_as_template"
            | "save_all_visible_toolpaths_as_template"
            | "load_template" => self.template_action(id),
            id if TOP_BAR_TOGGLES.contains(&id) => self.top_bar_toggle(id),
            // Aspire's four Create Shape icons: the form with that profile chosen
            "create_shape_round"
            | "create_shape_angled"
            | "create_shape_smooth"
            | "create_shape_flat" => {
                let profile = match id {
                    "create_shape_angled" => "angular",
                    _ => &id["create_shape_".len()..],
                };
                self.start_command("create_shape");
                if let Some(form) = self.form.as_mut().filter(|f| f.id == "create_shape") {
                    form.params["profile"] = Value::from(profile);
                }
            }
            // the Align Objects group's spacing buttons, by pick order
            "left_to_right" | "right_to_left" | "top_to_bottom" | "bottom_to_top" => {
                let command = if id.contains("left") {
                    "space_horizontally"
                } else {
                    "space_vertically"
                };
                if let Err(e) = self.apply(command, &serde_json::json!({ "order": id })) {
                    self.status.set_message(&e);
                }
            }
            "edit_toolpath" => self.edit_toolpath(),
            // a drawing tool's toolbox variant: its form, the drag read the variant's way
            id if crate::flyout::variant(id).is_some() => self.start_variant(id),
            // Edit Text: Draw Text opens on the selected text's settings
            "edit_text" => self.start_command("draw_text"),
            // Aspire's three Join icons: Join / Close Vectors their way
            "join_straight_line" | "join_smooth_curve" | "join_move_end_points" => {
                let method = id.trim_start_matches("join_");
                let params = serde_json::json!({ "method": method });
                if let Err(e) = self.apply("join_close_vectors", &params) {
                    self.status.set_message(&e);
                }
            }
            "import_vectors" | "export_vectors" | "open_folder" | "open_clipart_folder" => {
                self.file_icon_action(id);
            }
            // the Components tab, as Aspire's Component Levels icon shows
            "component_levels" => {
                self.set_design_tab(
                    DESIGN_TABS
                        .iter()
                        .position(|&t| t == "Components")
                        .unwrap_or_default(),
                );
                self.pages.open(Page::Design);
            }
            "delete_all_guides" => self.set_guides(vec![]),
            // the current layer becomes the selected vectors' (the first picked)
            "activate_layer_of_selection" => {
                if let Some(&id) = self.doc.selection.iter().next() {
                    let layer = self.doc.layer_of(id);
                    self.layer_action(ctx, layer, "set_current_layer");
                }
            }
            "lock_all_existing_guides" | "unlock_all_existing_guides" => {
                let locked = id.starts_with("lock");
                let guides = self
                    .doc
                    .guides
                    .iter()
                    .map(|g| cut_craft_core::document::Guide { locked, ..*g })
                    .collect();
                self.set_guides(guides);
            }
            "machine_configuration" => {
                self.machine_window = Some(cut_craft_machines::MachineManager::new(&self.machines));
            }
            "tool_changer" => self.tool_changer_window = true,
            "add_machine" => {
                self.machine_window =
                    Some(cut_craft_machines::MachineManager::adding(&self.machines));
            }
            "install_offline_package" => {
                let dialog = file_dialog().add_filter("Machine package", &["json"]);
                if let Some(path) = dialog.pick_file() {
                    self.install_package(&path);
                }
            }
            "search_for_machine_online" => {
                ctx.open_url(egui::OpenUrl::new_tab(super::MACHINES_ONLINE));
            }
            "post_processor_management" => {
                self.post_window = Some(cut_craft_machines::PostManager::default());
            }
            "tool_database" => {
                let mut manager = cut_craft_tool_database::Manager::new(&self.tools);
                // the active machine's tool changer, its tools loaded with the tools
                manager.changer = self
                    .machines
                    .active()
                    .filter(|m| !m.slots.is_empty())
                    .map(|m| cut_craft_tool_database::Changer {
                        machine: m.name.clone(),
                        slots: m.slots.clone(),
                    });
                // tools loaded before slots set their numbers show those numbers now
                if let Some(changer) = &manager.changer {
                    manager.draft.number_by_slots(&changer.slots);
                }
                self.tool_manager = Some(manager);
            }
            id if self.display.toggle(id) => ctx.request_repaint(),
            id if self.resolution_checked(id).is_some() => self.set_resolution(id),
            id if self.units_checked(id).is_some() => self.set_units(id),
            id if self.layout_checked(id).is_some() => self.set_layout(id),
            "find_tool" => self.search = Some(String::new()),
            "refresh_2d_view" => ctx.request_repaint(),
            "create_component_from_toolpath_preview" => self.component_from_preview(),
            "copy"
            | "cut"
            | "paste"
            | "paste_in_place"
            | "increment_and_save"
            | "open_application_data_folder"
            | "print"
            | "print_preview" => {
                self.clipboard_or_file_action(ctx, id);
            }
            "keyboard_shortcuts"
            | "third_party_licenses"
            | "toolpath_summary"
            | "about_cut_craft"
            | "help_contents"
            | "snap_options" => self.open_window(id),
            "options" => self.options_window = true,
            "addons" => self.addons.show(ctx),
            "check_for_updates" => {
                let skipped = self.options.updates.skipped_version.clone();
                self.updates.check(ctx, true, skipped);
            }
            // the Select tool: put away whatever tool is in hand
            "select" => {
                (self.form, self.view_tool) = (None, None);
                "Select".clone_into(&mut self.status.current_tool);
            }
            // menu items that are an existing tool reached another way
            "select_all_vectors_on_current_layer" => {
                let layer = serde_json::json!({ "layer": self.doc.current_layer });
                if let Err(e) = self.apply("select_layer_vectors", &layer) {
                    self.status.set_message(&e);
                }
            }
            "job_size_and_position" => self.start_command("job_setup"),
            "show_toolpaths_tab" => self.pages.open(Page::Toolpaths),
            "design_layout" => self.pages = Pages::design_layout(),
            "machining_layout" => self.pages = Pages::machining_layout(),
            "interface_layout" => self.pages.switch_layout(),
            "pan" | "zoom_box" | "zoom_to_job" | "zoom_to_drawing" | "zoom_selected"
            | "node_edit" | "measure_inspect" | "transform" => {
                self.view_action(ctx, id);
            }
            "recalculate_toolpath"
            | "recalculate_visible_toolpaths"
            | "recalculate_all_toolpaths"
            | "delete_toolpath"
            | "save_toolpaths" => {
                self.toolpath_action(id);
            }
            "preview_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    self.preview_toolpath(id);
                } else if let Some(group) = &self.selected_group {
                    // a picked group: its toolpaths in list order, as one preview
                    self.preview_ids(&self.doc.toolpaths_in_group(group));
                }
            }
            "job_sheet_editor" => self.job_sheet_window = true,
            "part_labels" => self.part_labels_window = true,
            "create_job_sheet" => self.create_job_sheet(),
            "print_job_sheet" => self.print_job_sheet(),
            "edit_object" => {
                if let Some((id, command, params)) = self.object_to_edit() {
                    self.edit_shape(id, command, params);
                } else if self.shape_to_edit() {
                    // the form opens on the selected shape's own settings
                    self.form = None;
                    self.start_command("create_shape");
                }
            }
            "object_properties" => {
                if let Some(id) = self.object_properties() {
                    self.invoke(ctx, id);
                }
            }
            // the 2D and 3D views tiled one above the other, side by side, or back to tabs
            "tile_windows_horizontal" => self.split = Some(super::Split::Stacked),
            "tile_windows_vertical" => self.split = Some(super::Split::SideBySide),
            "cascade_windows" => self.split = None,
            "update_post_processor_database" => {
                // custom posts are read from their folder each time they are listed, and
                // Cut Craft's own come with Cut Craft: an update check brings the newest
                self.post_window = Some(cut_craft_machines::PostManager::default());
                let skipped = self.options.updates.skipped_version.clone();
                self.updates.check(ctx, true, skipped);
                self.status
                    .set_message(&post_database_message(&self.posts.entries()));
            }
            "save_shaded_image" => {
                if let Some(path) = file_dialog()
                    .add_filter("PNG image", &["png"])
                    .set_file_name("shaded.png")
                    .save_file()
                {
                    self.save_shaded_image(path);
                }
            }
            "preview_all_toolpaths" => self.preview_all(false),
            "preview_visible_toolpaths" => self.preview_all(true),
            "undo_last_preview" => {
                if let Some(before) = self.before_preview.take() {
                    self.simulation = before.0;
                    (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
                    self.status.set_message("Undid the last preview");
                }
            }
            "reset_preview" => {
                self.before_preview = None;
                (
                    self.simulation,
                    self.simulation_coarse,
                    self.fine_3d,
                    self.surface_gpu,
                ) = (None, None, None, None);
                (self.playback, self.playing) = (None, false);
            }
            id if id.starts_with("open_recent:") => {
                if let Ok(index) = id["open_recent:".len()..].parse() {
                    self.open_recent(index);
                }
            }
            // the 3D view's menu: the component picked from those under the click
            id if id.starts_with(gui::SELECT_COMPONENT) => {
                if let Ok(id) = id[gui::SELECT_COMPONENT.len()..].parse() {
                    self.doc.selected_components = [EntityId(id)].into();
                }
            }
            id if id.starts_with(gui::EDIT_POLYLINE) => {
                let (vector, end) = id[gui::EDIT_POLYLINE.len()..]
                    .split_once(':')
                    .unwrap_or_default();
                if let Ok(vector) = vector.parse() {
                    self.edit_polyline(EntityId(vector), end == "start");
                }
            }
            id if id.starts_with(SELECT_LEVEL) || id.starts_with(EXPORT_LEVEL) => {
                self.level_action(id);
            }
            "export_as_stl" => (self.stl_window, self.stl_result) = (true, None),
            id if self.registry.exporter(id).is_some() => self.export(id),
            id if id.starts_with(gui::REIMPORT_MODEL) => self.reimport_model(id),
            id if id.starts_with(gui::GUIDE) => self.guide_action(id),
            id if id.starts_with(gui::SAVE_BRUSH) => self.save_brush(id),
            id if id.starts_with(gui::FAVOURITE) || id.starts_with(gui::FAVOURITES_ONLY) => {
                self.favourites_action(id);
            }
            id if !self.may_run_gadget(id) => {}
            _ => self.start_command(id),
        }
    }

    /// Whether gadget `id` may run: always for Cut Craft's own; for one from the My Gadgets
    /// folder, once the user has been warned and said yes (asked once a session).
    fn may_run_gadget(&mut self, id: &str) -> bool {
        let Some(label) = self.unsigned_gadgets.get(id) else {
            return true;
        };
        if self.unsigned_gadgets_allowed.contains(id) {
            return true;
        }
        if (self.unsigned_gadget_ok)(label) {
            self.unsigned_gadgets_allowed.insert(id.to_owned());
            true
        } else {
            false
        }
    }

    /// The folder the job is saved in, once it is.
    pub(super) fn job_folder(&self) -> Option<&std::path::Path> {
        self.session
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
    }

    /// Draw Text's dictionary: found once (Hunspell's, from the dictionaries folder or the
    /// system), with the user's own words.
    fn speller(&mut self) -> Option<&mut cut_craft_core::spell::Speller> {
        if !self.speller_looked {
            self.speller_looked = true;
            let words = self
                .words_path
                .as_ref()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            self.speller =
                cut_craft_core::spell::Speller::find().map(|s| s.with_user_words(words.lines()));
        }
        self.speller.as_mut()
    }

    /// Draw Text's spelling: the words of its text not in the dictionary, each with its
    /// suggestions to click and Add Word; Remove Word takes back a word added.
    fn spelling(&mut self, ui: &mut egui::Ui) {
        let text = self
            .form
            .as_ref()
            .and_then(|f| f.params["text"].as_str())
            .unwrap_or_default()
            .to_owned();
        let Some(speller) = self.speller() else {
            ui.weak("Spelling: no dictionary found (put a Hunspell .dic and .aff in Cut Craft's dictionaries folder)");
            return;
        };
        let wrong = speller.misspelled(&text);
        let (mut fix, mut add, mut remove) = (None, None, None);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Spelling");
            if wrong.is_empty() {
                ui.weak("No spelling mistakes");
            }
            // ponytail: listed under the text, not underlined in it (the form's text box is
            // the generic one)
            for word in wrong.iter().take(8) {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(ui.visuals().error_fg_color, word);
                    for s in speller.suggest(word) {
                        if ui.small_button(&s).clicked() {
                            fix = Some((word.clone(), s));
                        }
                    }
                    if ui.small_button("Add Word").clicked() {
                        add = Some(word.clone());
                    }
                });
            }
            if !speller.user_words().is_empty() {
                egui::ComboBox::from_label("Remove Word")
                    .selected_text("a word added…")
                    .show_ui(ui, |ui| {
                        for w in speller.user_words() {
                            if ui.selectable_label(false, w).clicked() {
                                remove = Some(w.clone());
                            }
                        }
                    });
            }
        });
        if let Some((from, to)) = fix {
            self.fix_spelling(&from, &to);
        }
        if let Some(word) = add {
            self.user_word(&word, true);
        }
        if let Some(word) = remove {
            self.user_word(&word, false);
        }
    }

    /// Every `from` in Draw Text's text made `to`; text already made follows.
    pub(super) fn fix_spelling(&mut self, from: &str, to: &str) {
        let Some(form) = self.form.as_mut().filter(|f| f.id == "draw_text") else {
            return;
        };
        let text = form.params["text"].as_str().unwrap_or_default();
        form.params["text"] = cut_craft_core::spell::replace_word(text, from, to).into();
        form.dirty = true;
        let params = form.params.clone();
        if params.get("replaces").is_some_and(|r| !r.is_null())
            && let Err(e) = self.apply_form("draw_text", &params)
        {
            self.status.set_message(&e);
        }
    }

    /// Adds `word` to the user's own words (or takes it off), kept in the words file.
    pub(super) fn user_word(&mut self, word: &str, add: bool) {
        let Some(speller) = self.speller() else {
            return;
        };
        if add {
            speller.add_word(word);
        } else {
            speller.remove_word(word);
        }
        let words: Vec<String> = speller.user_words().iter().cloned().collect();
        if let Some(path) = &self.words_path
            && let Err(e) = std::fs::write(path, words.join("\n"))
        {
            self.status
                .set_message(&format!("cannot keep the words in {}: {e}", path.display()));
        }
    }

    /// The component a Save as Sculpting Brush action names.
    fn brush_component(&self, action: &str) -> Option<EntityId> {
        let id = EntityId(action.strip_prefix(gui::SAVE_BRUSH)?.parse().ok()?);
        self.doc.components.contains_key(&id).then_some(id)
    }

    /// Save as Sculpting Brush: the component as a brush file, in the brushes folder unless
    /// saved elsewhere, for Sculpting's Load Brush….
    fn save_brush(&mut self, action: &str) {
        let Some(id) = self.brush_component(action) else {
            return;
        };
        let name = self.doc.components[&id].name.clone();
        if let Some(path) = brush_dialog()
            .set_file_name(format!("{name}.ccclip"))
            .save_file()
            && let Err(e) = self.save_brush_to(id, &path.with_extension("ccclip"))
        {
            self.status.set_message(&e);
        }
    }

    /// Writes component `id` as a sculpting brush to `path`.
    pub(super) fn save_brush_to(
        &mut self,
        id: EntityId,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let k = self.doc.components.get(&id).ok_or("no such component")?;
        let bytes = fmt_export_as_3d_clipart::clip_bytes(k).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.status
            .set_message(&format!("Saved sculpting brush {}", path.display()));
        Ok(())
    }

    /// The guide menus: Guide Properties, lock and delete on a guide, and guides inserted
    /// where the 2D view's menu opened (an angled one at 45°, its properties opened to set
    /// the angle).
    pub(super) fn guide_action(&mut self, id: &str) {
        use cut_craft_core::document::Guide;
        let rest = &id[gui::GUIDE.len()..];
        let (what, arg) = rest.split_once(':').unwrap_or((rest, ""));
        let index = arg
            .parse::<usize>()
            .ok()
            .filter(|i| *i < self.doc.guides.len());
        let mut guides = self.doc.guides.clone();
        let mut props_for = None;
        match (what, index) {
            ("props", Some(i)) => props_for = Some(i),
            ("lock", Some(i)) => guides[i].locked = !guides[i].locked,
            ("delete", Some(i)) => {
                guides.remove(i);
            }
            ("insert", _) => {
                let at = self.menu_at.unwrap_or_else(|| self.doc.job.area().center());
                guides.push(match arg {
                    "v" => Guide::vertical(at.x),
                    "h" => Guide::horizontal(at.y),
                    _ => Guide::horizontal(at.y).with_angle(45.0).through(at),
                });
                if arg == "a" {
                    props_for = Some(guides.len() - 1);
                }
            }
            _ => return,
        }
        if guides != self.doc.guides {
            self.set_guides(guides);
        }
        if let Some(i) = props_for
            && let Some(g) = self.doc.guides.get(i)
        {
            self.guide_props = Some(super::GuideProps {
                index: i,
                position: g.at,
                angle: g.angle(),
                locked: g.locked,
                new_at: if g.tilted() { 10.0 } else { g.at },
                relative: g.tilted(),
                count: 1,
            });
        }
    }

    /// The component a Re-import Model action names, and where it came from.
    fn model_source(&self, action: &str) -> Option<&cut_craft_core::component::Source> {
        let id = action.strip_prefix(gui::REIMPORT_MODEL)?.parse().ok()?;
        self.doc.components.get(&EntityId(id))?.source.as_ref()
    }

    /// Orientate 3D Model again on a model's own file, with the settings it was imported
    /// with, its Apply replacing the model: only while the file is where it was; a file
    /// that has changed since is taken as it is now, with a warning (Aspire re-imports a
    /// model "provided it has not been changed significantly").
    fn reimport_model(&mut self, action: &str) {
        let Some(source) = self.model_source(action).cloned() else {
            return;
        };
        let (path, changed) = match source.located(self.job_folder()) {
            Ok(found) => found,
            Err(e) => {
                self.status.set_message(&e);
                return;
            }
        };
        let id = action[gui::REIMPORT_MODEL.len()..]
            .parse()
            .map(EntityId)
            .ok();
        self.start_command("orientate_3d_model");
        self.view = View::ThreeD;
        if let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") {
            form.params = source.settings;
            form.params["file"] = Value::from(path.display().to_string());
            form.drawn = id;
            self.apply_live();
        }
        if changed {
            self.status.set_message(&format!(
                "Warning: {} has changed since it was imported; imported again as it is now",
                path.display()
            ));
        }
    }

    pub(super) fn file_action(&mut self, id: &str) {
        match id {
            // a new job starts in Job Setup, as Aspire's does
            "new" => {
                if self.confirm_discard() {
                    self.reset_job();
                    // the material being set up shows in the 2D view
                    self.view = View::TwoD;
                    self.start_command("job_setup");
                    if let Some(form) = &mut self.form {
                        form.new_job = true;
                    }
                    self.screen = Screen::NewJob;
                }
            }
            "open" => {
                if self.confirm_discard()
                    && let Some(path) = job_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            "close" => {
                if self.confirm_discard() {
                    self.reset_job();
                    self.screen = Screen::Start;
                }
            }
            "save" => match self.session.path.clone() {
                Some(path) => self.save_to(&path),
                None => self.file_action("save_as"),
            },
            "save_as" => {
                if let Some(path) = job_dialog()
                    .set_file_name(format!("untitled.{JOB_EXTENSION}"))
                    .save_file()
                {
                    self.save_to(&path.with_extension(JOB_EXTENSION));
                }
            }
            "import_vectors_bitmap" | "import_component_3d_model" => {
                let mut dialog = file_dialog();
                let models = id == "import_component_3d_model";
                for importer in self.registry.importers() {
                    if MODEL_IMPORTERS.contains(&importer.id) == models {
                        dialog =
                            dialog.add_filter(importer.format.label, importer.format.extensions);
                    }
                }
                if let Some(path) = dialog.pick_file() {
                    self.import_file(&path);
                }
            }
            "import_cut_list" => {
                if let Some(path) = file_dialog()
                    .add_filter("Cut list (CSV)", &["csv"])
                    .pick_file()
                {
                    self.import_cut_list(&path);
                }
            }
            "add_clipart_folder" => {
                if let Some(folder) = file_dialog().pick_folder()
                    && let Err(e) = self.clipart.add(folder)
                {
                    self.status.set_message(&e);
                }
            }
            "segment_3d_model_file" => {
                // the segmenting form, on an STL picked first
                if let Some(path) = file_dialog()
                    .add_filter("STL or OBJ", &["stl", "obj"])
                    .pick_file()
                {
                    self.start_command("segment_3d_model");
                    if let Some(form) = self.form.as_mut().filter(|f| f.id == "segment_3d_model") {
                        form.params["file"] = Value::from(path.display().to_string());
                    }
                }
            }
            "import_toolpaths_file" => {
                // G-code read back as toolpaths: the form opens on the file picked, to say
                // which tool cut it
                if let Some(path) = file_dialog()
                    .add_filter(
                        "G-code",
                        &["nc", "tap", "gcode", "ngc", "cnc", "txt", "gc", "nc1"],
                    )
                    .pick_file()
                {
                    self.import_toolpaths_from(&path);
                }
            }
            "install_post_processor" => {
                if let Some(path) = file_dialog()
                    .add_filter("Post-processor", &["pp"])
                    .pick_file()
                {
                    self.install_post(&path);
                }
            }
            _ => unreachable!("not a file action: {id}"),
        }
    }

    /// Job and toolpath templates, saved and loaded through the templates folder.
    pub(super) fn template_action(&mut self, id: &str) {
        match id {
            "save_as_template" => {
                if let Some(path) = template_dialog()
                    .set_file_name(format!("template.{TEMPLATE}"))
                    .save_file()
                {
                    let path = path.with_extension(TEMPLATE);
                    match cut_craft_core::file::write_atomic(
                        &path,
                        &cut_craft_core::file::save_job(&self.doc),
                    ) {
                        Ok(()) => self
                            .status
                            .set_message(&format!("Saved template {}", path.display())),
                        Err(e) => self
                            .status
                            .set_message(&format!("could not save {}: {e}", path.display())),
                    }
                }
            }
            "save_selected_toolpath_as_template" | "save_all_visible_toolpaths_as_template" => {
                let ids: Vec<_> = if id == "save_selected_toolpath_as_template" {
                    self.selected_toolpath.into_iter().collect()
                } else {
                    self.doc
                        .toolpaths
                        .iter()
                        .filter(|(_, t)| t.visible)
                        .map(|(&i, _)| i)
                        .collect()
                };
                if let Some(path) = toolpath_template_dialog()
                    .set_file_name(format!("toolpaths.{}", files::TOOLPATH_TEMPLATE_EXTENSION))
                    .save_file()
                {
                    let path = path.with_extension(files::TOOLPATH_TEMPLATE_EXTENSION);
                    let template = files::toolpath_template(&self.doc, &ids);
                    let message = match cut_craft_core::file::write_atomic(&path, &template) {
                        Ok(()) => format!("Saved toolpath template {}", path.display()),
                        Err(e) => format!("could not save {}: {e}", path.display()),
                    };
                    self.status.set_message(&message);
                }
            }
            "load_template" => {
                if let Some(path) = toolpath_template_dialog().pick_file() {
                    let change = std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| files::load_toolpath_template(&self.doc, &bytes))
                        .map_err(cut_craft_core::CommandError::Failed);
                    self.record(change, &format!("Loaded {}", path.display()));
                }
            }
            "new_from_template" => {
                // like New and Open: unsaved work is asked about first
                if self.confirm_discard()
                    && let Some(path) = template_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            _ => unreachable!("not a template action: {id}"),
        }
    }

    /// The active machine's keep-out zones put on the job, as job millimetres (the G-code's,
    /// which they are given in); true when they changed.
    pub(super) fn sync_machine_keep_out(&mut self) -> bool {
        let zones: Vec<_> = self
            .machines
            .active()
            .map(|m| m.keep_out.as_slice())
            .unwrap_or_default()
            .iter()
            .filter(|z| z.width > 0.0 && z.height > 0.0)
            .map(|z| z.path())
            .collect();
        let changed = zones != self.doc.machine_keep_out;
        self.doc.machine_keep_out = zones;
        changed
    }

    /// The keep-out zones changed: every toolpath is made again round them, as Aspire's
    /// are, and any that still cannot keep out is warned about.
    pub(super) fn keep_out_changed(&mut self) {
        if !self.doc.toolpaths.is_empty() {
            let ids: Vec<EntityId> = self.doc.toolpaths.keys().copied().collect();
            let warned = ids.clone();
            self.recalculate_in_background(ids, "All toolpaths recalculated", move |app| {
                app.warn(&warned, &[]);
            });
        }
    }

    /// The toolpaths of `plan` that pass over a keep-out zone, by name.
    pub(super) fn not_kept_out(&self, plan: &[Vec<EntityId>]) -> Vec<String> {
        let zones = self.doc.keep_out_zones();
        plan.iter()
            .flatten()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .filter(|t| cut_craft_core::keep_out::crossed(&zones, &t.moves))
            .map(|t| t.name.clone())
            .collect()
    }

    pub(super) fn toolpath_action(&mut self, id: &str) {
        match id {
            "recalculate_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    self.recalculate_in_background(vec![id], "Toolpath recalculated", |_| {});
                }
            }
            // the cascade: Recalculate on each shown toolpath
            "recalculate_visible_toolpaths" => {
                let shown = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
                let ids: Vec<EntityId> = shown.map(|(&id, _)| id).collect();
                self.recalculate_in_background(ids, "Visible toolpaths recalculated", |_| {});
            }
            "recalculate_all_toolpaths" => {
                let ids: Vec<EntityId> = self.doc.toolpaths.keys().copied().collect();
                self.recalculate_in_background(ids, "All toolpaths recalculated", |_| {});
            }
            // the toolpath list's button: the Delete Toolpath command on the selected one
            "delete_toolpath" => {
                if let Some(id) = self.selected_toolpath.take()
                    && let Err(e) =
                        self.apply("delete_toolpath", &serde_json::json!({ "toolpath": id }))
                {
                    self.status.set_message(&e);
                }
            }
            "save_toolpaths" => {
                self.save_form = Some(SaveForm {
                    mode: files::SaveMode::VisibleToOneFile,
                    group: false,
                    post: None,
                    add_side: self.doc.job.double_sided.is_some(),
                    changes: self.default_changes(),
                });
            }
            _ => unreachable!("not a toolpath action: {id}"),
        }
    }

    /// The posts of the active machine that suit this job (flat, or wrapping its side), for
    /// Save Toolpaths to choose from; none without a machine (Cut Craft's own is used).
    pub(super) fn post_choices(&self) -> Vec<String> {
        let wraps = self.doc.job.rotary.map(|r| match r.along {
            cut_craft_core::document::Along::X => "Y",
            cut_craft_core::document::Along::Y => "X",
        });
        self.machines.active().map_or_else(Vec::new, |machine| {
            machine
                .posts
                .iter()
                .filter(|name| {
                    self.posts
                        .load(name)
                        .is_ok_and(|p| p.wraps() == wraps && (wraps.is_none() || machine.rotary))
                })
                .cloned()
                .collect()
        })
    }

    /// Save Toolpaths' Save: the checks before cutting, a file name, then each file of the
    /// form's plan through its post (and each tile, when the job is tiled).
    pub(super) fn save_toolpaths(&mut self, form: &SaveForm) {
        let post = match self.form_post(form) {
            Ok(post) => post,
            Err(e) => return self.status.set_message(&e),
        };
        let plan = match self.plan(form, &post) {
            Ok(plan) => plan,
            Err(e) => return self.status.set_message(&e),
        };
        let found = self.preflight(&plan, form.changes);
        if let Some(refused) = Self::refused(&found) {
            return self.status.set_message(&refused);
        }
        let problems: Vec<String> = found.into_iter().map(|f| f.message).collect();
        let ask = self.options.confirmations.cutting_checks == CuttingChecks::Ask;
        if !problems.is_empty() && ask && !Self::save_anyway(&problems) {
            return self.status.set_message("Toolpaths not saved");
        }
        let extension = post.extension.clone();
        let mut dialog = file_dialog().add_filter(&post.name, &[extension.as_str()]);
        // a group saves under its own name, as Aspire's does
        if form.mode == files::SaveMode::Selected
            && let Some(group) = &self.selected_group
        {
            dialog = dialog.set_file_name(format!("{group}.{extension}"));
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let told = if problems.is_empty() || ask {
            String::new()
        } else {
            format!(" (check before cutting: {})", problems.join("; "))
        };
        let result = self.write_toolpaths(form, &post, &path.with_extension(&extension));
        self.status
            .set_message(&result.map_or_else(|e| e, |done| format!("{done}{told}")));
    }

    /// Saves the form's plan through `post` at `path`, unless the checks before cutting
    /// find an error, and says what was saved: Save Toolpaths' Save short of its dialogs.
    #[cfg(test)]
    pub(super) fn save_toolpaths_to(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let plan = self.plan(form, post)?;
        match Self::refused(&self.preflight(&plan, form.changes)) {
            Some(refused) => Err(refused),
            None => self.write_toolpaths(form, post, path),
        }
    }

    /// How tools are changed unless the form says otherwise: by the tool changer when the
    /// active machine has one, else a file for each tool.
    pub(super) fn default_changes(&self) -> files::ToolChanges {
        if self.machines.active().is_some_and(|m| !m.slots.is_empty()) {
            files::ToolChanges::ToolChanger
        } else {
            files::ToolChanges::SeparateFiles
        }
    }

    /// The post the form saves through: the one picked, else the machine's default, with
    /// its tool changes made pauses when the form says to pause, or tools are changed by
    /// hand between files on a machine without a tool changer.
    pub(super) fn form_post(&self, form: &SaveForm) -> Result<cut_craft_post::Post, String> {
        let post = match form.post.as_deref() {
            Some(name) => self.posts.load(name),
            None => self.output_post(),
        }?;
        let changer = self.machines.active().is_some_and(|m| !m.slots.is_empty());
        Ok(match form.changes {
            files::ToolChanges::Pause => post.with_pauses(),
            // changed by hand between files: no tool changer call (M6) for the first tool
            files::ToolChanges::SeparateFiles if !changer => post.with_pauses(),
            _ => post,
        })
    }

    /// The form's toolpaths, file by file: a file for each tool in turn unless the tool
    /// changer or pauses change them (and the post can).
    pub(super) fn plan(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
    ) -> Result<Vec<Vec<EntityId>>, String> {
        let plan = files::save_plan(
            &self.doc,
            form.mode,
            self.selected_toolpath,
            self.selected_group.as_deref(),
            form.group,
        )?;
        Ok(if Self::by_tool(form, post) {
            files::by_tool(&self.doc, plan)
        } else {
            plan
        })
    }

    /// Whether each tool gets files of its own: chosen, or the post cannot change tools.
    fn by_tool(form: &SaveForm, post: &cut_craft_post::Post) -> bool {
        form.changes == files::ToolChanges::SeparateFiles || !post.changes_tools()
    }

    /// Whether files are split per tool (not by the form's Multiple Files), and so named
    /// after their tool.
    pub(super) fn split_for(form: &SaveForm, post: &cut_craft_post::Post) -> bool {
        Self::by_tool(form, post) && form.mode != files::SaveMode::VisibleToMultipleFiles
    }

    /// The checks before cutting for Save Toolpaths' `plan`, worked out again only when the
    /// plan, the job or the machine changed.
    pub(super) fn save_checks(
        &mut self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> &[cut_craft_core::preflight::Finding] {
        let (revision, machine) = (self.history.revision(), self.machines.active);
        let fresh = self.save_checks.as_ref().is_some_and(|c| {
            c.plan == plan && c.changes == changes && c.revision == revision && c.machine == machine
        });
        if !fresh {
            self.save_checks = Some(SaveChecks {
                plan: plan.to_vec(),
                changes,
                revision,
                machine,
                found: self.preflight(plan, changes),
            });
        }
        &self.save_checks.as_ref().expect("just filled").found
    }

    /// What the checks before cutting find in `plan`'s toolpaths, cut in its order on the
    /// active machine; its tool changer's slots are checked only when it changes the tools.
    pub(super) fn preflight(
        &self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Vec<cut_craft_core::preflight::Finding> {
        let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> = plan
            .iter()
            .flatten()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        let mut limits = self.limits();
        if changes != files::ToolChanges::ToolChanger {
            limits.slots.clear();
        }
        cut_craft_core::preflight::check(&self.doc, &toolpaths, &limits)
    }

    /// Why toolpaths with `found` are not saved: the errors, each with its fix; `None` when
    /// there are none.
    pub(super) fn refused(found: &[cut_craft_core::preflight::Finding]) -> Option<String> {
        let errors: Vec<&str> = found
            .iter()
            .filter(|f| f.severity == cut_craft_core::preflight::Severity::Error)
            .map(|f| f.message.as_str())
            .collect();
        (!errors.is_empty()).then(|| format!("Not saved: {}", errors.join("; ")))
    }

    /// Writes the form's plan through `post` at `path`: each file, or each tile of each file
    /// when the job is tiled, and says what was saved.
    fn write_toolpaths(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let plan = self.plan(form, post)?;
        // as Aspire's: a toolpath that cannot abide by the keep-out zones is not saved
        let crossing = self.not_kept_out(&plan);
        if !crossing.is_empty() {
            return Err(format!(
                "Not saved: {} could not abide by the keep-out zones. Check the keep-out zones \
                 and their clearance are right",
                crossing.join(", ")
            ));
        }
        let thinned = self.options.toolpaths.thinned(&self.doc);
        let side = (form.add_side && self.doc.job.double_sided.is_some()).then_some(
            if self.doc.bottom_side {
                "Bottom"
            } else {
                "Top"
            },
        );
        let job = self.session.path.as_deref();
        let slots = self.slots();
        let per_tool = Self::split_for(form, post);
        let parts = files::save_files(post, &thinned, &plan, path, side, per_tool)?;
        for (path, part) in &parts {
            match self.doc.tiling {
                // one file per tile, each tile moved to the job's corner
                Some(tiling) => {
                    let tiles = tiling.tiles(self.doc.job.area());
                    for (n, &tile) in tiles.iter().enumerate() {
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        let extension = path.extension().unwrap_or_default().to_string_lossy();
                        let named =
                            path.with_file_name(format!("{stem}_tile{}.{extension}", n + 1));
                        files::save_through(post, &slots, &tiling.tiled(part, tile), &named, job)?;
                    }
                }
                None => files::save_through(post, &slots, part, path, job)?,
            }
        }
        Ok(match parts.len() {
            1 => format!("Saved toolpaths to {}", parts[0].0.display()),
            n => format!("Saved {n} toolpath files beside {}", path.display()),
        })
    }

    /// The post toolpaths are saved through: the active machine's default when it suits the
    /// job, else the machine's first post that does: a flat one for a flat job, and for a
    /// rotary job (on a machine with a 4th axis) one wrapping the side that goes round, Y
    /// for a cylinder along X and X for one along Y. Without a machine, Cut Craft's
    /// `FluidNC` post for this kind of job.
    pub(super) fn output_post(&self) -> Result<cut_craft_post::Post, String> {
        use cut_craft_core::document::Along;
        let Some(machine) = self.machines.active().filter(|m| !m.posts.is_empty()) else {
            return Ok(cut_craft_post::Post::default_for(&self.doc.job));
        };
        let wraps = self.doc.job.rotary.map(|r| match r.along {
            Along::X => "Y",
            Along::Y => "X",
        });
        if wraps.is_some() && !machine.rotary {
            return Err(format!(
                "{} has no 4th axis for this rotary job: tick its 4th axis (A) in Machine \
                 Configuration",
                machine.name
            ));
        }
        // the default first: one that will not load says so rather than being passed over
        let default = machine
            .post()
            .map(|name| self.posts.load(name))
            .transpose()?;
        if let Some(post) = default.filter(|p| p.wraps() == wraps) {
            return Ok(post);
        }
        machine
            .posts
            .iter()
            .filter_map(|name| self.posts.load(name).ok())
            .find(|p| p.wraps() == wraps)
            .ok_or_else(|| match wraps {
                Some(axis) => format!(
                    "{} has no post wrapping {axis} onto A for this rotary job: add a Wrap \
                     {axis} to A post in Machine Configuration",
                    machine.name
                ),
                None => format!(
                    "{} has only rotary posts: add a flat one in Machine Configuration",
                    machine.name
                ),
            })
    }

    /// What the active machine can do, for the checks before cutting; nothing is checked
    /// against a machine without one.
    pub(super) fn limits(&self) -> cut_craft_core::preflight::Limits {
        let mut limits = self
            .machines
            .active()
            .map(|m| cut_craft_core::preflight::Limits {
                name: m.name.clone(),
                bed: [m.width, m.height],
                z_travel: m.depth,
                max_rate: m.max_rate,
                spindle: m.spindle,
                slots: m.slots.clone(),
                left: None,
            })
            .unwrap_or_default();
        limits.left = Some(self.options.confirmations.left());
        limits
    }

    /// The active machine's tool changer slots (empty without one), for `[T]`.
    pub(super) fn slots(&self) -> Vec<Option<String>> {
        self.machines
            .active()
            .map(|m| m.slots.clone())
            .unwrap_or_default()
    }

    /// Reads the machines and finds the custom posts, kept in the config folder.
    pub(super) fn load_machines(&mut self) {
        let dir = files::config_dir();
        self.machines_path = dir.as_ref().map(|d| d.join("machines.json"));
        if let Some(path) = &self.machines_path {
            self.machines = cut_craft_machines::Machines::load(path);
        }
        self.posts = cut_craft_machines::Library::new(dir.map(|d| d.join("posts")));
    }

    /// The active machine's tool changer was loaded differently: the machines are saved,
    /// and each tool now in a slot takes the slot as its tool number.
    pub(super) fn tool_changer_changed(&mut self) {
        self.machines_changed("Tool changer changed");
        let slots = self
            .machines
            .active()
            .map(|m| m.slots.clone())
            .unwrap_or_default();
        if self.tools.number_by_slots(&slots) {
            self.tools_changed("Tool changer changed; tools numbered by their slots");
            self.refresh_form_tool();
        }
    }

    /// Writes the machines, if they have a file, and reports `done` or the failure.
    pub(super) fn machines_changed(&mut self, done: &str) {
        let result = self
            .machines_path
            .as_deref()
            .map_or(Ok(()), |path| self.machines.save(path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Install Offline Package: the machines in a package file (`machines/` in the repository)
    /// that are not here already by name.
    pub(super) fn install_package(&mut self, path: &std::path::Path) {
        let package = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()));
        let package: cut_craft_machines::Machines = match package {
            Ok(package) => package,
            Err(e) => {
                return self
                    .status
                    .set_message(&format!("{}: not a machine package ({e})", path.display()));
            }
        };
        let (added, here) = self.machines.install(package);
        let here = if here > 0 {
            format!(", {here} already here")
        } else {
            String::new()
        };
        self.machines_changed(&format!("Installed {added} machines{here}"));
    }

    /// Install Post-Processor: copied into the custom posts. A machine with no posts yet (or
    /// none at all) takes it, so toolpaths save through it as soon as it is installed.
    pub(super) fn install_post(&mut self, path: &std::path::Path) {
        let entry = match self.posts.install(path) {
            Ok(entry) => entry,
            Err(e) => return self.status.set_message(&e),
        };
        if self.machines.active().is_none() {
            self.machines.add();
        }
        let machine = self.machines.active_mut().expect("a machine was just made");
        if machine.posts.is_empty() {
            machine.posts.push(entry.name.clone());
            let done = format!(
                "Installed {}; {} saves toolpaths through it",
                entry.name, machine.name
            );
            self.machines_changed(&done);
        } else {
            self.status
                .set_message(&format!("Installed {}", entry.name));
        }
    }

    /// The Machine menu's windows, and what they ask for.
    pub(super) fn machine_windows(&mut self, ctx: &egui::Context) {
        let icons = &self.icons;
        let picture = |path: &str| icons.find(path, "").map(|p| icons.image(p));
        let look = cut_craft_machines::Look { picture: &picture };
        let machine = self.machine_window.as_mut().and_then(|w| {
            w.show(ctx, &self.posts, &look)
                .map(|o| (o, w.draft.clone()))
        });
        let post = self
            .post_window
            .as_mut()
            .and_then(|w| w.show(ctx, &self.posts, &look));
        if let Some((outcome, draft)) = machine {
            use cut_craft_machines::MachineOutcome;
            match outcome {
                MachineOutcome::Apply | MachineOutcome::Ok => {
                    self.machines = draft;
                    if outcome == MachineOutcome::Ok {
                        self.machine_window = None;
                    }
                    self.machines_changed("Machines saved");
                    if self.sync_machine_keep_out() {
                        self.keep_out_changed();
                    }
                }
                MachineOutcome::Cancel => self.machine_window = None,
                MachineOutcome::ManagePosts => {
                    self.post_window = Some(cut_craft_machines::PostManager::default());
                }
            }
        }
        if let Some(outcome) = post {
            self.post_outcome(outcome);
        }
    }

    pub(super) fn post_outcome(&mut self, outcome: cut_craft_machines::PostOutcome) {
        use cut_craft_machines::PostOutcome;
        match outcome {
            PostOutcome::Install => {
                if let Some(path) = file_dialog()
                    .add_filter("Post-processor", &["pp"])
                    .pick_file()
                {
                    self.install_post(&path);
                }
            }
            PostOutcome::Select(name) => {
                // into Machine Configuration's edits when it is open, else the machines
                let machines = match &mut self.machine_window {
                    Some(window) => &mut window.draft,
                    None => &mut self.machines,
                };
                if machines.active().is_none() {
                    machines.add();
                }
                let machine = machines.active_mut().expect("a machine was just made");
                if !machine.posts.contains(&name) {
                    machine.posts.push(name.clone());
                }
                let done = format!("{} can save through {name}", machine.name);
                if self.machine_window.is_some() {
                    self.status
                        .set_message(&format!("{done} once Machine Configuration is applied"));
                } else {
                    self.machines_changed(&done);
                }
            }
            PostOutcome::OpenFolder(dir) => {
                on_the_desktop();
                if let Err(e) = files::show_folder(&dir) {
                    self.status.set_message(&format!("{}: {e}", dir.display()));
                }
            }
            PostOutcome::Message(m) => self.status.set_message(&m),
            PostOutcome::Close => self.post_window = None,
        }
    }

    /// Runs a registered command: straight away if it has no settings, else opens its form.
    pub(super) fn start_command(&mut self, id: &str) {
        self.import_level = None;
        // a new job's setup is accepted, as OK would, before another form takes its place:
        // the new-job screen shows only that form, so it would be left bare (issue #17)
        if id != "job_setup" && self.form.as_ref().is_some_and(|f| f.new_job) {
            self.job_setup_ok();
            // settings it refuses keep it open, saying why
            if self.screen == Screen::NewJob {
                return;
            }
        }
        let Some(cmd) = self.registry.get(id) else {
            self.status
                .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
            return;
        };
        let page = self.page_of(cmd.id());
        let (meta, fields, mut params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        // Orientate 3D Model offers what Options ▸ Importing Models says; one kind alone
        // is the kind, with no choice shown
        if cmd.id() == "orientate_3d_model" {
            let offered = match self.options.import.model_types {
                crate::options::ModelTypes::Full => "full",
                crate::options::ModelTypes::Flat => "flat",
                crate::options::ModelTypes::Both => "both",
            };
            params["offered"] = Value::from(offered);
            if offered != "both" {
                params["model_type"] = Value::from(offered);
            }
            params["rotary"] = Value::from(self.doc.job.rotary.is_some());
        }
        self.fill_drill_bank(cmd.id(), &mut params);
        // a toolpath's form in Aspire's order
        let fields = if fields.iter().any(|f| f.key == "tool") {
            form::arrange(fields)
        } else {
            fields
        };
        meta.label.clone_into(&mut self.status.current_tool);
        self.view_tool = None;
        if fields.is_empty() {
            if let Err(e) = self.apply(cmd.id(), &params) {
                self.status.set_message(&e);
            }
        } else {
            self.form = Some(OpenForm {
                id: cmd.id(),
                label: meta.label,
                icon: meta.icon,
                fields,
                params,
                error: None,
                new_job: false,
                page,
                editing: None,
                drawn: None,
                dirty: false,
                shown_size: None,
                typed_size: [None; 3],
                dragged: None,
                model: None,
                model_whole: true,
                applied: None,
                undo_from: self.history.undo_depth(),
                way: crate::flyout::DragWay::AsIs,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
        }
    }

    /// A toolbox variant of a drawing tool (Draw Circle from Edge): its command's form, named
    /// for the variant, reading the drag the variant's way.
    fn start_variant(&mut self, id: &str) {
        let Some(variant) = crate::flyout::variant(id) else {
            return;
        };
        self.start_command(variant.command);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == variant.command) {
            form.way = variant.way;
            form.label = variant.label;
            variant.label.clone_into(&mut self.status.current_tool);
        }
    }

    pub(super) fn save_to(&mut self, path: &std::path::Path) {
        // imported models' files found again from the job's folder, should the two move
        if let Some(folder) = path.parent() {
            for source in self
                .doc
                .components
                .values_mut()
                .filter_map(|k| k.source.as_mut())
            {
                source.relate_to(folder);
            }
        }
        match self.session.save(&self.doc, &self.history, path) {
            Ok(()) => {
                self.status
                    .set_message(&format!("Saved {}", path.display()));
                self.remember(path);
                self.clear_recovery();
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Opens a job file in place of whatever is open (already confirmed); whether it opened
    /// (or, a big one, is still opening). A big job is read off the window's thread, its
    /// progress shown; one that will not open leaves the recent files.
    pub(super) fn open_path(&mut self, path: &std::path::Path) -> bool {
        let outcome = std::rc::Rc::new(std::cell::Cell::new(None));
        let told = std::rc::Rc::clone(&outcome);
        let (reading, path) = (path.to_owned(), path.to_owned());
        let what = format!("Opening {}", super::file_name(&path));
        self.in_background(
            &what,
            move || {
                let mut session = Session::default();
                session
                    .open(&reading)
                    .map(|(doc, history)| (session, doc, history))
            },
            move |app, opened| told.set(Some(app.opened(&path, opened))),
        );
        outcome.get().unwrap_or(true)
    }

    /// A job file read (or why it could not be) put in place of whatever is open; whether
    /// it opened.
    fn opened(
        &mut self,
        path: &std::path::Path,
        opened: Result<(Session, Document, History), String>,
    ) -> bool {
        match opened {
            Ok((mut session, doc, history)) => {
                self.reset_job();
                let template = path
                    .extension()
                    .is_some_and(|e| e == cut_craft_core::file::TEMPLATE_EXTENSION);
                if template {
                    // a template opens as a new file: saving asks where, the template stays
                    session.path = None;
                }
                (self.doc, self.history, self.session) = (doc, history, session);
                self.screen = Screen::Job;
                if !template {
                    self.remember(path);
                }
                if self.doc.notes.starts_with('.') {
                    self.start_command("notes");
                }
                true
            }
            Err(e) => {
                self.status.set_message(&e);
                if self.recent.iter().any(|p| p == path) {
                    self.recent.retain(|p| p != path);
                    self.save_recent();
                }
                false
            }
        }
    }

    /// Opens one of the recent files; one that no longer opens leaves the list.
    pub(super) fn open_recent(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard() {
            self.open_path(&path);
        }
    }

    pub(super) fn remember(&mut self, path: &std::path::Path) {
        files::remember(&mut self.recent, path, self.options.recent_count());
        self.save_recent();
    }

    pub(super) fn save_recent(&mut self) {
        if let Some(path) = &self.recent_path
            && let Err(e) = files::save_recent(&self.recent, path)
        {
            self.status.set_message(&e);
        }
    }

    pub(super) fn load_recent(&mut self) {
        self.recent_path = files::recent_files_path();
        if let Some(path) = &self.recent_path {
            self.recent = files::load_recent(path);
        }
    }

    /// Once a frame: unsaved work copied to the recovery file as often as Options says,
    /// on a thread of its own so a big job does not stall the window.
    pub(super) fn autosave(&mut self, ctx: &egui::Context) {
        if let Some(writing) = self.autosave.take() {
            if writing.is_finished() {
                let outcome = writing
                    .join()
                    .unwrap_or_else(|_| Err("the recovery copy could not be written".into()));
                if let Err(e) = outcome {
                    self.status.set_message(&e);
                }
            } else {
                self.autosave = Some(writing);
            }
        }
        let now = ctx.input(|i| i.time);
        let unsaved = self.screen == Screen::Job
            && self.session.is_dirty(&self.history)
            && self.history.revision() != self.autosaved.0;
        if unsaved
            && now - self.autosaved.1 >= self.options.general.autosave_interval()
            && self.autosave.is_none()
            && let Some(path) = self.recovery_path.clone()
        {
            self.autosaved = (self.history.revision(), now);
            let doc = self.doc.clone();
            self.autosave = Some(std::thread::spawn(move || {
                cut_craft_core::file::write_atomic(&path, &cut_craft_core::file::save_job(&doc))
                    .map_err(|e| format!("could not keep a recovery copy: {e}"))
            }));
        }
    }

    /// The work is saved or let go on purpose: nothing is left to recover.
    pub(super) fn clear_recovery(&mut self) {
        if let Some(writing) = self.autosave.take() {
            let _ = writing.join();
        }
        if let Some(path) = &self.recovery_path {
            let _ = std::fs::remove_file(path);
        }
        self.autosaved.0 = self.history.revision();
    }

    /// The recovery copy a run that ended badly left, opened as an unsaved job; whether
    /// there was one. It stays until the job is saved or closed.
    pub(super) fn open_recovery(&mut self) -> bool {
        let Some(doc) = self
            .recovery_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| cut_craft_core::file::load_job(&bytes).ok())
        else {
            return false;
        };
        self.reset_job();
        self.doc = doc;
        self.session = Session::unsaved();
        self.screen = Screen::Job;
        self.status.set_message(
            "Recovered unsaved work from the last run: save it (File ▸ Save As) or close it",
        );
        true
    }

    /// Back to nothing: no job, no form, no preview.
    pub(super) fn reset_job(&mut self) {
        (self.doc, self.history, self.session, self.form) = Default::default();
        if let Some(job) = self.options.default_job {
            self.doc.job = job;
        }
        (self.camera, self.view_tool) = (None, None);
        (
            self.selected_toolpath,
            self.simulation,
            self.simulation_coarse,
            self.fine_3d,
            self.surface_gpu,
        ) = (None, None, None, None, None);
        self.clear_recovery();
    }

    /// Set as Default in Job Setup: new jobs start with the job its settings make.
    pub(super) fn job_setup_default(&mut self) {
        let Some(form) = &mut self.form else { return };
        // made on an empty document, so only the job's own settings count
        let made = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms")
            .run_json(&Document::default(), &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| {
                change
                    .job()
                    .copied()
                    .ok_or_else(|| "no job made".to_owned())
            });
        match made {
            Ok(job) => {
                form.error = None;
                self.options.default_job = Some(job);
                self.save_options();
                self.status
                    .set_message("New jobs will start with these settings");
            }
            Err(e) => form.error = Some(e),
        }
    }

    /// OK in Job Setup for a new job: the job is made, with nothing to undo yet.
    pub(super) fn job_setup_ok(&mut self) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let cmd = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms");
        let made = cmd
            .run_json(&self.doc, &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| self.doc.apply(&change).map_err(|e| e.to_string()));
        match made {
            Ok(()) => {
                (self.history, self.session) = Default::default();
                self.screen = Screen::Job;
                self.status.set_message("New job created");
                "Select".clone_into(&mut self.status.current_tool);
            }
            Err(e) => {
                form.error = Some(e);
                self.form = Some(form);
            }
        }
    }

    pub(super) fn job_setup_cancel(&mut self) {
        self.reset_job();
        "Select".clone_into(&mut self.status.current_tool);
        self.screen = Screen::Start;
    }

    pub(super) fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = file_dialog().add_filter(format.label, format.extensions);
        if let Some(path) = dialog.save_file() {
            let path = path.with_extension(format.extensions[0]);
            let result = files::export(&self.registry, id, &self.doc, &path);
            self.status.set_message(
                &result.map_or_else(|e| e, |()| format!("Exported {}", path.display())),
            );
        }
    }

    /// True when it is fine to throw the current job away: no unsaved work, or the user chose
    /// to save it or not to. False if they cancelled.
    pub(super) fn confirm_discard(&mut self) -> bool {
        if !self.session.is_dirty(&self.history) {
            return true;
        }
        let answer = message_dialog()
            .set_title(APP_NAME)
            .set_description(format!(
                "Save changes to {}?",
                self.session.title(APP_NAME, &self.history)
            ))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => {
                match self
                    .session
                    .path
                    .clone()
                    .or_else(|| job_dialog().save_file())
                {
                    Some(path) => self.save_to(&path.with_extension(JOB_EXTENSION)),
                    None => return false,
                }
                !self.session.is_dirty(&self.history)
            }
            rfd::MessageDialogResult::No => {
                // asked once: a repeated close request must not ask again
                self.session.discard_changes(&self.history);
                true
            }
            _ => false,
        }
    }

    /// Applies a computed change for undo, reporting how it went in the status bar.
    pub(super) fn record(
        &mut self,
        change: Result<Change, cut_craft_core::CommandError>,
        done: &str,
    ) {
        let (mut made, mut notes) = (vec![], vec![]);
        let result = change.map_err(|e| e.to_string()).and_then(|c| {
            made = c.toolpaths_made();
            notes.clone_from(&c.notes);
            self.history
                .apply(&mut self.doc, c)
                .map_err(|e| e.to_string())
        });
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
        self.warn(&made, &notes);
    }

    /// A warning in the status bar about what was just made: the command's `notes`, and for
    /// each toolpath in `made`, crossing a keep-out zone and the problems the checks before
    /// cutting find.
    pub(super) fn warn(&mut self, made: &[EntityId], notes: &[String]) {
        let mut warnings = notes.to_vec();
        for t in made.iter().filter_map(|id| self.doc.toolpaths.get(id)) {
            if cut_craft_core::keep_out::crossed(&self.doc.keep_out_zones(), &t.moves) {
                warnings.push(format!(
                    "{} could not be made without passing over a keep-out zone",
                    t.name
                ));
            }
            warnings.extend(cut_craft_core::checks::problems_over(
                &self.doc,
                t,
                self.options.confirmations.left(),
            ));
        }
        if !warnings.is_empty() {
            self.status
                .set_message(&format!("Warning: {}", warnings.join("; ")));
        }
    }

    /// Asks whether to save toolpaths the checks before cutting found `problems` in.
    /// Crossing vectors chosen for a V-carve or pocket toolpath, or for an offset: what the
    /// Options' confirmation choices say to do. An error stops the command.
    pub(super) fn crossings_allowed(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let c = &self.options.confirmations;
        let check = match id {
            "pocket_toolpath" | "vcarve_engrave_toolpath" | "vcarve_inlay_toolpath" => {
                c.toolpath_intersections
            }
            "offset_vectors" if params["corners"] == "sharp" => c.sharp_offset_intersections,
            "offset_vectors" => c.radiused_offset_intersections,
            _ => return Ok(()),
        };
        if check == IntersectionCheck::DontCheck || self.doc.selection.is_empty() {
            return Ok(());
        }
        let only_crossings = cmd_vector_validator::Params {
            overlapping: false,
            intersections: true,
            zero_length_spans: false,
            open: false,
            ..cmd_vector_validator::Params::default()
        };
        // the validator fails when it finds nothing
        let Ok(crossing) = <cmd_vector_validator::VectorValidator as cut_craft_core::Command>::run(
            &self.doc,
            &only_crossings,
        ) else {
            return Ok(());
        };
        let validator = match check {
            IntersectionCheck::Ask => match Self::crossing_choice() {
                Some(validator) => validator,
                None => return Ok(()),
            },
            IntersectionCheck::OpenVectorValidator => true,
            IntersectionCheck::DontCheck => unreachable!("returned above"),
        };
        if validator {
            self.history
                .apply(&mut self.doc, crossing)
                .map_err(|e| e.to_string())?;
            self.start_command("vector_validator");
            return Err(
                "The selected vectors cross: they are selected for the Vector Validator".into(),
            );
        }
        Err("Stopped: the selected vectors cross".into())
    }

    /// Asks what to do about crossing vectors: `None` to go on, `Some(true)` for the Vector
    /// Validator, `Some(false)` to stop.
    fn crossing_choice() -> Option<bool> {
        let (go_on, validator) = ("Continue", "Vector Validator");
        let answer = message_dialog()
            .set_title(APP_NAME)
            .set_level(rfd::MessageLevel::Warning)
            .set_description(
                "Some of the selected vectors cross themselves or each other, which can spoil \
                 the result. Continue anyway, or select them in the Vector Validator?",
            )
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                go_on.into(),
                validator.into(),
                "Cancel".into(),
            ))
            .show();
        match answer {
            rfd::MessageDialogResult::Custom(choice) if choice == go_on => None,
            rfd::MessageDialogResult::Custom(choice) => Some(choice == validator),
            rfd::MessageDialogResult::Yes => None,
            rfd::MessageDialogResult::No => Some(true),
            _ => Some(false),
        }
    }

    fn save_anyway(problems: &[String]) -> bool {
        let answer = message_dialog()
            .set_title(APP_NAME)
            .set_level(rfd::MessageLevel::Warning)
            .set_description(format!(
                "Check before cutting:\n\n• {}\n\nSave the toolpaths anyway?",
                problems.join("\n• ")
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        answer == rfd::MessageDialogResult::Yes
    }

    /// Aspire's icons over the toolpath list: edit and hide the selected toolpath, and save
    /// it, or every visible one, as a template.
    pub(super) fn toolpath_list_buttons(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y * TOOL_ICON_ROWS;
        let mut clicked = None;
        ui.horizontal(|ui| {
            for (id, icon, tip) in [
                (
                    "edit_toolpath",
                    "toolpaths/edit_toolpath.png",
                    "Edit Toolpath",
                ),
                (
                    "hide_toolpath",
                    "toolpaths/hide_toolpaths.png",
                    "Show or Hide Toolpath",
                ),
                (
                    "save_selected_toolpath_as_template",
                    "toolpaths/save_toolpath_template.png",
                    "Save Toolpath as Template",
                ),
                (
                    "save_all_visible_toolpaths_as_template",
                    "toolpaths/save_all_as_template.png",
                    "Save All Visible Toolpaths as Template",
                ),
            ] {
                let available =
                    id == "hide_toolpath" && self.selected_toolpath.is_some() || self.available(id);
                let button = self.icon_button(icon, "", tip, edge);
                if ui
                    .add_enabled(available, button)
                    .on_hover_hint(tip)
                    .clicked()
                {
                    clicked = Some(id);
                }
            }
        });
        match clicked {
            Some("hide_toolpath") => {
                if let Some((id, t)) = self
                    .selected_toolpath
                    .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?)))
                {
                    let params = serde_json::json!({ "toolpath": id, "name": t.name, "visible": !t.visible });
                    if let Err(e) = self.apply("toolpath_properties", &params) {
                        self.status.set_message(&e);
                    }
                }
            }
            Some(id) => self.invoke(ui.ctx(), id),
            None => {}
        }
    }

    /// The file icons Aspire has beside its menus: Import Vectors and Export Vectors (vector
    /// formats only), Open Folder (the job's folder) and Open Clipart Folder.
    pub(super) fn file_icon_action(&mut self, id: &str) {
        let result = match id {
            "import_vectors" => {
                let mut dialog = file_dialog();
                for importer in self.registry.importers() {
                    if VECTOR_IMPORTERS.contains(&importer.id) {
                        dialog =
                            dialog.add_filter(importer.format.label, importer.format.extensions);
                    }
                }
                if let Some(path) = dialog.pick_file() {
                    self.import_file(&path);
                }
                Ok(())
            }
            "export_vectors" => {
                let mut dialog = file_dialog();
                let exporters: Vec<_> = self
                    .registry
                    .exporters()
                    .filter(|e| VECTOR_EXPORTERS.contains(&e.id))
                    .map(|e| (e.id, e.format))
                    .collect();
                for (_, format) in &exporters {
                    dialog = dialog.add_filter(format.label, format.extensions);
                }
                match dialog.save_file() {
                    // the extension picks the format; DXF when there is none
                    Some(path) => {
                        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
                        let (id, path) = exporters
                            .iter()
                            .find(|(_, f)| {
                                ext.as_deref().is_some_and(|x| f.extensions.contains(&x))
                            })
                            .map_or(("export_dxf", path.with_extension("dxf")), |(id, _)| {
                                (*id, path.clone())
                            });
                        files::export(&self.registry, id, &self.doc, &path).map(|()| {
                            self.status
                                .set_message(&format!("Exported {}", path.display()));
                        })
                    }
                    None => Ok(()),
                }
            }
            "open_folder" => {
                let folder = self
                    .session
                    .path
                    .as_deref()
                    .and_then(std::path::Path::parent)
                    .map(std::path::Path::to_path_buf)
                    .or_else(files::config_dir);
                folder.map_or(Err("no folder to open".to_owned()), |f| {
                    on_the_desktop();
                    files::show_folder(&f).map_err(|e| e.to_string())
                })
            }
            // the folder looked in, or the first when looking in all of them
            _ => match (self.clipart.open.as_ref()).or(self.clipart.folders.first()) {
                Some(folder) => {
                    on_the_desktop();
                    files::show_folder(folder).map_err(|e| e.to_string())
                }
                None => Err("add a folder to the Library first".to_owned()),
            },
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Edit Toolpath: the selected toolpath's form, filled in with its settings.
    pub(super) fn edit_toolpath(&mut self) {
        let Some((id, toolpath)) = self
            .selected_toolpath
            .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?.clone())))
        else {
            return;
        };
        self.start_command(&toolpath.strategy);
        let mut params = toolpath.params;
        // the bank as the machine has it now, so a bank changed since is used
        self.fill_drill_bank(&toolpath.strategy, &mut params);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == toolpath.strategy) {
            form.params = params;
            form.editing = Some(id);
        }
    }

    /// A Drill Bank Toolpath's form drills with the active machine's bank (none when it has
    /// none, which the toolpath then says how to set up).
    pub(super) fn fill_drill_bank(&self, command: &str, params: &mut Value) {
        if command != "drill_bank_toolpath" {
            return;
        }
        let bank = self
            .machines
            .active()
            .and_then(|m| m.drill_bank.clone())
            .unwrap_or_default();
        params["bank"] = serde_json::to_value(bank).expect("a drill bank serialises");
    }

    /// A form's Apply: its command, or for an edited toolpath that toolpath made again with
    /// the form's settings, in its place.
    pub(super) fn apply_form(&mut self, id: &str, params: &Value) -> Result<(), String> {
        if let Some(toolpath) = self.form.as_ref().and_then(|f| f.editing) {
            self.edit_in_background(id, toolpath, params);
            return Ok(());
        }
        let before = self.entity_ids();
        let same = self.form.as_ref().and_then(|f| f.applied.as_ref()) == Some(params);
        match self.drawn_shape() {
            // the shape already follows these settings: nothing to do again
            Some(_) if same => return Ok(()),
            Some(old) => self.redraw(id, params, old)?,
            // a shape or model the form goes on changing is made in place; anything else
            // (a toolpath, a relief) may take a while, so it shows its progress
            None if self.makes_one(id) => self.apply(id, params)?,
            None => self.apply_in_background(id, params)?,
        }
        self.remember_drawn(&before);
        if let Some(form) = &mut self.form {
            form.applied = form.drawn.map(|_| params.clone());
        }
        Ok(())
    }

    /// Every vector and component, to tell what a command made.
    pub(super) fn entity_ids(&self) -> std::collections::BTreeSet<EntityId> {
        self.doc
            .vectors
            .keys()
            .chain(self.doc.components.keys())
            .copied()
            .collect()
    }

    /// Whether form `id` makes one shape or model that it goes on to change: the drawing
    /// tools, and the forms whose model follows them as they change.
    pub(super) fn makes_one(&self, id: &str) -> bool {
        self.drawing_tool() == Some(id) || LIVE_FORMS.contains(&id)
    }

    /// Once a frame: a form's shape or model made again with its settings once they have
    /// changed, a dragged value as it goes (the drag one undo).
    pub(super) fn follow_form(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| i.pointer.any_down());
        let Some(form) = self.form.as_mut() else {
            return;
        };
        if !pressed {
            form.dragged = None;
        }
        if !form.dirty {
            return;
        }
        form.dirty = false;
        let dragged = form.dragged.take();
        if self.drawn_shape().is_none() {
            return;
        }
        // the last frame's step of this drag taken back first, so the drag is one undo
        if let Some((depth, drawn, applied)) = dragged
            && self.history.undo_depth() == depth
            && self.history.undo(&mut self.doc)
            && let Some(form) = self.form.as_mut()
        {
            form.drawn = drawn;
            form.applied = applied;
        }
        let Some(form) = &self.form else { return };
        let before = (self.history.undo_depth(), form.drawn, form.applied.clone());
        self.apply_live();
        let depth = self.history.undo_depth();
        if let (true, Some(form)) = (pressed && depth > before.0, self.form.as_mut()) {
            form.dragged = Some((depth, before.1, before.2));
        }
    }

    /// Applies the open form, Orientate 3D Model's Model Size following the model's own
    /// but for the sizes the user typed.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a model grid's columns and rows"
    )]
    #[expect(
        clippy::float_cmp,
        reason = "a size is edited when not the very value shown"
    )]
    pub(super) fn apply_live(&mut self) {
        const SIZES: [&str; 3] = ["x", "y", "z"];
        let Some(form) = &mut self.form else { return };
        let (id, mut params) = (form.id, form.params.clone());
        let sized = params.get("size").is_some();
        if sized {
            let lock = params["size"]["lock_ratio"].as_bool().unwrap_or(false);
            for (i, key) in SIZES.iter().enumerate() {
                let value = params["size"][key].as_f64().unwrap_or(0.0);
                let edited = form
                    .shown_size
                    .map_or(value != 0.0, |shown| value != shown[i]);
                if edited {
                    // locked, the size just typed sets the others
                    if lock {
                        form.typed_size = [None; 3];
                    }
                    form.typed_size[i] = (value > 0.0).then_some(value);
                }
            }
            if lock {
                let first = form.typed_size.iter().position(Option::is_some);
                form.typed_size =
                    std::array::from_fn(|i| form.typed_size[i].filter(|_| Some(i) == first));
            }
            for (key, typed) in SIZES.iter().zip(form.typed_size) {
                params["size"][key] = Value::from(typed.unwrap_or(0.0));
            }
        }
        let wrapped = self.doc.job.rotary.is_some() && params["model_type"] != "flat";
        let result = self.apply_form(id, &params);
        // the component the settings just made judges what a cutter cannot reach, so the
        // model is rasterised once per change, not twice
        let top = self
            .form
            .as_ref()
            .and_then(|f| f.drawn)
            .and_then(|id| self.doc.components.get(&id));
        let model = serde_json::from_value(params)
            .ok()
            .filter(|_| id == "orientate_3d_model")
            .and_then(|p| {
                cmd_orientate_3d_model::preview_over(&self.doc, &p, top)
                    .ok()
                    .flatten()
            });
        // a huge model is shown as the surface it makes, only its unreachable faces drawn
        let cap = usize::try_from(self.options.import.preview_faces).unwrap_or(usize::MAX);
        let (model, whole) = match model {
            Some(all) if all.len() > cap => (
                Some(all.into_iter().filter(|(_, red)| *red).collect()),
                false,
            ),
            all => (all, true),
        };
        let model = model.map(std::sync::Arc::new);
        // the model's size: its triangles' box, or (wrapped) its component's
        let made = model
            .as_ref()
            .and_then(|tris| {
                let corners = tris.iter().flat_map(|(t, _)| t);
                let lo = corners
                    .clone()
                    .copied()
                    .reduce(cut_craft_core::glam::DVec3::min)?;
                let hi = corners.copied().reduce(cut_craft_core::glam::DVec3::max)?;
                Some((hi - lo).to_array())
            })
            .or_else(|| {
                let k = self.doc.components.get(&self.form.as_ref()?.drawn?)?;
                let (lo, hi) = k.height_range().unwrap_or_default();
                Some([k.cell * k.columns as f64, k.cell * k.rows as f64, hi - lo])
            });
        let (area, thickness) = (self.doc.job.area(), self.doc.job.thickness);
        if let Some(form) = &mut self.form {
            form.error = result.err();
            form.model = model;
            form.model_whole = whole;
            if let (true, Some(made)) = (sized, made) {
                // to the hundredth of a millimetre shown, not the cell's float noise
                let round = |v: f64| (v * 100.0).round() / 100.0;
                let shown = std::array::from_fn(|i| form.typed_size[i].unwrap_or(round(made[i])));
                for (key, value) in SIZES.iter().zip(shown) {
                    form.params["size"][key] = Value::from(value);
                }
                // Depth Below Top reads the model's height while the plane is at its
                // bottom (0, or where its bottom was before it was sized), as Aspire's
                // does; a depth typed above the bottom stays, and one past it lifts the
                // model off the plane, up to the material's top
                let depth = &mut form.params["placing"]["depth_below_top"];
                let (at, tall) = (depth.as_f64().unwrap_or(0.0), shown[2]);
                let was_bottom = form.shown_size.is_some_and(|s| s[2] == at);
                if at <= 0.0 || was_bottom {
                    *depth = Value::from(tall);
                } else if at > tall.max(thickness) {
                    *depth = Value::from(tall.max(thickness));
                }
                // centred, the corner it gives is shown instead, for the numbers to move it
                let placing = &mut form.params["placing"];
                if placing["center"] == true && !wrapped {
                    let middle = area.center();
                    placing["position"]["x"] = Value::from(round(middle.x - shown[0] / 2.0));
                    placing["position"]["y"] = Value::from(round(middle.y - shown[1] / 2.0));
                    placing["center"] = Value::Bool(false);
                }
                form.shown_size = Some(shown);
            }
        }
    }

    /// The shape (while it is the one selected) or model the open form made.
    pub(super) fn drawn_shape(&self) -> Option<EntityId> {
        let form = self.form.as_ref()?;
        let id = form.drawn?;
        let only = self.doc.selection.len() == 1 && self.doc.selection.contains(&id);
        let vector = only && self.doc.vectors.contains_key(&id);
        (vector || self.doc.components.contains_key(&id)).then_some(id)
    }

    /// After a form that makes one shape or model makes it, the form goes on to change it.
    pub(super) fn remember_drawn(&mut self, before: &std::collections::BTreeSet<EntityId>) {
        let made: Vec<EntityId> = self.entity_ids().difference(before).copied().collect();
        let one = self.form.as_ref().is_some_and(|f| self.makes_one(f.id));
        let ids = self.entity_ids();
        if let (true, Some(form)) = (one, self.form.as_mut()) {
            form.drawn = match made[..] {
                [made] => Some(made),
                // made again in its own place, it keeps its id
                [] => form.drawn.filter(|id| ids.contains(id)),
                _ => None,
            };
        }
    }

    /// `id`'s command run again with `params`, its shape taking the place of `old`: one undo.
    pub(super) fn redraw(&mut self, id: &str, params: &Value, old: EntityId) -> Result<(), String> {
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let mut taken = Change::builder(&self.doc);
        if self.doc.components.contains_key(&old) {
            taken.remove_component(old);
        } else {
            taken.remove(old);
        }
        let taken = taken.finish();
        let mut without = self.doc.clone();
        without.apply(&taken).map_err(|e| e.to_string())?;
        let made = cmd.run_json(&without, params).map_err(|e| e.to_string())?;
        let change = Change {
            edits: taken.edits.into_iter().chain(made.edits).collect(),
            notes: made.notes,
        };
        let change = self.onto_import_level(id, change);
        let (toolpaths, notes) = (change.toolpaths_made(), change.notes.clone());
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status
            .set_message(&format!("{} changed", cmd.meta().label));
        self.warn(&toolpaths, &notes);
        Ok(())
    }

    /// File ▸ Import ▸ Import Cut List: the list's parts drawn and selected, then Nest Parts
    /// opened on them to put them on the material (moving them, not copying). A mistake in
    /// the file is shown in a message box, since the person has to go and fix the file.
    pub(super) fn import_cut_list(&mut self, path: &std::path::Path) {
        let read = std::fs::read(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))
            .and_then(|bytes| {
                <fmt_cut_list::ImportCutList as cut_craft_core::Importer>::import(&self.doc, &bytes)
                    .map_err(|e| e.to_string())
            });
        match read {
            Ok(change) => {
                self.record(Ok(change), &format!("Imported {}", path.display()));
                self.start_command("nest_parts");
                if let Some(form) = self.form.as_mut().filter(|f| f.id == "nest_parts") {
                    form.params["remove_original_parts"] = Value::from(true);
                }
            }
            Err(e) => {
                message_dialog()
                    .set_title("Import Cut List")
                    .set_description(format!("The cut list could not be imported.\n\n{e}"))
                    .show();
            }
        }
    }

    /// Toolpaths ▸ Print Part Labels ▸ Open Labels to Print: the labels, in the window's
    /// style, opened in the web browser, which prints them.
    pub(super) fn open_part_labels(&mut self) {
        let job = self
            .session
            .path
            .as_deref()
            .and_then(std::path::Path::file_stem)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let path = std::env::temp_dir().join("cut-craft-part-labels.html");
        let opened = fmt_part_labels::labels(&self.doc, &self.options.part_labels, &job)
            .map_err(|e| e.to_string())
            .and_then(|html| {
                cut_craft_core::file::write_atomic(&path, html.as_bytes())
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                on_the_desktop();
                files::show_folder(&path).map_err(|e| e.to_string())
            });
        self.status.set_message(&opened.map_or_else(
            |e| e,
            |()| "Part labels opened in your web browser: print them from there".to_owned(),
        ));
    }

    /// Toolpaths ▸ Create Job Sheet: the page for each sheet with toolpaths (and the side
    /// worked on), in the Job Setup Sheet Editor's style, beside the file named.
    pub(super) fn create_job_sheet(&mut self) {
        let Some(path) = file_dialog().add_filter("Job sheet", &["html"]).save_file() else {
            return;
        };
        self.write_job_sheets(&path);
    }

    /// Toolpaths ▸ Print Job Sheet: the job sheet for the active sheet and side, opened in
    /// the web browser, which prints it.
    pub(super) fn print_job_sheet(&mut self) {
        let path = std::env::temp_dir().join("cut-craft-job-sheet.html");
        let setup = self.job_setup();
        let opened = fmt_create_job_sheet::sheet_for(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| e.to_string())
            .and_then(|html| {
                cut_craft_core::file::write_atomic(&path, html.as_bytes())
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                on_the_desktop();
                files::show_folder(&path).map_err(|e| e.to_string())
            });
        self.status.set_message(&opened.map_or_else(
            |e| e,
            |()| "Job sheet opened in your web browser: print it from there".to_owned(),
        ));
    }

    /// What the job sheet says about where the job is cut: the job's file (when it is on
    /// this computer; else the job is copied to the recovery file, and the sheet says
    /// where), the active machine, its post and tool changer, and the Toolpath Summary's
    /// rapid rate (and the machine's accelerations when the summary allows for them).
    pub(super) fn job_setup(&mut self) -> fmt_create_job_sheet::Setup {
        let job_file = self
            .session
            .path
            .as_deref()
            .filter(|p| p.is_file())
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let job_copy = match (&self.recovery_path, job_file.is_empty()) {
            (Some(path), true) => {
                cut_craft_core::file::write_atomic(path, &cut_craft_core::file::save_job(&self.doc))
                    .map_or_else(|_| String::new(), |()| path.display().to_string())
            }
            _ => String::new(),
        };
        let machine = self.machines.active();
        fmt_create_job_sheet::Setup {
            job_file,
            job_copy,
            machine: machine.map(|m| m.name.clone()).unwrap_or_default(),
            post: machine
                .and_then(|m| m.post())
                .unwrap_or_default()
                .to_owned(),
            outputs: vec![],
            slots: machine.map(|m| m.slots.clone()).unwrap_or_default(),
            printed: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            rapid: self.summary.rapid,
            acceleration: machine
                .filter(|_| self.summary.acceleration)
                .map_or([0.0; 3], |m| m.acceleration),
        }
    }

    /// The job sheets written beside `path`, one a page, the page's suffix after its name.
    pub(super) fn write_job_sheets(&mut self, path: &std::path::Path) {
        let stem = path
            .file_stem()
            .map_or_else(|| "job".into(), |s| s.to_string_lossy().into_owned());
        let setup = self.job_setup();
        let result = fmt_create_job_sheet::sheets(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| e.to_string())
            .and_then(|pages| {
                for (suffix, html) in &pages {
                    let file = path.with_file_name(format!("{stem}{suffix}.html"));
                    cut_craft_core::file::write_atomic(&file, html.as_bytes())
                        .map_err(|e| format!("could not write {}: {e}", file.display()))?;
                }
                Ok(pages.len())
            });
        self.status.set_message(&result.map_or_else(
            |e| e,
            |n| format!("Saved {n} job sheet{}", if n == 1 { "" } else { "s" }),
        ));
    }

    /// Edit Object (E): the one selected vector, if a drawing tool drew it, with that tool's
    /// id and the settings that draw it again.
    pub(super) fn object_to_edit(&self) -> Option<(EntityId, &'static str, Value)> {
        let [id] = self.doc.selection.iter().copied().collect::<Vec<_>>()[..] else {
            return None;
        };
        let path = self.doc.vectors.get(&id)?;
        let (command, params) = self.registry.drawn_by(&self.doc, path)?;
        Some((id, command, params))
    }

    /// Import Toolpaths' form, open on G-code file `path` and named after it.
    pub(super) fn import_toolpaths_from(&mut self, path: &std::path::Path) {
        let id = <cmd_import_toolpaths::ImportToolpaths as cut_craft_core::Command>::ID;
        self.start_command(id);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id) {
            form.params["file"] = Value::from(path.display().to_string());
            if let Some(stem) = path.file_stem() {
                form.params["name"] = Value::from(stem.to_string_lossy().into_owned());
            }
        }
    }

    /// Object Properties on the selection: the form that edits what is selected, the one
    /// way a beginner can always reach: the shape's own tool, a picture's, a component's,
    /// else the selected vectors' position and size.
    pub(super) fn object_properties(&self) -> Option<&'static str> {
        let doc = &self.doc;
        if self.object_to_edit().is_some() || self.shape_to_edit() {
            Some("edit_object")
        } else if !doc.selected_bitmaps.is_empty() {
            Some("bitmap_properties")
        } else if doc.selection.is_empty() && !doc.selected_components.is_empty() {
            Some("component_properties")
        } else if !doc.selection.is_empty() {
            Some("set_size")
        } else {
            None
        }
    }

    /// A component Create Shape made is picked (and no vector): E edits it in the form.
    pub(super) fn shape_to_edit(&self) -> bool {
        self.doc.selection.is_empty()
            && self.doc.selected_components.iter().any(|id| {
                self.doc
                    .components
                    .get(id)
                    .is_some_and(|k| k.shape.is_some())
            })
    }

    /// Opens drawing tool `command` on vector `id` with `params`: changing the form redraws
    /// that shape rather than drawing another, as Aspire's E and Shift-click do.
    pub(super) fn edit_shape(&mut self, id: EntityId, command: &'static str, params: Value) {
        if self.form.as_ref().is_none_or(|f| f.id != command) {
            self.start_command(command);
        }
        self.doc.selection = [id].into();
        if let Some(form) = self.form.as_mut().filter(|f| f.id == command) {
            form.params = params.clone();
            form.drawn = Some(id);
            form.applied = Some(params);
        }
        let label = self.registry.get(command).map_or("", |c| c.meta().label);
        self.status.set_message(&format!(
            "Editing the shape in {label}: change the form to change it"
        ));
    }

    /// After Swap Sides, a double-sided preview turned over with the job: its other side on
    /// top, the side that was up underneath.
    fn turn_preview_over(&mut self) {
        self.finish_playback();
        let Some(mut top) = self.simulation.take() else {
            return;
        };
        let under = top.other_side.take();
        self.simulation = under.map(|under| {
            let mut now_up = std::sync::Arc::unwrap_or_clone(under);
            now_up.other_side = Some(std::sync::Arc::new(top));
            now_up
        });
        (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
    }

    /// `change` with the components Orientate 3D Model makes put on the level clipart was
    /// imported to, if it was (Import to Level).
    fn onto_import_level(&self, id: &str, change: Change) -> Change {
        match self.import_level {
            Some(level) if id == "orientate_3d_model" => crate::clipart::onto_level(change, level),
            _ => change,
        }
    }

    /// Runs a command and records it for undo.
    pub(super) fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        self.crossings_allowed(id, params)?;
        let change = super::background::command_change(&self.registry, &self.doc, id, params);
        self.applied(id, params, change)
    }

    /// What command `id` made with `params` (or why it made nothing) put into the job: onto
    /// the import level, into the picked toolpath group, as one undo; then whatever the
    /// command goes on to do.
    pub(super) fn applied(
        &mut self,
        id: &str,
        params: &Value,
        change: Result<Change, String>,
    ) -> Result<(), String> {
        let change = change?;
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = self.onto_import_level(id, change);
        // a toolpath made while a group is picked in the list joins it
        let change = match &self.selected_group {
            Some(group) => change.into_toolpath_group(group),
            None => change,
        };
        let (made, notes) = (change.toolpaths_made(), change.notes.clone());
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        // a form that edits what it made (Draw Text) goes on editing it, not adding another
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id)
            && let Some(made) = cmd.initial_params(&self.doc).get("replaces").cloned()
        {
            form.params["replaces"] = made;
        }
        self.status
            .set_message(&format!("{} applied", cmd.meta().label));
        // new keep-out zones: the toolpaths are made again round them, as Aspire's are
        if id == "keep_out_zones" {
            self.keep_out_changed();
        }
        // the spiral gadget says how many times round each strand goes, as Aspire's does
        if id == "wrapped_spiral_layout"
            && let Ok(p) = serde_json::from_value(params.clone())
            && let Some(turns) = cmd_wrapped_spiral_layout::revolutions(&self.doc, &p)
        {
            self.status
                .set_message(&format!("Each strand makes {turns:.2} revolutions"));
        }
        self.warn(&made, &notes);
        if id == "swap_sides" {
            self.turn_preview_over();
        }
        Ok(())
    }

    /// The Toolpath Summary's rows: each toolpath with its estimated time and notes.
    fn summary_rows(&self) -> Vec<super::SummaryRow> {
        self.doc
            .toolpaths
            .iter()
            .map(|(&id, t)| super::SummaryRow {
                id,
                name: t.name.clone(),
                minutes: self.estimated_minutes(t),
                visible: t.visible,
                notes: t.summary_notes().to_owned(),
            })
            .collect()
    }

    /// A toolpath's time for the Toolpath Summary: at the rapid rate, allowing for the
    /// active machine's accelerations when asked, times the scale factor.
    pub(super) fn estimated_minutes(&self, t: &cut_craft_core::toolpath::Toolpath) -> f64 {
        let s = &self.summary;
        let minutes = match self.machines.active().filter(|_| s.acceleration) {
            Some(m) => t.machining_minutes_accelerating(s.rapid, m.acceleration),
            None => t.machining_minutes(s.rapid),
        };
        minutes * s.scale
    }

    /// Moves the picked nodes by `by`; across a mirror line (H: up and down through the
    /// selection's middle; V: across it) the nodes on the far side move the other way, so
    /// mirrored pairs stay mirrored.
    pub(super) fn nudge_nodes(&mut self, by: Point) {
        use cut_craft_core::nodes::{move_node, node_near};
        let Some(middle) = self
            .doc
            .selection_bounds()
            .map(cut_craft_core::geometry::Rect::center)
        else {
            return;
        };
        let [mirror_h, mirror_v] = self.node_mirror;
        let delta = |at: Point| {
            Point::new(
                if mirror_h && at.x > middle.x {
                    -by.x
                } else {
                    by.x
                },
                if mirror_v && at.y > middle.y {
                    -by.y
                } else {
                    by.y
                },
            )
        };
        let mut c = Change::builder(&self.doc);
        let vectors: std::collections::BTreeSet<EntityId> =
            self.picked_nodes.iter().map(|n| n.vector).collect();
        for vector in vectors {
            let Some(original) = self.doc.vectors.get(&vector) else {
                continue;
            };
            let mut path = original.clone();
            for n in self.picked_nodes.iter().filter(|n| n.vector == vector) {
                let at = Point::from(n.at);
                if let Some(k) = node_near(original, at, n.reach.max(1e-3)) {
                    path = move_node(&path, k, at + delta(at));
                }
            }
            c.replace(vector, path);
        }
        let change = c.finish();
        match self.history.apply(&mut self.doc, change) {
            Ok(()) => {
                for n in &mut self.picked_nodes {
                    let at = Point::from(n.at) + delta(Point::from(n.at));
                    n.at = cut_craft_core::params::Xy::new(at.x, at.y);
                }
            }
            Err(e) => self.status.set_message(&e.to_string()),
        }
    }

    /// The arrow keys nudge the selection by the snapping grid's squares: a whole one, half
    /// with Shift, a tenth with Ctrl + Shift (Matthew's steps; Aspire nudges with Ctrl +
    /// Shift); with Ctrl alone, by Snapping Options' Fixed Nudge Distance.
    pub(super) fn nudge(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Job || ctx.egui_wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        let g = self.snap.grid_spacing;
        let step = match (shift, ctrl) {
            (true, true) => 0.1 * g,
            (true, false) => 0.5 * g,
            // Snapping Options' Fixed Nudge Distance
            (false, true) => self.snap.fixed_nudge,
            (false, false) => g,
        };
        let by = ctx.input(|i| {
            [
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowDown, 0.0, -1.0),
                (Key::ArrowUp, 0.0, 1.0),
            ]
            .iter()
            .filter(|(key, ..)| i.key_pressed(*key))
            .fold(Point::ZERO, |sum, &(_, x, y)| sum + Point::new(x, y) * step)
        });
        if by == Point::ZERO {
            return;
        }
        // node editing with nodes picked: the arrows move those, mirrored across the lines
        if self.view_tool == Some(super::ViewTool::NodeEdit) && !self.picked_nodes.is_empty() {
            self.nudge_nodes(by);
            return;
        }
        if self.doc.selection_bounds().is_none() {
            return;
        }
        let params =
            serde_json::json!({ "relative": true, "x": by.x, "y": by.y, "anchor": "bottom_left" });
        if let Err(e) = self.apply("move", &params) {
            self.status.set_message(&e);
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // while a slow job runs, the window only shows its progress
        if self.busy.is_some() {
            return;
        }
        // Ctrl+X, Ctrl+C and Ctrl+V reach egui as clipboard events, never as keys; the system
        // clipboard carries what Copy took, so another Cut Craft window pastes it too
        let (cut, copy, pasted) = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .fold((false, false, None), |(x, c, p), e| match e {
                    egui::Event::Cut => (true, c, p),
                    egui::Event::Copy => (x, true, p),
                    egui::Event::Paste(text) => (x, c, Some(text.clone())),
                    _ => (x, c, p),
                })
        });
        if !ctx.egui_wants_keyboard_input() && self.screen == Screen::Job {
            if cut && self.available("cut") {
                self.invoke(ctx, "cut");
            } else if copy && self.available("copy") {
                self.invoke(ctx, "copy");
            }
            if let Some(paths) = pasted.and_then(|text| serde_json::from_str(&text).ok()) {
                self.clipboard = paths;
                self.invoke(ctx, "paste");
            }
        }
        // node editing with nodes picked: H and V toggle the mirror lines, not Mirror
        if !ctx.egui_wants_keyboard_input()
            && self.view_tool == Some(super::ViewTool::NodeEdit)
            && !self.picked_nodes.is_empty()
        {
            for (key, line) in [(Key::H, 0), (Key::V, 1)] {
                let shortcut = KeyboardShortcut::new(Modifiers::NONE, key);
                if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
                    self.node_mirror[line] = !self.node_mirror[line];
                }
            }
        }
        // single-letter shortcuts must not fire while typing in a form field
        if !ctx.egui_wants_keyboard_input() {
            let job = self.screen == Screen::Job;
            let fired: Vec<String> = self
                .bindings(ctx)
                .into_iter()
                .filter(|(id, _)| job || ["new", "open"].contains(&id.as_str()))
                .filter(|(_, keys)| ctx.input_mut(|i| i.consume_shortcut(keys)))
                .map(|(id, _)| id)
                .collect();
            for id in fired {
                self.invoke(ctx, &id);
            }
        }
        // Escape during a 3D handle drag cancels only the drag, staying in Transform mode
        let escape = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
        if self.drag_3d.is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.cancel_drag_3d();
        }
        // Escape in a field being typed in only leaves the field
        if !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            })
        {
            if self.screen == Screen::NewJob {
                self.job_setup_cancel();
            }
            self.form = None;
            self.search = None;
            self.view_tool = None;
            self.pages.done_with(Page::Design);
            self.pages.done_with(Page::Toolpaths);
        }
        self.nudge(ctx);
        ctx.input(|i| {
            if i.key_pressed(Key::F1) {
                let topic = self.form.as_ref().map_or("contents", |f| f.id);
                self.info = Some(Info::Help(topic));
            }
            if i.key_pressed(Key::F2) {
                self.view = View::TwoD;
            }
            if i.key_pressed(Key::F3) {
                self.view = View::ThreeD;
            }
        });
    }

    /// Every key binding: each command's own shortcut, plus the built-ins as the menus spell them,
    /// and on a Mac its own keys for some of them ([`super::mac::extra_bindings`]).
    pub(super) fn bindings(&self, ctx: &egui::Context) -> Vec<(String, KeyboardShortcut)> {
        let commands = self
            .registry
            .iter()
            .filter_map(|c| Some((c.id().to_owned(), shortcut::parse(c.meta().shortcut)?)));
        let built_ins = self
            .menus
            .items()
            .into_iter()
            .filter(|i| BUILT_INS.contains(&i.action.as_str()))
            .filter_map(|i| Some((i.action.clone(), shortcut::parse(&i.shortcut)?)));
        let layouts = LAYOUT_KEYS
            .iter()
            .filter_map(|&(id, keys)| Some((id.to_owned(), shortcut::parse(keys)?)));
        let mut bindings: Vec<_> = commands.chain(built_ins).chain(layouts).collect();
        if super::mac::is_mac(ctx) {
            let extra = super::mac::extra_bindings(&bindings);
            bindings.extend(extra);
        }
        bindings
    }

    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Menu bar");
        let mut clicked = None;
        let recent: Vec<gui::MenuItem> = self
            .recent
            .iter()
            .enumerate()
            .map(|(i, path)| {
                gui::MenuItem::with_action(&file_name(path), &format!("open_recent:{i}"))
            })
            .collect();
        // on a Mac without its title bar, the menus share the title bar's row with the window's
        // traffic-light buttons, which sit at the left
        let mac_row = super::mac::is_mac(ui.ctx()) && !self.title_bar_at_start;
        egui::MenuBar::new().ui(ui, |ui| {
            if mac_row {
                ui.allocate_space(egui::vec2(
                    super::mac::TRAFFIC_LIGHTS,
                    super::mac::TITLE_BAR_HEIGHT - 2.0 * f32::from(ui.spacing().menu_margin.top),
                ));
            }
            for menu in &self.current_menus().menus {
                ui.menu_button(&menu.name, |ui| {
                    menu_items(
                        ui,
                        &menu.items,
                        &Menus {
                            available: &|id| self.available(id),
                            checked: &|id| {
                                self.display
                                    .checked(id)
                                    .or_else(|| self.resolution_checked(id))
                                    .or_else(|| self.units_checked(id))
                                    .or_else(|| self.layout_checked(id))
                            },
                            shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                            recent: &recent,
                            image: &|path| self.icons.find(path, "").map(|p| self.icons.image(p)),
                        },
                        &mut clicked,
                    );
                });
            }
            if mac_row {
                super::title_bar::title_and_drag(ui, &self.title);
            } else if !self.options.appearance.system_title_bar && !super::mac::is_mac(ui.ctx()) {
                super::title_bar::controls(ui, &self.title);
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The menu bar with or without a job open.
    pub(super) fn current_menus(&self) -> &gui::MenuSystem {
        if self.screen == Screen::Start {
            &self.start_menus
        } else {
            &self.menus
        }
    }

    #[cfg(test)]
    pub(super) fn menu_names(&self) -> Vec<&str> {
        self.current_menus()
            .menus
            .iter()
            .map(|m| m.name.as_str())
            .collect()
    }

    /// No job open: Aspire's startup tasks and the jobs opened lately.
    pub(super) fn start_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Start page");
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        ui.strong("Startup Tasks");
        for (id, label, icon) in STARTUP_TASKS {
            let button = match self.icons.find(icon, id) {
                Some(path) => egui::Button::image_and_text(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                    label,
                ),
                None => egui::Button::new(label),
            }
            .frame(false);
            let response = ui
                .add_enabled(self.available(id), button)
                .on_disabled_hover_text(format!("{label}: not implemented yet"));
            if response.clicked() {
                clicked = Some(id.to_owned());
            }
        }
        ui.separator();
        ui.strong("Recently opened files");
        if self.recent.is_empty() {
            ui.weak("None yet");
        }
        let glyph = self.icons.find(RECENT_FILE_ICON, "");
        for (i, path) in self.recent.iter().enumerate() {
            let name = file_name(path);
            let button = match glyph {
                Some(glyph) => egui::Button::image_and_text(
                    self.icons
                        .image(glyph)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                    name,
                ),
                None => egui::Button::new(name),
            }
            .frame(false);
            if ui
                .add(button)
                .on_hover_hint(path.display().to_string())
                .clicked()
            {
                clicked = Some(format!("open_recent:{i}"));
            }
        }
        ui.separator();
        ui.strong(format!("What's new in v{}", env!("CARGO_PKG_VERSION")));
        for change in WHATS_NEW.lines().filter(|l| !l.trim().is_empty()) {
            // a bullet, with the change wrapping beside it
            ui.horizontal_top(|ui| {
                ui.label("•");
                ui.add(egui::Label::new(change.trim_start_matches("- ")).wrap());
            });
        }
        ui.separator();
        ui.strong("Links");
        for (label, url, icon) in LINKS {
            ui.horizontal(|ui| {
                if let Some(path) = self.icons.find(icon, "") {
                    ui.add(
                        self.icons
                            .image(path)
                            .fit_to_exact_size(egui::vec2(edge, edge)),
                    );
                }
                ui.hyperlink_to(label, url);
            });
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The middle of the window with no job open: the Cut Craft mark.
    pub(super) fn splash(ui: &mut egui::Ui) {
        let space = ui.available_size();
        let edge = space.min_elem() * SPLASH_SHARE;
        ui.vertical_centered(|ui| {
            ui.add_space((space.y - edge) / 2.0);
            ui.add(
                egui::Image::from_bytes("bytes://cut-craft.svg", LOGO)
                    .fit_to_exact_size(egui::vec2(edge, edge)),
            );
            ui.heading(APP_NAME);
        });
    }

    /// The Design page: the chosen design tab's tools, under any form opened from them.
    pub(super) fn design_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design page");
        self.page_header(ui, Page::Design, DESIGN_TABS[self.design_tab]);
        let tab = DESIGN_TABS[self.design_tab];
        let form_open = self.form.as_ref().is_some_and(|f| f.page == Page::Design);
        egui::ScrollArea::vertical().show(ui, |ui| {
            // a tool's form takes the page's place, as Aspire's does
            if form_open {
                self.command_form(ui);
                return;
            }
            match tab {
                "Design" => {
                    if self.options.favourites.of(Page::Design).1 {
                        // favourites name every tool, whatever sub-tab it is on
                        let groups = self.design_groups("Design");
                        self.tools_or_favourites(ui, &groups, Page::Design);
                    } else {
                        self.design_subtabs(ui);
                        let groups = self.toolbar.subtab_groups(self.design_subtab);
                        self.tool_groups(ui, &groups, Page::Design);
                    }
                }
                "Sheets" => self.sheets_panel(ui),
                "Library" => self.clipart_panel(ui),
                "Layers" => self.layers_panel(ui),
                "Components" => self.component_list(ui),
                tab => {
                    ui.weak(format!("{tab} tools not implemented yet"));
                }
            }
        });
        // the hovered tool's guidance, or the tool in hand's, in the room left under the icons
        if tab == "Design" && !form_open {
            self.tip_box(ui);
        }
    }

    /// The tool groups a design tab shows: Design has the drawing then the modeling tools.
    pub(super) fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        match tab {
            "Design" => self.toolbar.all_design_groups(),
            _ => vec![],
        }
    }

    /// The Design tab's sub-tabs across the page, under the groups always shown: Create,
    /// Edit, Arrange, Model. Each names what its tools are for on hover, so a beginner
    /// looks under a word rather than down one long page of icons.
    pub(super) fn design_subtabs(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design sub-tabs");
        let always = self.toolbar.named(gui::ALWAYS_SHOWN);
        self.tool_groups(ui, &always, Page::Design);
        ui.separator();
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            for (i, tab) in gui::DESIGN_SUBTABS.iter().enumerate() {
                let response = ui
                    .selectable_label(i == self.design_subtab, tab.name)
                    .on_hover_hint(tab.hint);
                if response.clicked() {
                    picked = Some(i);
                }
            }
        });
        ui.add_space(ui.spacing().item_spacing.y);
        if let Some(i) = picked {
            self.design_subtab = i;
        }
    }

    /// Opens the sub-tab `id` is on, so a tool picked from Find Tool, a menu or a shortcut
    /// shows highlighted where it lives.
    pub(super) fn show_subtab_of(&mut self, id: &str) {
        if let Some(tab) = self.toolbar.subtab_of(id, self.design_subtab) {
            self.design_subtab = tab;
        }
    }

    /// A page's title with its push-pin and Aspire's Switch Layout button.
    pub(super) fn page_header(&mut self, ui: &mut egui::Ui, page: Page, title: &str) {
        let row = ui.spacing().interact_size.y;
        let (mut pin, mut switch, mut help) = (false, false, false);
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pinned = self.pages.pinned(page);
                pin = ui
                    .selectable_label(pinned, "📌")
                    .on_hover_hint(if pinned {
                        "Unpin: hide this page to its side tab"
                    } else {
                        "Pin this page open"
                    })
                    .clicked();
                switch = ui
                    .add(self.icon_button("view/interface_layout.png", "", "Switch Layout", row))
                    .on_hover_hint("Switch Layout (F11 designing, F12 machining)")
                    .clicked();
                help = ui.button("?").on_hover_hint("Help (F1)").clicked();
            });
        });
        ui.separator();
        if pin {
            let pinned = self.pages.pinned(page);
            self.pages.set_pinned(page, !pinned);
        }
        if switch {
            self.pages.switch_layout();
        }
        if help {
            self.info = Some(Info::Help(match page {
                Page::Design => "design",
                Page::Toolpaths => "toolpaths",
            }));
        }
    }

    /// The side tabs and the docked pages around the view; flown-out pages over it.
    pub(super) fn job_pages(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let style = ctx.global_style();
        let (row, margin) = (style.spacing.interact_size.y, style.spacing.item_spacing.y);
        let strip = || egui::Frame::side_top_panel(&style).inner_margin(margin);
        let left_tabs = egui::Panel::left("design_tabs")
            .resizable(false)
            .exact_size(row + 2.0 * margin)
            .frame(strip())
            .show(ui, |ui| self.design_tabs(ui))
            .response
            .rect;
        let right_tabs = egui::Panel::right("toolpath_tabs")
            .resizable(false)
            .exact_size(row + 2.0 * margin)
            .frame(strip())
            .show(ui, |ui| self.toolpath_tabs(ui))
            .response
            .rect;
        if self.pages.shown(Page::Design) == Shown::Docked {
            side_panel(egui::Panel::left("design_panel"), ctx).show(ui, |ui| self.design_page(ui));
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            side_panel(egui::Panel::right("toolpaths_panel"), ctx)
                .show(ui, |ui| self.toolpaths_page(ui));
        }
        let room = ui.available_rect_before_wrap();
        // a click on the view puts a flown-out page away, unless its form is open
        let pressed = ctx.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        for (page, tabs) in [(Page::Design, left_tabs), (Page::Toolpaths, right_tabs)] {
            if self.pages.shown(page) != Shown::Flyout {
                continue;
            }
            let out = self.flyout(ctx, page, room);
            let form_here = self.form.as_ref().is_some_and(|f| f.page == page);
            let on_view = |at: egui::Pos2| {
                room.contains(at)
                    && ctx.layer_id_at(at).map(|l| l.order) != Some(egui::Order::Foreground)
            };
            if let Some(at) = pressed
                && on_view(at)
                && !out.contains(at)
                && !tabs.contains(at)
                && !form_here
            {
                self.pages.done_with(page);
            }
        }
    }

    /// The tool page a command's form shows in.
    pub(super) fn page_of(&self, id: &str) -> Page {
        let toolpaths = self
            .toolbar
            .toolpath_groups
            .iter()
            .any(|g| g.tools.contains(&id));
        if toolpaths {
            Page::Toolpaths
        } else {
            Page::Design
        }
    }

    /// A tool picked from a page: a page that flew out for it goes away again, unless the
    /// tool opened a form there.
    pub(super) fn pick(&mut self, ctx: &egui::Context, page: Page, id: &str) {
        self.invoke(ctx, id);
        if self.form.is_none() {
            self.pages.done_with(page);
        }
    }

    /// Aspire's design tabs, down the left edge.
    pub(super) fn design_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Design) != Shown::Hidden;
        let only = self.options.favourites.of(Page::Design).1;
        let labels = DESIGN_TABS.map(|t| {
            if t == "Design" && only {
                "Design ★"
            } else {
                t
            }
        });
        let tabs = side_tabs(ui, &labels, shown.then_some(self.design_tab));
        let (hovered, clicked) = (
            tabs.iter().position(egui::Response::hovered),
            tabs.iter().position(egui::Response::clicked),
        );
        let design = DESIGN_TABS.iter().position(|&t| t == "Design");
        if let Some(action) =
            design.and_then(|i| self.popup(&tabs[i], || gui::favourites_menu(Page::Design, only)))
        {
            self.invoke(ui.ctx(), &action);
        }
        // an auto-hidden page comes out on hover; a docked one changes tab on click
        let unpinned = !self.pages.pinned(Page::Design);
        if let Some(tab) = clicked.or(hovered.filter(|_| unpinned)) {
            self.set_design_tab(tab);
            self.pages.open(Page::Design);
        }
    }

    /// A favourites action from a tool's or a tab's menu, kept in the options at once.
    pub(super) fn favourites_action(&mut self, action: &str) {
        if let Some(rest) = action.strip_prefix(gui::FAVOURITES_ONLY)
            && let Some((page, on)) = rest.split_once(':')
            && let Some(page) = gui::page_of(page)
        {
            *self.options.favourites.of_mut(page).1 = on == "true";
        } else if let Some(rest) = action.strip_prefix(gui::FAVOURITE)
            && let [page, what, id] = rest.splitn(3, ':').collect::<Vec<_>>()[..]
            && let Some(page) = gui::page_of(page)
        {
            let list = self.options.favourites.of_mut(page).0;
            let at = list.iter().position(|f| f == id);
            match (what, at) {
                ("add", None) => list.push(id.to_owned()),
                ("remove", Some(i)) => {
                    list.remove(i);
                }
                ("earlier", Some(i)) if i > 0 => list.swap(i, i - 1),
                ("later", Some(i)) if i + 1 < list.len() => list.swap(i, i + 1),
                _ => return,
            }
        } else {
            return;
        }
        self.save_options();
    }

    /// Going to another design tab closes a form open on the Design page, as Aspire's does.
    pub(super) fn set_design_tab(&mut self, tab: usize) {
        if tab != self.design_tab && self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
            self.form = None;
        }
        self.design_tab = tab;
    }

    /// The Toolpaths tab, down the right edge.
    pub(super) fn toolpath_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Toolpaths) != Shown::Hidden;
        let only = self.options.favourites.of(Page::Toolpaths).1;
        let label = if only { "Toolpaths ★" } else { "Toolpaths" };
        let tabs = side_tabs(ui, &[label], shown.then_some(0));
        let (hovered, clicked) = (
            tabs.iter().position(egui::Response::hovered),
            tabs.iter().position(egui::Response::clicked),
        );
        if let Some(action) = self.popup(&tabs[0], || gui::favourites_menu(Page::Toolpaths, only)) {
            self.invoke(ui.ctx(), &action);
        }
        let unpinned = !self.pages.pinned(Page::Toolpaths);
        if clicked.or(hovered.filter(|_| unpinned)).is_some() {
            self.pages.open(Page::Toolpaths);
        }
    }

    /// An auto-hidden page flown out over the view from its side; returns where it is.
    pub(super) fn flyout(
        &mut self,
        ctx: &egui::Context,
        page: Page,
        room: egui::Rect,
    ) -> egui::Rect {
        let width = DESIGN_PANEL_WIDTHS * ctx.global_style().spacing.interact_size.x;
        let frame = egui::Frame::side_top_panel(&ctx.global_style());
        let height = room.height() - frame.inner_margin.sum().y - frame.outer_margin.sum().y;
        let left = match page {
            Page::Design => room.left(),
            Page::Toolpaths => room.right() - width - frame.inner_margin.sum().x,
        };
        egui::Area::new(egui::Id::unique(("flyout", page)))
            .fixed_pos(egui::pos2(left, room.top()))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(height);
                    match page {
                        Page::Design => self.design_page(ui),
                        Page::Toolpaths => self.toolpaths_page(ui),
                    }
                });
            })
            .response
            .rect
    }

    /// The open form's buttons: OK and Cancel for a new job's setup, else Apply and Close.
    pub(super) fn form_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        if open.id == "orientate_3d_model" {
            return self.import_buttons(ui);
        }
        let new_job = open.new_job;
        let (apply_label, close_label) = if new_job {
            ("OK", "Cancel")
        } else {
            ("Apply", "Close")
        };
        let job_setup = open.id == "job_setup";
        // what the form has done that Cancel would take back
        let done = self.history.undo_depth() > open.undo_from;
        let (mut apply, mut close, mut default, mut cancel) = (false, false, false, false);
        let (mut save, mut load) = (false, false);
        if open.page == Page::Toolpaths {
            ui.horizontal(|ui| {
                save = ui
                    .button("Save Settings…")
                    .on_hover_hint("Save this form's settings to a file, to use again later")
                    .clicked();
                load = ui
                    .button("Load Settings…")
                    .on_hover_hint("Fill this form from settings you saved before")
                    .clicked();
            });
        }
        ui.horizontal(|ui| {
            apply = ui.button(apply_label).clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui
                    .button(close_label)
                    .on_hover_hint(if new_job {
                        "Leave the job as it was"
                    } else {
                        "Close, keeping what was applied"
                    })
                    .clicked();
                if !new_job {
                    cancel = ui
                        .add_enabled(done, egui::Button::new("Cancel"))
                        .on_hover_hint("Undo what this tool did since it opened, and close it")
                        .on_disabled_hover_text("Nothing applied to take back")
                        .clicked();
                }
                if job_setup {
                    default = ui
                        .button("Set as Default")
                        .on_hover_hint("Start every new job with these settings")
                        .clicked();
                }
            });
        });
        if default {
            self.job_setup_default();
            return;
        }
        if cancel {
            self.cancel_form();
            return;
        }
        if save || load {
            self.form_settings_file(save);
            return;
        }
        match (new_job, apply, close) {
            (true, true, _) => self.job_setup_ok(),
            (true, _, true) => self.job_setup_cancel(),
            (false, true, _) => {
                let (id, params) = (open.id, open.params.clone());
                // a job that goes on in the background puts what goes wrong on the form
                if let Some(open) = &mut self.form {
                    open.error = None;
                }
                let result = self.apply_form(id, &params);
                if let (Err(why), Some(open)) = (result, &mut self.form) {
                    open.error = Some(why);
                }
            }
            (false, _, true) => {
                let page = open.page;
                self.form = None;
                self.pages.done_with(page);
            }
            _ => {}
        }
    }

    /// Save Settings… (or Load Settings… when not `save`) on a toolpath's form.
    fn form_settings_file(&mut self, save: bool) {
        let Some(form) = &self.form else { return };
        let dialog = form_settings_dialog();
        if save {
            let Some(path) = dialog
                .set_file_name(format!("{}.{}", form.label, files::FORM_SETTINGS_EXTENSION))
                .save_file()
            else {
                return;
            };
            self.save_form_settings(&path.with_extension(files::FORM_SETTINGS_EXTENSION));
        } else if let Some(path) = dialog.pick_file() {
            self.load_form_settings(&path);
        }
    }

    /// The open form's settings saved to `path`, saying so in the status bar.
    pub(super) fn save_form_settings(&mut self, path: &std::path::Path) {
        let Some(form) = &self.form else { return };
        let bytes = files::form_settings(form.id, form.label, &form.params);
        let message = match cut_craft_core::file::write_atomic(path, &bytes) {
            Ok(()) => format!("Saved the settings as {}", path.display()),
            Err(e) => format!("Could not save the settings to {}: {e}", path.display()),
        };
        self.status.set_message(&message);
    }

    /// The open form filled from the settings saved at `path`, saying so in the status bar.
    pub(super) fn load_form_settings(&mut self, path: &std::path::Path) {
        let Some(form) = &mut self.form else { return };
        let loaded = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                files::load_form_settings(form.id, form.label, &mut form.params, &bytes)
            });
        let message = match loaded {
            Ok(()) => {
                form.dirty = true;
                format!("Loaded the settings from {}", path.display())
            }
            Err(e) => format!("Could not load {}: {e}", path.display()),
        };
        self.status.set_message(&message);
    }

    /// Import 3D Model's buttons: Import keeps the model as placed and closes; Cancel takes
    /// back what the form made and closes.
    fn import_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        let page = open.page;
        let (mut import, mut cancel) = (false, false);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                cancel = ui
                    .button("Cancel")
                    .on_hover_hint("Close without importing the model")
                    .clicked();
                import = ui
                    .button("Import")
                    .on_hover_hint("Keep the model as placed")
                    .clicked();
            });
        });
        if cancel {
            self.cancel_form();
        } else if import {
            self.form = None;
            self.pages.done_with(page);
            self.status.set_message("Model imported");
        }
    }

    /// Cancel on a tool's form: everything done since the form opened is undone (and can be
    /// redone), and the form closes.
    pub(super) fn cancel_form(&mut self) {
        let Some(form) = self.form.take() else { return };
        while self.history.undo_depth() > form.undo_from && self.history.undo(&mut self.doc) {}
        self.pages.done_with(form.page);
        self.status
            .set_message(&format!("{} cancelled", form.label));
    }

    /// A form's heading: its tool's icon and name, then what to do first while the tool has
    /// nothing to work on, or how to draw with it (see [`Self::form_guidance`]).
    fn form_heading(&self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        ui.horizontal(|ui| {
            if let Some(path) = self.icons.find(open.icon, open.id) {
                let edge = ui.spacing().interact_size.y;
                ui.add(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                );
            }
            ui.strong(open.label);
        });
        if let Some(step) = self.form_guidance() {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Next:");
                ui.label(step);
            });
        }
    }

    pub(super) fn command_form(&mut self, ui: &mut egui::Ui) {
        let form = self.form.as_ref().map_or("none", |f| f.label);
        cut_craft_bug_report::mark(ui, &format!("Tool form: {form}"));
        let slots = self.slots();
        self.form_heading(ui);
        let Some(open) = &mut self.form else { return };
        // a form that sets the units (Job Setup) shows its lengths in them as they change
        let edge = SECTION_PICTURE_ROWS * ui.spacing().interact_size.y;
        let picture = |path: &str| {
            self.icons.find(path, "").map(|p| {
                self.icons
                    .image(p)
                    .fit_to_exact_size(egui::vec2(edge, edge))
            })
        };
        let units = form::units_chosen(&open.params).unwrap_or(self.doc.job.units);
        // the tool block, where the form has its tool: what was picked, and Edit…
        let asked = std::cell::RefCell::new((None, false));
        // a tool in a list whose Edit… was clicked
        let edit_listed = std::cell::RefCell::new(None);
        let listed = std::cell::Cell::new(false);
        let tools = &self.tools;
        let layers: Vec<String> = self.doc.layers.iter().map(|l| l.name.clone()).collect();
        let tool = |ui: &mut egui::Ui, key: &str, params: &mut Value| {
            if key != "tool" {
                // a list of tools, each picked as the form's own tool is
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.strong(match key {
                        "clearance_tools" => "Clearance Tools",
                        _ => "Tools",
                    });
                    let largest_first = key == "clearance_tools";
                    let (changed, edit) =
                        tool_list(ui, key, &mut params[key], tools, &slots, largest_first);
                    listed.set(listed.get() | changed);
                    if edit.is_some() {
                        *edit_listed.borrow_mut() = edit;
                    }
                });
                return;
            }
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("Tool");
                *asked.borrow_mut() = tool_picker(ui, &params["tool"], params, tools, &slots);
                listed.set(listed.get() | super::passes::edit_passes(ui, params, units));
            });
            // every toolpath that cuts with a tool can rest machine after the tools that cut
            // before it (3D Finishing has its own, as `after`)
            if params.get("after").is_none()
                && let Some(edit) = rest_section(ui, params, tools, &slots)
            {
                *edit_listed.borrow_mut() = Some(edit);
            }
            selector_section(ui, params, &layers, units);
        };
        let look = form::Look {
            units,
            picture: &picture,
            material: form::material(&self.doc, units),
            tool: Some(&tool),
            group_end: Some(&super::tabs::group_end),
        };
        let icon = |path: &str| self.icons.find(path, "").map(|p| self.icons.image(p));
        open.dirty |= show_form(ui, open, &look, &self.doc.job, &icon);
        open.dirty |= listed.get();
        let (id, new_job) = (open.id, open.new_job);
        if let Some(error) = &open.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let (chosen, edit_tool) = asked.into_inner();
        let edit_listed = edit_listed.into_inner();
        self.form_extras(ui, id);
        // Job Setup for a new job keeps its buttons pinned under the panel instead
        if !new_job {
            self.form_buttons(ui);
            ui.separator();
        }
        if let Some(name) = chosen {
            self.choose_tool(&name);
        }
        if edit_tool {
            let name = self
                .form
                .as_ref()
                .and_then(|f| f.params["tool"]["name"].as_str().map(str::to_owned));
            self.edit_tool(ui.ctx(), name);
        }
        if edit_listed.is_some() {
            self.edit_tool(ui.ctx(), edit_listed);
        }
    }

    /// Texture Toolpath's Load… or Save…: the texture as a `.vtexture` file, in the textures
    /// folder unless taken elsewhere.
    pub(super) fn texture_file(&mut self, save: bool) {
        let extension = cmd_texture_toolpath::TEXTURE_EXTENSION;
        let picked = if save {
            texture_dialog()
                .set_file_name(format!("texture.{extension}"))
                .save_file()
                .map(|path| path.with_extension(extension))
        } else {
            texture_dialog().pick_file()
        };
        let Some(path) = picked else { return };
        let result = if save {
            self.save_texture(&path)
        } else {
            self.load_texture(&path)
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Writes the open Texture Toolpath form's texture to `path`.
    pub(super) fn save_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        let texture = self.texture_params()?.to_vtexture();
        std::fs::write(path, texture)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.status
            .set_message(&format!("Saved texture {}", path.display()));
        Ok(())
    }

    /// Reads the texture in `path` into the open Texture Toolpath form; its name, tool,
    /// seed and boundary offset stay.
    pub(super) fn load_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let params = self.texture_params()?.with_vtexture(&text)?;
        let Some(form) = &mut self.form else {
            return Err("no Texture Toolpath form is open".into());
        };
        form.params = serde_json::to_value(params).expect("texture settings serialise");
        form.dirty = true;
        self.status
            .set_message(&format!("Loaded texture {}", path.display()));
        Ok(())
    }

    /// The open Texture Toolpath form's settings.
    fn texture_params(&self) -> Result<cmd_texture_toolpath::Params, String> {
        let form = self.form.as_ref().filter(|f| f.id == "texture_toolpath");
        let form = form.ok_or("no Texture Toolpath form is open")?;
        serde_json::from_value(form.params.clone())
            .map_err(|e| format!("the form's settings do not read: {e}"))
    }

    /// What some forms have under their fields: Texture Toolpath's Load / Save Texture, and
    /// Quick Engrave's Immediate Output.
    fn form_extras(&mut self, ui: &mut egui::Ui, id: &str) {
        match id {
            "texture_toolpath" => match texture_buttons(ui) {
                (true, _) => self.texture_file(false),
                (_, true) => self.texture_file(true),
                _ => {}
            },
            "draw_text" => self.spelling(ui),
            "relief_from_photo" => self.depth_section(ui),
            "profile_toolpath" => self.edit_tabs(ui),
            "sculpt" => {
                let (load, round) = ui
                    .horizontal(|ui| {
                        let load = ui
                            .button("Load Brush…")
                            .on_hover_hint(
                                "A component saved as a sculpting brush, for Deposit and Remove",
                            )
                            .clicked();
                        let round = ui.button("Round Brush").clicked();
                        (load, round)
                    })
                    .inner;
                let brush = if round {
                    Some(String::new())
                } else if load {
                    brush_dialog().pick_file().map(|p| p.display().to_string())
                } else {
                    None
                };
                if let (Some(brush), Some(form)) = (brush, self.form.as_mut()) {
                    form.params["brush"] = brush.into();
                }
            }
            "quick_engrave_toolpath" => {
                let choices = self.post_choices();
                let default = self.output_post().map(|p| p.name);
                let two_sided = self.doc.job.double_sided.is_some();
                let ready = self.immediate_output_toolpath().is_some();
                let output = &mut self.immediate_output;
                if super::views::immediate_output(ui, output, &default, &choices, two_sided, ready)
                {
                    self.immediate_save();
                }
            }
            _ => {}
        }
    }

    /// Relief from Photo's Depth: Estimate Depth when the add-on is installed, else how to
    /// get it.
    fn depth_section(&mut self, ui: &mut egui::Ui) {
        let installed = self.addons.dir().and_then(crate::depth::find);
        let chosen = self
            .doc
            .selected_bitmaps
            .iter()
            .rev()
            .copied()
            .find(|id| self.doc.bitmaps.contains_key(id))
            .or_else(|| self.doc.bitmaps.keys().next_back().copied());
        let has_depth = chosen.is_some_and(|id| {
            let b = &self.doc.bitmaps[&id];
            !b.grey.is_empty() && b.depth.len() == b.grey.len()
        });
        let from_picture = self
            .form
            .as_ref()
            .is_some_and(|f| f.params["depth_from"] == "picture");
        let mut start = false;
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Depth");
            if from_picture {
                ui.label(
                    "The picture is used as the depth map itself (white nearest), so no \
                     estimate is needed. Tick Invert if its black is nearest.",
                );
                return;
            }
            let Some(_) = &installed else {
                ui.label(
                    "Turning a photo into a relief needs the free Depth add-on \
                     (about 100 MB, downloaded once). If the picture is already a depth \
                     map, set Depth from to Picture instead.",
                );
                self.depth_install(ui);
                return;
            };
            if self.depth_estimate.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Estimating depth… (up to a minute on a large photo)");
                });
                return;
            }
            ui.label(if has_depth {
                "This photo has its depth. Estimate again after editing the photo."
            } else {
                "First estimate how far each part of the photo is from the camera."
            });
            start = ui
                .add_enabled(chosen.is_some(), egui::Button::new("Estimate Depth"))
                .on_hover_hint("Works out the photo's depth on this computer; nothing is uploaded")
                .on_disabled_hover_text("Import a photo first")
                .clicked();
        });
        if let (true, Some(installed), Some(id)) = (start, installed, chosen) {
            match crate::depth::Estimating::start(&installed, id, &self.doc.bitmaps[&id], ui.ctx())
            {
                Ok(running) => self.depth_estimate = Some(running),
                Err(e) => self.status.set_message(&e),
            }
        }
    }

    /// Install Depth Add-on, or how its install is going (the Add-ons window's install).
    fn depth_install(&mut self, ui: &mut egui::Ui) {
        use crate::addons::Phase;
        match &self.addons.phase {
            Phase::Finding(_) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Looking for the newest Depth add-on...");
                });
            }
            Phase::Installing { done, total, .. } => {
                ui.label("Downloading the Depth add-on...");
                #[expect(clippy::cast_precision_loss, reason = "a fraction for the bar")]
                let fraction = if *total > 0 {
                    *done as f32 / *total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(fraction.min(1.0)).show_percentage());
            }
            Phase::Asking(_) => {
                ui.label("The Add-ons window is asking whether to install it.");
            }
            phase => {
                if let Phase::Failed(e) = phase {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                let can = self.addons.dir().is_some();
                if ui
                    .add_enabled(can, egui::Button::new("Install Depth Add-on"))
                    .on_hover_hint("Downloads it from GitHub once; Gadgets ▸ Add-ons updates it")
                    .on_disabled_hover_text("Cut Craft has no settings folder to put it in")
                    .clicked()
                {
                    self.install_depth(ui.ctx());
                }
            }
        }
    }

    /// Starts installing the official Depth add-on.
    pub(super) fn install_depth(&mut self, ctx: &egui::Context) {
        let depth = cut_craft_updater::addon::OFFICIAL
            .iter()
            .find(|a| a.id == crate::depth::ADDON_ID)
            .expect("Depth is an official add-on");
        self.addons.install_from(ctx, depth.repo);
    }

    /// Gives the picture its depth once the Depth add-on answers (undoable), and shows the
    /// form's preview again.
    pub(super) fn take_depth(&mut self) {
        let Some(answer) = self
            .depth_estimate
            .as_ref()
            .and_then(crate::depth::Estimating::answer)
        else {
            return;
        };
        let sent = self.depth_estimate.take().expect("answered");
        let id = sent.bitmap;
        let result = answer.and_then(|depth| {
            let bitmap = self
                .doc
                .bitmaps
                .get(&id)
                .ok_or("the photo was deleted while its depth was estimated")?;
            // another job opened, or the photo edited, while the add-on worked
            if *bitmap != sent.photo || depth.len() != bitmap.grey.len() {
                return Err("the photo changed while its depth was estimated; try again".into());
            }
            let after = crate::depth::with_depth(bitmap, depth);
            let mut c = cut_craft_core::Change::builder(&self.doc);
            c.replace_bitmap(id, after);
            let change = c.finish();
            self.history
                .apply(&mut self.doc, change)
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => {
                self.status.set_message("Depth estimated");
                if let Some(form) = self.form.as_mut() {
                    form.dirty = true;
                }
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Reads the user's tool database; a broken one is reported and left untouched on disk.
    pub(super) fn load_tools(&mut self) {
        self.tools_path = files::tool_database_path();
        if let Some(path) = &self.tools_path {
            match files::load_tools(path) {
                Ok(tools) => self.tools = tools,
                Err(e) => {
                    self.status
                        .set_message(&format!("{e}; tool changes will not be saved"));
                    self.tools_path = None;
                }
            }
        }
    }

    /// Copies a database tool, cutting the active material, into the open form.
    pub(super) fn choose_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.pick(name)) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// The Tool Database, opened on the open form's tool; saving it updates the form.
    pub(super) fn edit_tool(&mut self, ctx: &egui::Context, name: Option<String>) {
        self.invoke(ctx, "tool_database");
        if let Some(manager) = &mut self.tool_manager {
            manager.pick = name
                .and_then(|n| manager.draft.tools.iter().position(|t| t.name == n))
                .map(cut_craft_tool_database::Pick::Tool);
        }
    }

    /// The open form's tool again from the database, after the database changed.
    pub(super) fn refresh_form_tool(&mut self) {
        let name = self
            .form
            .as_ref()
            .and_then(|f| f.params.get("tool")?["name"].as_str().map(str::to_owned));
        if let Some(name) = name.filter(|n| self.tools.get(n).is_some()) {
            self.choose_tool(&name);
            if let Some(form) = &mut self.form {
                form.dirty = true;
            }
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    pub(super) fn tools_changed(&mut self, done: &str) {
        let result = self
            .tools_path
            .as_deref()
            .map_or(Ok(()), |path| files::save_tools(&self.tools, path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Model > Create Component from Toolpath Preview: the block as previewed, standing on
    /// the material's bottom.
    pub(super) fn component_from_preview(&mut self) {
        if let Some(preview) = &self.simulation {
            // the block as cut, standing on the material's bottom
            let mut block = preview.clone();
            for h in &mut block.heights {
                *h -= preview.bottom;
            }
            let component = cut_craft_core::component::Component::from_material(
                "Toolpath Preview",
                cut_craft_core::component::Combine::Add,
                &block,
            );
            let mut c = Change::builder(&self.doc);
            c.add_component(component);
            self.record(Ok(c.finish()), "Made a component from the toolpath preview");
        }
    }

    /// Cut, Copy, Paste, Increment and Save, Open Application Data Folder.
    pub(super) fn clipboard_or_file_action(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            "copy" | "cut" => {
                self.clipboard = self
                    .doc
                    .selection
                    .iter()
                    .map(|id| self.doc.vectors[id].clone())
                    .collect();
                self.clipboard_components = self
                    .doc
                    .selected_components
                    .iter()
                    .filter_map(|id| self.doc.components.get(id).cloned())
                    .collect();
                let text = serde_json::to_string(&self.clipboard).expect("paths serialize");
                ctx.copy_text(text);
                if id == "cut"
                    && let Err(e) = self.apply("delete", &serde_json::json!({}))
                {
                    self.status.set_message(&e);
                }
            }
            // pasted where they were copied from, in this job or another (Paste in Place)
            "paste" | "paste_in_place" => {
                let mut c = Change::builder(&self.doc);
                let ids: Vec<_> = self.clipboard.iter().map(|p| c.add(p.clone())).collect();
                c.select(ids);
                for k in &self.clipboard_components {
                    c.add_component(k.clone());
                }
                self.record(Ok(c.finish()), "Pasted");
            }
            "increment_and_save" => match self.session.path.clone() {
                Some(path) => self.save_to(&files::incremented(&path, std::path::Path::exists)),
                None => self.file_action("save_as"),
            },
            // ponytail: the drawing as a PDF opened in the system viewer, which prints it;
            // Aspire's own print window (scale, tiling) is not done
            "print" | "print_preview" => {
                let path = std::env::temp_dir().join("cut-craft-print.pdf");
                let opened = files::export(&self.registry, "export_pdf", &self.doc, &path)
                    .and_then(|()| {
                        on_the_desktop();
                        files::show_folder(&path).map_err(|e| e.to_string())
                    });
                if let Err(e) = opened {
                    self.status.set_message(&format!("could not print: {e}"));
                }
            }
            "open_application_data_folder" => {
                let opened = files::config_dir()
                    .ok_or_else(|| "no home folder".to_owned())
                    .and_then(|dir| {
                        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                        on_the_desktop();
                        files::show_folder(&dir).map_err(|e| e.to_string())
                    });
                if let Err(e) = opened {
                    self.status
                        .set_message(&format!("could not open the folder: {e}"));
                }
            }
            _ => {}
        }
    }

    /// The Help windows: About, Keyboard Shortcuts, Third Party Licenses.
    pub(super) fn info_window(&mut self, ctx: &egui::Context) {
        let Some(info) = self.info else { return };
        if let Info::Help(topic) = info {
            self.help_window(ctx, topic);
            return;
        }
        let mut open = true;
        let title = match info {
            Info::About => "About Cut Craft".to_owned(),
            Info::Shortcuts => "Keyboard Shortcuts".to_owned(),
            Info::Licenses => "Third Party Licenses".to_owned(),
            Info::Summary => "Toolpath Summary".to_owned(),
            Info::Help(_) => unreachable!("returned above"),
        };
        let shortcuts: Vec<(String, String)> = if info == Info::Shortcuts {
            let mut rows: Vec<(String, String)> = self
                .bindings(ctx)
                .into_iter()
                .map(|(id, keys)| {
                    let label = self
                        .registry
                        .get(&id)
                        .map_or_else(|| gui::title_case(&id), |c| c.meta().label.to_owned());
                    (ctx.format_shortcut(&keys), label)
                })
                .collect();
            rows.sort_by(|a, b| a.1.cmp(&b.1));
            rows
        } else {
            vec![]
        };
        let rows = self.summary_rows();
        let mut noted = None;
        let machine = self
            .machines
            .active()
            .map(|m| (m.name.clone(), m.acceleration));
        let summary = &mut self.summary;
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique(("info", &title)), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| match info {
            Info::Help(_) => unreachable!("returned above"),
            Info::Summary => noted = summary_table(ui, &rows, summary, units, machine.clone()),
            Info::About => {
                ui.heading("Cut Craft");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("2D and 3D design and CNC toolpaths.");
                ui.separator();
                ui.strong("Acknowledgements");
                ui.label(ACKNOWLEDGEMENT);
                ui.separator();
                ui.strong("Contributors on GitHub");
                for (login, commits, url) in contributors() {
                    ui.horizontal(|ui| {
                        ui.hyperlink_to(login, url);
                        ui.weak(format!("{commits} commits"));
                    });
                }
            }
            Info::Shortcuts => {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                            for (keys, label) in &shortcuts {
                                ui.monospace(keys);
                                ui.label(label);
                                ui.end_row();
                            }
                        });
                    });
            }
            Info::Licenses => {
                egui::Grid::new("licenses").striped(true).show(ui, |ui| {
                    for (name, licence) in THIRD_PARTY {
                        ui.label(name);
                        ui.label(licence);
                        ui.end_row();
                    }
                });
            }
        });
        // Apply under the summary's notes keeps them with their toolpath
        if let Some((id, notes)) = noted
            && let Some(t) = self.doc.toolpaths.get(&id)
        {
            let name = t.name.clone();
            let params = serde_json::json!({
                "toolpath": id, "name": name, "visible": t.visible, "notes": notes
            });
            match self.apply("toolpath_properties", &params) {
                Ok(()) => self.status.set_message(&format!("Notes kept with {name}")),
                Err(e) => self.status.set_message(&e),
            }
        }
        if !open {
            self.info = None;
        }
    }
}

/// The About window's thanks to the program that inspired Cut Craft (ACKNOWLEDGEMENTS.md in full).
pub(super) const ACKNOWLEDGEMENT: &str = "Thank you to Vectric Ltd. Cut Craft was inspired by Vectric Aspire, \
whose tools and workflow showed what a hobby CNC program can be. Cut Craft is an independent \
program written from scratch in Rust; it is not affiliated with or endorsed by Vectric and uses \
none of Vectric's code. Vectric and Aspire are trademarks of their owners.";

/// The GitHub contributors built in: login, commits and profile, most commits first.
pub(super) fn contributors() -> Vec<(&'static str, &'static str, &'static str)> {
    CONTRIBUTORS
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some((fields.next()?, fields.next()?, fields.next()?))
        })
        .filter(|(login, ..)| !login.is_empty())
        .collect()
}

/// Texture Toolpath's Load… and Save…; whether each was clicked.
fn texture_buttons(ui: &mut egui::Ui) -> (bool, bool) {
    let mut clicked = (false, false);
    ui.horizontal(|ui| {
        clicked.0 = ui
            .button("Load Texture...")
            .on_hover_hint("A texture saved before, from the textures folder or anywhere")
            .clicked();
        clicked.1 = ui
            .button("Save Texture...")
            .on_hover_hint("Keep these texture settings to use again")
            .clicked();
    });
    clicked
}

/// The open form's fields, and Import 3D Model's alignment and Update Undercuts; true when
/// anything changed.
fn show_form(
    ui: &mut egui::Ui,
    open: &mut OpenForm,
    look: &form::Look,
    job: &cut_craft_core::document::Job,
    icon: &dyn Fn(&str) -> Option<egui::Image<'static>>,
) -> bool {
    let changed = form::show(ui, &open.fields, &mut open.params, look);
    changed | align_in_material(ui, open, job, icon) | update_undercuts(ui, open)
}

/// Import 3D Model's alignment, as Align Objects' in the material: the model against the
/// material's sides or centred on it, and in height on the zero plane, in the middle of the
/// material or at its top. The position numbers are set to match; true when clicked.
fn align_in_material(
    ui: &mut egui::Ui,
    open: &mut OpenForm,
    job: &cut_craft_core::document::Job,
    icon: &dyn Fn(&str) -> Option<egui::Image<'static>>,
) -> bool {
    /// Each button: its icon, hint, and how far across and up the room it puts the model.
    type Align = (&'static str, &'static str, Option<f64>, Option<f64>);
    const ACROSS: [Align; 7] = [
        ("left", "Left side on the material's left", Some(0.0), None),
        (
            "center_horizontally_in_material",
            "Centred left to right",
            Some(0.5),
            None,
        ),
        (
            "right",
            "Right side on the material's right",
            Some(1.0),
            None,
        ),
        ("top", "Top side on the material's top", None, Some(1.0)),
        (
            "center_vertically_in_material",
            "Centred top to bottom",
            None,
            Some(0.5),
        ),
        (
            "bottom",
            "Bottom side on the material's bottom",
            None,
            Some(0.0),
        ),
        (
            "center_in_material",
            "Centred on the material",
            Some(0.5),
            Some(0.5),
        ),
    ];
    const HEIGHT: [(&str, &str, f64); 3] = [
        ("Bottom", "The model's bottom on the zero plane", 0.0),
        (
            "Middle",
            "The model in the middle of the material's thickness",
            0.5,
        ),
        ("Top", "The model's top at the material's top", 1.0),
    ];
    if open.id != "orientate_3d_model" {
        return false;
    }
    let size = |key: &str| open.params["size"][key].as_f64().unwrap_or(0.0);
    let (sx, sy, tall) = (size("x"), size("y"), size("z"));
    let area = job.area();
    let round = |v: f64| (v * 100.0).round() / 100.0;
    let mut clicked = false;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Align in Material");
        let edge = ui.spacing().interact_size.y;
        ui.horizontal_wrapped(|ui| {
            for (name, hint, ax, ay) in ACROSS {
                let path = format!("drawing/align/{name}.png");
                let button = match icon(&path) {
                    Some(image) => {
                        egui::Button::image(image.fit_to_exact_size(egui::vec2(edge, edge)))
                    }
                    None => egui::Button::new(name),
                };
                if !ui.add(button).on_hover_hint(hint).clicked() {
                    continue;
                }
                let placing = &mut open.params["placing"];
                placing["center"] = Value::Bool(false);
                if let Some(ax) = ax {
                    let x = area.min.x + (area.size().x - sx) * ax;
                    placing["position"]["x"] = Value::from(round(x));
                }
                if let Some(ay) = ay {
                    let y = area.min.y + (area.size().y - sy) * ay;
                    placing["position"]["y"] = Value::from(round(y));
                }
                clicked = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Height");
            for (label, hint, up) in HEIGHT {
                if ui.button(label).on_hover_hint(hint).clicked() {
                    // Depth Below Top is the model's top above the zero plane
                    let depth = tall + (job.thickness - tall) * up;
                    open.params["placing"]["depth_below_top"] = Value::from(round(depth));
                    clicked = true;
                }
            }
        });
    });
    clicked
}

/// Import 3D Model's Update Undercuts: the model is judged again (it already follows every
/// change, so this is the same judging once more); true when clicked.
fn update_undercuts(ui: &mut egui::Ui, open: &OpenForm) -> bool {
    if open.id != "orientate_3d_model" {
        return false;
    }
    let on = open.params["show_undercuts"].as_bool().unwrap_or(false);
    ui.add_enabled(on, egui::Button::new("Update Undercuts"))
        .on_hover_hint("Judge again what a cutter from above cannot reach")
        .on_disabled_hover_text("Tick Highlight undercuts first")
        .clicked()
}

/// What the user is told before an unsigned gadget first runs.
pub(super) fn unsigned_warning(label: &str) -> String {
    format!(
        "\u{201c}{label}\u{201d} is not part of Cut Craft. It came from your My Gadgets folder, \
         not from a signed Cut Craft build, so nobody has checked what it does.\n\n\
         Cut Craft and its author are not responsible for what this gadget does. It can only \
         change the job that is open, and Undo takes back everything it did in one step. \
         Check its result before you cut.\n\nRun it?"
    )
}

/// Asks, in a system dialog, whether to run the unsigned gadget `label`.
fn ask_to_run_unsigned(label: &str) -> bool {
    let answer = super::message_dialog()
        .set_title("Unsigned gadget")
        .set_description(unsigned_warning(label))
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            "Run Gadget".into(),
            "Cancel".into(),
        ))
        .show();
    matches!(answer, rfd::MessageDialogResult::Custom(c) if c == "Run Gadget")
}

/// What Update Post-Processor Database says once it has looked through the posts.
pub(super) fn post_database_message(entries: &[cut_craft_machines::Entry]) -> String {
    let custom = entries.iter().filter(|e| e.is_custom()).count();
    let broken = entries.iter().filter(|e| e.broken.is_some()).count();
    let unreadable = if broken > 0 {
        format!("; {broken} could not be read: Post-Processor Management shows why")
    } else {
        String::new()
    };
    format!(
        "{} post-processors ({custom} of your own){unreadable}; checking for a newer Cut \
         Craft, which brings its newest post-processors",
        entries.len()
    )
}
