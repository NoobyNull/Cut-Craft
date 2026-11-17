//! What the window does: every action's dispatch, files and templates, forms, shortcuts,
//! menus, the start page and the Design and Toolpaths pages.

use super::{
    APP_NAME, Align, BUILT_INS, CONTRIBUTORS, Change, CutCraftApp, DESIGN_TABS, Dialog, Document,
    EXPORT_LEVEL, EntityId, History, Icons, Info, JOB_EXTENSION, KEY_ACTION_NAMES, Key,
    KeyboardShortcut, LAYOUT_KEYS, LINKS, LIVE_FORMS, Layout, Menus, Modifiers, OpenForm,
    PLAYBACK_SPEED, Page, Pages, Point, RECENT_FILE_ICON, REMAKE_FORMS, SECTION_PICTURE_ROWS,
    SELECT_LEVEL, SPLASH_LOGO, SPLASH_SHARE, STARTUP_TASKS, SaveChecks, SaveForm, Screen, Session,
    Shown, TEMPLATE, THIRD_PARTY, TOOL_ICON_ROWS, TOP_BAR_TOGGLES, ToolDatabase, VECTOR_EXPORTERS,
    Value, View, WHATS_NEW, brush_dialog, commands, commit, dialog_size, egui, file_dialog,
    file_name, files, filtered_dialog, form, form_settings_dialog, gui, job_dialog, menu_items,
    message_dialog, on_the_desktop, page_width, rest_section, saving_path, selector_section,
    shortcut, show_page, side_tabs, side_tabs_width, summary_table, template_dialog,
    texture_dialog, tool_picker, toolpath_template_dialog, view3d,
};
use crate::options::IntersectionCheck;
use cut_craft_bug_report::hint::HoverHint;

/// The buttons of "Save changes?": what each does, rather than Yes and No.
pub(super) const SAVE: &str = "Save";
pub(super) const DONT_SAVE: &str = "Don't Save";

