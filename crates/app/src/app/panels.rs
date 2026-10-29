//! The side panels and windows: Tool Database, layers, sheets, clipart, the component tree,
//! playback and the 3D view.

use super::{
    Align, Cow, CutCraftApp, EXPORT_LEVEL, EntityId, FINE_MESH_CELLS, FineKey, LayerId, Layout,
    MESH_CELLS, Material, Menus, PLAYBACK_SPEEDS, Page, SETTLE_SECONDS, TOOL_HEIGHT_DIAMETERS,
    TOOL_ICON_ROWS, VIEW_ZOOM_QUARTERS, Value, View, egui, file_dialog, file_name, files, gui,
    menu_items, sheet_length, view3d, wrap_for,
};

impl CutCraftApp {
    /// The Tool Database manager; its edits are kept on Apply or OK.
    pub(super) fn tool_database_window(&mut self, ctx: &egui::Context) {
        use cut_craft_tool_database::Outcome;
        let Some(manager) = &mut self.tool_manager else {
            return;
        };
        let icons = &self.icons;
        let picture = |path: &str| icons.find(path, "").map(|p| icons.image(p));
        let look = cut_craft_tool_database::Look {
            inch: self.doc.job.units == cut_craft_core::document::Units::Inch,
            picture: &picture,
        };
        let Some(outcome) = manager.show(ctx, &look) else {
            return;
        };
        match outcome {
            Outcome::Apply | Outcome::Ok => {
                self.tools = manager.draft.clone();
                let changer = manager.changer.clone();
                if outcome == Outcome::Ok {
                    self.tool_manager = None;
                }
                self.tools_changed("Tool database saved");
                if let Some(changer) = changer
                    && let Some(machine) = self
                        .machines
                        .machines
                        .iter_mut()
                        .find(|m| m.name == changer.machine)
                    && machine.slots != changer.slots
                {
                    machine.slots = changer.slots;
                    self.machines_changed(&format!(
                        "Tool database saved; {} loaded",
                        changer.machine
                    ));
                }
            }
            Outcome::Cancel => self.tool_manager = None,
            Outcome::Import => {
                if let Some(path) = file_dialog()
                    .add_filter("Tool databases", &files::TOOL_FILE_EXTENSIONS)
                    .pick_file()
                {
                    self.import_tools(&path);
                }
            }
            Outcome::Export(tools) => {
                if let Some(path) = file_dialog()
                    .add_filter("Cut Craft tool database", &["json"])
                    .save_file()
                {
                    let done = format!(
                        "Exported {} tools to {}",
                        tools.tools.len(),
                        file_name(&path)
                    );
                    let result = files::save_tools(&tools, &path);
                    self.status
                        .set_message(&result.map_or_else(|e| e, |()| done));
                }
            }
        }
    }

