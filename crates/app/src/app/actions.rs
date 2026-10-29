//! What the window does: every action's dispatch, files and templates, forms, shortcuts,
//! menus, the start page and the Design and Toolpaths pages.

use super::{
    APP_NAME, AUTOSAVE_SECONDS, Align, BUILT_INS, Change, CutCraftApp, DESIGN_PANEL_WIDTHS,
    DESIGN_TABS, Document, EXPORT_LEVEL, EntityId, History, Icons, Info, JOB_EXTENSION, Key,
    KeyboardShortcut, LAYOUT_KEYS, LINKS, LIVE_FORMS, LOGO, Layout, MODEL_IMPORTERS, Menus,
    Modifiers, OpenForm, PLAYBACK_SPEED, Page, Pages, Point, RECENT_FILE_ICON,
    SECTION_PICTURE_ROWS, SELECT_LEVEL, SPLASH_SHARE, STARTUP_TASKS, SUMMARY_DEFAULTS, Screen,
    Session, Shown, TEMPLATE, THIRD_PARTY, TOOL_ICON_ROWS, TOP_BAR_TOGGLES, Tool, ToolDatabase,
    VECTOR_EXPORTERS, VECTOR_IMPORTERS, Value, View, WHATS_NEW, commands, dialog_size, egui,
    file_dialog, file_name, files, form, gui, job_dialog, menu_items, message_dialog,
    on_the_desktop, rest_section, shortcut, side_tabs, summary_table, template_dialog,
    toolpath_template_dialog, view3d,
};
use crate::options::{CuttingChecks, IntersectionCheck};

