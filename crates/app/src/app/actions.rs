//! What the window does: the window's start-up state, which actions are available, every
//! action's dispatch (`invoke`), gadgets, spelling, brushes, model re-import and the About and
//! Info windows. Each area the dispatch hands work to has its own file beside this one:
//! `job_files`, `toolpath_output`, `machine_setup`, `form_apply`, `edit_apply`, `keyboard`,
//! `page_layout`, `form_panel`, `command_form` and `tools_materials`.

use super::built_ins;
use super::{
    CONTRIBUTORS, Change, CutCraftApp, Document, EXPORT_LEVEL, EntityId, History, Icons, Info,
    Page, Pages, SELECT_LEVEL, SaveForm, Screen, Session, Shown, ToolDatabase, Value, View,
    brush_dialog, commands, dialog_size, egui, files, gui, saving_path, summary_table, view3d,
};
use cut_craft_i18n::tr;

/// A startup task's words' key (`locales/en/start.ftl`): `start-task-` and its id.
pub(super) fn start_task_key(id: &str) -> String {
    format!("start-task-{}", id.replace('_', "-"))
}

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
        // tests never read the answers kept on this computer
        let trust = if cfg!(test) {
            crate::trust::Trust::new(None)
        } else {
            crate::trust::Trust::in_settings_folder()
        };
        let gadgets =
            crate::gadgets::load(&mut registry, my_gadgets.as_deref(), &addon_gadgets, &trust);
        let mine: Vec<(String, &'static str)> = gadgets
            .installed()
            .filter_map(|g| Some((g.label.clone(), registry.get(&g.id)?.id())))
            .collect();
        let mut menus = gui::MenuSystem::new();
        let items: Vec<(String, String)> = mine
            .iter()
            .map(|(label, id)| (label.clone(), (*id).to_owned()))
            .collect();
        menus.add_gadgets(&gadgets.built_in, &items);
        let mut status = gui::StatusBar::new();
        if !gadgets.problems.is_empty() {
            status.set_message(&cut_craft_i18n::tr!(
                "status-gadgets-not-loaded",
                problems = gadgets.problems.join("; ")
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
            view_2d_at: None,
            view_tool: None,
            pages: Pages::default(),
            search: None,
            inspector_tab: 0,
            rail_group: None,
            rail_pinned: false,
            rail_favourites: false,
            page_rects: [None, None],
            flyout_rect: None,
            buttons_across: std::cell::Cell::new(0.0),
            ribbon_tab: 0,
            canvas: super::canvas_layout::CanvasState::default(),
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
            border: super::generate_border::BorderForm::default(),
            depth_high_detail: None,
            depth_detail_fetch: None,
            depth_detail_failed: None,
            busy: None,
            wait_before_showing: crate::busy::SHOW_AFTER,
            addons: crate::addons::Addons::new(None),
            language_packs: None,
            machines: cut_craft_machines::Machines::default(),
            machines_path: None,
            machines_unsaved: false,
            posts: cut_craft_machines::Library::new(None),
            machine_window: None,
            first_run: None,
            layout_question: None,
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
            reorder_ask: None,
            gcode_view: None,
            save_anyway: None,
            checks_accepted: Vec::new(),
            acknowledged: std::collections::BTreeSet::new(),
            save_after: None,
            form_back: None,
            shots: None,
            help_shots: None,
            space_mouse: None,
            my_gadgets: mine,
            installed_addons: vec![],
            rail_gadgets: false,
            unsigned_gadgets: gadgets.unsigned,
            trust,
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
            ring_grip: None,
            info: None,
            help_cache: egui_commonmark::CommonMarkCache::default(),
            help_search: String::new(),
            help_index: vec![],
            help_ranked: None,
            help_history: cut_craft_help::wiki::History::default(),
            tip: None,
            tips_cache: egui_commonmark::CommonMarkCache::default(),
            credits: crate::credits::Window::default(),
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
            held_detail: None,
            preview_grid_kept: std::cell::Cell::new(None),
            simulation: None,
            simulation_coarse: None,
            import_level: None,
            polyline: crate::polyline::Draft::default(),
            placed: Vec::new(),
            picked_constraint: None,
            fine_3d: None,
            rough_3d: None,
            turned_3d: None,
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
            gauge: cut_craft_core::work::Gauge::default(),
            detail_3d: (crate::options::Detail::default(), false),
            paced_pass: u64::MAX,
            orbit: view3d::Orbit::default(),
            framing: view3d::Framing::default(),
            view_cube: None,
            recent: vec![],
            player: super::player::Player::default(),
            close_up: None,
            turning: None,
            collapsed_levels: std::collections::HashSet::new(),
            guide_drag: None,
            rotating: false,
            options: crate::options::Options::default(),
            options_window: false,
            options_draft: None,
            job_defaults_form: None,
            options_closing: false,
            options_page: 0,
            show_advanced: false,
            web_api: super::web_api::WebApi::default(),
            options_goto: None,
            capture: None,
            shortcut_search: String::new(),
            split: None,
            split_layout: super::split_views::SplitLayout::default(),
            clipart: crate::clipart::Clipart::default(),
            import_queue: std::collections::VecDeque::new(),
            import_after_setup: None,
            recent_path: None,
            recovery_path: None,
            recovery_lock: None,
            work_kept_for_update: false,
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
        if let Some(action) = built_ins::get(id) {
            return (action.when)(self);
        }
        match id {
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
            id if id.starts_with(gui::COMMUNITY) || id.starts_with(gui::GUIDE) => true,
            id if id.starts_with(gui::SAVE_BRUSH) => self.brush_component(id).is_some(),
            id if id.starts_with(EXPORT_LEVEL) => {
                self.registry.exporter("export_as_3d_clipart").is_some()
            }
            id if self.display.checked(id).is_some() => true,
            id if self.resolution_checked(id).is_some() => self.screen == Screen::Job,
            id if self.units_checked(id).is_some() => self.screen == Screen::Job,
            id if self.layout_checked(id).is_some() => true,
            id if crate::flyout::variant(id).is_some() => self.variant_ready(id),
            // a command's form shows in a job's pages: with no job it could not be seen
            _ => {
                (self.registry.get(id).is_some() && self.screen != Screen::Start)
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Why `id` is greyed out, shown on hover: what it waits for (a selection, a toolpath
    /// picked, something to undo, a job open). "Not implemented yet" only for an action
    /// nothing runs.
    pub(super) fn unavailable_why(&self, id: &str) -> String {
        let key = match id {
            id if built_ins::get(id).is_some_and(|a| !a.why.is_empty()) => {
                built_ins::get(id).map_or_default(|a| a.why)
            }
            id if id.starts_with(gui::REIMPORT_MODEL) => "actions-why-model-file-moved",
            id if id.starts_with(EXPORT_LEVEL) => "actions-why-no-clipart-export",
            id if built_ins::get(id).is_some()
                || self.registry.get(id).is_some()
                || self.registry.exporter(id).is_some()
                || crate::flyout::variant(id).is_some()
                || gui::command_action(id).is_some()
                || self.resolution_checked(id).is_some()
                || self.units_checked(id).is_some() =>
            {
                "actions-why-job-first"
            }
            _ => return tr!("actions-why-not-made", tool = self.tool_label(id)),
        };
        cut_craft_i18n::tr(key)
    }

    /// Menu items, tool buttons and shortcuts all end up here: a few special cases, then the
    /// built-in actions ([`built_ins`]), then the actions that come in families (a recent
    /// file, a layout, a display switch...) and last the command registry.
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
        if let Some(action) = built_ins::get(id) {
            (action.run)(self, ctx);
            return;
        }
        self.invoke_family(ctx, id);
    }

    /// The actions that come in families, each named by a prefix and what follows it or
    /// found in a list of its own (the display switches, the layouts...), else a command.
    fn invoke_family(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            // a drawing tool's toolbox variant: its form, the drag read the variant's way
            id if crate::flyout::variant(id).is_some() => self.start_variant(id),
            // the labels' setting is kept with the options, so they stay as left
            "show_keep_out_zone_labels" => {
                self.display.toggle(id);
                self.options.toolpaths.keep_out_labels = self.display.keep_out_labels;
                self.save_options();
                ctx.request_repaint();
            }
            "draw_vectors_in_3d_view" => {
                self.show_vectors_3d(!self.display.vectors_3d);
                ctx.request_repaint();
            }
            // Toolpaths ▸ Preview Simulation Quality: the detail slider to that pick, and
            // a preview showing worked out again at it
            id if crate::display::Display::is_quick_pick(id) => {
                self.display.toggle(id);
                self.rework_preview();
                ctx.request_repaint();
            }
            id if self.display.toggle(id) => ctx.request_repaint(),
            id if self.resolution_checked(id).is_some() => self.set_resolution(id),
            id if self.units_checked(id).is_some() => self.set_units(id),
            id if self.layout_checked(id).is_some() => self.set_layout(id),
            _ => self.invoke_prefixed(ctx, id),
        }
    }

    /// The families named by a prefix with a value after it (a link's number, a recent
    /// file's place, a level...), else a command.
    fn invoke_prefixed(&mut self, ctx: &egui::Context, id: &str) {
        match id {
            id if id.starts_with(gui::COMMUNITY) => {
                let link = id[gui::COMMUNITY.len()..].parse::<usize>().ok();
                if let Some((_, url, _)) = link.and_then(|n| super::LINKS.get(n)) {
                    ctx.open_url(egui::OpenUrl::new_tab(*url));
                }
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
            id if id.starts_with(gui::RIBBON_PIN) => {
                self.set_ribbon_pin(&id[gui::RIBBON_PIN.len()..]);
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
    pub(super) fn spelling(&mut self, ui: &mut egui::Ui) {
        let text = self
            .form
            .as_ref()
            .and_then(|f| f.params["text"].as_str())
            .unwrap_or_default()
            .to_owned();
        let Some(speller) = self.speller() else {
            ui.weak(tr!("spelling-no-dictionary"));
            return;
        };
        let wrong = speller.misspelled(&text);
        let (mut fix, mut add, mut remove) = (None, None, None);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong(tr!("spelling-title"));
            if wrong.is_empty() {
                ui.weak(tr!("spelling-none-wrong"));
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
                    if ui.small_button(tr!("spelling-add-word")).clicked() {
                        add = Some(word.clone());
                    }
                });
            }
            if !speller.user_words().is_empty() {
                egui::ComboBox::new("remove_word", tr!("spelling-remove-word"))
                    .selected_text(tr!("spelling-a-word-added"))
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
        self.status.set_message(&cut_craft_i18n::tr!(
            "status-saved-brush",
            path = path.display().to_string()
        ));
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
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-import-changed",
                path = path.display().to_string()
            ));
        }
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

    /// The Help windows: About, Keyboard Shortcuts, Credits.
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
            Info::Credits => "Credits".to_owned(),
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
        let credits = &mut self.credits;
        let mut open_credits = false;
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique(("info", &title)), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, &title, size, &mut open, |ui| match info {
            Info::Help(_) => unreachable!("returned above"),
            Info::Summary => {
                noted = summary_table(ui, &rows, total, summary, units, machine.clone());
            }
            Info::About => open_credits = about(ui),
            Info::Shortcuts => {
                if ui
                    .button(tr!("shortcuts-change"))
                    .on_hover_text(tr!("shortcuts-change.tooltip"))
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
            Info::Credits => crate::credits::show(ui, credits),
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
                Ok(()) => self.status.set_message(&cut_craft_i18n::tr!(
                    "status-notes-kept",
                    name = name.clone()
                )),
                Err(e) => self.status.set_message(&e),
            }
        }
        if !open {
            self.info = None;
        }
        if open_credits {
            self.info = Some(Info::Credits);
        }
        if change_shortcuts {
            self.info = None;
            self.options_window = true;
            self.options_goto = Some(super::shortcuts_page::CATEGORY);
        }
    }
}

/// The About window: what Cut Craft is, its license, thanks and contributors. Gives whether
/// Credits was clicked.
fn about(ui: &mut egui::Ui) -> bool {
    ui.heading(tr!("about-name"));
    ui.label(tr!("about-version", version = env!("CARGO_PKG_VERSION")));
    ui.label(tr!("about-what"));
    ui.separator();
    ui.strong(tr!("about-license"));
    ui.label(LICENCE_SUMMARY);
    ui.hyperlink_to(
        tr!("about-read-license"),
        "https://github.com/NoobyNull/Cut-Craft/blob/main/LICENSE",
    );
    ui.separator();
    ui.strong(tr!("about-acknowledgements"));
    ui.label(ACKNOWLEDGEMENT);
    let credits = ui
        .button(tr!("about-credits"))
        .on_hover_text(tr!("about-credits.tooltip"))
        .clicked();
    ui.separator();
    ui.strong(tr!("about-contributors"));
    for (login, commits, url) in contributors() {
        ui.horizontal(|ui| {
            ui.hyperlink_to(login, url);
            // a number, so "1 commit" takes its one-form
            match commits.parse::<u64>() {
                Ok(count) => ui.weak(tr!("about-commits", count = count)),
                Err(_) => ui.weak(tr!("about-commits", count = commits)),
            };
        });
    }
    credits
}

/// The About window's plain summary of Cut Craft's own license (the top of LICENSE).
pub(super) const LICENCE_SUMMARY: &str = "Cut Craft is source-available, not open source. \
It is licensed under the PolyForm Strict License 1.0.0 with an extra permission: people and \
one-person businesses may use it free of charge for any purpose, including making and selling \
things with it. Larger businesses may use it for noncommercial purposes only. Nobody may share, \
sell or change Cut Craft itself, except to offer changes back to the project. The LICENSE file \
is what applies.";

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