    /// Adds the tools of a tool database file in a group named after it: to the open Tool
    /// Database window, kept on its OK, else to the database, saved now.
    pub(super) fn import_tools(&mut self, path: &std::path::Path) {
        let name = path.file_stem().map_or_else(
            || "Imported".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        match files::read_tool_file(path) {
            Ok(tools) => {
                let n = tools.tools.len();
                let done = format!("Imported {n} tools from {}", file_name(path));
                if let Some(manager) = &mut self.tool_manager {
                    manager.import(tools, &name);
                    self.status.set_message(&format!("{done}; OK keeps them"));
                } else {
                    cut_craft_tool_database::tree::import(&mut self.tools, tools, &name);
                    self.tools_changed(&done);
                }
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// A tool page's tools: all its groups, or only its favourites when it shows those.
    pub(super) fn tools_or_favourites(
        &mut self,
        ui: &mut egui::Ui,
        groups: &[gui::ToolGroup],
        page: Page,
    ) {
        let (favourites, only) = self.options.favourites.of(page);
        if !only {
            self.tool_groups(ui, groups, page);
            return;
        }
        // in the order picked; a tool no longer on the page is left out
        let tools: Vec<&'static str> = favourites
            .iter()
            .filter_map(|f| {
                groups
                    .iter()
                    .flat_map(|g| g.tools.iter().copied())
                    .find(|id| id == f)
            })
            .collect();
        if tools.is_empty() {
            ui.weak("No favourites yet: right-click a tool ▸ Add to Favourites");
            if ui.button("Show All Tools").clicked() {
                let action = format!("{}{}:false", gui::FAVOURITES_ONLY, gui::page_key(page));
                self.invoke(ui.ctx(), &action);
            }
            return;
        }
        self.tool_sections(ui, &[("Favourites", tools)], page, true);
    }

    /// Icon buttons for groups of command ids: implemented ones run, the rest are greyed out.
    pub(super) fn tool_groups(&mut self, ui: &mut egui::Ui, groups: &[gui::ToolGroup], page: Page) {
        let sections: Vec<(&str, Vec<&'static str>)> =
            groups.iter().map(|g| (g.name, g.tools.to_vec())).collect();
        self.tool_sections(ui, &sections, page, false);
    }

    /// Headed sections of tool buttons; a right-click on one adds it to `page`'s favourites
    /// or takes it off (`among` them, it moves along them too).
    fn tool_sections(
        &mut self,
        ui: &mut egui::Ui,
        sections: &[(&str, Vec<&'static str>)],
        page: Page,
        among: bool,
    ) {
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        let mut chosen = None;
        // plain sections, as Aspire's tool pages have: a heading over its tools
        for (name, tools) in sections {
            ui.strong(*name);
            ui.horizontal_wrapped(|ui| {
                for &id in tools {
                    let cmd = self.registry.get(id);
                    let label =
                        cmd.map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
                    let explicit = cmd.map_or("", |c| c.meta().icon);
                    let button = self
                        .icon_button(explicit, id, &label, edge)
                        .selected(self.status.current_tool == label);
                    let response = ui
                        .add_enabled(self.available(id), button)
                        .on_hover_text(&label)
                        .on_disabled_hover_text(format!("{label}: not implemented yet"));
                    if response.clicked() {
                        clicked = Some(id);
                    }
                    let favourite = self.options.favourites.of(page).0.iter().any(|f| f == id);
                    if let Some(action) =
                        self.popup(&response, || gui::tool_menu(page, id, favourite, among))
                    {
                        chosen = Some(action);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(id) = clicked {
            self.pick(ui.ctx(), page, id);
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// An icon button, or a text one when the icon is missing.
    pub(super) fn icon_button(
        &self,
        icon: &str,
        id: &str,
        label: &str,
        edge: f32,
    ) -> egui::Button<'static> {
        match self.icons.find(icon, id) {
            Some(path) => egui::Button::image(
                self.icons
                    .image(path)
                    .fit_to_exact_size(egui::vec2(edge, edge)),
            ),
            None => egui::Button::new(label.to_owned()),
        }
    }

    /// Layers, top first: visibility and lock toggles, the current layer highlighted, and
    /// Aspire's layer menu on a right-click.
    pub(super) fn layers_panel(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Layers tab");
        let edge = ui.spacing().interact_size.y;
        let label = |id: &str| self.registry.get(id).map_or("", |c| c.meta().label);
        let mut clicked: Option<(LayerId, &str)> = None;
        let mut chosen = None;
        for layer in self.doc.layers.iter().rev() {
            ui.horizontal(|ui| {
                let (eye, eye_tip) = if layer.visible {
                    ("layers/visible.png", "Hide")
                } else {
                    ("layers/hidden.png", "Show")
                };
                let (lock, lock_tip) = if layer.locked {
                    ("layers/locked.png", "Unlock")
                } else {
                    ("layers/unlocked.png", "Lock")
                };
                if ui
                    .add(self.icon_button(eye, "", eye_tip, edge))
                    .on_hover_text(eye_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_visible"));
                }
                if ui
                    .add(self.icon_button(lock, "", lock_tip, edge))
                    .on_hover_text(lock_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_locked"));
                }
                let count = self.doc.vectors_on(layer.id).count();
                let row = ui
                    .selectable_label(self.doc.current_layer == layer.id, &layer.name)
                    .on_hover_text(format!("{count} vectors"));
                if row.clicked() {
                    clicked = Some((layer.id, "set_current_layer"));
                }
                if row.double_clicked() {
                    clicked = Some((layer.id, "layer_properties"));
                }
                if let Some(id) = self.popup(&row, || gui::layer_menu(&self.doc, layer)) {
                    chosen = Some(id);
                }
            });
        }
        let add = ui.button(label("add_layer")).clicked();
        if let Some((layer, action)) = clicked {
            self.layer_action(ui.ctx(), layer, action);
        }
        if add {
            self.invoke(ui.ctx(), "add_layer");
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The level menu's Select Components (`SELECT_LEVEL<id>`) and Export as 3D Clipart
    /// (`EXPORT_LEVEL<id>`, which selects them and exports).
    pub(super) fn level_action(&mut self, id: &str) {
        let (how, level) = id.split_once(':').unwrap_or_default();
        if let Ok(level) = level.parse() {
            self.doc.selected_components = self.doc.level_components(level).into_iter().collect();
            if format!("{how}:") == EXPORT_LEVEL {
                self.export("export_as_3d_clipart");
            }
        }
    }

    /// The Clipart tab: its library, and the chosen folder's clipart to double-click in.
    pub(super) fn clipart_panel(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Clipart tab");
        let registry = &self.registry;
        let importable = |ext: &str| registry.importer_for(ext).is_some();
        match crate::clipart::panel(ui, &mut self.clipart, &importable) {
            Some(crate::clipart::Picked::AddFolder) => self.invoke(ui.ctx(), "add_clipart_folder"),
            Some(crate::clipart::Picked::OpenFolder) => {
                self.invoke(ui.ctx(), "open_clipart_folder");
            }
            Some(crate::clipart::Picked::Import(path)) => self.import_file(&path),
            None => {}
        }
    }

    /// Brings `path` into the job: meshes through Orientate 3D Model, as Aspire's import
    /// does; anything else through its importer.
    pub(super) fn import_file(&mut self, path: &std::path::Path) {
        let stl = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("stl"));
        if stl {
            self.start_command("orientate_3d_model");
            if let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") {
                form.params["file"] = Value::from(path.display().to_string());
            }
            // issue #3: the model is in the scene at once, and follows the form from then on
            self.apply_live();
            return;
        }
        let change = files::import(&self.registry, &self.doc, path)
            .map_err(cut_craft_core::CommandError::Failed);
        self.record(change, &format!("Imported {}", path.display()));
    }

    /// The Sheets tab: Aspire's Sheets List (each sheet's name and size, the active one
    /// marked) and its Add New Sheet button. A click activates a sheet; a right-click has its
    /// menu.
    pub(super) fn sheets_panel(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Sheets tab");
        ui.strong("Sheets List");
        let units = self.doc.job.units;
        let (mut activate, mut chosen) = (None, None);
        egui::Grid::new("sheets_list").striped(true).show(ui, |ui| {
            for heading in ["Name", "X", "Y", "Z"] {
                ui.strong(heading);
            }
            ui.end_row();
            for (i, sheet) in self.doc.sheets.iter().enumerate() {
                let job = self.doc.sheet_job(i).unwrap_or(sheet.job);
                let active = i == self.doc.active_sheet;
                let row = ui.radio(active, &sheet.name);
                if row.clicked() && !active {
                    activate = Some(i);
                }
                if let Some(id) = self.popup(&row, || gui::sheet_menu(&self.doc, i)) {
                    chosen = Some(id);
                }
                for mm in [job.width, job.height, job.thickness] {
                    ui.label(sheet_length(mm, units));
                }
                ui.end_row();
            }
        });
        let add = ui.button("Add New Sheet...").clicked();
        if let Some(sheet) = activate
            && let Err(e) = self.apply("activate_sheet", &serde_json::json!({ "sheet": sheet }))
        {
            self.status.set_message(&e);
        }
        if add {
            self.invoke(ui.ctx(), "add_sheet");
        }
        if let Some(id) = chosen {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// A right-click menu on `response`, built when it opens: the action chosen, if any.
    pub(super) fn popup(
        &self,
        response: &egui::Response,
        menu: impl FnOnce() -> Vec<gui::MenuItem>,
    ) -> Option<String> {
        let mut clicked = None;
        if response.secondary_clicked() {
            cut_craft_bug_report::claim(&response.ctx);
        }
        response.context_menu(|ui| {
            let menus = Menus {
                available: &|id| self.available(id),
                checked: &|_| None,
                shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                recent: &[],
            };
            menu_items(ui, &menu(), &menus, &mut clicked);
            ui.separator();
            if ui.button("Report a Bug Here…").clicked() {
                cut_craft_bug_report::open_here(ui.ctx());
                ui.close();
            }
        });
        clicked
    }

    /// A layer-panel action: the toggles are undoable; choosing a layer is view state like the
    /// selection, so it is not.
    pub(super) fn layer_action(&mut self, ctx: &egui::Context, layer: LayerId, action: &str) {
        let Some(l) = self.doc.layer(layer).cloned() else {
            return;
        };
        let properties = |visible: bool, locked: bool| serde_json::json!({ "layer": l.id, "name": l.name, "visible": visible, "locked": locked });
        let result = match action {
            "toggle_visible" => self.apply("layer_properties", &properties(!l.visible, l.locked)),
            "toggle_locked" => self.apply("layer_properties", &properties(l.visible, !l.locked)),
            "set_current_layer" => self
                .registry
                .get(action)
                .expect("set_current_layer is registered")
                .run_json(&self.doc, &serde_json::json!({ "layer": layer }))
                .map_err(|e| e.to_string())
                .map(|change| {
                    self.doc
                        .apply(&change)
                        .expect("the panel lists existing layers");
                }),
            // the properties form starts from the current layer
            "layer_properties" => {
                self.layer_action(ctx, layer, "set_current_layer");
                self.invoke(ctx, action);
                Ok(())
            }
            _ => unreachable!("not a layer panel action: {action}"),
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Cuts the toolpath from the previewed material (uncut material if there is none yet).
    /// The cascade: start again from uncut material, then preview each visible toolpath.
    pub(super) fn preview_all(&mut self) {
        self.finish_playback();
        self.simulation = None;
        let visible: Vec<EntityId> = self
            .doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible)
            .map(|(&id, _)| id)
            .collect();
        if self.display.animate_preview {
            self.play(&visible);
            return;
        }
        for id in visible {
            self.preview_toolpath(id);
        }
    }

    /// Starts playing `toolpaths` into the previewed material (uncut if there is none yet).
    pub(super) fn play(&mut self, toolpaths: &[EntityId]) {
        self.finish_playback();
        let start = self
            .simulation
            .take()
            .unwrap_or_else(|| Material::new(&self.doc.job, self.display.preview_cells()));
        let list: Vec<&cut_craft_core::toolpath::Toolpath> = toolpaths
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        self.playback = Some(cut_craft_core::playback::Playback::new(start, &list));
        (self.playing, self.simulation_coarse, self.fine_3d) = (true, None, None);
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
    }

    /// Ends a playing preview, keeping what it has cut (Stop, or the end of the path).
    pub(super) fn finish_playback(&mut self) {
        if let Some(playback) = self.playback.take() {
            self.simulation = Some(playback.material().clone());
            (self.playing, self.simulation_coarse, self.fine_3d) = (false, None, None);
        }
    }

    pub(super) fn preview_toolpath(&mut self, id: EntityId) {
        if self.display.animate_preview {
            self.play(&[id]);
            return;
        }
        self.finish_playback();
        let Some(toolpath) = self.doc.toolpaths.get(&id) else {
            return;
        };
        self.simulation
            .get_or_insert_with(|| Material::new(&self.doc.job, self.display.preview_cells()))
            .cut(toolpath);
        (self.simulation_coarse, self.fine_3d) = (None, None);
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
        self.status
            .set_message(&format!("Previewed {}", toolpath.name));
    }

    /// Aspire's Preview Control while a preview plays: Run / Pause, Single Step, Run to
    /// Retract and Stop, the tool's place along the path (drag to scrub) and the speed. A
    /// playing preview moves on here each frame.
    pub(super) fn playback_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Toolpath playback bar");
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        if self.playing {
            let dt = f64::from(ui.input(|i| i.stable_dt));
            playback.seek(playback.at() + self.playback_speed * dt);
            ui.ctx().request_repaint();
        }
        let mut stop = playback.finished();
        ui.horizontal(|ui| {
            let run = if self.playing { "Pause" } else { "Run" };
            if ui.button(run).clicked() {
                self.playing = !self.playing;
            }
            if ui.button("Single Step").clicked() {
                self.playing = false;
                playback.seek(playback.next_step());
            }
            if ui.button("Run to Retract").clicked() {
                self.playing = false;
                playback.seek(playback.next_retract());
            }
            stop |= ui
                .button("Stop")
                .on_hover_text("Keep what is cut so far")
                .clicked();
            let mut at = playback.at();
            let length = playback.length();
            if ui
                .add(
                    egui::Slider::new(&mut at, 0.0..=length)
                        .suffix(" mm")
                        .show_value(false),
                )
                .on_hover_text("Where the tool is along the toolpaths")
                .changed()
            {
                self.playing = false;
                playback.seek(at);
            }
            ui.add(
                egui::Slider::new(&mut self.playback_speed, PLAYBACK_SPEEDS)
                    .logarithmic(true)
                    .suffix(" mm/s")
                    .text("Speed"),
            );
        });
        if stop {
            self.finish_playback();
        }
    }

    /// Aspire's arrows over the tree: the picked component up or down its level, as
    /// (component, direction, the id it moves to) when one is clicked.
    pub(super) fn tree_arrows(
        &self,
        ui: &mut egui::Ui,
    ) -> Option<(EntityId, cmd_reorder_component::Direction, EntityId)> {
        use cmd_reorder_component::{Direction, neighbour};
        let mut moved = None;
        let last = self.doc.selected_components.last();
        ui.horizontal(|ui| {
            for (label, direction) in [("Move Up", Direction::Up), ("Move Down", Direction::Down)] {
                let to = last.and_then(|id| neighbour(&self.doc, id, direction));
                if ui
                    .add_enabled(to.is_some(), egui::Button::new(label))
                    .clicked()
                {
                    moved = last.zip(to).map(|(id, to)| (id, direction, to));
                }
            }
        });
        moved
    }

    /// One component's row in the tree: whether a click picked it (Shift: adds or takes
    /// away; a right-click on an unpicked one picks it) and any action from its menu.
    pub(super) fn component_row(
        &self,
        ui: &mut egui::Ui,
        id: EntityId,
        component: &cut_craft_core::component::Component,
    ) -> (Option<(EntityId, bool)>, Option<String>) {
        let text = format!("{} ({:?})", component.name, component.combine);
        let on = self.doc.selected_components.contains(&id);
        let row = ui.selectable_label(on, text);
        let mut picked = None;
        if row.clicked() {
            picked = Some((id, ui.input(|i| i.modifiers.shift)));
        }
        if row.secondary_clicked() && !on {
            picked = Some((id, false));
        }
        let action = self.popup(&row, || gui::component_menu(&self.doc, id, component));
        (picked, action)
    }

    /// The component tree, top first: each level (the last on top) with its components
    /// under it, the model being built from the bottom up. Aspire's level and component
    /// menus on a right-click; right-clicking an unpicked component picks it first.
    /// A level's row in the component tree: Expand or Collapse, then its name and combine
    /// mode; the name's response (for its menu) and whether the icon was clicked.
    pub(super) fn level_header(
        &self,
        ui: &mut egui::Ui,
        level: &cut_craft_core::component::Level,
        collapsed: bool,
    ) -> (egui::Response, bool) {
        let shown = if level.visible { "" } else { " (hidden)" };
        ui.horizontal(|ui| {
            let (icon, tip) = if collapsed {
                ("view/expand.png", "Expand")
            } else {
                ("view/collapse.png", "Collapse")
            };
            let edge = ui.spacing().interact_size.y;
            let toggled = ui
                .add(self.icon_button(icon, "", tip, edge))
                .on_hover_text(tip)
                .clicked();
            let name = format!("{}{shown} ({:?})", level.name, level.combine);
            (ui.label(egui::RichText::new(name).strong()), toggled)
        })
        .inner
    }

    pub(super) fn component_list(&mut self, ui: &mut egui::Ui) {
        // the tree is the active sheet's, as Aspire's heading says
        if let Some(sheet) = self.doc.sheets.get(self.doc.active_sheet) {
            ui.strong(format!("Components ({})", sheet.name));
        }
        if self.doc.components.is_empty() {
            ui.weak("No components yet");
        }
        let components = &self.doc.components;
        self.doc
            .selected_components
            .retain(|id| components.contains_key(id));
        let first = self.doc.levels.keys().next().copied().unwrap_or(0);
        let on_level = |k: &cut_craft_core::component::Component, level: u64| {
            let at = if self.doc.levels.contains_key(&k.level) {
                k.level
            } else {
                first
            };
            at == level
        };
        let (mut chosen, mut picked) = (None, None);
        let moved = self.tree_arrows(ui);
        let mut toggle = None;
        for (&level_id, level) in self.doc.levels.iter().rev() {
            let collapsed = self.collapsed_levels.contains(&level_id);
            let (header, toggled) = self.level_header(ui, level, collapsed);
            if toggled {
                toggle = Some(level_id);
            }
            if let Some(action) =
                self.popup(&header, || gui::level_menu(&self.doc, level_id, level))
            {
                chosen = Some(action);
            }
            if collapsed {
                continue;
            }
            ui.indent(("level", level_id), |ui| {
                let on_this: Vec<(EntityId, &cut_craft_core::component::Component)> = self
                    .doc
                    .components
                    .iter()
                    .rev()
                    .filter(|(_, k)| on_level(k, level_id))
                    .map(|(&id, k)| (id, k))
                    .collect();
                let mut shown_groups = vec![];
                for &(id, component) in &on_this {
                    let Some(group) = &component.group else {
                        let (pick, action) = self.component_row(ui, id, component);
                        picked = pick.or(picked);
                        chosen = action.or(chosen.take());
                        continue;
                    };
                    // a group shows once, where its top member is, its members under it
                    if shown_groups.contains(&group.id) {
                        continue;
                    }
                    shown_groups.push(group.id);
                    let header = ui.label(format!("{} ({:?})", group.name, group.combine));
                    if let Some(action) = self.popup(&header, || gui::group_menu(group)) {
                        chosen = Some(action);
                    }
                    ui.indent(("group", group.id), |ui| {
                        for &(member, k) in &on_this {
                            if k.group.as_ref().is_some_and(|g| g.id == group.id) {
                                let (pick, action) = self.component_row(ui, member, k);
                                picked = pick.or(picked);
                                chosen = action.or(chosen.take());
                            }
                        }
                    });
                }
            });
        }
        // click picks one; Shift-click adds or takes away
        match picked {
            Some((id, true)) => {
                if !self.doc.selected_components.remove(&id) {
                    self.doc.selected_components.insert(id);
                }
            }
            Some((id, false)) => self.doc.selected_components = [id].into(),
            None => {}
        }
        if let Some(level) = toggle
            && !self.collapsed_levels.remove(&level)
        {
            self.collapsed_levels.insert(level);
        }
        if let Some((component, direction, to)) = moved {
            let params = cmd_reorder_component::Params {
                component,
                direction,
            };
            let params = serde_json::to_value(params).expect("params serialize");
            if self.apply("reorder_component", &params).is_ok() {
                // the moved component took its neighbour's id; keep it picked
                self.doc.selected_components = [to].into();
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// What the 3D view shows: the toolpath preview if there is one, else the model.
    // ponytail: rebuilt on every call, measured at about 0.1 ms of a 7.5 ms frame; cache it on
    // the history revision if that grows
    pub(super) fn surface_3d(&mut self) -> Option<Cow<'_, Material>> {
        if let Some(playback) = &self.playback {
            return Some(Cow::Owned(view3d::coarse(playback.material(), MESH_CELLS)));
        }
        if let Some(material) = &self.simulation {
            return Some(Cow::Borrowed(
                self.simulation_coarse
                    .get_or_insert_with(|| view3d::coarse(material, MESH_CELLS)),
            ));
        }
        // before a preview, the model alone: nothing where no component is
        (!self.doc.components.is_empty()).then(|| Cow::Owned(self.model_shape(MESH_CELLS)))
    }

    /// The model's shape, `cells` across; while Orientate 3D Model shows its model whole,
    /// without the component the form made of it.
    pub(super) fn model_shape(&mut self, cells: usize) -> Material {
        let drawn = self
            .form
            .as_ref()
            .filter(|f| f.model.is_some())
            .and_then(|f| f.drawn);
        // hidden for the moment, not as an edit: nothing to undo
        let show = |doc: &mut cut_craft_core::document::Document, visible: bool| {
            let k = drawn.and_then(|id| doc.components.get_mut(&id))?;
            Some(std::mem::replace(&mut k.visible, visible))
        };
        let was = show(&mut self.doc, false);
        let shape = cut_craft_core::component::model_shape(&self.doc, cells);
        if let Some(was) = was {
            show(&mut self.doc, was);
        }
        shape
    }

    /// The preview or model as a block, with Aspire's 3D mouse controls: left- or right-drag
    /// rotates, middle-drag (or Ctrl+right, or left and right together) pans, the wheel zooms
    /// about the cursor, and a double-click or F fits it again.
    pub(super) fn view_3d(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "3D view");
        self.playback_bar(ui);
        if self.simulation.is_none() && self.playback.is_none() && self.doc.components.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.weak("Nothing in 3D yet: create a shape or preview the toolpaths")
            });
            return;
        }
        ui.horizontal(|ui| {
            for (label, orbit) in [
                ("Top", view3d::Orbit::TOP),
                ("Front", view3d::Orbit::FRONT),
                ("Right", view3d::Orbit::RIGHT),
                ("Iso", view3d::Orbit::default()),
            ] {
                if ui.small_button(label).clicked() {
                    self.orbit = orbit;
                    self.framing = self.framing.fitted();
                }
            }
            if ui.small_button("Fit").clicked() {
                self.framing = self.framing.fitted();
            }
            // Aspire's orthographic / perspective toggle
            let perspective = self.framing.perspective;
            if ui.selectable_label(perspective, "Perspective").clicked() {
                self.framing.perspective = !perspective;
            }
        });
        let (area, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        // the ViewCube over the view's corner, so the view does not see its clicks
        let font = egui::TextStyle::Small.resolve(ui.style());
        let cube = view3d::ViewCube::new(area, ui.painter(), &font, ui.spacing().item_spacing.x);
        let on_cube = ui.interact(
            cube.rect(),
            ui.scope_id().with("view_cube"),
            egui::Sense::click_and_drag(),
        );
        let (left, right, middle, ctrl, scroll, fit_key) = ui.input(|i| {
            (
                i.pointer.primary_down(),
                i.pointer.secondary_down(),
                i.pointer.middle_down(),
                i.modifiers.ctrl,
                i.smooth_scroll_delta.y,
                i.key_pressed(egui::Key::F) || i.key_pressed(egui::Key::F6),
            )
        });
        let before = (self.orbit, self.framing);
        let delta = response.drag_delta();
        if response.dragged() && delta != egui::Vec2::ZERO {
            match view3d::drag_mode([left, right, middle], ctrl) {
                Some(view3d::DragMode::Rotate) => {
                    self.orbit = self.orbit.dragged(delta, area.size());
                }
                Some(view3d::DragMode::Pan) => self.framing = self.framing.panned(delta),
                None => {}
            }
        }
        // the ViewCube: a drag turns the view, a click looks from the face, edge or corner
        // clicked, and a double-click from the other side
        let orbit = self.orbit;
        let pointed = |at: Option<egui::Pos2>| at.and_then(|at| cube.direction_at(orbit, at));
        let hovered = pointed(on_cube.hover_pos());
        if on_cube.dragged() {
            self.orbit = self.orbit.dragged(on_cube.drag_delta(), area.size());
        } else if on_cube.double_clicked() {
            self.orbit = view3d::Orbit::facing(-self.orbit.towards_viewer());
        } else if let Some(from) =
            pointed(on_cube.interact_pointer_pos()).filter(|_| on_cube.clicked())
        {
            // ponytail: turns at once; Aspire's turns smoothly
            self.orbit = view3d::Orbit::facing(from);
        }
        if let Some(at) = response.hover_pos().filter(|_| scroll != 0.0) {
            // as in the 2D view: a quarter of the view's height doubles the zoom
            let factor = 2f32.powf(scroll / (area.height() / VIEW_ZOOM_QUARTERS));
            self.framing = self.framing.zoomed_about(at - area.center(), factor);
        }
        // a double-click picks the component under it, as in Aspire; off the model it fits
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let double = response
            .double_clicked()
            .then(|| response.interact_pointer_pos())
            .flatten();
        let picked = double.and_then(|at| self.component_at_3d(fit, at));
        if let Some(id) = picked {
            if ui.input(|i| i.modifiers.shift) {
                if !self.doc.selected_components.remove(&id) {
                    self.doc.selected_components.insert(id);
                }
            } else {
                self.doc.selected_components = [id].into();
            }
        } else if double.is_some() || (response.hovered() && fit_key) {
            self.framing = self.framing.fitted();
        }
        if response.dragged() || on_cube.dragged() || before != (self.orbit, self.framing) {
            self.moved_3d = ui.input(|i| i.time);
        }
        self.paint_3d(ui, area);
        let painter = ui.painter_at(area);
        cube.paint(&painter, self.orbit, hovered, ui.visuals(), &font);
        self.view_cube = Some(cube);
    }

    /// The 3D view's surface: quick (`surface`) while it moves, then once still the
    /// model's own detail, made once.
    pub(super) fn surface_mesh(
        &mut self,
        ui: &egui::Ui,
        surface: &Material,
        (orbit, framing): (view3d::Orbit, view3d::Framing),
        look: view3d::Look,
        fit: egui::Rect,
    ) -> egui::Mesh {
        let since = ui.input(|i| i.time) - self.moved_3d;
        let simplify = self.options.view_3d.simplify_while_moving;
        if self.playback.is_none() && (!simplify || since >= SETTLE_SECONDS) {
            return self.fine_mesh(orbit, framing, look, fit);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f64(SETTLE_SECONDS));
        view3d::mesh(surface, orbit, framing, &look, fit)
    }

    /// The 3D view's mesh at the model's (or preview's) own detail, kept while nothing it
    /// shows changes.
    pub(super) fn fine_mesh(
        &mut self,
        orbit: view3d::Orbit,
        framing: view3d::Framing,
        look: view3d::Look,
        fit: egui::Rect,
    ) -> egui::Mesh {
        let key = FineKey {
            revision: self.history.revision(),
            job: self.doc.job,
            orbit,
            framing,
            fit,
            look,
            importing: self.form.as_ref().is_some_and(|f| f.model.is_some()),
        };
        if let Some((kept, mesh)) = &self.fine_3d
            && *kept == key
        {
            return mesh.clone();
        }
        let surface = if let Some(material) = &self.simulation {
            view3d::coarse(material, FINE_MESH_CELLS)
        } else {
            let job = &self.doc.job;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a small cell count"
            )]
            let own = (job.width.max(job.height) / job.model_cell()).ceil() as usize;
            let cells = own.clamp(MESH_CELLS, FINE_MESH_CELLS);
            self.model_shape(cells)
        };
        let mesh = view3d::mesh(&surface, orbit, framing, &look, fit);
        self.fine_3d = Some((key, mesh.clone()));
        mesh
    }

    /// The topmost shown component at the point of the 3D view's picture `fit` under `at`.
    pub(super) fn component_at_3d(&mut self, fit: egui::Rect, at: egui::Pos2) -> Option<EntityId> {
        let (job, previewing) = (self.shown_job(), self.simulation.is_some());
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d()?.into_owned();
        let wrap = wrap_for(&job, on, previewing, &surface);
        let (column, row) = view3d::pick(&surface, orbit, framing, fit, wrap, at)?;
        let p = surface.centre(column, row);
        self.doc
            .components
            .iter()
            .rev()
            .filter(|(_, k)| k.visible)
            .find(|(_, k)| k.height_at(p).is_some())
            .map(|(&id, _)| id)
    }

    /// Behind the 3D view: Options' solid colour, gradient or picture (the gradient when the
    /// picture cannot be read).
    fn paint_background(&self, ui: &egui::Ui, painter: &egui::Painter, area: egui::Rect) {
        use crate::options::BackgroundStyle;
        let v = &self.options.view_3d;
        let colour = |[r, g, b]: [u8; 3]| egui::Color32::from_rgb(r, g, b);
        let (top, bottom) = (
            colour(v.background_color),
            colour(v.gradient_background_color),
        );
        match v.background_style {
            BackgroundStyle::Solid => {
                painter.rect_filled(area, 0.0, top);
            }
            BackgroundStyle::Image if !v.image_file_path.is_empty() => {
                painter.add(view3d::background(area, top, bottom));
                let uri = format!("file://{}", v.image_file_path);
                egui::Image::new(uri)
                    .fit_to_exact_size(area.size())
                    .paint_at(ui, area);
            }
            _ => {
                painter.add(view3d::background(area, top, bottom));
            }
        }
    }

    /// The 3D view's picture: background, block and the overlays the View menu turns on.
    /// A full 3D model being imported into a rotary job, turned round the axis inside a
    /// wireframe of the stock, over the wrapped view.
    fn paint_turned(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        surface: &cut_craft_core::simulation::Material,
        wrap: view3d::Wrap,
        fit: egui::Rect,
        look: &view3d::Look,
    ) {
        use cut_craft_core::glam::DVec3;
        let Some(model) = self.form.as_ref().and_then(|f| f.model.as_ref()) else {
            return;
        };
        let (orbit, framing) = (self.orbit, self.framing);
        let screen = view3d::screen_of(surface, orbit, framing, fit, Some(wrap));
        // from the job's millimetres (the axis at y = z = 0 for an X axis) to the wrapped
        // view's: along from the material's start; across and up as they are, the model at
        // its true size inside the stock
        let start = if wrap.along_x {
            surface.origin.x
        } else {
            surface.origin.y
        };
        let to_view = |p: DVec3| {
            if wrap.along_x {
                DVec3::new(p.x - start, p.y, p.z)
            } else {
                DVec3::new(p.x, p.y - start, p.z)
            }
        };
        let shown: Vec<cmd_orientate_3d_model::Seen> = model
            .iter()
            .map(|(t, hidden)| (t.map(to_view), *hidden))
            .collect();
        let mark = ui.visuals().error_fg_color;
        painter.add(view3d::triangles(&shown, &screen, orbit, look, mark));
        let job = self.shown_job().area().size();
        let length = if wrap.along_x { job.x } else { job.y };
        let stroke = egui::Stroke::new(ui.visuals().widgets.noninteractive.fg_stroke.width, mark);
        for [a, b] in view3d::cylinder_edges(wrap.along_x, wrap.radius, 0.0, length) {
            painter.line_segment([screen(a), screen(b)], stroke);
        }
    }

    pub(super) fn paint_3d(&mut self, ui: &egui::Ui, area: egui::Rect) {
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let shown = self.display;
        let [r, g, b] = self.shown_job().colour;
        let look = view3d::Look {
            colour: if shown.colour_shaded {
                egui::Color32::from_rgb(r, g, b)
            } else {
                ui.visuals().widgets.inactive.fg_stroke.color
            },
            // the block's sides belong to a preview; the model alone has no excess material
            sides: shown.material_block && (self.simulation.is_some() || self.playback.is_some()),
            light: view3d::Look::light_from(self.orbit, shown.light_follows),
            wrap: None,
            // a wood from Toolpaths ▸ Materials shows its grain
            grain: cmd_materials::Material::of([r, g, b])
                .grain()
                .filter(|_| shown.colour_shaded)
                .map(|(spacing, depth)| view3d::Grain { spacing, depth }),
        };
        let (orbit, framing) = (self.orbit, self.framing);
        let painter = ui.painter_at(area);
        if shown.shaded_background {
            self.paint_background(ui, &painter, area);
        }
        let job = self.shown_job();
        let previewing = self.simulation.is_some() || self.playback.is_some();
        // a playing preview's tool: where its tip is, and its diameter
        let tool_at = self
            .playback
            .as_ref()
            .and_then(cut_craft_core::playback::Playback::tool)
            .map(|(tip, tool)| (tip, tool.diameter));
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        // a full 3D model being imported into a rotary job is shown turned round the axis,
        // so the job is drawn wrapped while it is
        let turned = job.rotary.is_some()
            && self.form.as_ref().is_some_and(|f| {
                f.id == "orientate_3d_model"
                    && f.model.is_some()
                    && f.params["model_type"] == "full"
            });
        let wrap = wrap_for(&job, shown.auto_wrapping || turned, previewing, &surface);
        let look = view3d::Look { wrap, ..look };
        painter.add(self.surface_mesh(ui, &surface, (orbit, framing), look, fit));
        if let Some(wrap) = wrap {
            if turned {
                self.paint_turned(ui, &painter, &surface, wrap, fit, &look);
            }
            // ponytail: the overlays are drawn flat, so they are left off the wrapped view
            return;
        }
        let screen = view3d::screen_of(&surface, orbit, framing, fit, None);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        // an imported model whole, what a cutter cannot reach marked, inside the material's
        // outline to judge its size by
        if let Some(model) = self.form.as_ref().and_then(|f| f.model.as_ref()) {
            let origin = local(cut_craft_core::geometry::Point::new(0.0, 0.0), 0.0);
            let mark = ui.visuals().error_fg_color;
            painter.add(view3d::triangles(
                model,
                |p| screen(p + origin),
                orbit,
                &look,
                mark,
            ));
            let area = job.area();
            let (lo, hi) = (local(area.min, 0.0), local(area.max, job.thickness));
            let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
            for [a, b] in view3d::box_edges(lo, hi) {
                painter.line_segment([screen(a), screen(b)], stroke);
            }
        }
        // a playing preview's tool, an upright cylinder at the tip
        // ponytail: drawn whole, over the surface it is cutting into
        if let Some((tip, diameter)) = tool_at {
            let at = local(tip.truncate(), tip.z);
            let outline = view3d::cylinder(
                &screen,
                at,
                diameter / 2.0,
                diameter * TOOL_HEIGHT_DIAMETERS,
            );
            let selection = ui.visuals().selection;
            painter.add(egui::Shape::convex_polygon(
                outline,
                selection.bg_fill,
                selection.stroke,
            ));
        }
        self.overlays_3d(ui, &painter, &surface, (orbit, framing), fit);
    }

    /// The vectors, modeling plane and origin over the 3D view's (unwrapped) `surface`, each
    /// as the View menu says, leaving out what the surface hides, a cell at a time.
    fn overlays_3d(
        &self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        surface: &Material,
        (orbit, framing): (view3d::Orbit, view3d::Framing),
        fit: egui::Rect,
    ) {
        let shown = self.display;
        let vectors: Vec<Vec<cut_craft_core::geometry::Point>> = if shown.vectors_3d {
            self.doc
                .vectors
                .iter()
                .filter(|(id, _)| self.doc.shown(**id))
                .map(|(_, path)| path.flatten(cut_craft_core::polygon::TOLERANCE))
                .collect()
        } else {
            vec![]
        };
        let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
        let screen = view3d::screen_of(surface, orbit, framing, fit, None);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        let seen = view3d::seen_of(surface, orbit, framing);
        let in_sight = |points: &[cut_craft_core::glam::DVec3]| {
            view3d::visible_runs(points, surface.cell, &seen)
                .into_iter()
                .map(|run| run.into_iter().map(&screen).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        };
        // each vector flat, in one plane, on top of what it outlines
        for points in &vectors {
            let z = view3d::vector_plane(surface, points);
            let line: Vec<_> = points.iter().map(|&p| local(p, z)).collect();
            for run in in_sight(&line) {
                painter.add(egui::Shape::line(run, stroke));
            }
        }
        let size = view3d::plan_size(surface);
        if shown.modeling_plane {
            let corners = [
                (0.0, 0.0),
                (size.x, 0.0),
                (size.x, size.y),
                (0.0, size.y),
                (0.0, 0.0),
            ];
            let line = corners
                .iter()
                .map(|&(x, y)| cut_craft_core::glam::DVec3::new(x, y, 0.0))
                .collect::<Vec<_>>();
            let dash = ui.spacing().item_spacing.x;
            for run in in_sight(&line) {
                painter.add(egui::Shape::dashed_line(&run, stroke, dash, dash / 2.0));
            }
        }
        if shown.origin {
            // X, Y and Z from the job's origin, a tenth of the job long, in the usual axis colours
            let o = local(cut_craft_core::geometry::Point::ZERO, 0.0);
            let reach = size.x.max(size.y) / 10.0;
            let width = stroke.width * 2.0;
            for (axis, colour) in [
                (cut_craft_core::glam::DVec3::X, egui::Color32::RED),
                (cut_craft_core::glam::DVec3::Y, egui::Color32::GREEN),
                (cut_craft_core::glam::DVec3::Z, egui::Color32::BLUE),
            ] {
                for run in in_sight(&[o, o + axis * reach]) {
                    painter.add(egui::Shape::line(run, (width, colour)));
                }
            }
        }
    }

    /// The Toolpaths page: strategies, any form opened from them, and the toolpath list.
    pub(super) fn toolpaths_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Toolpaths page");
        self.page_header(ui, Page::Toolpaths, "Toolpaths");
        let groups = self.toolbar.toolpath_groups.clone();
        if self
            .form
            .as_ref()
            .is_some_and(|f| f.page == Page::Toolpaths)
        {
            egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
            return;
        }
        // the toolpath list in its own bottom half, as Aspire's, so it never scrolls away
        egui::Panel::bottom("toolpath_list")
            .resizable(true)
            .default_size(ui.available_height() / 2.0)
            .show(ui, |ui| self.toolpath_list(ui));
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.tools_or_favourites(ui, &groups, Page::Toolpaths);
        });
    }

    /// The Toolpath List: its buttons, then each toolpath, the selected one highlighted and
    /// a right-click on one opening its menu.
    pub(super) fn toolpath_list(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Toolpath list");
        ui.strong("Toolpath List");
        self.toolpath_list_buttons(ui);
        let actions = [gui::ToolGroup {
            name: "Toolpath List Actions",
            tools: &[
                "recalculate_toolpath",
                "recalculate_all_toolpaths",
                "delete_toolpath",
                "preview_toolpath",
                "preview_all_toolpaths",
                "reset_preview",
            ],
        }];
        self.tool_groups(ui, &actions, Page::Toolpaths);
        if self.doc.toolpaths.is_empty() {
            ui.weak("No toolpaths yet");
        }
        let mut chosen = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (&id, toolpath) in &self.doc.toolpaths {
                let selected = self.selected_toolpath == Some(id);
                let label = if toolpath.stale {
                    let name = egui::RichText::new(format!("⚠ {}", toolpath.name))
                        .color(ui.visuals().warn_fg_color);
                    ui.selectable_label(selected, name).on_hover_text(format!(
                        "{}\nOut of date: what it was made from has changed since. \
                         Recalculate it.",
                        toolpath.tool.name
                    ))
                } else {
                    ui.selectable_label(selected, &toolpath.name)
                        .on_hover_text(&toolpath.tool.name)
                };
                if label.clicked() {
                    self.selected_toolpath = if selected { None } else { Some(id) };
                }
                // Aspire's toolpath menu acts on the row right-clicked, so it selects it
                if label.secondary_clicked() {
                    self.selected_toolpath = Some(id);
                }
                if let Some(action) = self.popup(&label, || gui::toolpath_menu(id, toolpath)) {
                    chosen = Some(action);
                }
            }
        });
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    pub(super) fn status_bar(&self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Status bar");
        ui.horizontal(|ui| {
            ui.label(&self.status.message);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(p) = self.cursor {
                    ui.monospace(format!("X {:8.2}  Y {:8.2} mm", p.x, p.y));
                    ui.separator();
                }
                ui.label(format!("Tool: {}", self.status.current_tool));
            });
        });
    }
}