impl CutCraftApp {
    pub(super) fn new() -> Self {
        let menus = gui::MenuSystem::new();
        Self {
            screen: Screen::Start,
            start_menus: menus.without_job(),
            doc: Document::default(),
            history: History::default(),
            registry: commands::registry(),
            icons: Icons::default_set(),
            menus,
            toolbar: gui::Toolbar::new(),
            status: gui::StatusBar::new(),
            design_tab: 0,
            view: View::TwoD,
            cursor: None,
            form: None,
            view_transform: None,
            view_area: None,
            camera: None,
            view_tool: None,
            pages: Pages::default(),
            search: None,
            inspector_tab: 0,
            rail_group: None,
            rail_pinned: false,
            ribbon_tab: 0,
            options_path: None,
            session: Session::default(),
            selected_toolpath: None,
            machines: cut_craft_machines::Machines::default(),
            machines_path: None,
            posts: cut_craft_machines::Library::new(None),
            machine_window: None,
            post_window: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tool_manager: None,
            shots: None,
            reporter: cut_craft_bug_report::Reporter::new(
                super::BUG_REPO,
                files::config_dir().map(|d| d.join("bug-reports")),
            ),
            clipboard: vec![],
            info: None,
            help_cache: egui_commonmark::CommonMarkCache::default(),
            summary: SUMMARY_DEFAULTS,
            stroke: Vec::new(),
            snap: cut_craft_core::snap::SnapOptions::default(),
            snap_window: false,
            display: crate::display::Display::default(),
            simulation: None,
            simulation_coarse: None,
            fine_3d: None,
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
            split: None,
            clipart: crate::clipart::Clipart::default(),
            recent_path: None,
            recovery_path: None,
            autosaved: (0, 0.0),
            autosave: None,
            title: String::new(),
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
            id if id.starts_with("open_recent:") => true,
            id if id.starts_with(gui::REIMPORT_MODEL) => self
                .model_source(id)
                .is_some_and(|s| s.find(self.job_folder()).is_some()),
            id if id.starts_with(SELECT_LEVEL) => true,
            "add_clipart_folder" => true,
            id if TOP_BAR_TOGGLES.contains(&id) => true,
            id if id.starts_with(EXPORT_LEVEL) => {
                self.registry.exporter("export_as_3d_clipart").is_some()
            }
            "redo" => self.history.can_redo(),
            "recalculate_toolpath"
            | "delete_toolpath"
            | "preview_toolpath"
            | "save_selected_toolpath_as_template"
            | "edit_toolpath" => self.selected_toolpath.is_some(),
            "edit_text" => self.doc.selection.iter().any(|id| {
                self.doc
                    .vector_groups
                    .get(id)
                    .is_some_and(|g| self.doc.texts.contains_key(g))
            }),
            "save_all_visible_toolpaths_as_template" => {
                self.doc.toolpaths.values().any(|t| t.visible)
            }
            "recalculate_all_toolpaths" | "preview_all_toolpaths" | "save_toolpaths" => {
                !self.doc.toolpaths.is_empty()
            }
            "recalculate_visible_toolpaths" => self.doc.toolpaths.values().any(|t| t.visible),
            "reset_preview" => self.simulation.is_some() || self.playback.is_some(),
            id if self.display.checked(id).is_some() => true,
            id if self.resolution_checked(id).is_some() => self.screen == Screen::Job,
            id if self.layout_checked(id).is_some() => true,
            "cut" | "copy" => !self.doc.selection.is_empty(),
            "create_component_from_toolpath_preview" => self.simulation.is_some(),
            "paste" | "paste_in_place" => !self.clipboard.is_empty(),
            "increment_and_save" | "find_tool" => self.screen == Screen::Job,
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
                self.tool_manager = Some(manager);
            }
            id if self.display.toggle(id) => ctx.request_repaint(),
            id if self.resolution_checked(id).is_some() => self.set_resolution(id),
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
                }
            }
            "preview_all_toolpaths" => self.preview_all(),
            "reset_preview" => {
                (self.simulation, self.simulation_coarse, self.fine_3d) = (None, None, None);
                (self.playback, self.playing) = (None, false);
            }
            id if id.starts_with("open_recent:") => {
                if let Ok(index) = id["open_recent:".len()..].parse() {
                    self.open_recent(index);
                }
            }
            id if id.starts_with(SELECT_LEVEL) || id.starts_with(EXPORT_LEVEL) => {
                self.level_action(id);
            }
            id if self.registry.exporter(id).is_some() => self.export(id),
            id if id.starts_with(gui::REIMPORT_MODEL) => self.reimport_model(id),
            _ => self.start_command(id),
        }
    }

    /// The folder the job is saved in, once it is.
    pub(super) fn job_folder(&self) -> Option<&std::path::Path> {
        self.session
            .path
            .as_deref()
            .and_then(std::path::Path::parent)
    }

    /// The component a Re-import Model action names, and where it came from.
    fn model_source(&self, action: &str) -> Option<&cut_craft_core::component::Source> {
        let id = action.strip_prefix(gui::REIMPORT_MODEL)?.parse().ok()?;
        self.doc.components.get(&EntityId(id))?.source.as_ref()
    }

    /// Orientate 3D Model again on a model's own file, with the settings it was imported
    /// with, its Apply replacing the model: only while the file is where it was and
    /// unchanged (its MD5 the same).
    fn reimport_model(&mut self, action: &str) {
        let Some(source) = self.model_source(action).cloned() else {
            return;
        };
        let path = match source.verified(self.job_folder()) {
            Ok(path) => path,
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
        if let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") {
            form.params = source.settings;
            form.params["file"] = Value::from(path.display().to_string());
            form.drawn = id;
            self.apply_live();
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
            "add_clipart_folder" => {
                if let Some(folder) = file_dialog().pick_folder()
                    && let Err(e) = self.clipart.add(folder)
                {
                    self.status.set_message(&e);
                }
            }
            "segment_3d_model_file" => {
                // the segmenting form, on an STL picked first
                if let Some(path) = file_dialog().add_filter("STL", &["stl"]).pick_file() {
                    self.start_command("segment_3d_model");
                    if let Some(form) = self.form.as_mut().filter(|f| f.id == "segment_3d_model") {
                        form.params["file"] = Value::from(path.display().to_string());
                    }
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

    pub(super) fn toolpath_action(&mut self, id: &str) {
        match id {
            "recalculate_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    let result = self.registry.recalculate(&self.doc, id);
                    self.record(result, "Toolpath recalculated");
                }
            }
            // the cascade: Recalculate on each shown toolpath
            "recalculate_visible_toolpaths" => {
                let shown = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
                let ids: Vec<EntityId> = shown.map(|(&id, _)| id).collect();
                let result =
                    Change::chain_over(&self.doc, &ids, |d, &id| self.registry.recalculate(d, id));
                self.record(result, "Visible toolpaths recalculated");
            }
            "recalculate_all_toolpaths" => {
                let result = self.registry.recalculate_all(&self.doc);
                self.record(result, "All toolpaths recalculated");
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
                let post = match self.output_post() {
                    Ok(post) => post,
                    Err(e) => return self.status.set_message(&e),
                };
                let problems: Vec<String> = self
                    .doc
                    .toolpaths
                    .values()
                    .flat_map(|t| cut_craft_core::checks::problems(&self.doc, t))
                    .collect();
                let ask = self.options.confirmations.cutting_checks == CuttingChecks::Ask;
                if !problems.is_empty() && ask && !Self::save_anyway(&problems) {
                    return self.status.set_message("Toolpaths not saved");
                }
                let thinned = self.options.toolpaths.thinned(&self.doc);
                let extension = post.extension.clone();
                let dialog = file_dialog().add_filter(&post.name, &[extension.as_str()]);
                if let Some(path) = dialog.save_file() {
                    let path = path.with_extension(&extension);
                    let job = self.session.path.as_deref();
                    let result = match self.doc.tiling {
                        // one file per tile, each tile moved to the job's corner
                        Some(tiling) => {
                            let tiles = tiling.tiles(self.doc.job.area());
                            tiles.iter().enumerate().try_for_each(|(n, &tile)| {
                                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                                let named = path
                                    .with_file_name(format!("{stem}_tile{}.{extension}", n + 1));
                                files::save_through(
                                    &post,
                                    &self.slots(),
                                    &tiling.tiled(&thinned, tile),
                                    &named,
                                    job,
                                )
                            })
                        }
                        None => files::save_through(&post, &self.slots(), &thinned, &path, job),
                    };
                    let told = if problems.is_empty() || ask {
                        String::new()
                    } else {
                        format!(" (check before cutting: {})", problems.join("; "))
                    };
                    self.status.set_message(&result.map_or_else(
                        |e| e,
                        |()| format!("Saved toolpaths to {}{told}", path.display()),
                    ));
                }
            }
            _ => unreachable!("not a toolpath action: {id}"),
        }
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
        let (meta, fields, params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
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
                model: None,
                applied: None,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
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

    /// Opens a job file in place of whatever is open (already confirmed); whether it opened.
    pub(super) fn open_path(&mut self, path: &std::path::Path) -> bool {
        let mut session = Session::default();
        match session.open(path) {
            Ok((doc, history)) => {
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
                false
            }
        }
    }

    /// Opens one of the recent files; one that no longer opens leaves the list.
    pub(super) fn open_recent(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard() && !self.open_path(&path) {
            self.recent.retain(|p| p != &path);
            self.save_recent();
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

    /// Once a frame: unsaved work copied to the recovery file every [`AUTOSAVE_SECONDS`],
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
            && now - self.autosaved.1 >= AUTOSAVE_SECONDS
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
        ) = (None, None, None, None);
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
            if cut_craft_core::keep_out::crossed(&self.doc.keep_out, &t.moves) {
                warnings.push(format!("{} passes over a keep-out zone", t.name));
            }
            warnings.extend(cut_craft_core::checks::problems(&self.doc, t));
        }
        if !warnings.is_empty() {
            self.status
                .set_message(&format!("Warning: {}", warnings.join("; ")));
        }
    }

    /// Asks whether to save toolpaths the checks before cutting found `problems` in.
    /// Crossing vectors chosen for a V-carve or pocket toolpath, or for an offset: what the
    /// Options' confirmation choices say to do. An error stops the command.
    fn crossings_allowed(&mut self, id: &str, params: &Value) -> Result<(), String> {
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
                    .on_hover_text(tip)
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
            _ => match self
                .clipart
                .chosen
                .or((!self.clipart.folders.is_empty()).then_some(0))
                .and_then(|i| self.clipart.folders.get(i))
            {
                Some(folder) => {
                    on_the_desktop();
                    files::show_folder(folder).map_err(|e| e.to_string())
                }
                None => Err("add a folder to the Clipart library first".to_owned()),
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
        if let Some(form) = self.form.as_mut().filter(|f| f.id == toolpath.strategy) {
            form.params = toolpath.params;
            form.editing = Some(id);
        }
    }

    /// A form's Apply: its command, or for an edited toolpath that toolpath made again with
    /// the form's settings, in its place.
    pub(super) fn apply_form(&mut self, id: &str, params: &Value) -> Result<(), String> {
        if let Some(toolpath) = self.form.as_ref().and_then(|f| f.editing) {
            let change = self.registry.edit(&self.doc, toolpath, Some(params));
            let failed = change.as_ref().err().map(ToString::to_string);
            self.record(change, "Toolpath edited");
            return failed.map_or(Ok(()), Err);
        }
        let before = self.entity_ids();
        let same = self.form.as_ref().and_then(|f| f.applied.as_ref()) == Some(params);
        match self.drawn_shape() {
            // the shape already follows these settings: nothing to do again
            Some(_) if same => return Ok(()),
            Some(old) => self.redraw(id, params, old)?,
            None => self.apply(id, params)?,
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
    /// changed and the pointer is let go (so a dragged value is not re-run every frame).
    pub(super) fn follow_form(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| i.pointer.any_down());
        let Some(form) = self.form.as_mut().filter(|f| f.dirty && !pressed) else {
            return;
        };
        form.dirty = false;
        if self.drawn_shape().is_some() {
            self.apply_live();
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
        let result = self.apply_form(id, &params);
        let model = serde_json::from_value(params)
            .ok()
            .filter(|_| id == "orientate_3d_model")
            .and_then(|p| {
                cmd_orientate_3d_model::preview(&self.doc, &p)
                    .ok()
                    .flatten()
            });
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
        if let Some(form) = &mut self.form {
            form.error = result.err();
            form.model = model;
            if let (true, Some(made)) = (sized, made) {
                // to the hundredth of a millimetre shown, not the cell's float noise
                let shown = std::array::from_fn(|i| {
                    form.typed_size[i].unwrap_or((made[i] * 100.0).round() / 100.0)
                });
                for (key, value) in SIZES.iter().zip(shown) {
                    form.params["size"][key] = Value::from(value);
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
        let (toolpaths, notes) = (change.toolpaths_made(), change.notes.clone());
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status
            .set_message(&format!("{} changed", cmd.meta().label));
        self.warn(&toolpaths, &notes);
        Ok(())
    }

    /// Runs a command and records it for undo.
    pub(super) fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        self.crossings_allowed(id, params)?;
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = cmd.run_json(&self.doc, params).map_err(|e| e.to_string())?;
        // rest machining, for any toolpath whose settings ask for it
        let change = cut_craft_core::rest::with_rest(&self.doc.job, params, change)
            .map_err(|e| e.to_string())?;
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
        // the spiral gadget says how many times round each strand goes, as Aspire's does
        if id == "wrapped_spiral_layout"
            && let Ok(p) = serde_json::from_value(params.clone())
            && let Some(turns) = cmd_wrapped_spiral_layout::revolutions(&self.doc, &p)
        {
            self.status
                .set_message(&format!("Each strand makes {turns:.2} revolutions"));
        }
        self.warn(&made, &notes);
        Ok(())
    }

    /// The arrow keys nudge the selection by the snapping grid's squares: a whole one, half
    /// with Shift, a tenth with Ctrl + Shift (Matthew's steps; Aspire nudges with Ctrl + Shift).
    pub(super) fn nudge(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Job || ctx.egui_wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        let share = match (shift, ctrl) {
            (true, true) => 0.1,
            (true, false) => 0.5,
            _ => 1.0,
        };
        let step = self.snap.grid_spacing * share;
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
        if by == Point::ZERO || self.doc.selection_bounds().is_none() {
            return;
        }
        let params =
            serde_json::json!({ "relative": true, "x": by.x, "y": by.y, "anchor": "bottom_left" });
        if let Err(e) = self.apply("move", &params) {
            self.status.set_message(&e);
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
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
        // single-letter shortcuts must not fire while typing in a form field
        if !ctx.egui_wants_keyboard_input() {
            let job = self.screen == Screen::Job;
            let fired: Vec<String> = self
                .bindings()
                .into_iter()
                .filter(|(id, _)| job || ["new", "open"].contains(&id.as_str()))
                .filter(|(_, keys)| ctx.input_mut(|i| i.consume_shortcut(keys)))
                .map(|(id, _)| id)
                .collect();
            for id in fired {
                self.invoke(ctx, &id);
            }
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

    /// Every key binding: each command's own shortcut, plus the built-ins as the menus spell them.
    pub(super) fn bindings(&self) -> Vec<(String, KeyboardShortcut)> {
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
        commands.chain(built_ins).chain(layouts).collect()
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
        egui::MenuBar::new().ui(ui, |ui| {
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
                                    .or_else(|| self.layout_checked(id))
                            },
                            shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                            recent: &recent,
                        },
                        &mut clicked,
                    );
                });
            }
            if !self.options.appearance.system_title_bar {
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
                .on_hover_text(path.display().to_string())
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
        egui::ScrollArea::vertical().show(ui, |ui| {
            // a tool's form takes the page's place, as Aspire's does
            if self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
                self.command_form(ui);
                return;
            }
            match DESIGN_TABS[self.design_tab] {
                "Design" => {
                    let groups = self.design_groups("Design");
                    self.tool_groups(ui, &groups, Page::Design);
                }
                "Sheets" => self.sheets_panel(ui),
                "Clipart" => self.clipart_panel(ui),
                "Layers" => self.layers_panel(ui),
                "Components" => self.component_list(ui),
                tab => {
                    ui.weak(format!("{tab} tools not implemented yet"));
                }
            }
        });
    }

    /// The tool groups a design tab shows: Design has the drawing then the modeling tools.
    pub(super) fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        match tab {
            "Design" => {
                let toolbar = &self.toolbar;
                toolbar
                    .groups
                    .iter()
                    .chain(&toolbar.modeling_groups)
                    .copied()
                    .collect()
            }
            _ => vec![],
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
                    .on_hover_text(if pinned {
                        "Unpin: hide this page to its side tab"
                    } else {
                        "Pin this page open"
                    })
                    .clicked();
                switch = ui
                    .add(self.icon_button("view/interface_layout.png", "", "Switch Layout", row))
                    .on_hover_text("Switch Layout (F11 designing, F12 machining)")
                    .clicked();
                help = ui.button("?").on_hover_text("Help (F1)").clicked();
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
        let width = DESIGN_PANEL_WIDTHS * style.spacing.interact_size.x;
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
            egui::Panel::left("design_panel")
                .default_size(width)
                .show(ui, |ui| self.design_page(ui));
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            egui::Panel::right("toolpaths_panel")
                .default_size(width)
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
        let (hovered, clicked) = side_tabs(ui, &DESIGN_TABS, shown.then_some(self.design_tab));
        // an auto-hidden page comes out on hover; a docked one changes tab on click
        let unpinned = !self.pages.pinned(Page::Design);
        if let Some(tab) = clicked.or(hovered.filter(|_| unpinned)) {
            self.set_design_tab(tab);
            self.pages.open(Page::Design);
        }
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
        let (hovered, clicked) = side_tabs(ui, &["Toolpaths"], shown.then_some(0));
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
        let new_job = open.new_job;
        let (apply_label, close_label) = if new_job {
            ("OK", "Cancel")
        } else {
            ("Apply", "Close")
        };
        let job_setup = open.id == "job_setup";
        let (mut apply, mut close, mut default) = (false, false, false);
        ui.horizontal(|ui| {
            apply = ui.button(apply_label).clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui.button(close_label).clicked();
                if job_setup {
                    default = ui
                        .button("Set as Default")
                        .on_hover_text("Start every new job with these settings")
                        .clicked();
                }
            });
        });
        if default {
            self.job_setup_default();
            return;
        }
        match (new_job, apply, close) {
            (true, true, _) => self.job_setup_ok(),
            (true, _, true) => self.job_setup_cancel(),
            (false, true, _) => {
                let (id, params) = (open.id, open.params.clone());
                let result = self.apply_form(id, &params);
                if let Some(open) = &mut self.form {
                    open.error = result.err();
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

    pub(super) fn command_form(&mut self, ui: &mut egui::Ui) {
        let form = self.form.as_ref().map_or("none", |f| f.label);
        cut_craft_bug_report::mark(ui, &format!("Tool form: {form}"));
        let slots = self.slots();
        let Some(open) = &mut self.form else { return };
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
        let (mut chosen, mut save_tool) = (None, false);
        if open.params.get("tool").is_some_and(Value::is_object) {
            let current = open.params["tool"]["name"].as_str().unwrap_or_default();
            ui.horizontal(|ui| {
                // the tools loaded in the machine's tool changer first, by slot
                let loaded: Vec<(usize, &str)> = slots
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| Some((i + 1, s.as_deref()?)))
                    .filter(|(_, t)| self.tools.get(t).is_some())
                    .collect();
                egui::ComboBox::from_label("Tool database")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for &(n, name) in &loaded {
                            let label = format!("T{n}  {name}");
                            if ui.selectable_label(name == current, label).clicked() {
                                chosen = Some(name.to_owned());
                            }
                        }
                        if !loaded.is_empty() {
                            ui.separator();
                        }
                        for tool in &self.tools.tools {
                            if loaded.iter().any(|&(_, t)| t == tool.name) {
                                continue;
                            }
                            if ui
                                .selectable_label(tool.name == current, &tool.name)
                                .clicked()
                            {
                                chosen = Some(tool.name.clone());
                            }
                        }
                    });
                save_tool = ui.button("Save to database").clicked();
            });
        }
        // a list of tools in running order (rest machining), picked from the database
        let mut earlier = None;
        if open.params.get("tools").is_some_and(Value::is_array) {
            egui::ComboBox::from_label("Add a tool from the database")
                .selected_text("")
                .show_ui(ui, |ui| {
                    for tool in &self.tools.tools {
                        if ui.selectable_label(false, &tool.name).clicked() {
                            earlier = Some(tool.name.clone());
                        }
                    }
                });
        }
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
        let look = form::Look {
            units,
            picture: &picture,
            material: form::material(&self.doc, units),
        };
        open.dirty |= form::show(ui, &open.fields, &mut open.params, &look);
        // every toolpath that cuts with a tool can rest machine (3D Finishing has its own, as
        // `after`); Material Setup, Materials and the toolpath list's tools have no tool
        let cuts = open.params.get("tool").is_some_and(Value::is_object);
        if cuts && open.params.get("after").is_none() {
            rest_section(ui, &mut open.params, &self.tools.tools);
        }
        if let Some(error) = &open.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        // Job Setup for a new job keeps its buttons pinned under the panel instead
        if !open.new_job {
            self.form_buttons(ui);
            ui.separator();
        }
        if let Some(name) = chosen {
            self.choose_tool(&name);
        }
        if save_tool {
            self.save_form_tool();
        }
        if let Some(name) = earlier {
            self.add_earlier_tool(&name);
        }
    }

    /// Adds a database tool (cutting the active material) to the end of the open form's
    /// tool list.
    pub(super) fn add_earlier_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.pick(name))
            && let Some(tools) = form.params.get_mut("tools").and_then(Value::as_array_mut)
        {
            tools.push(serde_json::to_value(tool).expect("tools serialize"));
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

    /// Stores the open form's tool in the database, replacing one of the same name.
    pub(super) fn save_form_tool(&mut self) {
        let tool = self
            .form
            .as_ref()
            .and_then(|f| serde_json::from_value::<Tool>(f.params["tool"].clone()).ok());
        match tool {
            Some(tool) => {
                let done = format!("Saved {} to the tool database", tool.name);
                self.tools.put_picked(tool);
                self.tools_changed(&done);
            }
            None => self
                .status
                .set_message("the form has no valid tool to save"),
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
                .bindings()
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
        let times: Vec<(String, f64, bool)> = self
            .doc
            .toolpaths
            .values()
            .map(|t| {
                (
                    t.name.clone(),
                    t.machining_minutes(self.summary.0) * self.summary.1,
                    t.visible,
                )
            })
            .collect();
        let summary = &mut self.summary;
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique(("info", &title)), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| match info {
            Info::Help(_) => unreachable!("returned above"),
            Info::Summary => summary_table(ui, &times, summary, units),
            Info::About => {
                ui.heading("Cut Craft");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("2D and 3D design and CNC toolpaths.");
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
        if !open {
            self.info = None;
        }
    }
}