impl CutCraftApp {
    #[expect(
        clippy::too_many_lines,
        reason = "one line per field of the window's state"
    )]
    pub(super) fn new() -> Self {
        let mut registry = commands::registry();
        let my_gadgets = crate::gadgets::my_gadgets_dir();
        let addon_gadgets =
            crate::depth::addons_dir().map_or_default(|dir| crate::gadgets::from_addons(&dir));
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
            design_unfold: None,
            toolpath_subtab: 0,
            toolpath_unfold: None,
            more_toolpaths: std::collections::BTreeSet::new(),
            toolpath_list_clicked: false,
            form_behind: None,
            ask_material: true,
            list_height: None,
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
            page_rects: [None, None],
            flyout_rect: None,
            buttons_across: std::cell::Cell::new(0.0),
            ribbon_tab: 0,
            options_path: None,
            words_path: None,
            job_times_path: None,
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
                job_sheet: false,
            },
            depth_estimate: None,
            busy: None,
            wait_before_showing: crate::busy::SHOW_AFTER,
            addons: crate::addons::Addons::new(None),
            machines: cut_craft_machines::Machines::default(),
            machines_path: None,
            machines_unsaved: false,
            posts: cut_craft_machines::Library::new(None),
            machine_window: None,
            first_run: None,
            post_window: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tools_unreadable: None,
            tool_manager: None,
            materials: cut_craft_core::material_library::MaterialLibrary::starter(),
            materials_path: None,
            materials_unreadable: None,
            materials_window: None,
            picking: None,
            deep_cut: None,
            units_ask: None,
            earlier_ask: None,
            gcode_view: None,
            save_anyway: None,
            checks_accepted: false,
            acknowledged: std::collections::BTreeSet::new(),
            save_after: None,
            form_back: None,
            shots: None,
            help_shots: None,
            space_mouse: None,
            unsigned_gadgets: gadgets
                .mine
                .iter()
                .map(|g| (g.id.clone(), g.label.clone()))
                .collect(),
            unsigned_gadgets_allowed: std::collections::HashSet::new(),
            unsigned_gadget_ok: ask_to_run_unsigned,
            reporter: {
                let mut reporter = cut_craft_bug_report::Reporter::new(
                    super::BUG_REPO,
                    files::config_dir().map(|d| d.join("bug-reports")),
                );
                // tests never reach the live service: each names its own stand-in
                reporter.crash_service = (!cfg!(test)).then(|| {
                    std::env::var(super::CRASH_SERVICE_VARIABLE)
                        .unwrap_or_else(|_| super::CRASH_SERVICE.to_owned())
                });
                reporter.retry_dir = files::config_dir();
                reporter
            },
            updates: crate::update::Updates::default(),
            restart_after_close: None,
            clipboard: vec![],
            clipboard_from: super::Copied::default(),
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
            help_search: String::new(),
            help_index: vec![],
            help_ranked: None,
            help_history: cut_craft_help::wiki::History::default(),
            tip: None,
            tips_cache: egui_commonmark::CommonMarkCache::default(),
            sketch_areas: None,
            carve_area: None,
            reach_found: None,
            summary: super::Summary::default(),
            stroke: Vec::new(),
            snap_window: false,
            snap_draft: None,
            snap_closing: false,
            job_sheet_window: false,
            part_labels_window: false,
            clearing_window: false,
            clearing_before: None,
            stl_window: false,
            stl_result: None,
            layers_on_sheet: false,
            save_form: None,
            machine_files: None,
            save_checks: vec![],
            before_preview: None,
            preview_side: false,
            picked_nodes: Vec::new(),
            node_mirror: [false, false],
            menu_at: None,
            guide_props: None,
            tool_changer_window: false,
            fit_import: None,
            measure_rows: Vec::new(),
            display: crate::display::Display::default(),
            simulation: None,
            simulation_coarse: None,
            import_level: None,
            polyline: crate::polyline::Draft::default(),
            placed: Vec::new(),
            picked_constraint: None,
            fine_3d: None,
            rough_3d: None,
            model_tall: std::cell::Cell::new(None),
            notes_ui: super::notes_panel::NotesUi::default(),
            note_vars: std::cell::RefCell::new(None),
            cut_times: std::cell::RefCell::new(None),
            height_applied_at: None,
            job_epoch: 0,
            toolpath_keys: super::follow::Keys::default(),
            preview_marks: std::collections::BTreeMap::new(),
            tool_note: None,
            collisions: std::collections::BTreeMap::new(),
            stale_known: 0,
            stale_while_form: false,
            recalculate_ask: None,
            drawn_on: None,
            model_3d: None,
            gpu_3d: None,
            surface_gpu: None,
            solid_gpu: None,
            model_gpu: None,
            ghost_gpu: None,
            trace_view: None,
            moved_3d: 0.0,
            spinning_3d: None,
            picked_outline: None,
            pacer: crate::frame_rate::Pacer::default(),
            detail_3d: (crate::options::Detail::default(), false),
            paced_pass: u64::MAX,
            orbit: view3d::Orbit::default(),
            framing: view3d::Framing::default(),
            view_cube: None,
            recent: vec![],
            playback: None,
            playing: false,
            playback_goal: None,
            kept_playback: None,
            playback_revision: 0,
            close_up: None,
            playback_view: None,
            playback_gpu: None,
            playback_builds: 0,
            playback_shown: None,
            playback_redraw: super::PLAYBACK_REDRAW,
            playback_budget: super::PLAYBACK_BUDGET,
            playback_speed: PLAYBACK_SPEED,
            collapsed_levels: std::collections::HashSet::new(),
            guide_drag: None,
            rotating: false,
            options: crate::options::Options::default(),
            options_window: false,
            options_draft: None,
            options_closing: false,
            options_page: 0,
            show_advanced: false,
            web_api: super::web_api::WebApi::default(),
            options_goto: None,
            capture: None,
            shortcut_search: String::new(),
            split: None,
            clipart: crate::clipart::Clipart::default(),
            import_queue: std::collections::VecDeque::new(),
            import_after_setup: None,
            recent_path: None,
            recovery_path: None,
            recovery_lock: None,
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
            id if id.starts_with(gui::SUBTAB_PLACE)
                || id.starts_with(gui::TOOLPATH_SUBTAB_PLACE)
                || id.starts_with(gui::FOLD_TOOL_GROUPS) =>
            {
                true
            }
            "add_clipart_folder" | "tool_help" | "view_2d" | "view_3d" => true,
            id if id.starts_with(gui::COMMUNITY) => true,
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
            "job_sheet_editor" => true,
            "reset_preview" => {
                self.simulation.is_some() || self.playback.is_some() || self.kept_playback.is_some()
            }
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

    /// Why `id` is greyed out, shown on hover: what it waits for (a selection, a toolpath
    /// picked, something to undo, a job open). "Not implemented yet" only for an action
    /// nothing runs.
    pub(super) fn unavailable_why(&self, id: &str) -> String {
        const PICK: &str = "Pick a toolpath in the Toolpath List first";
        const SHOWN: &str = "No toolpath is shown: tick one in the Toolpath List first";
        const PREVIEW: &str = "Preview a toolpath first";
        let job_first = "Open or start a job first";
        let why = match id {
            "undo" => "Nothing to undo yet",
            "redo" => "Nothing to redo: undo something first",
            "close" => "No job is open",
            "zoom_selected" | "activate_layer_of_selection" => "Select something first",
            "cut" => "Select the vectors or 3D parts to cut first",
            "copy" => "Select the vectors or 3D parts to copy first",
            "paste" | "paste_in_place" => "Nothing copied yet: copy something first",
            "recalculate_toolpath"
            | "delete_toolpath"
            | "save_selected_toolpath_as_template"
            | "edit_toolpath" => PICK,
            "preview_toolpath" => "Pick a toolpath or a group in the Toolpath List first",
            "edit_text" => "Select a piece of text first",
            "save_all_visible_toolpaths_as_template"
            | "recalculate_visible_toolpaths"
            | "preview_visible_toolpaths" => SHOWN,
            "recalculate_all_toolpaths" | "preview_all_toolpaths" | "save_toolpaths" => {
                "No toolpaths yet: make one first"
            }
            "undo_last_preview" => "No preview to undo",
            "save_shaded_image" => "Preview a toolpath or bring in a 3D model first",
            "edit_object" | "object_properties" => "Select a shape first",
            "reset_preview" | "create_component_from_toolpath_preview" => PREVIEW,
            id if id.starts_with(gui::REIMPORT_MODEL) => {
                "The model's file is not in the job's folder any more"
            }
            id if id.starts_with(EXPORT_LEVEL) => "Export as 3D clipart is not available",
            id if self.registry.get(id).is_some()
                || self.registry.exporter(id).is_some()
                || crate::flyout::variant(id).is_some()
                || gui::command_action(id).is_some()
                || BUILT_INS.contains(&id)
                || self.resolution_checked(id).is_some()
                || self.units_checked(id).is_some() =>
            {
                job_first
            }
            _ => return format!("{}: not implemented yet", self.tool_label(id)),
        };
        why.to_owned()
    }

    /// Menu items, tool buttons and shortcuts all end up here: built-in actions first, then
    /// the command registry.
    #[expect(clippy::too_many_lines, reason = "the one dispatch of every action")]
    pub(super) fn invoke(&mut self, ctx: &egui::Context, id: &str) {
        // the log says which tool was used, so a crash's log shows what led up to it
        cut_craft_bug_report::logs::step(&format!("Tool: {} ({id})", self.tool_label(id)));
        self.show_subtab_of(id);
        // Delete with a snap mark picked lets that point go, rather than deleting vectors
        if id == "delete" && self.release_picked_snap() {
            return;
        }
        // Delete in Node Edit with nodes picked deletes those nodes, not the whole shape
        if id == "delete"
            && self.view_tool == Some(super::ViewTool::NodeEdit)
            && !self.picked_nodes.is_empty()
        {
            self.delete_picked_nodes();
            return;
        }
        // Delete just after picking a toolpath in the list deletes that toolpath
        if id == "delete"
            && self.toolpath_list_clicked
            && self.selected_toolpath.is_some()
            && self.pages.shown(Page::Toolpaths) != Shown::Hidden
        {
            self.invoke(ctx, "delete_toolpath");
            return;
        }
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
            // The four Create Shape icons: the form with that profile chosen
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
            // The three Join icons: Join / Close Vectors their way
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
            // the Library tab, where the clipart is
            "clipart_library" => {
                self.set_design_tab(
                    DESIGN_TABS
                        .iter()
                        .position(|&t| t == "Library")
                        .unwrap_or_default(),
                );
                self.pages.open(Page::Design);
            }
            // the Components tab, as its Component Levels icon shows
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
            "import_into_shape" => self.import_into_shape(),
            "machine_files" => match self.machines.active() {
                Some(machine) => {
                    let (name, send_to) = (machine.name.clone(), machine.send_to.clone());
                    self.open_machine_files(&name, &send_to);
                }
                None => self
                    .status
                    .set_message("No machine yet: add one in Machine ▸ Machine Configuration"),
            },
            "add_machine" => {
                self.machine_window =
                    Some(cut_craft_machines::MachineManager::adding(&self.machines));
            }
            "install_offline_package" => {
                let dialog = filtered_dialog(Dialog::MachinePackage, &self.registry);
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
                let manager = cut_craft_tool_database::Manager::new(&self.tools);
                self.open_tool_manager(manager);
            }
            // the labels' setting is kept with the options, so they stay as left
            "show_keep_out_zone_labels" => {
                self.display.toggle(id);
                self.options.toolpaths.keep_out_labels = self.display.keep_out_labels;
                self.save_options();
                ctx.request_repaint();
            }
            // hidden construction lines cannot stay picked: they are let go
            "show_construction_lines" => {
                self.display.toggle(id);
                self.doc.hide_construction = !self.display.construction;
                if !self.display.construction {
                    let kept: Vec<_> = (self.doc.selection.iter().copied())
                        .filter(|v| !self.doc.is_construction(*v))
                        .collect();
                    if kept.len() < self.doc.selection.len() {
                        let mut c = Change::builder(&self.doc);
                        c.select(kept);
                        self.record(Ok(c.finish()), "Construction lines hidden");
                    }
                }
                ctx.request_repaint();
            }
            "draw_vectors_in_3d_view" => {
                self.show_vectors_3d(!self.display.vectors_3d);
                ctx.request_repaint();
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
            | "print" => {
                self.clipboard_or_file_action(ctx, id);
            }
            "keyboard_shortcuts"
            | "third_party_licenses"
            | "toolpath_summary"
            | "about_cut_craft"
            | "help_contents"
            | "snap_options" => self.open_window(id),
            "options" => self.options_window = true,
            // F1: the open form's help page, else the contents
            "tool_help" => {
                let topic = self.form.as_ref().map_or("contents", |f| f.id);
                self.info = Some(Info::Help(topic));
            }
            "view_2d" => self.view = View::TwoD,
            "view_3d" => self.view = View::ThreeD,
            "first_run_setup" => self.open_first_run(),
            "did_you_know" => self.open_tips(self.options.next_tip),
            "try_sample_project" => self.open_sample(),
            id if id.starts_with(gui::COMMUNITY) => {
                let link = id[gui::COMMUNITY.len()..].parse::<usize>().ok();
                if let Some((_, url, _)) = link.and_then(|n| super::LINKS.get(n)) {
                    ctx.open_url(egui::OpenUrl::new_tab(*url));
                }
            }
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
            "clearing_settings" => self.clearing_window = true,
            "create_job_sheet" => self.create_job_sheet(),
            "print_job_sheet" => self.print_job_sheet(),
            "edit_object" => {
                if let Some(command) = self.made_to_edit() {
                    // the form opens on what it made, with its own settings
                    self.form = None;
                    self.start_command(command);
                } else if let Some((id, command, params)) = self.object_to_edit() {
                    self.edit_shape(id, command, params);
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
            "save_shaded_image" => {
                if let Some(path) = filtered_dialog(Dialog::SaveShadedImage, &self.registry)
                    .set_file_name("shaded.png")
                    .save_file()
                {
                    self.save_shaded_image(path);
                }
            }
            "preview_all_toolpaths" => self.preview_all(false),
            "preview_visible_toolpaths" => self.preview_all(true),
            "undo_last_preview" => {
                if let Some(super::BeforePreview(material, side)) = self.before_preview.take() {
                    self.simulation = material;
                    self.kept_playback = None;
                    (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
                    // made before Swap Sides: turned over to the side the job is on now
                    self.preview_side = side;
                    self.follow_side();
                    self.status.set_message("Undid the last preview");
                }
            }
            "reset_preview" => {
                // a playing preview stops first, keeping what it has cut, so Undo Last
                // Preview brings back what was showing
                self.finish_playback();
                self.before_preview = Some(super::BeforePreview(
                    self.simulation.take(),
                    self.preview_side,
                ));
                (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
                (self.playback, self.playing, self.playback_goal) = (None, false, None);
                self.kept_playback = None;
                self.status
                    .set_message("Preview reset: Undo Last Preview brings it back");
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
            id if id.starts_with(gui::FOLD_TOOL_GROUPS) => {
                self.options.general.fold_tool_groups = id.ends_with("true");
                self.save_options();
            }
            id if id.starts_with(gui::SUBTAB_PLACE) => {
                if let Some(sections) = gui::DESIGN.placed(&self.options.design_sections, id) {
                    self.options.design_sections = sections;
                    self.save_options();
                }
            }
            id if id.starts_with(gui::TOOLPATH_SUBTAB_PLACE) => {
                if let Some(sections) = gui::TOOLPATHS.placed(&self.options.toolpath_sections, id) {
                    self.options.toolpath_sections = sections;
                    self.save_options();
                }
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
                .set_message(&crate::messages::cannot_keep_words(path, &e));
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
            && let Some(path) = saving_path(&path, &["ccclip"])
            && let Err(e) = self.save_brush_to(id, &path)
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
    /// that has changed since is taken as it is now, with a warning.
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
            // where the model sits in the material is the job's now, not as it was then
            form.params["in_material"] =
                serde_json::to_value(self.doc.job.model_position).expect("a position serializes");
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
            // a new job starts in Job Setup
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
            "save_as" => self.save_as(),
            "import_vectors_bitmap" | "import_component_3d_model" => {
                let which = if id == "import_component_3d_model" {
                    Dialog::ImportModel
                } else {
                    Dialog::ImportVectorsOrPicture
                };
                if let Some(path) = filtered_dialog(which, &self.registry).pick_file() {
                    self.import_file(&path);
                }
            }
            "import_cut_list" => {
                if let Some(path) =
                    filtered_dialog(Dialog::ImportCutList, &self.registry).pick_file()
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
                if let Some(path) =
                    filtered_dialog(Dialog::SegmentModel, &self.registry).pick_file()
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
                if let Some(path) =
                    filtered_dialog(Dialog::ImportToolpaths, &self.registry).pick_file()
                {
                    self.import_toolpaths_from(&path);
                }
            }
            "install_post_processor" => {
                if let Some(path) =
                    filtered_dialog(Dialog::InstallPostProcessor, &self.registry).pick_file()
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
                    && let Some(path) = saving_path(&path, &[TEMPLATE])
                {
                    match cut_craft_core::file::write_atomic(
                        &path,
                        &cut_craft_core::file::save_job(&self.doc),
                    ) {
                        Ok(()) => self
                            .status
                            .set_message(&format!("Saved template {}", path.display())),
                        Err(e) => self
                            .status
                            .set_message(&crate::messages::could_not_save(&path, &e)),
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
                    && let Some(path) = saving_path(&path, &[files::TOOLPATH_TEMPLATE_EXTENSION])
                {
                    let template = files::toolpath_template(&self.doc, &ids);
                    let message = match cut_craft_core::file::write_atomic(&path, &template) {
                        Ok(()) => format!("Saved toolpath template {}", path.display()),
                        Err(e) => crate::messages::could_not_save(&path, &e),
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
    /// which they are given in), and whether Options allow clipping toolpaths at them; true
    /// when either changed.
    pub(super) fn sync_machine_keep_out(&mut self) -> bool {
        let zones: Vec<_> = self
            .machines
            .active()
            .map_or_default(|m| m.keep_out.as_slice())
            .iter()
            .enumerate()
            .filter(|(_, z)| z.width > 0.0 && z.height > 0.0)
            .map(|(i, z)| {
                cut_craft_core::keep_out::KeepOutZone::new(z.name_at(i), z.path())
                    .standing(z.stands)
            })
            .collect();
        let clip = self.options.toolpaths.clip_at_keep_out_zones;
        let changed = zones != self.doc.machine_keep_out || clip != self.doc.clip_keep_out;
        self.doc.machine_keep_out = zones;
        self.doc.clip_keep_out = clip;
        changed
    }

    /// The keep-out zones changed: every toolpath is made again round them
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

    /// The toolpaths of `plan` in `doc` that pass over a keep-out zone, by name.
    pub(super) fn not_kept_out(&self, doc: &Document, plan: &[Vec<EntityId>]) -> Vec<String> {
        plan.iter()
            .flatten()
            .filter_map(|id| doc.toolpaths.get(id))
            .filter(|t| !self.zones_entered(t).is_empty())
            .map(|t| t.name.clone())
            .collect()
    }

    /// The job with its toolpaths thinned to their tolerances, as they are saved; refused
    /// when a toolpath of `plan`, as saved, would cut into a keep-out zone.
    pub(super) fn thinned_for_saving(&self, plan: &[Vec<EntityId>]) -> Result<Document, String> {
        let thinned = self.options.toolpaths.thinned(&self.doc);
        // a toolpath that cannot abide by the keep-out zones is not saved; checked on the
        // moves saved, which the tolerance may have straightened
        let crossing = self.not_kept_out(&thinned, plan);
        if !crossing.is_empty() {
            let clippable = plan
                .iter()
                .flatten()
                .filter_map(|id| thinned.toolpaths.get(id))
                .any(|t| crossing.contains(&t.name) && can_clip(t));
            let way = if clippable {
                format!(", or turn on clipping ({CLIP_SETTING}) and recalculate")
            } else {
                " (laser and drill bank toolpaths are not clipped: move the part or the zone)"
                    .to_owned()
            };
            return Err(format!(
                "Not saved: {} would cut into a keep-out zone. Check the keep-out zones and \
                 their clearance are right{way}",
                crossing.join(", ")
            ));
        }
        Ok(thinned)
    }

    /// The names of the keep-out zones `toolpath` cuts into (its cutter's edge reaching
    /// one) or rapids over.
    pub(super) fn zones_entered(
        &self,
        toolpath: &cut_craft_core::toolpath::Toolpath,
    ) -> Vec<String> {
        let radius = toolpath.tool.diameter / 2.0;
        cut_craft_core::keep_out::entered(self.doc.named_keep_out(), &toolpath.moves, radius)
            .into_iter()
            .map(|z| format!("\"{}\"", z.name))
            .collect()
    }

    /// Why `change` may not be made: a toolpath it makes would cut into a keep-out zone,
    /// Options not allowing it to be clipped there (or it being a laser or drill bank
    /// toolpath, which are not clipped). A clamp is never cut by mistake.
    pub(super) fn keep_out_refusal(&self, change: &Change) -> Option<String> {
        let mut clippable = false;
        let refused: Vec<String> = change
            .toolpaths_making()
            .filter_map(|t| {
                let zones = self.zones_entered(t);
                clippable |= !zones.is_empty() && can_clip(t);
                (!zones.is_empty())
                    .then(|| format!("{} would cut into {}", t.name, zones.join(" and ")))
            })
            .collect();
        // clipping can't help a laser or drill bank toolpath: they are never clipped
        let way = if self.doc.clip_keep_out || !clippable {
            "Move the part or the zone (laser and drill bank toolpaths are not clipped)"
        } else {
            &format!(
                "Move the part or the zone, or let toolpaths be clipped there ({CLIP_SETTING})"
            )
        };
        (!refused.is_empty()).then(|| format!("{KEEP_OUT_REFUSED}: {}. {way}", refused.join("; ")))
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
                self.save_form = Some(self.default_save_form());
            }
            _ => unreachable!("not a toolpath action: {id}"),
        }
    }

    /// Save Toolpaths' form as it opens: every shown toolpath in one file through the
    /// machine's default post.
    pub(super) fn default_save_form(&self) -> SaveForm {
        SaveForm {
            mode: files::SaveMode::VisibleToOneFile,
            group: false,
            post: None,
            add_side: self.doc.job.double_sided.is_some(),
            changes: self.default_changes(),
            job_sheet: true,
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

    /// Save Toolpaths' Save: the checks before cutting, a file name (the dialog opening in
    /// the project's folder in the machine's G-code programs folder, when it has one), then
    /// each file of the form's plan through its post (and each tile, when the job is tiled),
    /// with the job sheet beside them when the form says so.
    pub(super) fn save_toolpaths(&mut self, form: &SaveForm) {
        let Some(post) = self.ready_to_save(form, false) else {
            return;
        };
        let extension = post.extension.clone();
        let which = Dialog::SaveToolpaths {
            post: &post.name,
            extension: &extension,
        };
        let mut dialog = filtered_dialog(which, &self.registry);
        let named = cut_craft_machines::send::safe_name(&self.output_name(form), false);
        dialog = dialog.set_file_name(format!("{named}.{extension}"));
        let (key, name) = self.project();
        let programs = self
            .machines
            .active_mut()
            .and_then(|m| m.send_to.programs_folder(&key, &name));
        if let Some(folder) = &programs {
            dialog = dialog.set_directory(folder);
            self.machines_changed("");
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = files::with_extension_added(&path, &extension);
        self.save_over_earlier(form, post, path);
    }

    /// Save Toolpaths' Save once the file is named: straight on when no earlier save left
    /// G-code there, else as Options say (Replace or Archive), or held for the question.
    pub(super) fn save_over_earlier(
        &mut self,
        form: &SaveForm,
        post: cut_craft_post::Post,
        path: std::path::PathBuf,
    ) {
        let earlier = files::earlier_save(&path);
        if earlier.is_empty() {
            self.finish_save(form, &post, &path, "");
            return;
        }
        match self.options.confirmations.earlier_gcode {
            crate::options::EarlierGcode::Ask => {
                self.earlier_ask = Some(super::EarlierAsk {
                    form: form.clone(),
                    post,
                    path,
                    earlier,
                    remember: false,
                    sending: false,
                });
            }
            choice => self.save_with(choice, form, &post, &path, &earlier),
        }
    }

    /// The answer to the earlier-save question: `None` saves nothing. `remember` keeps the
    /// answer as the default (Edit ▸ Options ▸ Confirmation Choices).
    pub(super) fn answer_earlier(
        &mut self,
        choice: Option<crate::options::EarlierGcode>,
        remember: bool,
    ) {
        let Some(ask) = self.earlier_ask.take() else {
            return;
        };
        let Some(choice) = choice else {
            return self.status.set_message("Toolpaths not saved");
        };
        if remember {
            self.options.confirmations.earlier_gcode = choice;
            self.save_options();
        }
        if ask.sending {
            return self.send_choosing(&ask.form, ask.post, choice);
        }
        self.save_with(choice, &ask.form, &ask.post, &ask.path, &ask.earlier);
    }

    /// Replaces (deletes) or archives `earlier`, then saves. They are moved aside first
    /// (into the archive, or a hidden folder for Replace) so the new files never mix with
    /// them, and put back if the new files cannot be written: a failed save never leaves
    /// the user with no G-code at all.
    fn save_with(
        &mut self,
        choice: crate::options::EarlierGcode,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
        earlier: &[std::path::PathBuf],
    ) {
        use crate::options::EarlierGcode;
        let archiving = choice == EarlierGcode::Archive;
        let moved = if archiving {
            files::archive(earlier, path, chrono::Local::now().naive_local())
        } else {
            files::set_aside(earlier, path)
        };
        let folder = match moved {
            Ok(folder) => folder,
            Err(e) => {
                let doing = if archiving { "archive" } else { "replace" };
                return self
                    .status
                    .set_message(&format!("Not saved: could not {doing} {e}"));
            }
        };
        let done = if archiving {
            format!("; the earlier files are in {}", folder.display())
        } else {
            String::new()
        };
        if self.finish_save(form, post, path, &done) {
            if !archiving {
                let _ = std::fs::remove_dir_all(&folder);
            }
            return;
        }
        let kept = match files::put_back(earlier, &folder, path) {
            Ok(()) => "; the earlier files were left as they were".to_owned(),
            Err(e) => format!(
                "; the earlier files are in {} (could not put back {e})",
                folder.display()
            ),
        };
        let message = format!("{}{kept}", self.status.message);
        self.status.set_message(&message);
    }

    /// Writes the files (and the job sheet) and says what was saved, then `after`; whether
    /// the G-code files were written.
    fn finish_save(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
        after: &str,
    ) -> bool {
        let (_, name) = self.project();
        let written = match self.write_toolpath_files(form, post, path) {
            Ok(written) => written,
            Err(e) => {
                self.status.set_message(&e);
                return false;
            }
        };
        let done = Self::saved_message(&written, path, self.doc.tiling.is_some());
        let result = if form.job_sheet {
            let folder = path.parent().unwrap_or(std::path::Path::new("."));
            let outputs = (written.iter())
                .map(|p| (file_name(p), p.is_file()))
                .collect();
            self.job_sheet_with(outputs, folder, &name)
                .map(|sheet| format!("{done}, with the job sheet {sheet}"))
        } else {
            Ok(done)
        };
        self.status
            .set_message(&result.map_or_else(|e| e, |done| format!("{done}{after}")));
        true
    }

    /// Save Toolpaths' post and the checks before cutting, asked about when they find
    /// something not ticked as read (and so may say no): the post; `None` when not saving,
    /// the status bar saying why.
    pub(super) fn ready_to_save(
        &mut self,
        form: &SaveForm,
        sending: bool,
    ) -> Option<cut_craft_post::Post> {
        let post = match self.form_post(form) {
            Ok(post) => post,
            Err(e) => {
                self.status.set_message(&e);
                return None;
            }
        };
        let plan = match self.plan(form, &post) {
            Ok(plan) => plan,
            Err(e) => {
                self.status.set_message(&e);
                return None;
            }
        };
        let Some(found) = self.save_checks(&plan, form.changes).map(<[_]>::to_vec) else {
            self.status.set_message(if self.checking() {
                "Still checking the toolpaths before cutting: save again once the checks finish"
            } else {
                "The checks before cutting were cancelled: press Check Again, then save"
            });
            return None;
        };
        if let Some(refused) = Self::refused(&found) {
            self.status.set_message(&refused);
            return None;
        }
        // every one ticked as read in Save Toolpaths: that was the asking, but for the
        // collet or spindle hitting the material, which is always asked about
        let ticked = self.all_acknowledged(&found);
        let must_ask = found
            .iter()
            .any(|f| f.severity == cut_craft_core::preflight::Severity::MustAsk);
        let problems: Vec<String> = found.into_iter().map(|f| f.message).collect();
        let ask = must_ask || !ticked;
        if !problems.is_empty() && ask && !std::mem::take(&mut self.checks_accepted) {
            // asked over the window: the answer saves (or sends) again
            self.ask_save_anyway(form, problems, sending);
            return None;
        }
        self.checks_accepted = false;
        Some(post)
    }

    /// The name the form's file is saved under (before its extension): a group's own name,
    ///, the selected toolpath's when saving only it, else the project's.
    pub(super) fn output_name(&self, form: &SaveForm) -> String {
        let selected = (form.mode == files::SaveMode::Selected)
            .then(|| {
                self.selected_group.clone().or_else(|| {
                    let id = self.selected_toolpath?;
                    Some(self.doc.toolpaths.get(&id)?.name.clone())
                })
            })
            .flatten();
        selected.unwrap_or_else(|| self.project().1)
    }

    /// The project, as its folders for G-code are kept: by its file ("" before it is saved)
    /// and named after it ("Untitled" before it is saved).
    pub(super) fn project(&self) -> (String, String) {
        self.session.path.as_deref().map_or_else(
            || (String::new(), "Untitled".to_owned()),
            |path| {
                let kept = crate::clipart::network::kept(path);
                let name = path
                    .file_stem()
                    .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned());
                (kept.display().to_string(), name)
            },
        )
    }

    /// The job sheet for the active sheet and side, its list of toolpath files `outputs`
    /// (each with whether it is there), saved in `folder` as `<name> job sheet.html`; its
    /// file name.
    pub(super) fn job_sheet_with(
        &mut self,
        outputs: Vec<(String, bool)>,
        folder: &std::path::Path,
        name: &str,
    ) -> Result<String, String> {
        let mut setup = self.job_setup();
        setup.outputs = outputs;
        let html = fmt_create_job_sheet::sheet_for(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| format!("Toolpaths saved, but not the job sheet: {e}"))?;
        let sheet =
            cut_craft_machines::send::safe_file_name(&format!("{name} job sheet.html"), false);
        let path = folder.join(&sheet);
        cut_craft_core::file::write_atomic(&path, html.as_bytes()).map_err(|e| {
            format!(
                "Toolpaths saved, but not the job sheet: could not write {}: {e}",
                path.display()
            )
        })?;
        Ok(sheet)
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

    /// The active machine has a tool changer (slots in Machine Configuration); with no
    /// machine set up there is none.
    pub(super) fn has_tool_changer(&self) -> bool {
        self.machines.active().is_some_and(|m| !m.slots.is_empty())
    }

    /// How tools are changed unless the form says otherwise: by the tool changer when the
    /// active machine has one, else a file for each tool.
    pub(super) fn default_changes(&self) -> files::ToolChanges {
        if self.has_tool_changer() {
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
        let changer = self.has_tool_changer();
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

    /// The checks before cutting for `plan` (Save Toolpaths', or the Analyse tab's), worked
    /// out again only when the plan, the job or the machine changed. They run off the
    /// window's thread, with their progress and Cancel over the window: a 3D finish of
    /// millions of moves takes a while to check, and the window must keep drawing. `None`
    /// while they run, or once they were cancelled ([`Self::check_again`] runs them again).
    pub(super) fn save_checks(
        &mut self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Option<&[cut_craft_core::preflight::Finding]> {
        let key = SaveChecks {
            plan: plan.to_vec(),
            changes,
            revision: self.history.revision(),
            machine: self.machines.active,
            found: None,
        };
        if !self.save_checks.iter().any(|c| c.same(&key)) {
            // the Analyse tab's and Save Toolpaths' can both be showing: keep one for each
            if self.save_checks.len() >= super::KEPT_CHECKS {
                self.save_checks.remove(0);
            }
            self.save_checks.push(key.clone());
            let doc = self.doc.clone();
            let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
            let limits = self.check_limits(&ids, changes);
            let started = key.clone();
            self.in_background(
                super::CHECKING,
                "check",
                move || {
                    let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> =
                        ids.iter().filter_map(|id| doc.toolpaths.get(id)).collect();
                    cut_craft_core::preflight::check(&doc, &toolpaths, &limits)
                },
                move |app, found| {
                    if let Some(checks) = app.save_checks.iter_mut().find(|c| c.same(&started)) {
                        checks.found = Some(found);
                    }
                },
            );
        }
        self.save_checks
            .iter()
            .find(|c| c.same(&key))?
            .found
            .as_deref()
    }

    /// Whether the checks before cutting are running now (else, when [`Self::save_checks`]
    /// has none, they were cancelled).
    pub(super) fn checking(&self) -> bool {
        self.busy
            .as_ref()
            .is_some_and(|b| b.what() == super::CHECKING)
    }

    /// Runs the cancelled checks before cutting again, next time they are asked for.
    pub(super) fn check_again(&mut self) {
        self.save_checks.retain(|c| c.found.is_some());
    }

    /// What the checks before cutting find in `plan`'s toolpaths, cut in its order on the
    /// active machine; its tool changer's slots are checked only when it changes the tools.
    #[cfg(test)]
    pub(super) fn preflight(
        &self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Vec<cut_craft_core::preflight::Finding> {
        let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
        let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> = ids
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        let limits = self.check_limits(&ids, changes);
        let mut found = cut_craft_core::preflight::check(&self.doc, &toolpaths, &limits);
        if changes == files::ToolChanges::ToolChanger && !self.has_tool_changer() {
            found.push(Self::no_tool_changer());
        }
        found
    }

    /// The warning when the tool changer is chosen but the machine has none set up.
    pub(super) fn no_tool_changer() -> cut_craft_core::preflight::Finding {
        cut_craft_core::preflight::Finding {
            severity: cut_craft_core::preflight::Severity::Warning,
            message: "Tool changer (ATC) is chosen, but the machine has no tool changer set \
                      up: the file changes tools with M6, which a machine without one stops \
                      on or ignores, cutting on with the wrong bit. Choose Separate file for \
                      each tool, or add the tool changer's slots in Machine Configuration"
                .into(),
            check: cut_craft_core::preflight::Check::ToolChanger,
        }
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
    #[cfg(test)]
    fn write_toolpaths(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        Ok(Self::saved_message(
            &self.write_toolpath_files(form, post, path)?,
            path,
            self.doc.tiling.is_some(),
        ))
    }

    /// What the status bar says once `written` are saved, beside `path`; `tiled` while
    /// Toolpath Tiling is on, which names the files by tile.
    pub(super) fn saved_message(
        written: &[std::path::PathBuf],
        path: &std::path::Path,
        tiled: bool,
    ) -> String {
        let saved = match written {
            [one] => format!("Saved toolpaths to {}", one.display()),
            _ => format!(
                "Saved {} toolpath files beside {}",
                written.len(),
                path.display()
            ),
        };
        if tiled {
            format!(
                "{saved}, one for each tile with something to cut: Toolpath Tiling is on \
                 (Toolpaths ▸ Toolpath Tiling to turn it off)"
            )
        } else {
            saved
        }
    }

    /// Writes the form's plan through `post` at `path` (and beside it); the files written,
    /// in the order they are run.
    pub(super) fn write_toolpath_files(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        let made = chrono::Local::now().naive_local();
        let texts = self.toolpath_texts(form, post, path, made)?;
        let mut written = vec![];
        for (path, text) in texts {
            files::write_text(&path, &text)?;
            written.push(path);
        }
        Ok(written)
    }

    /// The files the form's plan makes through `post` at `path` (and beside it), with the
    /// G-code each holds as made at `made`, in the order they are run: what Save Toolpaths
    /// writes and View G-code shows.
    pub(super) fn toolpath_texts(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
        made: chrono::NaiveDateTime,
    ) -> Result<Vec<(std::path::PathBuf, String)>, String> {
        let plan = self.plan(form, post)?;
        let thinned = self.thinned_for_saving(&plan)?;
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
        // the notes' variables are the whole job's, not one file's; the post is the one
        // these files are made through, whichever the machine has first
        let mut vars = (*self.note_variables()).clone();
        if let Some(v) = vars.iter_mut().find(|v| v.key == "machine.post") {
            v.value.clone_from(&post.name);
        }
        let notes = cut_craft_core::notes::gcode_lines(&self.doc.notes, &vars);
        let mut texts = vec![];
        for (path, part) in &parts {
            // one file per tile, each tile moved to the job's corner
            if let Some(tiling) = self.doc.tiling {
                let tiles = tiling.tiles(self.doc.job.area());
                for (n, &tile) in tiles.iter().enumerate() {
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                    let extension = path.extension().unwrap_or_default().to_string_lossy();
                    let named = path.with_file_name(format!("{stem}_tile{}.{extension}", n + 1));
                    let in_tile = tiling.tiled(part, tile);
                    // nothing to cut there: no file to change tools and start the spindle for
                    if in_tile.toolpaths.is_empty() {
                        continue;
                    }
                    let text =
                        files::render_through(post, &slots, &in_tile, &named, job, made, &notes)?;
                    texts.push((named, text));
                }
            } else {
                let text = files::render_through(post, &slots, part, path, job, made, &notes)?;
                texts.push((path.clone(), text));
            }
        }
        Ok(texts)
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
        let mut limits =
            self.machines
                .active()
                .map_or_default(|m| cut_craft_core::preflight::Limits {
                    name: m.name.clone(),
                    bed: [m.width, m.height],
                    z_travel: m.depth,
                    max_rate: m.max_rate,
                    spindle: m.spindle,
                    slots: m.slots.clone(),
                    left: None,
                    library: None,
                    holding: m.holding(),
                });
        limits.left = Some(self.options.confirmations.left());
        limits
    }

    /// [`Self::limits`] for checking the toolpaths `ids` saved with `changes`: the tool
    /// changer's slots only when it changes the tools, and the Tool Database's copy of
    /// each of their bits, to compare with.
    pub(super) fn check_limits(
        &self,
        ids: &[EntityId],
        changes: files::ToolChanges,
    ) -> cut_craft_core::preflight::Limits {
        let mut limits = self.limits();
        if changes != files::ToolChanges::ToolChanger {
            limits.slots.clear();
        }
        let mut library = std::collections::BTreeMap::new();
        for toolpath in ids.iter().filter_map(|id| self.doc.toolpaths.get(id)) {
            let name = &toolpath.tool.name;
            // a bit copied in from a maker's catalogue is compared with the catalogue's
            let catalogue =
                (self.tools.tools.iter()).find(|t| t.name == *name && self.tools.is_catalogue(t));
            if let Some(listed) = catalogue.or_else(|| self.tools.get(name)) {
                library.insert(name.clone(), listed.for_material(&self.tools.material));
            }
        }
        limits.library = Some(library);
        limits
    }

    /// The checks before cutting that cannot be made on `plan` saved with `changes`, each
    /// with why.
    pub(super) fn not_checked(
        &self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Vec<(cut_craft_core::preflight::Check, String)> {
        let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
        let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> = ids
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        cut_craft_core::preflight::not_checked(
            &self.doc,
            &toolpaths,
            &self.check_limits(&ids, changes),
        )
    }

    /// Whether every warning in `found` was ticked as read in the checks before cutting.
    pub(super) fn all_acknowledged(&self, found: &[cut_craft_core::preflight::Finding]) -> bool {
        found
            .iter()
            .filter(|f| f.severity != cut_craft_core::preflight::Severity::Error)
            .all(|f| self.acknowledged.contains(&f.message))
    }

    /// How the active machine holds its bits, for the collet and holder checks (the usual
    /// holder without a machine).
    pub(super) fn holding(&self) -> cut_craft_core::collision::Holding {
        self.machines
            .active()
            .map_or_default(cut_craft_machines::Machine::holding)
    }

    /// The active machine's tool changer slots (empty without one), for `[T]`.
    pub(super) fn slots(&self) -> Vec<Option<String>> {
        self.machines.active().map_or_default(|m| m.slots.clone())
    }

    /// Reads the machines and finds the custom posts, kept in the config folder.
    pub(super) fn load_machines(&mut self) {
        let dir = files::config_dir();
        if let Some(path) = dir.as_ref().map(|d| d.join("machines.json")) {
            self.load_machines_at(&path);
        }
        self.posts = cut_craft_machines::Library::new(dir.map(|d| d.join("posts")));
    }

    /// Reads the machines kept at `path`. One that cannot be read is said, copied aside to
    /// `machines.json.bad`, and never saved over this run: machine changes are not saved
    /// until it is put right or moved away and Cut Craft is started again.
    pub(super) fn load_machines_at(&mut self, path: &std::path::Path) {
        match cut_craft_machines::Machines::load(path) {
            Ok(machines) => {
                self.machines = machines;
                self.machines_path = Some(path.to_owned());
            }
            Err(e) => {
                log::warn!("machines not read: {e}");
                let aside = path.with_extension("json.bad");
                let kept = match std::fs::copy(path, &aside) {
                    Ok(_) => format!("a copy is kept as {}", aside.display()),
                    Err(_) => "it is left as it is".to_owned(),
                };
                self.status.set_message(&format!(
                    "Your machines could not be read ({e}); {kept}. Machine changes will not \
                     be saved until it is fixed or moved away and Cut Craft is restarted"
                ));
                self.machines = cut_craft_machines::Machines::default();
                self.machines_path = None;
                self.machines_unsaved = true;
            }
        }
    }

    /// The active machine's tool changer was loaded differently: the machines are saved,
    /// and each tool now in a slot takes the slot as its tool number.
    pub(super) fn tool_changer_changed(&mut self) {
        self.machines_changed("Tool changer changed");
        let slots = self.machines.active().map_or_default(|m| m.slots.clone());
        if self.tools.number_by_slots(&slots) {
            self.tools_changed("Tool changer changed; tools numbered by their slots");
            self.refresh_form_tool();
        }
    }

    /// Writes the machines, if they have a file, and reports `done` or the failure.
    pub(super) fn machines_changed(&mut self, done: &str) {
        if self.machines_unsaved {
            self.status.set_message(&format!(
                "{done}, but not saved: the machines file could not be read when Cut Craft \
                 started, so it is not saved over"
            ));
            return;
        }
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
        let Some(package) = self.read_package(path) else {
            return;
        };
        let (added, here) = self.machines.install(package);
        self.machines_changed(&installed_machines(added, here));
    }

    /// Machine Configuration's Install Offline Package: the package's machines added to the
    /// machines being edited, kept with Apply or OK like any other edit there.
    pub(super) fn machine_outcome_install(&mut self, path: &std::path::Path) {
        let Some(package) = self.read_package(path) else {
            return;
        };
        let Some(window) = self.machine_window.as_mut() else {
            return self.install_package(path);
        };
        let (added, here) = window.draft.install(package);
        let done = installed_machines(added, here);
        self.status
            .set_message(&format!("{done}; Apply or OK keeps them"));
    }

    /// The machine package at `path`, read; `None` with the status bar saying why not.
    fn read_package(&mut self, path: &std::path::Path) -> Option<cut_craft_machines::Machines> {
        let package = std::fs::read(path)
            .map_err(|e| crate::messages::could_not_read(path, &e))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes).map_err(|e| {
                    log::warn!("{}: not a machine package ({e})", path.display());
                    crate::messages::not_a_machine_package(path)
                })
            });
        match package {
            Ok(package) => Some(package),
            Err(message) => {
                self.status.set_message(&message);
                None
            }
        }
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
        } else if entry.name.contains(" (copy ") {
            self.status.set_message(&format!(
                "Installed as {}: a post with its name was already listed, so this one is \
                 kept beside it. Pick it in Machine Configuration to use it",
                entry.name
            ));
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
                    // the checks made so far used the holder and limits just replaced
                    self.save_checks.clear();
                    if outcome == MachineOutcome::Ok {
                        self.machine_window = None;
                    } else if let Some(w) = &mut self.machine_window {
                        w.applied();
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
                MachineOutcome::PickFolder(which) => {
                    if let Some(folder) = file_dialog().pick_folder()
                        && let Some(machine) =
                            (self.machine_window.as_mut()).and_then(|w| w.draft.active_mut())
                    {
                        let kept = crate::clipart::network::kept(&folder).display().to_string();
                        match which {
                            cut_craft_machines::FolderFor::Programs => {
                                machine.send_to.programs = kept;
                            }
                            cut_craft_machines::FolderFor::Machine => machine.send_to.folder = kept,
                        }
                    }
                }
                MachineOutcome::InstallPackage => {
                    let dialog = filtered_dialog(Dialog::MachinePackage, &self.registry);
                    if let Some(path) = dialog.pick_file() {
                        self.machine_outcome_install(&path);
                    }
                }
                MachineOutcome::ManageFiles => {
                    if let Some(machine) = draft.active() {
                        let (name, send_to) = (machine.name.clone(), machine.send_to.clone());
                        self.open_machine_files(&name, &send_to);
                    }
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
                if let Some(path) =
                    filtered_dialog(Dialog::InstallPostProcessor, &self.registry).pick_file()
                {
                    self.install_post(&path);
                }
            }
            PostOutcome::Select(name) => {
                let wraps = self.doc.job.rotary.map(|r| match r.along {
                    cut_craft_core::document::Along::X => "Y",
                    cut_craft_core::document::Along::Y => "X",
                });
                let picked = self
                    .posts
                    .load(&name)
                    .ok()
                    .map(|p| p.wraps().map(str::to_owned));
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
                // the post picked is the one toolpaths save through from now on
                machine.default_post = Some(name.clone());
                let mut done = format!("{} now saves through {name}", machine.name);
                // the default is used only for jobs it suits (flat, or wrapping the same
                // side): a job it does not suit saves through the machine's first that does
                if let Some(picked) = picked.filter(|p| p.as_deref() != wraps) {
                    let kind = match &picked {
                        None => "flat jobs".to_owned(),
                        Some(axis) => format!("rotary jobs wrapping {axis} onto A"),
                    };
                    let job = if wraps.is_some() {
                        "this job wraps round"
                    } else {
                        "this job is flat"
                    };
                    let used = (machine.posts.iter())
                        .find(|n| self.posts.load(n).is_ok_and(|p| p.wraps() == wraps));
                    done = match used {
                        Some(used) => format!(
                            "{} now saves {kind} through {name}; {job}, so it saves through \
                             {used}",
                            machine.name
                        ),
                        None => format!(
                            "{} now saves {kind} through {name}; {job}, and none of its posts \
                             suits it: add one in Machine Configuration",
                            machine.name
                        ),
                    };
                }
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
        let (id, interactive) = (cmd.id(), cmd.interactive());
        let (meta, fields, mut params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        // Orientate 3D Model offers what Options ▸ Importing and Exporting says; one kind alone
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
        // the untouched default bit given the sizes a real one has, so a new form warns of
        // nothing the beginner did not set
        starter_tool(&mut params);
        // a toolpath's form in the standard order
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
            // a placing tool opened over a form keeps that form, edits and all, for Esc to go
            // back to; any other form lets it go
            if self.placing(id) {
                if let Some(behind) =
                    (self.form.take()).filter(|f| f.id != id && !f.new_job && !self.placing(f.id))
                {
                    self.form_behind = Some(behind);
                }
            } else {
                self.form_behind = None;
            }
            self.form = Some(OpenForm {
                id: cmd.id(),
                label: meta.label,
                icon: meta.icon,
                fields,
                params,
                error: None,
                warning: None,
                new_job: false,
                page,
                editing: None,
                drawn: None,
                dirty: false,
                suggested: None,
                shown_size: None,
                typed_size: [None; 3],
                dragged: None,
                model: None,
                model_whole: true,
                beyond: None,
                applied: None,
                remade: None,
                undo_from: self.history.undo_depth(),
                list_steps: Vec::new(),
                in_list: false,
                way: crate::flyout::DragWay::AsIs,
                made: None,
                note: None,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
            // a shape this tool draws, selected as it opens: the form edits that shape
            if let Some((shape, params)) = self.drawn_by_tool(id).filter(|_| interactive) {
                self.edit_shape(shape, id, params);
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
                let mut saved = format!("Saved {}", path.display());
                // said again as it happens: the opening warning is long gone by now
                if !self.session.newer_settings.is_empty() {
                    saved = format!(
                        "{saved}. Warning: settings this version of Cut Craft does not know \
                         were left out ({}). Update Cut Craft (Help ▸ Check for Updates) and \
                         open the original to keep them",
                        self.session.newer_settings.join(", ")
                    );
                }
                self.status.set_message(&saved);
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
            "open",
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
                // a fresh history counts its revisions from the start again
                self.picked_constraint = None;
                self.screen = Screen::Job;
                if !template {
                    self.remember(path);
                }
                if self.doc.notes.starts_with('.') {
                    self.start_command("notes");
                }
                // a job saved before one of its bits was renamed takes the new name
                let renamed: Vec<(String, String)> = (self.doc.toolpaths.values())
                    .filter_map(|t| {
                        let now = self.tools.renamed_now(&t.tool)?;
                        Some((t.tool.name.clone(), now.to_owned()))
                    })
                    .collect();
                let mut opened = format!("Opened {}", super::file_name(path));
                if !self.session.newer_settings.is_empty() {
                    opened = format!(
                        "{opened}. Warning: it was saved by a newer version of Cut Craft, and \
                         this one does not know some of its settings ({}): saving it here \
                         leaves them out. Update Cut Craft (Help ▸ Check for Updates) to keep \
                         them",
                        self.session.newer_settings.join(", ")
                    );
                }
                self.status.set_message(&opened);
                self.rename_toolpath_tools(&renamed, &opened);
                true
            }
            Err(e) => {
                self.status.set_message(&e);
                // only a file that has gone leaves Recent Files: a damaged one (perhaps a
                // sync still going) or one needing a newer Cut Craft stays to try again
                if !path.exists() && self.recent.iter().any(|p| p == path) {
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

    /// The recent files listed: as many as Options lists. More are kept (up to
    /// [`files::RECENT_KEPT`]), so a smaller list only hides the rest until it grows again.
    pub(super) fn recent_shown(&self) -> &[std::path::PathBuf] {
        &self.recent[..self.recent.len().min(self.options.recent_count())]
    }

    pub(super) fn remember(&mut self, path: &std::path::Path) {
        files::remember(&mut self.recent, path, files::RECENT_KEPT);
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

    /// At startup: this window's recovery copy claimed in the settings folder, and a crashed
    /// run's work opened if one left any.
    pub(super) fn start_recovery(&mut self) {
        if let Some(dir) = files::config_dir() {
            self.claim_recovery(&dir, &std::process::id().to_string());
        }
        self.open_recovery();
    }

    /// A file named on the command line (Open With, or dragged onto the launcher), handled
    /// much as if dropped on the window: a job opens; a drawing or picture is imported into
    /// the open job or, with none open (the start page), into a new one once Job Setup's OK
    /// has made it.
    pub(super) fn open_given(&mut self, path: Option<&std::path::Path>) {
        let Some(path) = path else {
            return;
        };
        if self.screen == Screen::Job || !Self::drop_imports(path) {
            self.drop_file(path);
            return;
        }
        self.file_action("new");
        if self.screen == Screen::NewJob {
            self.import_after_setup = Some(path.to_owned());
            self.status.set_message(&format!(
                "Set up the material, then OK makes the job and imports {} into it",
                file_name(path)
            ));
        }
    }

    /// This window's own recovery copy, in `dir`, named for `run` (the process), and locked
    /// for as long as the window is open (where the folder can lock files). A folder that
    /// cannot be written says so in the status bar.
    pub(super) fn claim_recovery(&mut self, dir: &std::path::Path, run: &str) {
        if let Some((own, lock)) = files::claim_recovery(dir, run) {
            (self.recovery_path, self.recovery_lock) = (Some(own), lock);
        } else {
            (self.recovery_path, self.recovery_lock) = (None, None);
            self.status.set_message(&format!(
                "Warning: unsaved work cannot be copied for recovery, because {} cannot be \
                 written. Save often",
                dir.display()
            ));
        }
    }

    /// The recovery copy a run that ended badly left, opened as an unsaved job; whether
    /// there was one. Another window's copy, still running, is left alone. The work becomes
    /// this window's copy, kept until the job is saved or closed.
    pub(super) fn open_recovery(&mut self) -> bool {
        let Some(own) = self.recovery_path.clone() else {
            return false;
        };
        let Some(left) = own
            .parent()
            .and_then(|dir| files::orphaned_recovery(dir, &own))
        else {
            return false;
        };
        let Some((bytes, doc)) = std::fs::read(&left).ok().and_then(|bytes| {
            let doc = cut_craft_core::file::load_job(&bytes).ok()?;
            Some((bytes, doc))
        }) else {
            return false;
        };
        self.reset_job();
        // kept as this run's copy, so the work is still there if this run ends badly too
        // before anything is changed (autosave writes it again then)
        match cut_craft_core::file::write_atomic(&own, &bytes) {
            Ok(()) => {
                let _ = std::fs::remove_file(&left);
                let _ = std::fs::remove_file(left.with_extension("lock"));
            }
            Err(e) => self
                .status
                .set_message(&format!("could not keep the recovery copy: {e}")),
        }
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
        self.picked_constraint = None;
        self.picked_outline = None;
        self.job_epoch += 1;
        // measured for the job before: a new history can be at the same revision
        self.model_tall.set(None);
        *self.note_vars.borrow_mut() = None;
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
            self.rough_3d,
        ) = (None, None, None, None, None, None);
        // the old job's preview playback, its controls and its undo go with it
        (self.playback, self.kept_playback, self.before_preview) = (None, None, None);
        (self.playing, self.playback_goal, self.playback_shown) = (false, None, None);
        (self.playback_view, self.playback_gpu) = (None, None);
        self.preview_side = self.doc.bottom_side;
        // the G-code shown, and a save waiting or asked about, were the job that was open
        self.gcode_view = None;
        self.save_anyway = None;
        self.save_after = None;
        self.form_back = None;
        self.checks_accepted = false;
        self.clear_recovery();
    }

    /// Set as Default in Job Setup: new jobs start with the job its settings make.
    pub(super) fn job_setup_default(&mut self) {
        let Some(form) = &mut self.form else { return };
        // made on an empty document, so only the job's own settings count; it starts from
        // the default kept now, so what is not on the form (Clearing Settings' Use as My
        // Default, the preview colours, where the model sits) is kept as it was
        let mut empty = Document::default();
        if let Some(job) = self.options.default_job {
            empty.job = job;
        }
        let made = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms")
            .run_json(&empty, &form.params)
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
                self.picked_constraint = None;
                self.screen = Screen::Job;
                self.status.set_message("New job created");
                "Select".clone_into(&mut self.status.current_tool);
                if let Some(path) = self.import_after_setup.take() {
                    self.import_file(&path);
                }
            }
            Err(e) => {
                form.error = Some(e);
                self.form = Some(form);
            }
        }
    }

    pub(super) fn job_setup_cancel(&mut self) {
        self.import_after_setup = None;
        self.reset_job();
        "Select".clone_into(&mut self.status.current_tool);
        self.screen = Screen::Start;
    }

    pub(super) fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = filtered_dialog(Dialog::Export(id), &self.registry);
        if let Some(path) = dialog.save_file()
            && let Some(path) = saving_path(&path, format.extensions)
        {
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
            .set_description(self.save_changes_question())
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                SAVE.into(),
                DONT_SAVE.into(),
                "Cancel".into(),
            ))
            .show();
        self.discard_answered(&answer)
    }

    /// File ▸ Save As: the job saved under the name and folder the person picks.
    fn save_as(&mut self) {
        let (folder, name) = self.save_as_suggestion();
        let dialog = filtered_dialog(Dialog::SaveJob, &self.registry).set_file_name(name);
        let dialog = match folder {
            Some(folder) => dialog.set_directory(folder),
            None => dialog,
        };
        if let Some(path) = dialog.save_file()
            && let Some(path) = saving_path(&path, &[JOB_EXTENSION])
        {
            self.save_to(&path);
        }
    }

    /// Where Save As starts: the job's own folder and name when it has a file, else
    /// `untitled.ccjob` wherever file dialogs open.
    pub(super) fn save_as_suggestion(&self) -> (Option<std::path::PathBuf>, String) {
        let Some(path) = &self.session.path else {
            return (None, format!("untitled.{JOB_EXTENSION}"));
        };
        let name = path.file_stem().map_or_else(
            || "untitled".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        (
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(std::path::Path::to_owned),
            format!("{name}.{JOB_EXTENSION}"),
        )
    }

    /// The question asked before unsaved work is replaced: the job's file name alone.
    pub(super) fn save_changes_question(&self) -> String {
        let name = self
            .session
            .path
            .as_deref()
            .and_then(std::path::Path::file_name)
            .map_or_else(
                || "Untitled".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
        format!("Save changes to {name}?")
    }

    /// What the answer at "Save changes?" means: true when the job may be replaced. No only
    /// lets the work go: the job stays marked unsaved until something actually replaces it,
    /// so backing out afterwards (a cancelled file picker, a file that will not open) loses
    /// nothing and is asked about again.
    pub(super) fn discard_answered(&mut self, answer: &rfd::MessageDialogResult) -> bool {
        let answer = match answer {
            rfd::MessageDialogResult::Custom(c) if c == SAVE => &rfd::MessageDialogResult::Yes,
            rfd::MessageDialogResult::Custom(c) if c == DONT_SAVE => &rfd::MessageDialogResult::No,
            other => other,
        };
        match answer {
            rfd::MessageDialogResult::Yes => {
                let path = self.session.path.clone().or_else(|| {
                    filtered_dialog(Dialog::SaveJob, &self.registry)
                        .save_file()
                        .and_then(|path| saving_path(&path, &[JOB_EXTENSION]))
                });
                match path {
                    Some(path) => self.save_to(&path),
                    None => return false,
                }
                !self.session.is_dirty(&self.history)
            }
            rfd::MessageDialogResult::No => true,
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
        if result.is_ok() {
            notes.extend_from_slice(self.history.last_notes());
            notes.dedup();
        }
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
        self.warn(&made, &notes);
    }

    /// More toolpaths out of date than last frame, after an edit (not as a job opens, nor
    /// while a drag goes on, as a height handle remakes the model): the status bar says how
    /// many, and that they and any G-code saved from them want making again. Toolpaths are
    /// never remade by themselves.
    pub(super) fn note_stale(&mut self) {
        if self.drag_3d.is_some() {
            return;
        }
        let stale = self.doc.toolpaths.values().filter(|t| t.stale).count();
        let more = stale.saturating_sub(self.stale_known);
        self.stale_known = stale;
        if more == 0 || self.history.undo_depth() == 0 {
            return;
        }
        // asked about once the edit is done (a form's, once it closes)
        self.stale_while_form = true;
        let (count, it, its) = if more == 1 {
            ("1 toolpath is".to_owned(), "it", "its")
        } else {
            (format!("{more} toolpaths are"), "them", "their")
        };
        let message = self.status.message.trim_end_matches('.').to_owned();
        self.status.set_message(&format!(
            "{message}. {count} now out of date (⚠ in the Toolpaths list): recalculate \
             {it}, and save {its} G-code again if you already have"
        ));
    }

    /// Once a form that left toolpaths out of date has closed: asks whether to recalculate
    /// them all now. No just leaves them marked; Save Toolpaths still warns.
    pub(super) fn ask_to_recalculate(&mut self, ctx: &egui::Context) {
        let done = self.form.is_none() && self.drag_3d.is_none();
        if done && std::mem::take(&mut self.stale_while_form) {
            let stale = self.doc.toolpaths.values().filter(|t| t.stale).count();
            let ask = self.options.confirmations.ask_to_recalculate;
            self.recalculate_ask = (ask && stale > 0).then_some((stale, false));
        }
        // recalculated meanwhile (Plate Production remakes its own straight away)
        let now = self.doc.toolpaths.values().filter(|t| t.stale).count();
        if now == 0 {
            self.recalculate_ask = None;
        }
        let Some((asked, mut silence)) = self.recalculate_ask else {
            return;
        };
        let stale = asked.min(now);
        let (mut yes, mut no) = (false, false);
        egui::Modal::new(egui::Id::unique("recalculate_ask")).show(ctx, |ui| {
            ui.set_max_width(ui.spacing().interact_size.x * 6.0);
            ui.heading("Toolpaths out of date");
            ui.label(recalculate_question(stale));
            ui.checkbox(&mut silence, "Don't ask again").on_hover_hint(
                "Stop this question (Options ▸ Confirmation Choices turns it back on). Out-of-date \
                 toolpaths are still marked ⚠, and saving them still asks first",
            );
            ui.separator();
            ui.horizontal(|ui| {
                yes = ui
                    .button("Recalculate All")
                    .on_hover_hint("Make every out-of-date toolpath again from its vectors now")
                    .clicked();
                no = ui
                    .button("Not Now")
                    .on_hover_hint(
                        "Leave them marked ⚠ in the Toolpaths list; Recalculate All is there \
                         when you want it",
                    )
                    .clicked();
            });
        });
        self.recalculate_ask = Some((asked, silence));
        if yes || no {
            self.recalculate_ask = None;
            if silence {
                self.options.confirmations.ask_to_recalculate = false;
                self.save_options();
            }
        }
        if yes {
            self.invoke(ctx, "recalculate_all_toolpaths");
        }
    }

    /// A warning in the status bar about what was just made: the command's `notes`, and for
    /// each toolpath in `made`, crossing a keep-out zone and the problems the checks before
    /// cutting find.
    pub(super) fn warn(&mut self, made: &[EntityId], notes: &[String]) {
        let mut warnings = notes.to_vec();
        for t in made.iter().filter_map(|id| self.doc.toolpaths.get(id)) {
            let zones = self.zones_entered(t);
            if !zones.is_empty() {
                warnings.push(format!(
                    "{} cuts into keep-out zone {} and will not be saved: move the part or the \
                     zone, or turn on clipping ({CLIP_SETTING})",
                    t.name,
                    zones.join(" and ")
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

    /// The icons over the toolpath list: edit and hide the selected toolpath, and with
    /// `templates` save it, or every visible one, as a template (the Toolpaths page has those
    /// under the list, with the other ways to reuse toolpaths).
    pub(super) fn toolpath_list_buttons(&mut self, ui: &mut egui::Ui, templates: bool) {
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
            ]
            .into_iter()
            .take(if templates { 4 } else { 2 })
            {
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

    /// The file icons beside the menus: Import Vectors and Export Vectors (vector
    /// formats only), Open Folder (the job's folder) and Open Clipart Folder.
    pub(super) fn file_icon_action(&mut self, id: &str) {
        let result = match id {
            "import_vectors" => {
                let dialog = filtered_dialog(Dialog::ImportVectors, &self.registry);
                if let Some(path) = dialog.pick_file() {
                    self.import_file(&path);
                }
                Ok(())
            }
            "export_vectors" => {
                let dialog = filtered_dialog(Dialog::ExportVectors, &self.registry);
                let exporters: Vec<_> = VECTOR_EXPORTERS
                    .iter()
                    .filter_map(|id| self.registry.exporter(id))
                    .map(|e| (e.id, e.format))
                    .collect();
                match dialog.save_file() {
                    // the extension picks the format; DXF when there is none
                    Some(path) => {
                        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
                        let (id, path) = exporters
                            .iter()
                            .find(|(_, f)| {
                                ext.as_deref().is_some_and(|x| f.extensions.contains(&x))
                            })
                            .map_or(
                                ("export_dxf", files::named_for_saving(&path, &["dxf"])),
                                |(id, _)| (*id, path.clone()),
                            );
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
        let mut params = toolpath.params.clone();
        // the bank as the machine has it now, so a bank changed since is used
        self.fill_drill_bank(&toolpath.strategy, &mut params);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == toolpath.strategy) {
            // Calculate with nothing changed keeps it (a drill bank changed since is a change)
            form.made = Some(super::calculate_again::Made {
                ids: vec![id],
                kept: vec![toolpath.params.clone()],
                params: toolpath.params.clone(),
                selection: None,
            });
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
        if let Some(form) = &mut self.form {
            form.note = None;
        }
        let editing = self.form.as_ref().and_then(|f| f.editing);
        // Calculate pressed again: kept when nothing changed, else updated in place
        match self.calculate_again(params) {
            super::calculate_again::Again::UpToDate => {
                self.say_up_to_date(params);
                return Ok(());
            }
            super::calculate_again::Again::Update(made) if editing.is_none() => {
                return self.calculate_in_background(id, params, Some(made));
            }
            _ => {}
        }
        if let Some(toolpath) = editing {
            self.edit_in_background(id, toolpath, params);
            return Ok(());
        }
        if REMAKE_FORMS.contains(&id) {
            return self.remake(id, params);
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

    /// A remaking form's Apply: its pieces made again with `params` in place of the ones it
    /// last made, while that is still the last thing done (one undo); else a new one.
    pub(super) fn remake(&mut self, id: &str, params: &Value) -> Result<(), String> {
        if let Some(applied) = self.remade_last() {
            if applied == *params {
                return Ok(());
            }
            self.history.undo(&mut self.doc);
        }
        if let Some(form) = &mut self.form {
            form.remade = None;
        }
        self.apply(id, params)?;
        let after = (self.history.revision(), self.doc.selection.clone());
        if let Some(form) = &mut self.form {
            form.remade = Some((after.0, after.1, params.clone()));
        }
        Ok(())
    }

    /// The settings the open remaking form last made its pieces with, while nothing has been done or picked
    /// since.
    pub(super) fn remade_last(&self) -> Option<Value> {
        let (revision, selection, applied) = self.form.as_ref()?.remade.as_ref()?;
        (self.history.revision() == *revision && self.doc.selection == *selection)
            .then(|| applied.clone())
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
        // a remaking form's pieces follow its settings once made, a drag only remaking it (one undo)
        if self.remade_last().is_some() {
            self.apply_live();
            return;
        }
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
        let sized = params["size"].is_object();
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
        // the model's size: its triangles' box, or (wrapped) its component's
        let made = model.as_deref().and_then(model_size);
        let cap = usize::try_from(self.options.import.preview_faces).unwrap_or(usize::MAX);
        // on a flat job, what is past the material's edges is cut off to show as a ghost;
        // past the preview limit, every so many of its faces only (faint, it still reads)
        let (model, beyond) = match model {
            Some(all) if self.doc.job.rotary.is_none() => {
                let (over, past) = crate::view3d::split_at_edges(&all, self.doc.job.area());
                let every = past.len().div_ceil(cap.max(1)).max(1);
                let past = past
                    .into_iter()
                    .step_by(every)
                    .map(|t| (t, false))
                    .collect();
                (Some(over), Some(std::sync::Arc::new(past)))
            }
            all => (all, None),
        };
        // a huge model is shown as the surface it makes, only its unreachable faces drawn
        let (model, whole) = match model {
            Some(all) if all.len() > cap => (
                Some(all.into_iter().filter(|(_, red)| *red).collect()),
                false,
            ),
            all => (all, true),
        };
        let model = model.map(std::sync::Arc::new);
        let made = made.or_else(|| {
            let k = self.doc.components.get(&self.form.as_ref()?.drawn?)?;
            let (lo, hi) = k.height_range().unwrap_or_default();
            Some([k.cell * k.columns as f64, k.cell * k.rows as f64, hi - lo])
        });
        let (area, thickness) = (self.doc.job.area(), self.doc.job.thickness);
        if let Some(form) = &mut self.form {
            form.error = result.err();
            form.model = model;
            form.model_whole = whole;
            form.beyond = beyond;
            if let (true, Some(made)) = (sized, made) {
                // to the hundredth of a millimetre shown, not the cell's float noise
                let round = |v: f64| (v * 100.0).round() / 100.0;
                let shown = std::array::from_fn(|i| form.typed_size[i].unwrap_or(round(made[i])));
                for (key, value) in SIZES.iter().zip(shown) {
                    form.params["size"][key] = Value::from(value);
                }
                // Depth Below Top reads the model's height while the plane is at its
                // bottom (0, or where its bottom was before it was sized)
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
        // a shape changed inside a group stays in that group
        let mut regrouped = Vec::new();
        if let Some(&group) = self.doc.vector_groups.get(&old) {
            let mut after = without.clone();
            after.apply(&made).map_err(|e| e.to_string())?;
            let mut c = Change::builder(&after);
            for edit in &made.edits {
                if let cut_craft_core::Edit::Add { id, .. } = edit {
                    c.set_group(*id, Some(group));
                }
            }
            regrouped = c.finish().edits;
        }
        let change = Change {
            edits: (taken.edits.into_iter().chain(made.edits))
                .chain(regrouped)
                .collect(),
            notes: made.notes,
            unit_guess: made.unit_guess,
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
            .map_or_default(|s| s.to_string_lossy().into_owned());
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
        let Some(path) = filtered_dialog(Dialog::JobSheet, &self.registry).save_file() else {
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
            .map_or_default(|p| p.display().to_string());
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
            machine: machine.map_or_default(|m| m.name.clone()),
            post: machine
                .and_then(|m| m.post())
                .unwrap_or_default()
                .to_owned(),
            outputs: vec![],
            slots: machine.map_or_default(|m| m.slots.clone()),
            printed: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            motion: self.motion(),
            scale: self.summary.scale,
            model_height: self.model_height_now(),
            holding: self.holding(),
            variables: (*self.note_variables()).clone(),
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

    /// The tool that made what is picked and keeps its settings, so E edits it in the form:
    /// Embellish for an embellishment (its component, or any of its pieces), Create Shape
    /// for a component it made (and no vector picked).
    pub(super) fn made_to_edit(&self) -> Option<&'static str> {
        use cut_craft_core::Command as _;
        if cmd_embellish::edits_selection(&self.doc) {
            return Some(cmd_embellish::Embellish::ID);
        }
        let shaped = self.doc.selection.is_empty()
            && self.doc.selected_components.iter().any(|id| {
                self.doc
                    .components
                    .get(id)
                    .and_then(|k| k.shape.as_ref())
                    .is_some_and(cmd_create_shape::made_here)
            });
        shaped.then_some(cmd_create_shape::CreateShape::ID)
    }

    /// See [`Self::made_to_edit`].
    pub(super) fn shape_to_edit(&self) -> bool {
        self.made_to_edit().is_some()
    }

    /// The one vector selected, if drawing tool `id` draws shapes like it, with the settings
    /// that draw it again (a square reads as Draw Rectangle's, or as Draw Polygon's).
    fn drawn_by_tool(&self, id: &str) -> Option<(EntityId, Value)> {
        let [shape] = self.doc.selection.iter().copied().collect::<Vec<_>>()[..] else {
            return None;
        };
        let path = self.doc.vectors.get(&shape)?;
        let cmd = self.registry.get(id)?;
        let params = cmd.params_from(path, &cmd.initial_params(&self.doc))?;
        Some((shape, params))
    }

    /// Opens drawing tool `command` on vector `id` with `params`: changing the form redraws
    /// that shape rather than drawing another (E, or Shift-click).
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

    /// Once the job has turned over (Swap Sides, its undo or its redo), the preview turns
    /// with it: its other side on top, the side that was up underneath. A side never
    /// previewed comes up uncut, so the one that was up is kept underneath, not lost.
    pub(super) fn follow_side(&mut self) {
        if self.doc.bottom_side == self.preview_side {
            return;
        }
        self.preview_side = self.doc.bottom_side;
        if self.simulation.is_none() {
            return;
        }
        self.finish_playback();
        let Some(mut top) = self.simulation.take() else {
            return;
        };
        let mut now_up = if let Some(under) = top.other_side.take() {
            std::sync::Arc::unwrap_or_clone(under)
        } else {
            let mut fresh = top.clone();
            fresh.heights.fill(0.0);
            fresh.cut_by.fill(0);
            (fresh.paint, fresh.under) = (vec![], vec![]);
            fresh
        };
        now_up.other_side = Some(std::sync::Arc::new(top));
        self.simulation = Some(now_up);
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
        let depth = self.history.undo_depth();
        self.applied(id, params, change)?;
        // made in the toolpath list beside an open form: noted, for its Cancel to make again
        if self.history.undo_depth() > depth
            && let Some(form) = self.form.as_mut().filter(|f| f.in_list)
        {
            form.list_steps.push((id.to_owned(), params.clone()));
        }
        Ok(())
    }

    /// A toolpath list change's settings made to fit the job as it is now, after Cancel took
    /// back what the form did: the toolpaths it names that are gone are left out (or the
    /// change, when it was on one of them), and a show/hide keeps each name as it now is.
    fn list_step_again(&self, id: &str, params: &Value) -> Option<Value> {
        let toolpaths = &self.doc.toolpaths;
        let exists = |v: &Value| {
            v.as_u64()
                .is_some_and(|n| toolpaths.contains_key(&EntityId(n)))
        };
        let mut params = params.clone();
        if let Some(one) = params.get("toolpath") {
            let n = one.as_u64().filter(|_| exists(one))?;
            if id == "toolpath_properties" {
                params["name"] = Value::from(toolpaths[&EntityId(n)].name.clone());
            }
        }
        if let Some(Value::Array(ids)) = params.get_mut("toolpaths") {
            ids.retain(exists);
        }
        if let Some(Value::Object(groups)) = params.get_mut("groups") {
            groups.retain(|key, _| {
                key.parse()
                    .is_ok_and(|n| toolpaths.contains_key(&EntityId(n)))
            });
        }
        if let Some(Value::Array(order)) = params.get_mut("order") {
            // every toolpath once, as the order needs: ones it did not name go at the end
            order.retain(exists);
            for id in toolpaths.keys() {
                if !order.iter().any(|v| v.as_u64() == Some(id.0)) {
                    order.push(Value::from(id.0));
                }
            }
        }
        Some(params)
    }

    /// `change` with the points it placed on snaps kept there: a drawn shape's snapped
    /// points (a line's nodes stretch to stay, other shapes move whole), or a node dragged
    /// onto a snap. Only for the drawing tool in use, or Node Editing's Move Node.
    pub(super) fn kept_on_snaps(&mut self, id: &str, mut change: Change) -> Change {
        use cut_craft_core::Edit;
        use cut_craft_core::constraint::{merged, pins};
        let placed = std::mem::take(&mut self.placed);
        let drawn = self.drawing_tool() == Some(id) && id != "draw_text";
        if placed.is_empty() || !(drawn || id == "move_node") {
            return change;
        }
        let stretch = matches!(id, "draw_polyline" | "move_node");
        let mut made = vec![];
        for edit in &change.edits {
            match edit {
                Edit::Add { id, path } if drawn => made.extend(pins(path, *id, &placed, stretch)),
                Edit::Replace { id, after, .. } if !drawn => {
                    made.extend(pins(after, *id, &placed, stretch));
                }
                _ => {}
            }
        }
        if !made.is_empty() {
            let before = self.doc.constraints.clone();
            let after = merged(&before, made);
            change.edits.push(Edit::SetConstraints { before, after });
        }
        change
    }

    /// The level `change`'s new components all went on, for the status bar to name. None
    /// when it makes no new component, shares them over several levels, sets out levels
    /// itself (Slice Model, Bake: it says what it showed and hid), or replaces components
    /// by removing and adding them (Split, Emboss, Sculpt): there is no new component then.
    pub(super) fn new_components_level(change: &Change) -> Option<u64> {
        use cut_craft_core::Edit;
        let mut levels = change.edits.iter().filter_map(|e| match e {
            Edit::AddComponent { component, .. } => Some(component.level),
            _ => None,
        });
        let first = levels.next()?;
        let replaces = change
            .edits
            .iter()
            .any(|e| matches!(e, Edit::RemoveComponent { .. } | Edit::SetLevel { .. }));
        (!replaces && levels.all(|l| l == first)).then_some(first)
    }

    /// ": new component on level <name>" for the status bar, saying when that level is
    /// hidden so a new shape that cannot be seen is not taken for lost.
    fn placed_on(&self, level: u64) -> String {
        self.doc.levels.get(&level).map_or_else(String::new, |l| {
            let hidden = if l.visible {
                ""
            } else {
                " (hidden: show the level to see it)"
            };
            format!(": new component on level {}{hidden}", l.name)
        })
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
        let change = self.kept_on_snaps(id, change?);
        if let Some(why) = self.keep_out_refusal(&change) {
            return Err(why);
        }
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
        // the level the new components went on, for the status bar
        let new_level = Self::new_components_level(&change);
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        let kept = self.history.last_notes().to_vec();
        // a form that edits what it made (Draw Text) goes on editing it, not adding another
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id)
            && let Some(made) = cmd.initial_params(&self.doc).get("replaces").cloned()
        {
            form.params["replaces"] = made;
        }
        // Bake made the shaping permanent and reset it: the form shows that reset (100 %,
        // Bake unticked), so pressing Apply again does not bake it in a second time
        if id == "component_properties"
            && params.get("bake") == Some(&Value::Bool(true))
            && let Ok(p) =
                serde_json::from_value::<cmd_component_properties::Params>(params.clone())
            && let Some(k) = self.doc.components.get(&p.component)
            && let Some(form) = self.form.as_mut().filter(|f| f.id == id)
            && let Ok(fresh) =
                serde_json::to_value(cmd_component_properties::Params::of(p.component, k))
        {
            form.params = fresh;
        }
        let placed = new_level.map_or_else(String::new, |l| self.placed_on(l));
        self.status
            .set_message(&format!("{} applied{placed}", cmd.meta().label));
        // new keep-out zones: the toolpaths are made again round them
        if id == "keep_out_zones" {
            self.keep_out_changed();
        }
        // the spiral gadget says how many times round each strand goes
        if id == "wrapped_spiral_layout"
            && let Ok(p) = serde_json::from_value(params.clone())
            && let Some(turns) = cmd_wrapped_spiral_layout::revolutions(&self.doc, &p)
        {
            self.status
                .set_message(&format!("Each strand makes {turns:.2} revolutions"));
        }
        // a toolpath made already out of date (Plate Production's, cutting each plate's own
        // text) is recalculated straight away, then warned about
        let stale: Vec<EntityId> = made
            .iter()
            .copied()
            .filter(|t| self.doc.toolpaths.get(t).is_some_and(|t| t.stale))
            .collect();
        let mut notes = [notes, kept].concat();
        notes.dedup();
        // the form's own command: its warnings stay in red at the form's foot until its
        // next Apply
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id) {
            form.warning = (!notes.is_empty()).then(|| notes.join("\n"));
        } else {
            // a tool with no form: in red at the foot of the tool panel until the next change
            self.tool_note = (!notes.is_empty())
                .then(|| (self.job_epoch, self.history.revision(), notes.join("\n")));
        }
        if stale.is_empty() {
            self.warn(&made, &notes);
        } else {
            self.recalculate_in_background(stale, "Toolpaths recalculated", move |app| {
                app.warn(&made, &notes);
            });
        }
        // the preview turns over with the job (follow_side)
        self.follow_side();
        Ok(())
    }

    /// The Toolpath Summary's rows: each toolpath with its estimated time and notes.
    fn summary_rows(&self) -> Vec<super::SummaryRow> {
        let times = self.cut_times();
        self.doc
            .toolpaths
            .iter()
            .map(|(&id, t)| super::SummaryRow {
                id,
                name: t.name.clone(),
                minutes: times.each.get(&id).copied().unwrap_or_default(),
                visible: t.visible,
                notes: t.summary_notes().to_owned(),
            })
            .collect()
    }

    /// A toolpath's time for the Toolpath Summary, minutes: see [`super::CutTimes`].
    pub(super) fn estimated_minutes(&self, t: &cut_craft_core::toolpath::Toolpath) -> f64 {
        let times = self.cut_times();
        self.doc
            .toolpaths
            .iter()
            .find(|(_, v)| std::ptr::eq(*v, t))
            .and_then(|(id, _)| times.each.get(id).copied())
            .unwrap_or_else(|| {
                cut_craft_core::cut_time::toolpath(t, &self.motion()).total() / 60.0
                    * self.summary.scale
            })
    }

    /// The whole job's cutting time, minutes: every shown toolpath, the moves between them,
    /// the tool changes and the trip back home.
    pub(super) fn job_minutes(&self) -> f64 {
        self.cut_times().total
    }

    /// What the cutting times go by: the Toolpath Summary's rapid rate, the active
    /// machine's top speeds, cornering and (unless the Toolpath Summary is told not to)
    /// accelerations, its tool changer, and the tool change times in Options.
    pub(super) fn motion(&self) -> cut_craft_core::cut_time::Motion {
        let machine = self.machines.active();
        let o = &self.options.toolpaths;
        cut_craft_core::cut_time::Motion {
            rapid: self.summary.rapid,
            max_rate: machine.map_or([0.0; 3], |m| m.max_rate),
            acceleration: machine
                .filter(|_| self.summary.acceleration)
                .map_or([0.0; 3], |m| m.acceleration),
            junction_deviation: machine.map_or(0.0, |m| m.junction_deviation),
            change_by_hand: o.change_by_hand * 60.0,
            change_by_changer: o.change_by_changer * 60.0,
            loaded: machine.map_or_default(|m| m.slots.iter().flatten().cloned().collect()),
        }
    }

    /// Every toolpath's cutting time and the job's, worked out again only when the job or
    /// what the times go by changes.
    pub(super) fn cut_times(&self) -> std::rc::Rc<super::CutTimes> {
        let motion = self.motion();
        let shown: Vec<(EntityId, bool)> = self
            .doc
            .toolpaths
            .iter()
            .map(|(&id, t)| (id, t.visible))
            .collect();
        let key = format!(
            "{}|{}|{motion:?}|{}|{shown:?}",
            self.job_epoch,
            self.history.revision(),
            self.summary.scale
        );
        if let Some((at, times)) = self.cut_times.borrow().as_ref()
            && *at == key
        {
            return times.clone();
        }
        let ctx = cut_craft_core::notes::Context {
            motion,
            scale: self.summary.scale,
            ..cut_craft_core::notes::Context::default()
        };
        let (each, total) = cut_craft_core::notes::job_minutes(&self.doc, &ctx);
        let times = std::rc::Rc::new(super::CutTimes { each, total });
        *self.cut_times.borrow_mut() = Some((key, times.clone()));
        times
    }

    /// Lets go of picked nodes that no longer stand for a node to move: all of them once
    /// Node Edit is put away, and any whose vector is no longer selected or has no node
    /// where it was picked any more (after an undo, redo or delete).
    pub(super) fn prune_picked_nodes(&mut self) {
        if self.picked_nodes.is_empty() {
            return;
        }
        if self.view_tool != Some(super::ViewTool::NodeEdit) {
            self.picked_nodes.clear();
            return;
        }
        let doc = &self.doc;
        self.picked_nodes.retain(|n| {
            doc.selection.contains(&n.vector)
                && doc.vectors.get(&n.vector).is_some_and(|path| {
                    cut_craft_core::nodes::node_near(path, n.at.into(), n.reach.max(1e-3)).is_some()
                })
        });
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
                if let Some(note) = self.history.last_notes().last() {
                    self.status.set_message(note);
                }
                for n in &mut self.picked_nodes {
                    let at = Point::from(n.at) + delta(Point::from(n.at));
                    n.at = cut_craft_core::params::Xy::new(at.x, at.y);
                }
            }
            Err(e) => self.status.set_message(&e.to_string()),
        }
    }

    /// Del in Node Edit: the picked nodes go, each shape's spans either side joined by a
    /// line, as Delete Point does, in one step to undo. A shape keeps the nodes it needs
    /// (two, or three when closed); the status bar says when some were kept for that.
    pub(super) fn delete_picked_nodes(&mut self) {
        use cut_craft_core::nodes::{delete_node, node_near};
        let mut c = Change::builder(&self.doc);
        let vectors: std::collections::BTreeSet<EntityId> =
            self.picked_nodes.iter().map(|n| n.vector).collect();
        let (mut deleted, mut kept) = (0, 0);
        for vector in vectors {
            let Some(original) = self.doc.vectors.get(&vector) else {
                continue;
            };
            let mut picked: Vec<usize> = (self.picked_nodes.iter())
                .filter(|n| n.vector == vector)
                .filter_map(|n| node_near(original, n.at.into(), n.reach.max(1e-3)))
                .collect();
            // the last first, so the nodes still to go keep their places
            picked.sort_unstable_by(|a, b| b.cmp(a));
            picked.dedup();
            let mut path = original.clone();
            for k in picked {
                match delete_node(&path, k) {
                    Some(fewer) => {
                        path = fewer;
                        deleted += 1;
                    }
                    None => kept += 1,
                }
            }
            if path != *original {
                c.replace(vector, path);
            }
        }
        let change = c.finish();
        self.picked_nodes.clear();
        if deleted > 0
            && let Err(e) = self.history.apply(&mut self.doc, change)
        {
            self.status.set_message(&e.to_string());
            return;
        }
        let s = |n: usize| if n == 1 { "" } else { "s" };
        let mut said = format!("Deleted {deleted} node{}", s(deleted));
        if kept > 0 {
            said = format!(
                "{said}. {kept} node{} kept: a shape needs at least two (three when \
                 closed). Delete the whole shape instead",
                s(kept)
            );
        }
        self.status.set_message(&said);
    }

    /// The arrow keys nudge the selection by the snapping grid's squares: a whole one, half
    /// with Shift, a tenth with Ctrl + Shift (Matthew's steps); with Ctrl alone, by Snapping Options' Fixed Nudge Distance.
    pub(super) fn nudge(&mut self, ctx: &egui::Context) {
        if self.screen != Screen::Job || ctx.egui_wants_keyboard_input() {
            return;
        }
        let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
        let g = self.options.snapping.grid_spacing;
        let step = match (shift, ctrl) {
            (true, true) => 0.1 * g,
            (true, false) => 0.5 * g,
            // Snapping Options' Fixed Nudge Distance
            (false, true) => self.options.snapping.fixed_nudge,
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
        self.prune_picked_nodes();
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

    /// Ctrl+X, Ctrl+C and Ctrl+V, which egui gives as clipboard events rather than keys.
    fn clipboard_keys(&mut self, ctx: &egui::Context) {
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
        // each only while Options still gives the action its key (taken away, it does nothing)
        let bindings = self.bindings(ctx);
        let bound = |id: &str, key: Key| {
            let keys = KeyboardShortcut::new(Modifiers::COMMAND, key);
            bindings.contains(&(id.to_owned(), shortcut::Trigger::Key(keys)))
        };
        let (cut, copy) = (cut && bound("cut", Key::X), copy && bound("copy", Key::C));
        let pasted = pasted.filter(|_| bound("paste", Key::V));
        if !ctx.egui_wants_keyboard_input() && self.screen == Screen::Job {
            if cut && self.available("cut") {
                self.invoke(ctx, "cut");
            } else if copy && self.available("copy") {
                self.invoke(ctx, "copy");
            }
            if let Some(paths) = pasted.and_then(|text| serde_json::from_str(&text).ok()) {
                // vectors another window copied: what they belonged to there is not known,
                // and this window's own copied components are not part of them
                if paths != self.clipboard {
                    self.clipboard_from = super::Copied::default();
                    self.clipboard_components.clear();
                }
                self.clipboard = paths;
                self.invoke(ctx, "paste");
            }
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        // while a slow job runs, the window only shows its progress; while a shortcut is
        // being set, the keys and buttons pressed are for it
        if self.busy.is_some() || self.capture.is_some() {
            return;
        }
        self.clipboard_keys(ctx);
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
        self.run_shortcuts(ctx);
        // Escape during a 3D handle drag cancels only the drag, staying in Transform mode
        let escape = KeyboardShortcut::new(Modifiers::NONE, Key::Escape);
        if self.drag_3d.is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.cancel_drag_3d();
        }
        // Escape with an open end picked lets the end go, keeping Join Open Ends open
        if self.join_ends_first().is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.join_ends_forget();
            self.status.set_message("End let go");
        }
        // Escape stops a preview playing (keeping what it has cut), and does nothing else
        if self.playback.is_some()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.finish_playback();
            self.status
                .set_message("Preview stopped: what it had cut is kept");
        }
        // Escape with no form open, stepped inside a group, steps out one level
        if let Some(group) = self.doc.inside_group
            && self.form.is_none()
            && !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_shortcut(&escape))
        {
            self.step_out_of_group(group);
        }
        // Escape in a field being typed in only leaves the field
        if !ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| {
                i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            })
        {
            // a window in front (Find Tool, the library's Properties) closes alone
            let in_front = self.search.take().is_some() || self.clipart.close_properties();
            if !in_front {
                self.escape_closes();
            }
        }
        self.nudge(ctx);
    }

    /// Whether `id` is a placing tool (Transform Objects: Move, Rotate, Set Size, Mirror,
    /// Distort), which Esc leaves for the form it was opened over.
    fn placing(&self, id: &str) -> bool {
        self.registry
            .get(id)
            .is_some_and(|c| c.meta().group == "Transform Objects")
    }

    /// Esc with nothing in front: the open form closes (Import 3D Model's as its Cancel), with
    /// a new job's setup, the view tool and the pages it opened. A placing tool's form opened
    /// over another goes back to that one, edits and all; what it placed stays placed (the
    /// toolpaths it changes are out of date, as after any move).
    fn escape_closes(&mut self) {
        let behind = self.form_behind.take();
        if let Some(mut behind) = behind
            && self.form.as_ref().is_some_and(|f| self.placing(f.id))
        {
            // what was placed is kept: Cancel on the form back now leaves it
            behind.undo_from = self.history.undo_depth();
            behind.list_steps.clear();
            if let Some(placing) = self.form.take().filter(|f| f.page != behind.page) {
                self.pages.done_with(placing.page);
            }
            if self.pages.shown(behind.page) == Shown::Hidden {
                self.pages.open(behind.page);
            }
            behind.label.clone_into(&mut self.status.current_tool);
            self.status
                .set_message(&format!("Back to {}, as you left it", behind.label));
            self.form = Some(behind);
            return;
        }
        if self.screen == Screen::NewJob {
            self.job_setup_cancel();
        }
        // a form whose buttons are Import and Cancel: Esc is its Cancel
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.id == "orientate_3d_model")
        {
            self.cancel_form();
        }
        self.form = None;
        self.view_tool = None;
        self.pages.done_with(Page::Design);
        self.pages.done_with(Page::Toolpaths);
    }

    /// Runs the actions whose shortcut was pressed: a key's, or a mouse button's.
    fn run_shortcuts(&mut self, ctx: &egui::Context) {
        // single-letter shortcuts must not fire while typing in a form field; a mouse
        // button's can, as it types nothing
        let typing = ctx.egui_wants_keyboard_input();
        let job = self.screen == Screen::Job;
        let fired: Vec<String> = shortcut::most_specific_first(self.bindings(ctx))
            .into_iter()
            .filter(|(id, _)| id != shortcut::PAN_DRAG)
            .filter(|(id, _)| job || ["new", "open", "tool_help"].contains(&id.as_str()))
            .filter(|(id, trigger)| match trigger {
                // help and the views work from a field being typed in too, as they always did,
                // while on a function key (which types nothing)
                shortcut::Trigger::Key(keys) => {
                    let name = keys.logical_key.name();
                    let function = name.len() > 1 && name.starts_with('F');
                    let any_time =
                        function && ["tool_help", "view_2d", "view_3d"].contains(&id.as_str());
                    (!typing || any_time) && ctx.input_mut(|i| i.consume_shortcut(keys))
                }
                shortcut::Trigger::Mouse(held, button) => ctx.input(|i| {
                    i.pointer.button_pressed(*button) && i.modifiers.matches_exact(*held)
                }),
            })
            .map(|(id, _)| id)
            .collect();
        for id in fired {
            self.invoke(ctx, &id);
        }
    }

    /// Every action's own shortcuts: each command's, the built-ins' as the menus spell them,
    /// the layouts', and dragging with the wheel pressed to pan.
    pub(super) fn default_bindings(&self) -> Vec<(String, shortcut::Trigger)> {
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
        let keys = commands.chain(built_ins).chain(layouts);
        let mut bindings: Vec<_> = keys
            .map(|(id, keys)| (id, shortcut::Trigger::Key(keys)))
            .collect();
        bindings.extend(
            shortcut::parse_trigger(shortcut::PAN_DRAG_DEFAULT)
                .map(|t| (shortcut::PAN_DRAG.to_owned(), t)),
        );
        bindings
    }

    /// Every shortcut in use: [`Self::default_bindings`] with those set in Options in their
    /// place, and on a Mac its own keys for some of them ([`super::mac::extra_bindings`]).
    pub(super) fn bindings(&self, ctx: &egui::Context) -> Vec<(String, shortcut::Trigger)> {
        self.bindings_with(ctx, &self.options.controls.shortcuts)
    }

    /// [`Self::bindings`] with `set` as the shortcuts set in Options: those in the Options
    /// window, not applied yet, when a new one is checked for clashes.
    pub(super) fn bindings_with(
        &self,
        ctx: &egui::Context,
        set: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> Vec<(String, shortcut::Trigger)> {
        let mut bindings = shortcut::effective(self.default_bindings(), set);
        if super::mac::is_mac(ctx) {
            // ⌘Q quits there: Quick Engrave's Ctrl+Q is no shortcut on a Mac
            bindings.retain(
                |(_, t)| !matches!(t, shortcut::Trigger::Key(k) if super::mac::kept_by_the_mac(*k)),
            );
            let keys: Vec<(String, KeyboardShortcut)> = bindings
                .iter()
                .filter_map(|(id, t)| match t {
                    shortcut::Trigger::Key(k) => Some((id.clone(), *k)),
                    shortcut::Trigger::Mouse(..) => None,
                })
                .collect();
            let extra = super::mac::extra_bindings(&keys);
            bindings.extend(
                extra
                    .into_iter()
                    .map(|(id, k)| (id, shortcut::Trigger::Key(k))),
            );
        }
        bindings
    }

    /// The mouse buttons that pan either view while held and dragged.
    pub(super) fn pan_buttons(&self) -> Vec<egui::PointerButton> {
        let defaults = (shortcut::parse_trigger(shortcut::PAN_DRAG_DEFAULT))
            .map(|t| (shortcut::PAN_DRAG.to_owned(), t));
        shortcut::effective(
            defaults.into_iter().collect(),
            &self.options.controls.shortcuts,
        )
        .into_iter()
        .filter(|(id, _)| id == shortcut::PAN_DRAG)
        .filter_map(|(_, t)| t.button())
        .collect()
    }

    /// Whether a pan button ([`Self::pan_buttons`]) is held down.
    pub(super) fn pan_button_down(&self, ctx: &egui::Context) -> bool {
        let buttons = self.pan_buttons();
        ctx.input(|i| buttons.iter().any(|&b| i.pointer.button_down(b)))
    }

    /// Every action a shortcut can be set for, by id, with its name as the menus write it:
    /// panning by dragging first, then the commands and the built-in menu items by name.
    pub(super) fn shortcut_actions(&self) -> Vec<(String, String)> {
        let mut actions: Vec<(String, String)> = vec![];
        let named = self.menus.items();
        let name_of = |id: &str| {
            named
                .iter()
                .find(|i| i.action == id)
                .map(|i| i.name.clone())
                .or_else(|| {
                    (KEY_ACTION_NAMES.iter())
                        .find(|(a, _)| *a == id)
                        .map(|(_, name)| (*name).to_owned())
                })
                .unwrap_or_else(|| gui::title_case(id))
        };
        actions.extend(
            self.registry
                .iter()
                .map(|c| (c.id().to_owned(), c.meta().label.to_owned())),
        );
        for id in BUILT_INS.iter().chain(LAYOUT_KEYS.iter().map(|(id, _)| id)) {
            if !actions.iter().any(|(a, _)| a == id) {
                actions.push(((*id).to_owned(), name_of(id)));
            }
        }
        actions.sort_by(|a, b| a.1.cmp(&b.1));
        // panning first: the mouse's own, and what most people come here to set
        let pan = (
            shortcut::PAN_DRAG.to_owned(),
            "Pan (hold and drag)".to_owned(),
        );
        actions.insert(0, pan);
        actions
    }

    /// The shortcut a menu shows for `id`: the first one set in Options, else the command's
    /// own; `None` leaves the menu item's own.
    pub(super) fn shortcut_text(&self, id: &str) -> Option<String> {
        match self.options.controls.shortcuts.get(id) {
            Some(set) => Some(set.first().cloned().unwrap_or_default()),
            None => self.registry.get(id).map(|c| c.meta().shortcut.to_owned()),
        }
    }

    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Menu bar");
        let mut clicked = None;
        let recent: Vec<gui::MenuItem> = self
            .recent_shown()
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
                            why: &|id| self.unavailable_why(id),
                            checked: &|id| {
                                self.display
                                    .checked(id)
                                    .or_else(|| self.resolution_checked(id))
                                    .or_else(|| self.units_checked(id))
                                    .or_else(|| self.layout_checked(id))
                            },
                            shortcut_of: &|id| self.shortcut_text(id),
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

    /// No job open: the startup tasks and the jobs opened lately.
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
                .on_disabled_hover_text(self.unavailable_why(id));
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
        for (i, path) in self.recent_shown().iter().enumerate() {
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
        // the rest of the community links are in Help ▸ Community, out of the way
        let (label, url, _) = LINKS[0];
        ui.horizontal_wrapped(|ui| {
            ui.weak("Questions or ideas?");
            ui.hyperlink_to(format!("Cut Craft on {label}"), url);
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The middle of the window with no job open: the Cut Craft logo, which carries the
    /// name itself, so no heading under it.
    pub(super) fn splash(ui: &mut egui::Ui) {
        let space = ui.available_size();
        let edge = space.min_elem() * SPLASH_SHARE;
        ui.vertical_centered(|ui| {
            ui.add_space((space.y - edge) / 2.0);
            ui.add(
                egui::Image::from_bytes("bytes://splash-logo.jpg", SPLASH_LOGO)
                    .fit_to_exact_size(egui::vec2(edge, edge))
                    .corner_radius(edge * 0.06),
            );
        });
    }

    /// The Design page: the chosen design tab's tools, under any form opened from them.
    pub(super) fn design_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design page");
        self.page_header(ui, Page::Design, DESIGN_TABS[self.design_tab]);
        let tab = DESIGN_TABS[self.design_tab];
        // a tool's form takes the page's place
        if self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
            self.form_panel(ui);
            return;
        }
        // room kept at the foot for a tool's red note, so the icons above never move for it
        let note = self.tool_note_height(ui);
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - note)
            .show(ui, |ui| {
                match tab {
                    "Design" => {
                        if self.options.favourites.of(Page::Design).1 {
                            // favourites name every tool, whatever sub-tab it is on
                            let groups = self.design_groups("Design");
                            self.tools_or_favourites(ui, &groups, Page::Design);
                        } else {
                            self.design_subtabs(ui);
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
        self.tool_note_label(ui);
        // the hovered tool's guidance, or the tool in hand's, in the room left under the icons
        if tab == "Design" {
            self.tip_box(ui);
        }
    }

    /// What the last tool with no form warned of (Group turning text into curves), in red,
    /// while nothing has changed since.
    pub(super) fn tool_note_label(&self, ui: &mut egui::Ui) {
        if let Some(note) = self.tool_note() {
            ui.colored_label(ui.visuals().error_fg_color, note);
        }
    }

    /// The red note [`Self::tool_note_label`] shows, if any.
    pub(super) fn tool_note(&self) -> Option<&str> {
        let (job, revision, note) = self.tool_note.as_ref()?;
        ((*job, *revision) == (self.job_epoch, self.history.revision())).then_some(note.as_str())
    }

    /// How tall [`Self::tool_note_label`] draws at `ui`'s width, with the gap over it.
    fn tool_note_height(&self, ui: &egui::Ui) -> f32 {
        self.tool_note().map_or(0.0, |note| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let colour = egui::Color32::PLACEHOLDER;
            let galley = ui
                .painter()
                .layout(note.to_owned(), font, colour, ui.available_width());
            galley.size().y + ui.spacing().item_spacing.y
        })
    }

    /// The tool groups a design tab shows: Design has the drawing then the modeling tools.
    pub(super) fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        match tab {
            "Design" => self.toolbar.all_design_groups(),
            _ => vec![],
        }
    }

    /// The Design tab's sub-tabs, under the groups always shown: Create, Edit, Arrange,
    /// Model, Import (see [`Self::subtabs`]).
    pub(super) fn design_subtabs(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design sub-tabs");
        self.subtabs(ui, Page::Design);
    }

    /// A page's sub-tabs, under the groups always shown. Each names what its tools are for on
    /// hover, so a beginner looks under a word rather than down one long page of icons. A
    /// right-click moves one out of the row into a section of its own below it (or back), so
    /// tools used together can all show at once; the choice is kept in the options. The
    /// Design and Toolpaths pages follow the same rules.
    pub(super) fn subtabs(&mut self, ui: &mut egui::Ui, page: Page) {
        let open = self.subtab_head(ui, page);
        self.subtab_body(ui, page, open);
    }

    /// The top of a page with sub-tabs: the groups always shown, then the tab row. Returns
    /// the sub-tab open in the row, if any. The Toolpaths page keeps this in place while the
    /// sub-tab under it scrolls, so the tabs are always there to go back with.
    pub(super) fn subtab_head(&mut self, ui: &mut egui::Ui, page: Page) -> Option<usize> {
        /// The share of the usual padding and spacing the sub-tabs keep between them.
        const SUBTAB_PADDING: f32 = 0.5;
        let set = subtab_set(page);
        let always = match page {
            Page::Design => self.toolbar.named(set.always),
            Page::Toolpaths => self.toolbar.toolpath_named(set.always),
        };
        self.tool_groups(ui, &always, page);
        ui.separator();
        let sections = self.subtab_sections(page).to_vec();
        let (row, _) = set.layout(&sections);
        let open = set.open(*self.open_subtab_mut(page), &sections);
        let (mut picked, mut chosen) = (None, None);
        if !row.is_empty() {
            ui.horizontal_wrapped(|ui| {
                // a little tighter than a button row, so all five fit across the page
                ui.spacing_mut().button_padding.x *= SUBTAB_PADDING;
                ui.spacing_mut().item_spacing.x *= SUBTAB_PADDING;
                for &i in &row {
                    let tab = &set.tabs[i];
                    let response = ui
                        .selectable_label(Some(i) == open, tab.name)
                        .on_hover_hint(format!(
                            "{}. Right-click to show it as its own section",
                            tab.hint
                        ));
                    if response.clicked() {
                        picked = Some(i);
                    }
                    if let Some(action) = self.popup(&response, || set.menu(tab.name, &sections)) {
                        chosen = Some(action);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
        let open = picked.or(open);
        if let Some(i) = open {
            *self.open_subtab_mut(page) = i;
        }
        open
    }

    /// Under the tab row: the open sub-tab's tools, then each sub-tab shown as a section of
    /// its own.
    pub(super) fn subtab_body(&mut self, ui: &mut egui::Ui, page: Page, open: Option<usize>) {
        let set = subtab_set(page);
        let sections = self.subtab_sections(page).to_vec();
        let (_, apart) = set.layout(&sections);
        let mut chosen = None;
        if let Some(i) = open {
            self.subtab_contents(ui, page, i);
        }
        for i in apart {
            let tab = &set.tabs[i];
            ui.separator();
            // a tool picked elsewhere unfolds its section once; it can be folded again
            let unfold = (*self.unfold_mut(page) == Some(tab.name)).then_some(true);
            if unfold.is_some() {
                *self.unfold_mut(page) = None;
            }
            let label = egui::RichText::new(tab.name).strong();
            let header = egui::CollapsingHeader::new(label)
                .id_salt((page_key(page), "section", tab.name))
                .default_open(true)
                .open(unfold)
                .show(ui, |ui| self.subtab_contents(ui, page, i));
            let response = header.header_response.on_hover_hint(format!(
                "{}. Right-click to put it back in the tab row",
                tab.hint
            ));
            if let Some(action) = self.popup(&response, || set.menu(tab.name, &sections)) {
                chosen = Some(action);
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// What sub-tab `i` of `page` shows: its groups' tools, or the Toolpaths page's own.
    fn subtab_contents(&mut self, ui: &mut egui::Ui, page: Page, i: usize) {
        match page {
            Page::Design => {
                let groups = self.toolbar.subtab_groups(i);
                self.tool_groups(ui, &groups, Page::Design);
            }
            Page::Toolpaths => self.toolpath_subtab(ui, i),
        }
    }

    /// The sub-tabs of `page` shown as sections of their own.
    fn subtab_sections(&self, page: Page) -> &[String] {
        match page {
            Page::Design => &self.options.design_sections,
            Page::Toolpaths => &self.options.toolpath_sections,
        }
    }

    /// The open sub-tab of `page`.
    fn open_subtab_mut(&mut self, page: Page) -> &mut usize {
        match page {
            Page::Design => &mut self.design_subtab,
            Page::Toolpaths => &mut self.toolpath_subtab,
        }
    }

    /// The section of `page` to unfold next frame.
    fn unfold_mut(&mut self, page: Page) -> &mut Option<&'static str> {
        match page {
            Page::Design => &mut self.design_unfold,
            Page::Toolpaths => &mut self.toolpath_unfold,
        }
    }

    /// Opens the sub-tab `id` is on, so a tool picked from Find Tool, a menu or a shortcut
    /// shows highlighted where it lives: its tab in the row, or its section unfolded.
    pub(super) fn show_subtab_of(&mut self, id: &str) {
        let design = self.toolbar.subtab_of(id, self.design_subtab);
        let toolpaths = self.toolbar.toolpath_subtab_of(id, self.toolpath_subtab);
        for (page, tab) in [(Page::Design, design), (Page::Toolpaths, toolpaths)] {
            if let Some(tab) = tab {
                self.open_subtab(page, tab);
            }
        }
    }

    /// Opens sub-tab `tab` of `page`: in the row, or its section unfolded.
    pub(super) fn open_subtab(&mut self, page: Page, tab: usize) {
        let name = subtab_set(page).tabs[tab].name;
        if self.subtab_sections(page).iter().any(|s| s == name) {
            *self.unfold_mut(page) = Some(name);
        } else {
            *self.open_subtab_mut(page) = tab;
        }
    }

    /// A page's title with its push-pin and the Switch Layout button.
    pub(super) fn page_header(&mut self, ui: &mut egui::Ui, page: Page, title: &str) {
        let row = ui.spacing().interact_size.y;
        let (mut pin, mut switch, mut help) = (false, false, false);
        ui.horizontal(|ui| {
            ui.strong(title);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pinned = self.pages.pinned(page);
                pin = ui
                    .add(self.pin_button(pinned, row))
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
        // from the window's room alone, before anything is drawn in it
        let width = page_width(ui);
        let margin = style.spacing.item_spacing.y;
        let strip = || egui::Frame::side_top_panel(&style).inner_margin(margin);
        let left_tabs = egui::Panel::left("design_tabs")
            .resizable(false)
            .exact_size(side_tabs_width(&style))
            .frame(strip())
            .show(ui, |ui| self.design_tabs(ui))
            .response
            .rect;
        let right_tabs = egui::Panel::right("toolpath_tabs")
            .resizable(false)
            .exact_size(side_tabs_width(&style))
            .frame(strip())
            .show(ui, |ui| self.toolpath_tabs(ui))
            .response
            .rect;
        self.page_rects = [None, None];
        self.flyout_rect = None;
        if self.pages.shown(Page::Design) == Shown::Docked {
            let rect = show_page(egui::Panel::left("design_panel"), width, ui, |ui| {
                self.design_page(ui);
            });
            self.page_rects[0] = Some(rect);
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            let rect = show_page(egui::Panel::right("toolpaths_panel"), width, ui, |ui| {
                self.toolpaths_page(ui);
            });
            self.page_rects[1] = Some(rect);
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
            let out = self.flyout(ctx, page, room, width);
            self.flyout_rect = Some(out);
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
        // a button that opens another design tab leaves the page out to show it
        let navigates = matches!(id, "clipart_library" | "component_levels");
        if self.form.is_none() && !navigates {
            self.pages.done_with(page);
        }
    }

    /// The design tabs, down the left edge.
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

    /// Going to another design tab closes a form open on the Design page.
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

    /// An auto-hidden page flown out over the view from its side, `outer` wide as a docked
    /// page is; returns where it is.
    pub(super) fn flyout(
        &mut self,
        ctx: &egui::Context,
        page: Page,
        room: egui::Rect,
        outer: f32,
    ) -> egui::Rect {
        let frame = egui::Frame::side_top_panel(&ctx.global_style());
        let margins = frame.total_margin().sum();
        let (width, height) = (outer - margins.x, room.height() - margins.y);
        let left = match page {
            Page::Design => room.left(),
            Page::Toolpaths => room.right() - outer,
        };
        egui::Area::new(egui::Id::unique(("flyout", page)))
            .fixed_pos(egui::pos2(left, room.top()))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(height);
                    // laid out apart, so contents asking for more room never widen it
                    let room =
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, height));
                    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(room));
                    inside.shrink_clip_rect(room);
                    match page {
                        Page::Design => self.design_page(&mut inside),
                        Page::Toolpaths => self.toolpaths_page(&mut inside),
                    }
                });
            })
            .response
            .rect
    }

    /// The open form's commit button, its label and hint, if pressing it would do something:
    /// OK for a new job's setup, else named for what it does ([`commit`]).
    fn commit_button(&self) -> Option<(&'static str, &'static str)> {
        let open = self.form.as_ref()?;
        if open.new_job {
            return Some(("OK", "Accept these settings and start the job"));
        }
        let drawing = self.drawing_tool().is_some();
        let made = self.drawn_shape().is_some() || self.remade_last().is_some();
        // opened on an embellishment it made (E): pressing it changes that one
        let reopened = !made
            && open.id == <cmd_embellish::Embellish as cut_craft_core::Command>::ID
            && cmd_embellish::edits_selection(&self.doc);
        commit::commit(open.id, open.page, drawing, made).map(|c| {
            let c = if reopened && c == commit::Commit::Create {
                commit::Commit::Apply
            } else {
                c
            };
            let again = open.editing.is_some() || open.made.is_some();
            (c.label(), commit::hint(c, drawing, again))
        })
    }

    /// The open form's buttons: OK and Cancel for a new job's setup; else its commit button
    /// when pressing it would do something (named for what it does, [`commit`]), Close and
    /// Cancel.
    pub(super) fn form_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        if open.id == "orientate_3d_model" {
            return self.import_buttons(ui);
        }
        let new_job = open.new_job;
        let committing = self.commit_button();
        let drawing = self.drawing_tool().is_some();
        let made = self.drawn_shape().is_some() || self.remade_last().is_some();
        let close_label = if new_job { "Cancel" } else { "Close" };
        let job_setup = open.id == "job_setup";
        // what the form has done that Cancel would take back
        let done = self.history.undo_depth() > open.undo_from + open.list_steps.len();
        let (mut apply, mut close, mut default, mut cancel) = (false, false, false, false);
        let mut new_shape = false;
        let as_new_offered = self.offers_calculate_as_new();
        let (save, load) = if open.page == Page::Toolpaths {
            settings_file_row(ui)
        } else {
            (false, false)
        };
        if committing.is_none() {
            new_shape = live_row(ui, open.id, drawing, made);
        }
        let as_new = as_new_offered && calculate_as_new_row(ui);
        let mut reset = false;
        ui.horizontal(|ui| {
            if let Some((label, hint)) = committing {
                apply = marked_commit_button(ui, label, hint);
            }
            reset = reset_button(ui, open.id);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui
                    .button(close_label)
                    .on_hover_hint(if new_job {
                        "Leave the job as it was"
                    } else {
                        "Close, keeping what is there"
                    })
                    .clicked();
                if !new_job {
                    cancel = ui
                        .add_enabled(done, egui::Button::new("Cancel"))
                        .on_hover_hint(
                            "Undo what this tool did since it opened, and close it. A change \
                             made in the toolpath list meanwhile is kept",
                        )
                        .on_disabled_hover_text("Nothing done yet to take back")
                        .clicked();
                }
                if job_setup {
                    default = ui
                        .button("Set as Default")
                        .on_hover_hint(
                            "Start every new job with these settings (your Clearing \
                             Settings default is kept)",
                        )
                        .clicked();
                }
            });
        });
        if new_shape {
            if let Some(form) = self.form.as_mut() {
                (form.drawn, form.applied) = (None, None);
            }
            return;
        }
        if as_new {
            self.calculate_as_new();
            return;
        }
        if reset {
            self.reset_form();
            return;
        }
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
                self.calculate_checked(id, &params);
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
                .and_then(|path| saving_path(&path, &[files::FORM_SETTINGS_EXTENSION]))
            else {
                return;
            };
            self.save_form_settings(&path);
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
                    .on_hover_hint("Close without importing the model (Esc does the same)")
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

    /// Reset on a form: every setting back to its default, and what it made (or is editing)
    /// made again with them, as if it were new.
    pub(super) fn reset_form(&mut self) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let Some(command) = self.registry.get(form.id) else {
            return;
        };
        let mut fresh = command.default_params();
        // still the embellishment it was opened on
        if let Some(editing) = form.params.get("editing").filter(|e| !e.is_null()) {
            fresh["editing"] = editing.clone();
        }
        form.params = fresh;
        form.error = None;
        form.warning = None;
        form.dirty = true;
    }

    /// Cancel on a tool's form: everything done since the form opened is undone (and can be
    /// redone), and the form closes. What was changed in the toolpath list beside it
    /// meanwhile is not the form's: it is made again after (and Redo then has nothing).
    pub(super) fn cancel_form(&mut self) {
        let Some(form) = self.form.take() else { return };
        while self.history.undo_depth() > form.undo_from && self.history.undo(&mut self.doc) {}
        for (id, params) in &form.list_steps {
            if let Some(params) = self.list_step_again(id, params)
                && let Err(e) = self.apply(id, &params)
            {
                self.status.set_message(&e);
            }
        }
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
        let step = self.form_guidance();
        if FIXED_TEXT_FORMS.contains(&open.id) {
            // a fixed place for the Next line, there or not, so the form never jumps
            let rows = ui.text_style_height(&egui::TextStyle::Body) * NEXT_LINE_ROWS;
            crate::form::fixed_area(ui, egui::vec2(ui.available_width(), rows), |ui| {
                if let Some(step) = step {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong("Next:");
                        ui.label(step);
                    });
                }
            });
        } else if let Some(step) = step {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Next:");
                ui.label(step);
            });
        }
        // what a toolpath (or the Vector Selector) will be made from, and what it leaves out
        if open.id.ends_with("_toolpath") || open.id == "vector_selector" {
            let count = self.doc.selected_vector_count();
            if let Some(text) = count.text() {
                ui.weak(text)
                    .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
            if closed_only_toolpath(open.id, &open.params)
                && let Some(left_out) = count.left_out(open.label)
            {
                ui.colored_label(ui.visuals().warn_fg_color, left_out)
                    .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
            if let Some(area) = texture_area_note(open.id, &open.params, count) {
                if count.open > 0 {
                    ui.colored_label(ui.visuals().error_fg_color, area)
                } else {
                    ui.weak(area)
                }
                .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
        }
    }

    /// The open form filling `ui`: its settings scroll, while what went wrong and the
    /// buttons stay pinned at the foot, so Apply (or Calculate) and the reason it failed are
    /// never scrolled out of sight.
    pub(super) fn form_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("form_foot").show(ui, |ui| {
            ui.add_space(ui.spacing().item_spacing.y);
            self.form_error(ui);
            self.form_buttons(ui);
        });
        // Notes fill the panel with their own editor and Variable Explorer
        if self.form.as_ref().is_some_and(|f| f.id == "notes") {
            cut_craft_bug_report::mark(ui, "Tool form: Notes");
            self.form_heading(ui);
            self.notes_editor(ui);
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
    }

    /// The warning for a toolpath form whose tool is a drill (it cuts only straight down)
    /// when the toolpath is not Drilling.
    fn drill_sideways(form: &super::OpenForm) -> Option<String> {
        let tool = form.params.get("tool")?;
        if !form.id.ends_with("_toolpath") || tool.get("drill_only") != Some(&Value::Bool(true)) {
            return None;
        }
        let tool = cut_craft_core::toolpath::Tool {
            name: tool.get("name")?.as_str()?.to_owned(),
            drill_only: true,
            ..Default::default()
        };
        cut_craft_core::preflight::drill_sideways(form.id, &tool)
    }

    /// The warning for a toolpath form whose tool is the wrong kind: a router bit on a laser
    /// toolpath, or a laser on a router's (as soon as it is picked; never refused).
    fn wrong_kind_of_tool(form: &super::OpenForm) -> Option<String> {
        if !form.id.ends_with("_toolpath") {
            return None;
        }
        let tool: cut_craft_core::toolpath::Tool =
            serde_json::from_value(form.params.get("tool")?.clone()).ok()?;
        let laser_form = super::is_laser_form(form.id);
        if laser_form && !tool.is_laser() {
            Some(format!(
                "{} is not a laser: this toolpath takes its diameter as the kerf and its feed \
                 as the laser's speed. Select… a Laser from the Tool Database",
                tool.name
            ))
        } else if !laser_form && tool.shape == cut_craft_core::toolpath::ToolShape::Laser {
            Some(format!(
                "{} is a laser, which cannot cut this toolpath: Select… a bit",
                tool.name
            ))
        } else {
            None
        }
    }

    /// The warning for a Toolpath Tiling form whose tiles, each with its overlap all round
    /// (what a tile's file covers), are bigger than the active machine's bed; none when the
    /// machine's bed size was not given.
    fn tiles_past_bed(&self, form: &super::OpenForm) -> Option<String> {
        if form.id != "toolpath_tiling" || form.params["enabled"] != Value::Bool(true) {
            return None;
        }
        // a bed whose size was not given (0) is nothing to check against
        let machine = self
            .machines
            .active()
            .filter(|m| m.width > 0.0 && m.height > 0.0)?;
        let tiling: cut_craft_core::tiling::Tiling =
            serde_json::from_value(form.params["tiling"].clone()).ok()?;
        let file = tiling.largest_file(self.doc.job.area());
        if file.x <= machine.width + 1e-9 && file.y <= machine.height + 1e-9 {
            return None;
        }
        let units = self.doc.job.units;
        let length = |mm: f64| super::trimmed(mm / units.per());
        Some(format!(
            "A tile with its overlap is {} × {} {}, bigger than {}'s bed ({} × {}): make the \
             tiles or the overlap smaller",
            length(file.x),
            length(file.y),
            units.suffix(),
            machine.name,
            length(machine.width),
            length(machine.height),
        ))
    }

    /// What Model Position in Material needs beside its form's settings: the job's
    /// material and the model's height (measured only while a form showing it is open).
    pub(super) fn placement_room(&self) -> super::placement::Room {
        let shows = self
            .form
            .as_ref()
            .is_some_and(|f| matches!(f.id, "material_setup" | "orientate_3d_model"));
        let tall = if shows && !self.doc.components.is_empty() {
            let revision = self.history.revision();
            match self.model_tall.get() {
                Some((at, tall)) if at == revision => tall,
                _ => {
                    let tall = cmd_scale_model_z_height::model_height(&self.doc);
                    let tall = if tall.is_finite() { tall } else { 0.0 };
                    self.model_tall.set(Some((revision, tall)));
                    tall
                }
            }
        } else {
            0.0
        };
        super::placement::Room {
            thickness: self.doc.job.thickness,
            tall,
            units: self.doc.job.units,
            rotary: self.doc.job.rotary.is_some(),
        }
    }

    /// Why the open form's last Apply failed, or what it warned of, in red, over its buttons.
    fn form_error(&self, ui: &mut egui::Ui) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        if FIXED_TEXT_FORMS.contains(&form.id) {
            // what the setting under the pointer does, in a box of its own under the
            // settings, always the same size
            let rows = ui.text_style_height(&egui::TextStyle::Body) * HELP_BOX_ROWS;
            let said = form::pointed_at_help(ui.ctx());
            ui.group(|ui| {
                crate::form::fixed_area(
                    ui,
                    egui::vec2(ui.available_width(), rows),
                    |ui| match said {
                        Some(help) => {
                            ui.label(help);
                        }
                        None => {
                            ui.weak("Point at a setting to see what it does.");
                        }
                    },
                );
            });
            // a fixed place for the red text, shown or not, so the settings above never
            // jump as it comes and goes
            let rows = ui.text_style_height(&egui::TextStyle::Body) * WARNING_ROWS;
            crate::form::fixed_area(ui, egui::vec2(ui.available_width(), rows), |ui| {
                self.form_error_text(ui);
            });
        } else {
            self.form_error_text(ui);
        }
    }

    fn form_error_text(&self, ui: &mut egui::Ui) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        // a drill picked for a toolpath that cuts sideways: said as soon as it is picked
        if let Some(drill) = Self::drill_sideways(form) {
            ui.colored_label(ui.visuals().error_fg_color, drill);
        }
        // a router bit on a laser toolpath, or a laser on a router's: said as it is picked
        if let Some(wrong) = Self::wrong_kind_of_tool(form) {
            ui.colored_label(ui.visuals().error_fg_color, wrong);
        }
        // tiles that, with their overlap, will not fit the machine: said as they are set
        if let Some(past) = self.tiles_past_bed(form) {
            ui.colored_label(ui.visuals().error_fg_color, past);
        }
        // a model poking out of the material where Model Position in Material puts it
        if let Some(out) = super::placement::warning(&form.params, self.placement_room()) {
            ui.colored_label(ui.visuals().error_fg_color, out);
        }
        // Calculate found nothing changed: said plainly, not as a warning
        if let Some(note) = self.form_note() {
            ui.label(note);
        }
        // what the last Apply made is not quite what was asked: said in red, never a pop-up
        if let Some(warning) = form.warning.as_ref().filter(|_| form.error.is_none()) {
            ui.colored_label(ui.visuals().error_fg_color, warning);
        }
        let Some(error) = form.error.as_ref() else {
            return;
        };
        let shown = ui.colored_label(ui.visuals().error_fg_color, error);
        // a toolpath kept out of a zone: how to let it be clipped instead
        if error.starts_with(KEEP_OUT_REFUSED) {
            shown.on_hover_text(CLIP_HOW);
        }
    }

    /// The open form's heading and settings; [`Self::form_panel`] pins its buttons under it.
    #[expect(
        clippy::too_many_lines,
        reason = "one form's closures (tool, group ends, pictures) set up in one place"
    )]
    pub(super) fn command_form(&mut self, ui: &mut egui::Ui) {
        let form = self.form.as_ref().map_or("none", |f| f.label);
        cut_craft_bug_report::mark(ui, &format!("Tool form: {form}"));
        let slots = self.slots();
        self.form_heading(ui);
        let reach = self.form_reach();
        let room = self.placement_room();
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
        let asked = std::cell::Cell::new(None);
        // a list of tools whose Select…, Edit… or Add a tool… was clicked
        let list_ask = std::cell::RefCell::new(None);
        let listed = std::cell::Cell::new(false);
        let tools = &self.tools;
        let layers: Vec<String> = self.doc.layers.iter().map(|l| l.name.clone()).collect();
        let finishing_smallest = open.id != "vcarve_engrave_toolpath";
        // a form that only uses its tool's shape (Relief from Photo) has no passes to cut
        let cuts = open.id != "relief_from_photo";
        // a surfacing toolpath flattens all it covers, and 3D Pencil has its own bits that
        // finished before and follows the model: no rest machining or vector selector
        let extras = cuts && !["surfacing_toolpath", "3d_pencil_toolpath"].contains(&open.id);
        let suggested = open.suggested.clone();
        let form_id = open.id;
        let tool = |ui: &mut egui::Ui, key: &str, params: &mut Value| {
            if key == "clearance_tools" {
                let (changed, ask) = super::clearing_section(
                    ui,
                    params,
                    &slots,
                    finishing_smallest,
                    suggested.as_deref(),
                    units,
                );
                listed.set(listed.get() | changed);
                if let Some(ask) = ask {
                    *list_ask.borrow_mut() = Some((super::ToolList::Clearing, ask));
                }
                return;
            }
            if key != "tool" {
                // the form's tools in the order they cut, each picked as the form's own tool
                let (changed, ask) = super::tools_section(ui, key, params, tools, &slots, units);
                listed.set(listed.get() | changed);
                if let Some(ask) = ask {
                    *list_ask.borrow_mut() = Some((super::ToolList::of_key(key), ask));
                }
                return;
            }
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("Tool");
                asked.set(tool_picker(ui, form_id, params, tools, &slots, units));
                if let Some(line) = &reach {
                    super::reach::show(ui, line);
                }
                listed.set(listed.get() | super::passes::edit_passes(ui, params, units));
            });
            if extras
                && super::shows_rest(params)
                && let Some(ask) = rest_section(ui, params, tools, &slots, units)
            {
                *list_ask.borrow_mut() = Some((super::ToolList::Rest, ask));
            }
            if extras {
                selector_section(ui, params, &layers, units);
            }
        };
        let stock = form_stock(&self.doc, &open.params, units, bed(&self.machines));
        let group_end = |ui: &mut egui::Ui, group: &str, params: &mut Value| {
            super::tabs::group_end(ui, group, params, stock)
                | super::placement::section_end(ui, group, params, room)
        };
        let look = form::Look {
            units,
            picture: &picture,
            material: form::material(&self.doc, units),
            tool: Some(&tool),
            group_end: Some(&group_end),
            materials: Some(&self.materials),
            pattern: Some(pattern_maker()),
        };
        let icon = |path: &str| self.icons.find(path, "").map(|p| self.icons.image(p));
        // Trace Bitmap's settings are in its trace window, beside the preview
        if open.id != "trace_bitmap" {
            open.dirty |= show_form(ui, open, &look, &self.doc.job, &icon);
        }
        open.dirty |= listed.get();
        self.form_extras(ui, form_id);
        if let Some(ask) = asked.get() {
            let name = self
                .form
                .as_ref()
                .and_then(|f| f.params["tool"]["name"].as_str().map(str::to_owned));
            match ask {
                super::ToolAsk::Select => self.select_tool(name.as_deref().unwrap_or_default()),
                super::ToolAsk::Edit => self.edit_tool(ui.ctx(), name),
            }
        }
        if let Some((which, ask)) = list_ask.into_inner() {
            self.ask_listed(ui.ctx(), which, ask);
        }
    }

    /// A list of tools' Select…, Add a tool… or Edit…: the Tool Database, to pick a tool of
    /// list `which` in or change one.
    pub(super) fn ask_listed(
        &mut self,
        ctx: &egui::Context,
        which: super::ToolList,
        ask: super::ListAsk,
    ) {
        match ask {
            super::ListAsk::Select(at) => {
                let current = match at {
                    super::ListSlot::At(i) => self.form.as_ref().and_then(|form| {
                        let list = match which {
                            super::ToolList::Clearing => &form.params["clearance_tools"],
                            super::ToolList::Tools => &form.params["tools"],
                            super::ToolList::Before => &form.params["bits_before"],
                            super::ToolList::Rest => &form.params["rest"]["after"],
                        };
                        list[i]["name"].as_str().map(str::to_owned)
                    }),
                    super::ListSlot::New => None,
                };
                self.select_tool(&current.unwrap_or_default());
                self.picking = Some(super::PickFor::Listed(which, at));
            }
            super::ListAsk::Edit(name) => self.edit_tool(ctx, Some(name)),
            super::ListAsk::Suggest => self.suggest_clearing(),
        }
    }

    /// The bits Suggest may pick from: the user's own (not a read-only catalogue's, nor a
    /// laser), each with whether the tool changer holds it.
    pub(super) fn suggest_candidates(&self) -> Vec<cmd_pocket_toolpath::Candidate> {
        let slots = self.slots();
        self.tools
            .my_tools()
            .filter(|tool| tool.shape != cut_craft_core::toolpath::ToolShape::Laser)
            .map(|tool| cmd_pocket_toolpath::Candidate {
                tool: self.pick_tool(&tool.name).unwrap_or_else(|| tool.clone()),
                loaded: slots
                    .iter()
                    .any(|s| s.as_deref() == Some(tool.name.as_str())),
            })
            .collect()
    }

    /// Pocket's Suggest clearing bits: every plan of one or two bigger bits calculated and
    /// timed off the window's thread; the quickest becomes the form's clearing bits, and
    /// the form says how long it takes against the finishing bit alone.
    pub(super) fn suggest_clearing(&mut self) {
        let Some(form) = &self.form else { return };
        let params: cmd_pocket_toolpath::Params = match serde_json::from_value(form.params.clone())
        {
            Ok(p) => p,
            Err(e) => {
                self.status
                    .set_message(&format!("Suggest could not read the form: {e}"));
                return;
            }
        };
        let candidates = self.suggest_candidates();
        let finishing_loaded = self
            .slots()
            .iter()
            .any(|s| s.as_deref() == Some(params.tool.name.as_str()));
        let changes = cmd_pocket_toolpath::Changes {
            by_hand: self.options.toolpaths.change_by_hand,
            by_changer: self.options.toolpaths.change_by_changer,
        };
        let (doc, rapid) = (self.doc.clone(), self.summary.rapid);
        let finishing = params.tool.name.clone();
        self.in_background(
            "Suggesting clearing bits",
            "suggest-clearing",
            move || {
                cmd_pocket_toolpath::suggest(
                    &doc,
                    &params,
                    &candidates,
                    finishing_loaded,
                    rapid,
                    changes,
                )
                .map_err(|e| e.to_string())
            },
            move |app, made| {
                let Some(form) = &mut app.form else { return };
                match made {
                    Ok(s) => {
                        let note = suggestion_note(&s, &finishing);
                        form.params["clearance_tools"] =
                            serde_json::to_value(&s.clearing).expect("tools serialize");
                        form.dirty = true;
                        form.suggested = Some(note.clone());
                        app.status.set_message(&note);
                    }
                    Err(e) => form.suggested = Some(format!("Suggest: {e}")),
                }
            },
        );
    }

    /// Texture Toolpath's Load… or Save…: the texture as a `.vtexture` file, in the textures
    /// folder unless taken elsewhere.
    pub(super) fn texture_file(&mut self, save: bool) {
        let extension = cmd_texture_toolpath::TEXTURE_EXTENSION;
        let picked = if save {
            texture_dialog()
                .set_file_name(format!("texture.{extension}"))
                .save_file()
                .and_then(|path| saving_path(&path, &[extension]))
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

    /// Set Size with Link XY: the box typed in sets the other, keeping the selection's
    /// proportions (in Percent, the same share), so the form always shows the size it makes,
    /// and sizes go by it. Items one by one each keep their own proportions, so there the
    /// other box (in millimetres) is left as it is.
    fn link_set_size(&mut self) {
        let size = self
            .doc
            .selection_bounds()
            .map(cut_craft_core::geometry::Rect::size);
        let Some(form) = self.form.as_mut().filter(|f| f.id == "set_size") else {
            return;
        };
        let p = &mut form.params;
        let (Some(mut w), Some(mut h)) = (p["width"].as_f64(), p["height"].as_f64()) else {
            return;
        };
        let seen = form.shown_size.map(|s| (s[0], s[1]));
        let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(1.0);
        let linked = p["keep_proportions"].as_bool() == Some(true);
        if linked && let Some((sw, sh)) = seen {
            // the box typed last is the one sizes go by
            if same(w, sw) && !same(h, sh) {
                p["by_height"] = true.into();
            } else if !same(w, sw) {
                p["by_height"] = false.into();
            }
        }
        let items = p["mode"].as_str() == Some("scale_items_individually");
        let ratio = if p["percent"].as_bool() == Some(true) {
            Some(1.0)
        } else if items {
            None
        } else {
            size.filter(|s| s.x > 0.0 && s.y > 0.0).map(|s| s.y / s.x)
        };
        if let Some(ratio) = ratio.filter(|_| linked) {
            let height_typed = p["by_height"].as_bool() == Some(true);
            if height_typed {
                w = h / ratio;
                p["width"] = w.into();
            } else if !same(h, w * ratio) {
                h = w * ratio;
                p["height"] = h.into();
            }
        }
        form.shown_size = Some([w, h, 0.0]);
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
            "set_size" => self.link_set_size(),
            "relief_from_photo" => {
                self.depth_section(ui);
                self.bit_note(ui);
            }
            "profile_toolpath" => {
                self.edit_tabs(ui);
                self.deep_cut_warning(ui.ctx());
            }
            "trace_bitmap" => self.trace_form(ui),
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

    /// Relief from Photo, with Only What the Bit Can Carve ticked: says when the job's 3D
    /// model is too coarse to hold what the bit can cut, and what makes it finer.
    fn bit_note(&self, ui: &mut egui::Ui) {
        use cut_craft_core::document::Resolution;
        let Some(form) = &self.form else { return };
        if form.params["only_what_the_bit_can_carve"] != true {
            return;
        }
        let tip = form.params["tool"]["diameter"].as_f64().unwrap_or(0.0);
        let job = &self.doc.job;
        let finest = (job.width * job.height / Resolution::Highest.points()).sqrt();
        let (per, unit) = form::shown_in("mm", job.units).unwrap_or((1.0, "mm"));
        let places = if unit == "in" { 3 } else { 2 };
        let length = |mm: f64| format!("{:.places$} {unit}", mm / per);
        if let Some(note) =
            cmd_relief_from_photo::resolution_note(job.model_cell(), finest, tip, length)
        {
            ui.label(note);
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
        let name = "The Depth add-on";
        if let (true, Some(installed), Some(id)) = (start, installed, chosen)
            && self.addons.may_run(crate::depth::ADDON_ID, name)
        {
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

    /// Reads the user's Materials Library (the starter list when there is none); a broken
    /// one is reported, kept on disk and not saved over.
    pub(super) fn load_materials(&mut self) {
        self.load_materials_at(files::materials_path());
    }

    /// [`Self::load_materials`] from `path`.
    pub(super) fn load_materials_at(&mut self, path: Option<std::path::PathBuf>) {
        self.materials_path = path;
        if let Some(path) = &self.materials_path {
            match files::load_materials(path) {
                Ok(lib) => self.materials = lib,
                Err(e) => {
                    self.status
                        .set_message(&format!("{e}; material changes will not be saved"));
                    self.materials_unreadable = self.materials_path.take();
                }
            }
        }
    }

    /// Writes the Materials Library, if it has a file; an error when it could not be
    /// written, or when its file could not be read at start and so is not saved over.
    pub(super) fn save_materials(&self) -> Result<(), String> {
        if let Some(path) = &self.materials_unreadable {
            return Err(not_saved("Materials Library", path));
        }
        self.materials_path.as_deref().map_or(Ok(()), |p| {
            files::write_creating(p, &self.materials.to_json())
        })
    }

    /// Tools renamed in the Tool Database, (old name, new name): the job's toolpaths that
    /// cut with them take the new name too (one undoable step), so the file still finds the
    /// tool changer slot the tool is loaded in under its new name. Nothing is recalculated.
    /// The status bar says so after `done`.
    pub(super) fn rename_toolpath_tools(&mut self, renamed: &[(String, String)], done: &str) {
        let mut change = Change::builder(&self.doc);
        let mut count = 0;
        for (&id, toolpath) in &self.doc.toolpaths {
            let Some((_, to)) = renamed.iter().find(|(from, _)| *from == toolpath.tool.name) else {
                continue;
            };
            let mut after = toolpath.clone();
            after.tool.name.clone_from(to);
            if let Some(name) = after.params.pointer_mut("/tool/name") {
                *name = to.as_str().into();
            }
            change.replace_toolpath(id, after);
            count += 1;
        }
        if count == 0 {
            return;
        }
        let what = if count == 1 { "toolpath" } else { "toolpaths" };
        self.record(
            Ok(change.finish()),
            &format!("{done}; {count} {what} use the renamed tool's new name"),
        );
    }

    /// Reads the user's tool database; a broken one is reported and left untouched on disk.
    pub(super) fn load_tools(&mut self) {
        self.load_tools_at(files::tool_database_path());
    }

    /// [`Self::load_tools`] from `path`.
    pub(super) fn load_tools_at(&mut self, path: Option<std::path::PathBuf>) {
        self.tools_path = path;
        if let Some(path) = &self.tools_path {
            match files::load_tools_joining(path) {
                Ok((tools, report)) => {
                    self.tools = tools;
                    // a database from before lasers were a tool type gets the starter laser
                    self.tools.add_laser_if_none();
                    // sizes a bit's copies gave differently, not kept: said once
                    if let Some(first) = report.first() {
                        for line in &report {
                            log::warn!("joining a bit's per-material copies: {line}");
                        }
                        let more = match report.len() {
                            1 => String::new(),
                            n => format!(" (and {} more; see the log)", n - 1),
                        };
                        self.status.set_message(&format!(
                            "Bits listed once per material are now one bit each. {first}{more}"
                        ));
                    }
                }
                Err(e) => {
                    self.status
                        .set_message(&format!("{e}; tool changes will not be saved"));
                    self.tools_unreadable = self.tools_path.take();
                }
            }
        }
    }

    /// A database tool as a toolpath over the selected vectors cuts with it: for the hardest
    /// material they are on (their layers', else Job Setup's), else the active material.
    pub(super) fn pick_tool(&self, name: &str) -> Option<cut_craft_core::toolpath::Tool> {
        let material = self
            .doc
            .cutting_material(self.doc.selection.iter().copied(), &self.materials);
        self.tools.pick_in(name, &material, &self.materials)
    }

    /// Copies a database tool, cutting the material under the selection, into the open form.
    pub(super) fn choose_tool(&mut self, name: &str) {
        let picked = self.pick_tool(name);
        if let (Some(form), Some(tool)) = (&mut self.form, picked) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// The Tool Database window, `manager`, with the active machine's tool changer, its
    /// tools loaded with the tools.
    pub(super) fn open_tool_manager(&mut self, mut manager: cut_craft_tool_database::Manager) {
        manager.library = self.materials.clone();
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
        // what it opens on, slots and all, is as kept: nothing waits for Apply yet
        manager.applied();
        self.tool_manager = Some(manager);
    }

    /// A toolpath form's Select…: the Tool Database as Select Tool, on `current`; its Select
    /// gives the form the tool shown, with its own feeds, speeds and pass depth.
    pub(super) fn select_tool(&mut self, current: &str) {
        let manager = cut_craft_tool_database::Manager::selecting(&self.tools, current);
        self.open_tool_manager(manager);
        self.picking = None;
    }

    /// Select Tool's Select: `name` for what it was opened for (a tool of one of the form's
    /// lists, or a tool changer holder), or else as the open form's tool.
    pub(super) fn selected_tool(&mut self, name: &str) {
        match self.picking.take() {
            Some(super::PickFor::Listed(which, at)) => {
                let picked = self.pick_tool(name);
                if let (Some(form), Some(tool)) = (&mut self.form, picked) {
                    super::put_listed(&mut form.params, which, at, &tool);
                }
            }
            Some(super::PickFor::Holder(slot)) => {
                if let Some(machine) = self.machines.active_mut() {
                    machine.load_tool(name, slot);
                }
                self.tool_changer_changed();
            }
            // handled by the Materials Library window
            Some(super::PickFor::Material) => {}
            None => self.choose_tool(name),
        }
    }

    /// The Tool Database, opened on the open form's tool; saving it updates the form.
    pub(super) fn edit_tool(&mut self, ctx: &egui::Context, name: Option<String>) {
        self.invoke(ctx, "tool_database");
        if let Some(manager) = &mut self.tool_manager {
            manager.pick = name
                .and_then(|n| manager.draft.index_of(&n))
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
        // the form's lists of tools too, with what the database has for them now
        let material = self
            .doc
            .cutting_material(self.doc.selection.iter().copied(), &self.materials);
        let Some(form) = &mut self.form else { return };
        for at in ["/clearance_tools", "/tools", "/rest/after"] {
            let Some(list) = form.params.pointer_mut(at).and_then(Value::as_array_mut) else {
                continue;
            };
            for item in list {
                let name = item["name"].as_str().unwrap_or_default().to_owned();
                if let Some(tool) = self.tools.pick_in(&name, &material, &self.materials) {
                    *item = serde_json::to_value(tool).expect("tools serialize");
                }
            }
        }
    }

    /// [`Self::tools_changed`] after the Tool Database's Apply, OK or Select, with the
    /// job's toolpaths following the tools `renamed`: the status bar says what became of
    /// the save (never "saved" when it was not) and then the renamed toolpaths.
    pub(super) fn tools_kept(&mut self, done: &str, renamed: &[(String, String)]) {
        self.tools_changed(done);
        let said = self.status.message.clone();
        self.rename_toolpath_tools(renamed, &said);
        // a warning on the renamed toolpaths still leaves the save's outcome said
        if !self.status.message.starts_with(&said) {
            let message = format!("{said}. {}", self.status.message);
            self.status.set_message(&message);
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    pub(super) fn tools_changed(&mut self, done: &str) {
        if let Some(path) = &self.tools_unreadable {
            let message = not_saved("Tool database", path);
            self.status.set_message(&message);
            return;
        }
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
            let id = c.add_new_component(component);
            let level = c.component(id).map_or(0, |k| k.level);
            let done = format!(
                "Made a component from the toolpath preview{}",
                self.placed_on(level)
            );
            self.record(Ok(c.finish()), &done);
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
                let doc = &self.doc;
                let picked: std::collections::BTreeSet<EntityId> =
                    doc.selection.iter().copied().collect();
                let groups: Vec<Vec<u64>> = doc
                    .selection
                    .iter()
                    .map(|&id| doc.chain_within(id, &picked))
                    .collect();
                self.clipboard_from = super::Copied {
                    layers: doc.selection.iter().map(|&id| doc.layer_of(id)).collect(),
                    job: self.job_epoch,
                    texts: records_of(&groups, &doc.texts),
                    names: records_of(&groups, &doc.part_names),
                    groups,
                };
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
                // a copied group pastes as a group of its own, on its layer, text still text
                let from = &self.clipboard_from;
                let ids = if from.groups.len() == self.clipboard.len() {
                    let paths = self.clipboard.iter().cloned().zip(from.groups.clone());
                    let shift = Some(Point::ZERO);
                    c.add_regrouped(paths.collect(), &from.texts, &from.names, shift)
                } else {
                    self.clipboard.iter().map(|p| c.add(p.clone())).collect()
                };
                // on the layer it came from in this job; in another, on the current layer
                let layers = if from.job == self.job_epoch {
                    from.layers.as_slice()
                } else {
                    &[]
                };
                for (&id, layer) in ids.iter().zip(layers) {
                    if self.doc.layer(*layer).is_some() {
                        c.move_to_layer(id, *layer);
                    }
                }
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
            // the drawing as a PDF opened in the system viewer, which previews and prints it
            // (a print window of our own, with scale and tiling, is not done)
            "print" => {
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

    /// Help ▸ Keyboard Shortcuts' rows: each shortcut in use and what it runs, by name.
    fn shortcut_list(&self, ctx: &egui::Context) -> Vec<(String, String)> {
        let mut rows: Vec<(String, String)> = self
            .bindings(ctx)
            .into_iter()
            .map(|(id, trigger)| {
                let label = if id == shortcut::PAN_DRAG {
                    "Pan (hold and drag)".to_owned()
                } else {
                    self.registry
                        .get(&id)
                        .map_or_else(|| gui::title_case(&id), |c| c.meta().label.to_owned())
                };
                let keys = match trigger {
                    shortcut::Trigger::Key(keys) => ctx.format_shortcut(&keys),
                    mouse @ shortcut::Trigger::Mouse(..) => mouse.text(),
                };
                (keys, label)
            })
            .collect();
        rows.sort_by(|a, b| a.1.cmp(&b.1));
        rows
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
        let shortcuts = if info == Info::Shortcuts {
            self.shortcut_list(ctx)
        } else {
            vec![]
        };
        let rows = self.summary_rows();
        let total = self.job_minutes();
        let mut noted = None;
        let mut change_shortcuts = false;
        let machine = self
            .machines
            .active()
            .map(|m| (m.name.clone(), m.acceleration));
        let summary = &mut self.summary;
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique(("info", &title)), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| match info {
            Info::Help(_) => unreachable!("returned above"),
            Info::Summary => {
                noted = summary_table(ui, &rows, total, summary, units, machine.clone());
            }
            Info::About => about(ui),
            Info::Shortcuts => {
                if ui
                    .button("Change Shortcuts…")
                    .on_hover_text(
                        "Set your own keys, or the mouse's thumb buttons and wheel click, in \
                         Options ▸ Mouse and Shortcuts",
                    )
                    .clicked()
                {
                    change_shortcuts = true;
                }
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
        if change_shortcuts {
            self.info = None;
            self.options_window = true;
            self.options_goto = Some(super::shortcuts_page::CATEGORY);
        }
    }
}

/// The About window: what Cut Craft is, its license, thanks and contributors.
fn about(ui: &mut egui::Ui) {
    ui.heading("Cut Craft");
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
    ui.label("2D and 3D design and CNC toolpaths.");
    ui.separator();
    ui.strong("License");
    ui.label(LICENCE_SUMMARY);
    ui.hyperlink_to(
        "Read the full license",
        "https://github.com/NoobyNull/Cut-Craft/blob/main/LICENSE",
    );
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

/// The About window's plain summary of Cut Craft's own license (the top of LICENSE).
pub(super) const LICENCE_SUMMARY: &str = "Cut Craft is source-available, not open source. \
It is licensed under the PolyForm Noncommercial License 1.0.0: you may use, change and share it \
free of charge for noncommercial purposes only, not for any commercial purpose such as making \
things to sell. The LICENSE file is what applies.";

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
    // what the pointer was on last frame; the fields note it again this frame
    form::forget_help(ui.ctx());
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
        "\u{201c}{label}\u{201d} is not part of Cut Craft. It came from your My Gadgets folder \
         or an unsigned add-on, not from a signed Cut Craft build, so nobody has checked what \
         it does.\n\n\
         Cut Craft and its author are not responsible for what this gadget does or can do, \
         including anything harmful or malicious. It can only change the job that is open, \
         and Undo takes back everything it did in one step. Check its result before you \
         cut.\n\nRun it?"
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

/// The size of the box round a model's `faces`.
fn model_size(faces: &[cmd_orientate_3d_model::Seen]) -> Option<[f64; 3]> {
    let corners = faces.iter().flat_map(|(t, _)| t);
    let lo = corners
        .clone()
        .copied()
        .reduce(cut_craft_core::glam::DVec3::min)?;
    let hi = corners.copied().reduce(cut_craft_core::glam::DVec3::max)?;
    Some((hi - lo).to_array())
}

/// How a toolpath refused for cutting into a keep-out zone is said to be (the form's message
/// then explains how to allow clipping when hovered).
pub(super) const KEEP_OUT_REFUSED: &str = "Not made, to keep out of a keep-out zone";

/// Whether clipping at keep-out zones can keep `toolpath` out of them: not a laser's (its
/// power goes with each move) or a drill bank's (its head plunges whole).
fn can_clip(toolpath: &cut_craft_core::toolpath::Toolpath) -> bool {
    toolpath.power.is_empty() && toolpath.drill_bank.is_none()
}

/// Where clipping at keep-out zones is turned on.
pub(super) const CLIP_SETTING: &str =
    "Edit ▸ Options ▸ Toolpath Settings ▸ Clip toolpaths at keep-out zones";

/// What turning clipping on does, and how: the tooltip on a toolpath refused for a zone.
pub(super) const CLIP_HOW: &str = "Keep-out zones (red outlines, such as clamps) are kept \
    safe: a toolpath that would cut into one is not made.\n\nTo make it anyway with the part \
    under the zone left uncut, open Edit ▸ Options, and under Toolpath Settings tick \
    \"Clip toolpaths at keep-out zones\". The tool then lifts over each zone and comes back \
    down past it, as if the zone were cut out of whatever is under it.\n\nTo change a zone \
    instead: Toolpaths ▸ Keep-Out Zones… for the job's, Machine Configuration for the \
    machine's.";

/// The material a profile's tabs are judged against: how thick, whether a closed shape
/// is selected, and how many shapes the form's hand-placed tabs leave out.
fn form_stock(
    doc: &cut_craft_core::document::Document,
    params: &Value,
    units: cut_craft_core::document::Units,
    bed: [f64; 2],
) -> super::tabs::Stock {
    let selected: Vec<_> = doc.selection.iter().copied().collect();
    super::tabs::Stock {
        thickness: doc.job.thickness,
        closed: doc
            .selection
            .iter()
            .any(|id| doc.vectors.get(id).is_some_and(|v| v.closed)),
        units,
        bed,
        clearing: doc.job.clearing,
        untabbed: cut_craft_core::checks::shapes_without_placed_tabs(doc, &selected, params),
    }
}

/// What Suggest found, in a sentence for the form.
fn suggestion_note(s: &cmd_pocket_toolpath::Suggestion, finishing: &str) -> String {
    let minutes = |m: f64| format!("{:.0}", m.max(1.0).round());
    if s.clearing.is_empty() {
        return format!(
            "Suggest: {finishing} alone is quickest here, about {} min; no bigger bit of yours \
             saves time",
            minutes(s.alone)
        );
    }
    let names: Vec<&str> = s.clearing.iter().map(|t| t.name.as_str()).collect();
    format!(
        "Suggest: {} then {finishing}, about {} min against {} min with {finishing} alone",
        names.join(", "),
        minutes(s.minutes),
        minutes(s.alone)
    )
}

/// Whether toolpath `id` with `params` cuts only the closed vectors selected, leaving the
/// open ones out: the area toolpaths, and Profile inside or outside (on the line cuts open
/// ones too).
pub(super) fn closed_only_toolpath(id: &str, params: &Value) -> bool {
    match id {
        "profile_toolpath" => params["side"].as_str() != Some("on"),
        // a texture fills closed vectors, but follows any vector used as its pattern
        "texture_toolpath" => params["use_selected_vectors_as_pattern"] != Value::Bool(true),
        _ => CLOSED_ONLY_TOOLPATHS.contains(&id),
    }
}

/// What a Texture Toolpath form filling closed vectors will texture when no closed vector is
/// selected: the whole material, said plainly (in red when vectors are selected but open).
pub(super) fn texture_area_note(
    id: &str,
    params: &Value,
    count: cut_craft_core::document::VectorCount,
) -> Option<String> {
    if id != "texture_toolpath"
        || params["use_selected_vectors_as_pattern"] == Value::Bool(true)
        || count.closed > 0
    {
        return None;
    }
    Some(if count.open > 0 {
        "Nothing closed is selected, so the whole material will be textured: close the open \
         vectors, or clear the selection"
            .to_owned()
    } else {
        "Nothing selected: the whole material will be textured".to_owned()
    })
}

/// The toolpaths that take only closed vectors from the selection.
const CLOSED_ONLY_TOOLPATHS: [&str; 9] = [
    "chamfer_toolpath",
    "drill_bank_toolpath",
    "drilling_toolpath",
    "inlay_toolpath",
    "pocket_toolpath",
    "prism_toolpath",
    "sketch_carving_toolpath",
    "vcarve_engrave_toolpath",
    "vcarve_inlay_toolpath",
];

/// What the "recalculate all?" question says for `stale` toolpaths out of date.
pub(super) fn recalculate_question(stale: usize) -> String {
    let (count, them) = if stale == 1 {
        ("1 toolpath is".to_owned(), "it")
    } else {
        (format!("{stale} toolpaths are"), "them")
    };
    format!(
        "{count} now out of date: the vectors {them} cut have changed. Recalculate all \
         toolpaths now, so the G-code you save matches the drawing?"
    )
}

/// The sub-tabs of `page`.
fn subtab_set(page: Page) -> gui::SubTabSet {
    match page {
        Page::Design => gui::DESIGN,
        Page::Toolpaths => gui::TOOLPATHS,
    }
}

/// `page`'s name in ids.
fn page_key(page: Page) -> &'static str {
    match page {
        Page::Design => "design",
        Page::Toolpaths => "toolpaths",
    }
}

/// A form with no commit button: why there is none, and on a drawing tool whose shape follows
/// the form, New Shape to let it go and type another; whether New Shape was pressed.
/// Forms with a Reset button: their settings back to the defaults, the look made again as
/// if new.
const RESET_FORMS: &[&str] = &["embellish"];

/// Forms whose changing text (the Next line, the red text at the foot) has a fixed place,
/// so the form never grows, shrinks or flickers as it changes.
const FIXED_TEXT_FORMS: &[&str] = &["embellish"];

/// Lines kept for a fixed form's Next line.
const NEXT_LINE_ROWS: f32 = 2.0;

/// Lines kept for what the setting under the pointer does.
const HELP_BOX_ROWS: f32 = 4.0;

/// Lines kept for a fixed form's red text at its foot.
const WARNING_ROWS: f32 = 3.0;

/// A form's Reset button, for the forms that have one.
fn reset_button(ui: &mut egui::Ui, id: &str) -> bool {
    RESET_FORMS.contains(&id)
        && ui
            .button("Reset")
            .on_hover_hint("Put every setting back to its default and make it again, as if new")
            .clicked()
}

fn live_row(ui: &mut egui::Ui, id: &str, drawing: bool, made: bool) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.weak(commit::live_note(id, drawing));
        drawing
            && made
            && ui
                .button("New Shape")
                .on_hover_hint("Leave this shape as it is, to type the numbers for another")
                .clicked()
    })
    .inner
}

/// A toolpath form's Save Settings… and Load Settings…: whether each was pressed.
fn settings_file_row(ui: &mut egui::Ui) -> (bool, bool) {
    ui.horizontal(|ui| {
        let save = ui
            .button("Save Settings…")
            .on_hover_hint("Save this form's settings to a file, to use again later")
            .clicked();
        let load = ui
            .button("Load Settings…")
            .on_hover_hint("Fill this form from settings you saved before")
            .clicked();
        (save, load)
    })
    .inner
}

/// After Calculate: Calculate as New, another toolpath on purpose, the one made kept. A row
/// of its own over the buttons, so Calculate, Cancel and Close stay where they were;
/// whether it was pressed.
fn calculate_as_new_row(ui: &mut egui::Ui) -> bool {
    ui.button("Calculate as New")
        .on_hover_hint("Add another toolpath with these settings, keeping the one already made")
        .clicked()
}

/// A form's starting bit, when it is the bare default ([`Tool::default`]), made the
/// starter bit ([`Tool::starter`]): the same 6 mm end mill with its flute length and shank.
fn starter_tool(params: &mut Value) {
    use cut_craft_core::toolpath::Tool;
    let bare = serde_json::to_value(Tool::default()).expect("tools serialize");
    if let Some(tool) = params.get_mut("tool").filter(|t| **t == bare) {
        *tool = serde_json::to_value(Tool::starter()).expect("tools serialize");
    }
}

/// Embellish's samples and random sequences, for the pattern maker under its Sequence box.
fn pattern_maker() -> form::PatternMaker<'static> {
    form::PatternMaker {
        sample: &embellish_sample,
        random: &cmd_embellish::random_sequence,
    }
}

fn embellish_sample(params: &Value, entry: &str) -> form::SampleRings {
    cmd_embellish::sample(params, entry)
        .into_iter()
        .map(|ring| ring.into_iter().map(|q| [q.x, q.y]).collect())
        .collect()
}

/// The active machine's bed, width and height (none without a machine).
fn bed(machines: &cut_craft_machines::Machines) -> [f64; 2] {
    machines.active().map_or_default(|m| [m.width, m.height])
}

/// What Install Offline Package did: `added` machines, `here` already here by name.
fn installed_machines(added: usize, here: usize) -> String {
    let here = if here > 0 {
        format!(", {here} already here")
    } else {
        String::new()
    };
    format!("Installed {added} machines{here}")
}

/// The entries of `records` (text settings or part names, by group) for `groups`.
fn records_of<T: Clone>(
    groups: &[Vec<u64>],
    records: &std::collections::BTreeMap<u64, T>,
) -> std::collections::BTreeMap<u64, T> {
    groups
        .iter()
        .flatten()
        .filter_map(|g| Some((*g, records.get(g).cloned()?)))
        .collect()
}

/// What a save says when `what`'s file, at `path`, could not be read when Cut Craft started:
/// the changes are kept until it closes, but not written over that file.
fn not_saved(what: &str, path: &std::path::Path) -> String {
    format!(
        "{what} not saved: {} could not be read when Cut Craft started, so it is not written \
         over. Fix or rename that file, then restart Cut Craft; your changes are kept only \
         until it closes",
        path.display()
    )
}

/// A form's commit button, marked for the help's step pictures; whether it was pressed.
fn marked_commit_button(ui: &mut egui::Ui, label: &str, hint: &str) -> bool {
    let button = ui.button(label);
    crate::marks::note(ui, &format!("button:{label}"), button.rect);
    button.on_hover_hint(hint).clicked()
}
