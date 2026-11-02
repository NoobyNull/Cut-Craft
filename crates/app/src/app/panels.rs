//! The side panels and windows: Tool Database, layers, sheets, clipart, the component tree,
//! playback and the 3D view.

use super::RING_SHARE;
use super::{
    Align, Anchor, Change, Cow, CutCraftApp, EXPORT_LEVEL, EntityId, FINE_MESH_CELLS, FineKey,
    Handle, Handle3d, LayerId, Layout, MESH_CELLS, MODELING_PLANE_ALPHA, MODELING_PLANE_MARGIN,
    Material, Menus, PLAYBACK_SPEEDS, Page, Point, SETTLE_SECONDS, Screen, Settings3d,
    TOOL_HEIGHT_DIAMETERS, TOOL_ICON_ROWS, VIEW_ZOOM_QUARTERS, Value, View, ViewTool, egui,
    file_dialog, file_name, files, gui, handles, menu_items, sheet_length, view2d, view3d,
    wrap_for,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::{MachinedArea, Units};

impl CutCraftApp {
    /// The Tool Database manager; its edits are kept on Apply or OK.
    pub(super) fn tool_database_window(&mut self, ctx: &egui::Context) {
        use cut_craft_tool_database::Outcome;
        let Some(manager) = &mut self.tool_manager else {
            return;
        };
        let icons = &self.icons;
        let picture = |path: &str| icons.find(path, "").map(|p| icons.image(p));
        // a form tool takes its shape from the one open vector selected
        let profile = match self.doc.selection.iter().collect::<Vec<_>>()[..] {
            [id] => self.doc.vectors.get(id).filter(|p| !p.closed),
            _ => None,
        };
        let look = cut_craft_tool_database::Look {
            picture: &picture,
            profile,
        };
        let Some(outcome) = manager.show(ctx, &look) else {
            return;
        };
        match outcome {
            Outcome::Apply | Outcome::Ok => {
                self.tools = manager.draft.clone();
                let changer = manager.changer.clone();
                if let Some(changer) = &changer {
                    self.tools.number_by_slots(&changer.slots);
                }
                if outcome == Outcome::Ok {
                    self.tool_manager = None;
                }
                self.tools_changed("Tool database saved");
                self.refresh_form_tool();
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
                        .on_hover_hint(gui::about(id).unwrap_or(&label))
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
    /// The arrows over the Layers List: the active layer one place up or down the list.
    fn layer_order_arrows(&mut self, ui: &mut egui::Ui) {
        use cmd_reorder_layer::{Direction, moved_to};
        let layer = self.doc.current_layer;
        let mut moved = None;
        ui.horizontal(|ui| {
            for (direction, arrow, tip) in [
                (
                    Direction::Up,
                    "Move Up",
                    "Move the active layer up the list",
                ),
                (
                    Direction::Down,
                    "Move Down",
                    "Move the active layer down the list",
                ),
            ] {
                let can = moved_to(&self.doc, layer, direction).is_some();
                if ui
                    .add_enabled(can, egui::Button::new(arrow))
                    .on_hover_hint(tip)
                    .clicked()
                {
                    moved = Some(direction);
                }
            }
        });
        ui.checkbox(
            &mut self.layers_on_sheet,
            "Show only layers used on this sheet",
        )
        .on_hover_hint("Leave out the layers with nothing on the active sheet");
        if let Some(direction) = moved {
            let params = serde_json::json!({ "layer": layer, "direction": direction });
            if let Err(e) = self.apply("reorder_layer", &params) {
                self.status.set_message(&e);
            }
        }
    }

    pub(super) fn layers_panel(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Layers tab");
        self.layer_order_arrows(ui);
        let edge = ui.spacing().interact_size.y;
        let label = |id: &str| self.registry.get(id).map_or("", |c| c.meta().label);
        let mut clicked: Option<(LayerId, &str)> = None;
        let mut chosen = None;
        let shown = |l: &&cut_craft_core::document::Layer| {
            !self.layers_on_sheet
                || l.id == self.doc.current_layer
                || self.doc.vectors_on(l.id).next().is_some()
        };
        for layer in self.doc.layers.iter().rev().filter(shown) {
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
                    .on_hover_hint(eye_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_visible"));
                }
                if ui
                    .add(self.icon_button(lock, "", lock_tip, edge))
                    .on_hover_hint(lock_tip)
                    .clicked()
                {
                    clicked = Some((layer.id, "toggle_locked"));
                }
                let count = self.doc.vectors_on(layer.id).count();
                let row = ui
                    .selectable_label(self.doc.current_layer == layer.id, &layer.name)
                    .on_hover_hint(format!("{count} vectors"));
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

    /// The Library tab: its folders, and the chosen folder's clipart to add to the project.
    pub(super) fn clipart_panel(&mut self, ui: &mut egui::Ui) {
        use crate::clipart::Picked;
        cut_craft_bug_report::mark(ui, "Library tab");
        let importable: crate::clipart::Importable = (self.registry.importers())
            .flat_map(|i| i.format.extensions)
            .map(|e| e.to_lowercase())
            .collect();
        let levels: Vec<(u64, String)> = (self.doc.levels.iter())
            .map(|(&id, l)| (id, l.name.clone()))
            .collect();
        match crate::clipart::panel(ui, &mut self.clipart, &importable, &levels) {
            Some(Picked::AddFolder) => self.invoke(ui.ctx(), "add_clipart_folder"),
            Some(Picked::ShowFolder(folder)) => {
                if let Err(e) = files::show_folder(&folder) {
                    self.status
                        .set_message(&format!("Couldn't show {}: {e}", folder.display()));
                }
            }
            Some(Picked::Import(path)) => self.add_from_library(vec![path], None),
            Some(Picked::AddToProject(files)) => self.add_from_library(files, None),
            Some(Picked::AddToLevel(files, level)) => self.add_from_library(files, Some(level)),
            Some(Picked::Locate(path)) => {
                if let Err(e) = files::locate(&path) {
                    self.status
                        .set_message(&format!("Couldn't show {}: {e}", path.display()));
                }
            }
            Some(Picked::Delete(doomed)) => {
                if confirm_delete(&doomed)
                    && let Err(e) = self.clipart.delete(&doomed)
                {
                    self.status.set_message(&e);
                }
            }
            None => {}
        }
    }

    /// Adds `files` from the Library to the project, one after another: each model through
    /// Orientate 3D Model, the next once its form is closed (applied or cancelled).
    pub(super) fn add_from_library(
        &mut self,
        files: Vec<std::path::PathBuf>,
        level: Option<crate::clipart::ToLevel>,
    ) {
        self.import_queue
            .extend(files.into_iter().map(|f| (f, level)));
        self.next_import();
    }

    /// The next file waiting to be added from the Library, when no form is open.
    pub(super) fn next_import(&mut self) {
        while self.form.is_none()
            && let Some((path, level)) = self.import_queue.pop_front()
        {
            if path.is_file() {
                if level.is_some() {
                    self.import_clipart(&path, None, level);
                } else {
                    self.import_file(&path);
                }
            } else {
                self.status.set_message(&format!(
                    "Couldn't add {}: the file isn't there any more.",
                    path.display()
                ));
            }
        }
    }

    /// Brings clipart `path` into the job as a double-click does, then (dropped on a view)
    /// centred on `at`, and (Import to Level) on `level`: one of the job's, or a new level
    /// named after the file. A model goes through Orientate 3D Model, placed and
    /// levelled as it is made.
    pub(super) fn import_clipart(
        &mut self,
        path: &std::path::Path,
        at: Option<cut_craft_core::geometry::Point>,
        level: Option<crate::clipart::ToLevel>,
    ) {
        let name = path
            .file_stem()
            .map_or_else(|| "Clipart".into(), |s| s.to_string_lossy().into_owned());
        // a new level first, as its own step: the import goes onto it
        let level = level.map(|level| match level {
            crate::clipart::ToLevel::Existing(id) => id,
            crate::clipart::ToLevel::New => {
                let id = self
                    .doc
                    .levels
                    .keys()
                    .next_back()
                    .map_or(0, |last| last + 1);
                let mut c = Change::builder(&self.doc);
                c.set_level(
                    id,
                    Some(cut_craft_core::component::Level {
                        name: name.clone(),
                        ..Default::default()
                    }),
                );
                self.record(Ok(c.finish()), &format!("Inserted level {name}"));
                id
            }
        });
        if is_model(path) {
            self.import_file(path);
            self.import_level = level;
            if let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") {
                if let Some(at) = at {
                    let size = |k: &str| form.params["size"][k].as_f64().unwrap_or(0.0);
                    let corner = serde_json::json!({
                        "x": at.x - size("x") / 2.0, "y": at.y - size("y") / 2.0
                    });
                    form.params["placing"]["center"] = Value::from(false);
                    form.params["placing"]["position"] = corner;
                }
                self.apply_live();
            }
            return;
        }
        let change = files::import(&self.registry, &self.doc, path)
            .map(|c| match level {
                Some(level) => crate::clipart::onto_level(c, level),
                None => c,
            })
            .map(|c| match at {
                Some(at) => centred_on(c, at),
                None => c,
            })
            .map_err(cut_craft_core::CommandError::Failed);
        self.record(change, &format!("Imported {}", path.display()));
    }

    /// Brings `path` into the job: meshes through Orientate 3D Model, as Aspire's import
    /// does; anything else through its importer.
    pub(super) fn import_file(&mut self, path: &std::path::Path) {
        if is_model(path) {
            self.start_command("orientate_3d_model");
            // the model is placed in 3D, so it is shown there
            self.view = View::ThreeD;
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
        cut_craft_bug_report::context_menu(response, |ui| {
            let menus = Menus {
                available: &|id| self.available(id),
                checked: &|_| None,
                shortcut_of: &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                recent: &[],
                image: &|path| self.icons.find(path, "").map(|p| self.icons.image(p)),
            };
            menu_items(ui, &menu(), &menus, &mut clicked);
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

    /// Preview All (or, `visible_only`, Preview Visible): start again from uncut material,
    /// then preview each toolpath (each shown one) in list order.
    pub(super) fn preview_all(&mut self, visible_only: bool) {
        let visible: Vec<EntityId> = self
            .doc
            .toolpaths
            .iter()
            .filter(|(_, t)| t.visible || !visible_only)
            .map(|(&id, _)| id)
            .collect();
        self.preview_ids(&visible);
        self.preview_other_side(visible_only);
    }

    /// Previews `ids` in order on fresh material, as one preview to undo.
    pub(super) fn preview_ids(&mut self, ids: &[EntityId]) {
        self.finish_playback();
        self.before_preview = Some(super::BeforePreview(self.simulation.take()));
        if self.display.animate_preview {
            self.play(ids);
            return;
        }
        let before = self.before_preview.take();
        for &id in ids {
            self.preview_toolpath(id);
        }
        // the cascade is one preview to undo
        self.before_preview = before;
    }

    /// A double-sided job's other side previewed too (its shown toolpaths, or all), cut
    /// into the material's other side, which the 3D view shows as its underside.
    fn preview_other_side(&mut self, visible_only: bool) {
        if self.doc.job.double_sided.is_none() || self.doc.other_side.toolpaths.is_empty() {
            return;
        }
        let (job, cells) = (self.doc.job, self.display.preview_cells());
        let mut other = Material::new(&job, cells);
        for t in self.doc.other_side.toolpaths.values() {
            if t.visible || !visible_only {
                other.cut(t);
            }
        }
        let other = Some(std::sync::Arc::new(other));
        if let Some(playback) = self.playback.as_mut() {
            playback.set_other_side(other);
        } else {
            let material = self
                .simulation
                .get_or_insert_with(|| Material::new(&job, cells));
            material.other_side = other;
        }
        (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
    }

    /// What a preview cuts: the job's toolpaths, or (Toolpath Tiling with a tile active)
    /// only the active tile's moves, at the machine's corner or in their original place.
    fn preview_source(&self) -> Cow<'_, cut_craft_core::document::Document> {
        match self.doc.tiling.and_then(|t| t.preview(&self.doc)) {
            Some(tiled) => Cow::Owned(tiled),
            None => Cow::Borrowed(&self.doc),
        }
    }

    /// Starts playing `toolpaths` into the previewed material (uncut if there is none yet).
    pub(super) fn play(&mut self, toolpaths: &[EntityId]) {
        self.finish_playback();
        let source = self.preview_source();
        let (list, marks): (Vec<cut_craft_core::toolpath::Toolpath>, Vec<u32>) = toolpaths
            .iter()
            .filter_map(|&id| Some((source.toolpaths.get(&id)?.clone(), super::preview_mark(id))))
            .unzip();
        drop(source);
        let start = self
            .simulation
            .take()
            .unwrap_or_else(|| Material::new(&self.doc.job, self.display.preview_cells()));
        let list: Vec<&cut_craft_core::toolpath::Toolpath> = list.iter().collect();
        self.playback = Some(cut_craft_core::playback::Playback::new(start, &list).marked(&marks));
        (
            self.playing,
            self.simulation_coarse,
            self.fine_3d,
            self.surface_gpu,
        ) = (true, None, None, None);
        if self.display.auto_open_3d {
            self.view = View::ThreeD;
        }
    }

    /// Ends a playing preview, keeping what it has cut (Stop, or the end of the path).
    pub(super) fn finish_playback(&mut self) {
        if let Some(playback) = self.playback.take() {
            self.simulation = Some(playback.material().clone());
            (
                self.playing,
                self.simulation_coarse,
                self.fine_3d,
                self.surface_gpu,
            ) = (false, None, None, None);
        }
    }

    pub(super) fn preview_toolpath(&mut self, id: EntityId) {
        if self.display.animate_preview {
            self.finish_playback();
            self.before_preview = Some(super::BeforePreview(self.simulation.clone()));
            self.play(&[id]);
            return;
        }
        self.finish_playback();
        let Some(toolpath) = self.preview_source().toolpaths.get(&id).cloned() else {
            return;
        };
        self.before_preview = Some(super::BeforePreview(self.simulation.clone()));
        let material = self
            .simulation
            .get_or_insert_with(|| Material::new(&self.doc.job, self.display.preview_cells()));
        material.cutting = super::preview_mark(id);
        material.cut(&toolpath);
        (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
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
                .on_hover_hint("Keep what is cut so far")
                .clicked();
            ui.add(
                egui::Slider::new(&mut self.playback_speed, PLAYBACK_SPEEDS)
                    .logarithmic(true)
                    .suffix(" mm/s")
                    .text("Speed"),
            );
        });
        // the progress across the view: drag it to move the tool on through the toolpaths, or
        // back
        ui.horizontal(|ui| {
            ui.label("Progress");
            let mut at = playback.at();
            let length = playback.length();
            let percent = move |d: f64| {
                if length > 0.0 {
                    d / length * 100.0
                } else {
                    100.0
                }
            };
            ui.spacing_mut().slider_width =
                ui.available_width() - ui.spacing().interact_size.x * 2.0;
            if ui
                .add(
                    egui::Slider::new(&mut at, 0.0..=length)
                        .custom_formatter(move |d, _| format!("{:.0}%", percent(d)))
                        .custom_parser(move |text| {
                            let p: f64 = text.trim().trim_end_matches('%').trim().parse().ok()?;
                            Some(p / 100.0 * length)
                        }),
                )
                .on_hover_hint("Where the tool is along the toolpaths: drag to move it on or back")
                .changed()
            {
                self.playing = false;
                playback.seek(at);
            }
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
                .on_hover_hint(tip)
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
            let coarse = view3d::coarse(playback.material(), MESH_CELLS);
            return Some(Cow::Owned(self.painted(coarse)));
        }
        if let Some(material) = &self.simulation {
            if self.simulation_coarse.is_none() {
                self.simulation_coarse = Some(view3d::coarse(material, MESH_CELLS));
            }
            let coarse = self.simulation_coarse.as_ref().expect("made above");
            if self.doc.job.machined_area == MachinedArea::MaterialColor
                && coarse.other_side.is_none()
            {
                return Some(Cow::Borrowed(coarse));
            }
            return Some(Cow::Owned(self.painted(coarse.clone())));
        }
        // before a preview, the model alone: nothing where no component is
        (!self.doc.components.is_empty()).then(|| Cow::Owned(self.model_shape(MESH_CELLS)))
    }

    /// The preview's `material` painted as the job's Machined Area Color has it: in the
    /// material's colour, one fill colour, or each toolpath's own.
    fn painted(&self, material: Material) -> Material {
        let job = &self.doc.job;
        // a double-sided preview's other side, on the underside
        let material = match job.double_sided {
            Some(sides) => {
                let turn = sides.flip.mirror(job.area());
                material.with_underside(|p| turn.transform_point2(p))
            }
            None => material,
        };
        let rgb = |[r, g, b]: [u8; 3]| egui::Color32::from_rgb(r, g, b);
        match job.machined_area {
            MachinedArea::MaterialColor => material,
            MachinedArea::GlobalFillColor => {
                view3d::painted(material, |_| Some(rgb(job.fill_colour)))
            }
            MachinedArea::ToolpathColor => {
                let fills: std::collections::HashMap<u32, Option<[u8; 3]>> = self
                    .doc
                    .toolpaths
                    .iter()
                    .enumerate()
                    .map(|(place, (&id, t))| (super::preview_mark(id), t.preview_colour(place)))
                    .collect();
                view3d::painted(material, |id| fills.get(&id).copied().flatten().map(rgb))
            }
            MachinedArea::Lithophane => {
                view3d::lit_through(material, rgb(job.colour), job.lithophane_light)
            }
        }
    }

    /// The model's shape, `cells` across; while Orientate 3D Model shows its model whole,
    /// without the component the form made of it.
    pub(super) fn model_shape(&mut self, cells: usize) -> Material {
        let drawn = self
            .form
            .as_ref()
            .filter(|f| f.model.is_some() && f.model_whole)
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

    /// The SpaceMouse's moves since the last frame, turning, panning and zooming the 3D view
    /// while it shows (Options ▸ SpaceMouse); its Fit, Top, Right and Front buttons too.
    pub(super) fn space_mouse_input(&mut self, ctx: &egui::Context) {
        use crate::spacemouse::{ButtonView, Input, button_view, moved};
        let Some(inputs) = self
            .space_mouse
            .as_ref()
            .map(|rx| rx.try_iter().collect::<Vec<_>>())
        else {
            return;
        };
        if self.view != View::ThreeD && self.split.is_none() {
            return;
        }
        let rotation = self.options.view_3d.rotation.view();
        for input in inputs {
            match input {
                Input::Motion {
                    translate,
                    rotate,
                    period,
                } => {
                    // spacenavd's own period, else this frame's; a pause before it is not
                    // a motion
                    let seconds = if period > 0 {
                        f64::from(period) / 1000.0
                    } else {
                        f64::from(ctx.input(|i| i.stable_dt))
                    }
                    .min(crate::spacemouse::LONGEST_STEP);
                    let view = (self.orbit, self.framing);
                    let settings = &self.options.space_mouse;
                    (self.orbit, self.framing) =
                        moved(view, (translate, rotate), seconds, settings, rotation);
                    self.moved_3d = ctx.input(|i| i.time);
                }
                Input::Button {
                    number,
                    pressed: true,
                } => match button_view(number) {
                    Some(view) => {
                        if let ButtonView::Look(orbit) = view {
                            self.orbit = orbit;
                        }
                        self.framing = self.framing.fitted();
                    }
                    None => self.status.set_message(&format!(
                        "SpaceMouse button {number} does nothing in Cut Craft yet"
                    )),
                },
                Input::Button { .. } => {}
            }
        }
    }

    /// The 3D view's row of standard views, Fit and the perspective toggle.
    fn view_3d_buttons(&mut self, ui: &mut egui::Ui) {
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
    }

    /// The preview or model as a block, with Aspire's 3D mouse controls: left- or right-drag
    /// rotates, middle-drag (or Ctrl+right, or left and right together) pans, the wheel zooms
    /// about the cursor, and a double-click or F fits it again.
    /// A drag in the 3D view while Orientate 3D Model is open: with Drag turns on the model,
    /// a turning drag adds to its fine turn (the view's height is half a turn); with Drag
    /// moves the axis on, a pan drag moves the model off the axis (the view's height is
    /// about two diameters). Whether the drag was taken.
    pub(super) fn import_drag(
        &mut self,
        mode: Option<view3d::DragMode>,
        delta: egui::Vec2,
        size: egui::Vec2,
    ) -> bool {
        let (diameter, ring) = (self.doc.job.rotary.map(|r| r.diameter), self.turn_ring);
        let Some(form) = self.form.as_mut().filter(|f| f.id == "orientate_3d_model") else {
            return false;
        };
        let add = |value: &mut Value, by: f64| {
            *value = Value::from(value.as_f64().unwrap_or(0.0) + by);
        };
        let (dx, dy) = (f64::from(delta.x / size.x), f64::from(delta.y / size.y));
        match mode {
            Some(view3d::DragMode::Rotate) => {
                let turn = &mut form.params["turn"];
                let (about_x, about_z) = (dy * 180.0, dx * 180.0);
                // a drag from a rotation ring turns about its axis, whatever Drag turns says
                let mode = ring.or(turn["mode"].as_str()).map(str::to_owned);
                match mode.as_deref() {
                    Some("x") => add(&mut turn["x"], about_x),
                    Some("y") => add(&mut turn["y"], about_z),
                    Some("z") => add(&mut turn["z"], about_z),
                    Some("xyz") => {
                        add(&mut turn["x"], about_x);
                        add(&mut turn["z"], about_z);
                    }
                    _ => return false,
                }
            }
            Some(view3d::DragMode::Pan) => {
                let Some(diameter) = diameter.filter(|_| form.params["axis"]["drag"] == true)
                else {
                    return false;
                };
                let per_view = 2.0 * diameter;
                add(&mut form.params["axis"]["across"], dx * per_view);
                add(&mut form.params["axis"]["up"], -dy * per_view);
            }
            None => return false,
        }
        form.dirty = true;
        true
    }

    pub(super) fn view_3d(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "3D view");
        self.playback_bar(ui);
        if self.simulation.is_none() && self.playback.is_none() && self.doc.components.is_empty() {
            let empty = ui.centered_and_justified(|ui| {
                ui.weak("Nothing in 3D yet: create a shape or preview the toolpaths")
            });
            // clipart dropped on the empty view goes in the middle of the job
            if let Some(path) = empty.response.dnd_release_payload::<std::path::PathBuf>() {
                self.import_clipart(&path, None, None);
            }
            return;
        }
        ui.horizontal(|ui| self.view_3d_buttons(ui));
        let (area, response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        self.view_3d_area = Some(area);
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
        let (before, rotation) = (
            (self.orbit, self.framing),
            self.options.view_3d.rotation.view(),
        );
        // while sculpting the left button sculpts; the right turns the view, the middle pans
        let sculpting = self.view_3d_drop_and_sculpt(ui, &response, area);
        let left = left && !sculpting;
        let delta = response.drag_delta();
        self.take_ring(ui, &response);
        if response.dragged() && delta != egui::Vec2::ZERO {
            let mode = view3d::drag_mode([left, right, middle], ctrl);
            // Orientate 3D Model's Interactive Rotation and Rotation Axis Movement take the
            // drag for the model when they are on
            if !self.import_drag(mode, delta, area.size()) {
                match mode {
                    Some(view3d::DragMode::Rotate) => {
                        self.orbit = self.orbit.dragged_by(delta, area.size(), rotation);
                    }
                    Some(view3d::DragMode::Pan) => self.framing = self.framing.panned(delta),
                    None => {}
                }
            }
        }
        // the ViewCube: a drag turns the view, a click looks from the face, edge or corner
        // clicked, and a double-click from the other side
        let orbit = self.orbit;
        let pointed = |at: Option<egui::Pos2>| at.and_then(|at| cube.direction_at(orbit, at));
        let hovered = pointed(on_cube.hover_pos());
        if on_cube.dragged() {
            self.orbit = (self.orbit).dragged_by(on_cube.drag_delta(), area.size(), rotation);
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
        self.fit_3d = Some(fit);
        let double = response
            .double_clicked()
            .then(|| response.interact_pointer_pos())
            .flatten();
        // while a preview shows, a double-click takes away the piece cut free under it
        let removed = double
            .filter(|_| self.playback.is_none() && self.simulation.is_some())
            .is_some_and(|at| self.remove_preview_piece(fit, at));
        let picked = double
            .filter(|_| !removed)
            .and_then(|at| self.component_at_3d(fit, at));
        if removed {
        } else if let Some(id) = picked {
            self.pick_3d(ui, id);
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
        self.handles_3d(ui, area, fit);
        self.rings_3d(ui, area, fit);
        self.anchors_3d(ui, &response, area, fit);
        self.view_3d_menu(ui, &response, fit);
    }

    /// A double-click on component `id`: Shift adds or takes it away; again on a selected
    /// one, its editing handles, as in Aspire; else it alone is selected.
    fn pick_3d(&mut self, ui: &egui::Ui, id: EntityId) {
        if ui.input(|i| i.modifiers.shift) {
            if !self.doc.selected_components.remove(&id) {
                self.doc.selected_components.insert(id);
            }
        } else if self.doc.selected_components.contains(&id) {
            self.invoke(ui.ctx(), "transform");
        } else {
            self.doc.selected_components = [id].into();
        }
    }

    /// A right-click picks the topmost component under it (keeping a selection it is part
    /// of) and opens the view menu listing all of them, as the 2D view's does.
    fn view_3d_menu(&mut self, ui: &egui::Ui, response: &egui::Response, fit: egui::Rect) {
        if response.secondary_clicked()
            && let Some(at) = response.interact_pointer_pos()
        {
            self.under_3d = self.components_at_3d(fit, at);
            if let Some(&id) = self.under_3d.first()
                && !self.doc.selected_components.contains(&id)
            {
                self.doc.selected_components = [id].into();
            }
        }
        let menu = || self.menus.view_3d_menu(&self.doc, &self.under_3d);
        if let Some(action) = self.popup(response, menu) {
            self.invoke(ui.ctx(), &action);
        }
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
            importing: self
                .form
                .as_ref()
                .is_some_and(|f| f.model.is_some() && f.model_whole),
        };
        if let Some((kept, mesh)) = &self.fine_3d
            && *kept == key
        {
            return mesh.clone();
        }
        let surface = self.fine_surface();
        let mesh = view3d::mesh(&surface, orbit, framing, &look, fit);
        self.fine_3d = Some((key, mesh.clone()));
        mesh
    }

    /// The preview's material, or the model's shape, at its own detail up to
    /// [`FINE_MESH_CELLS`].
    fn fine_surface(&mut self) -> Material {
        if let Some(material) = &self.simulation {
            return self.painted(view3d::coarse(material, FINE_MESH_CELLS));
        }
        let job = &self.doc.job;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a small cell count"
        )]
        let own = (job.width.max(job.height) / job.model_cell()).ceil() as usize;
        self.model_shape(own.clamp(MESH_CELLS, FINE_MESH_CELLS))
    }

    /// The 3D view's surface as the GPU draws it: at its own detail, built once and kept
    /// while nothing it shows changes; a playing preview's at the moving detail, every frame.
    fn surface_geometry(
        &mut self,
        wrap: Option<view3d::Wrap>,
        sides: bool,
    ) -> std::sync::Arc<view3d::gpu::Geometry> {
        let key = super::GeometryKey {
            revision: self.history.revision(),
            job: self.doc.job,
            wrap,
            sides,
            importing: self
                .form
                .as_ref()
                .is_some_and(|f| f.model.is_some() && f.model_whole),
        };
        if let Some(playback) = &self.playback {
            self.solid_gpu = None;
            let surface = self.painted(view3d::coarse(playback.material(), MESH_CELLS));
            return std::sync::Arc::new(view3d::gpu::surface(&surface, wrap, sides));
        }
        if let Some((kept, geometry)) = &self.surface_gpu
            && *kept == key
        {
            return std::sync::Arc::clone(geometry);
        }
        // while a handle drags the model about, the quick surface, made again each frame
        // and not kept, so the fine one is made once the drag ends
        let dragging = self.drag_base.is_some();
        let mut surface = if dragging {
            self.model_shape(MESH_CELLS)
        } else {
            self.fine_surface()
        };
        // the components' own colours (Solid Color), where the model is theirs: painted as a
        // preview paints the toolpaths' colours
        if self.simulation.is_none() {
            let doc = &self.doc;
            let paint: Vec<u32> = (0..surface.rows)
                .flat_map(|r| (0..surface.columns).map(move |c| (c, r)))
                .map(|(c, r)| {
                    cut_craft_core::component::colour_at(doc, surface.centre(c, r))
                        .map_or(0, |[r, g, b]| u32::from_be_bytes([1, r, g, b]))
                })
                .collect();
            if paint.iter().any(|&p| p != 0) {
                surface.paint = paint;
            }
        }
        // the model alone (no preview) is drawn solid down to the modeling plane
        self.solid_gpu = self
            .simulation
            .is_none()
            .then(|| std::sync::Arc::new(view3d::gpu::model(&view3d::solid(&surface, wrap))));
        let geometry = std::sync::Arc::new(view3d::gpu::surface(&surface, wrap, sides));
        if !dragging {
            self.surface_gpu = Some((key, std::sync::Arc::clone(&geometry)));
        }
        geometry
    }

    /// The imported model as the GPU draws it, kept while the model stays.
    fn model_geometry(
        &mut self,
        model: &std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
    ) -> std::sync::Arc<view3d::gpu::Geometry> {
        if let Some(kept) = &self.model_gpu
            && std::sync::Arc::ptr_eq(&kept.model, model)
        {
            return std::sync::Arc::clone(&kept.geometry);
        }
        let geometry = std::sync::Arc::new(view3d::gpu::model(model));
        self.model_gpu = Some(super::ModelGpu {
            model: std::sync::Arc::clone(model),
            geometry: std::sync::Arc::clone(&geometry),
        });
        geometry
    }

    /// Transform mode's handles in the 3D view, over whatever is selected: Aspire's 3D
    /// Transform Handles (see [`Self::transform_handles_3d`]) for vectors and components
    /// alike, and for a component, Aspire 12's component editing icons, round buttons
    /// along the near edge of its box; a drag on the first three sets its total, shape or
    /// base height, and the last opens its settings. The model's own view has them, not a
    /// preview. With nothing selected the status bar says how to start.
    pub(super) fn handles_3d(&mut self, ui: &mut egui::Ui, area: egui::Rect, fit: egui::Rect) {
        self.handles_3d.clear();
        let importing = self.form.as_ref().is_some_and(|f| f.model.is_some());
        if self.simulation.is_some()
            || self.playback.is_some()
            || importing
            || self.view_tool != Some(ViewTool::Transform)
        {
            // a drag the mode was left in the middle of ends here, as far as it went
            self.drag_3d = None;
            self.drag_base = None;
            return;
        }
        let Some(bounds) = self.doc.selection_bounds() else {
            self.drag_3d = None;
            if ui.rect_contains_pointer(area) {
                cut_craft_bug_report::hint::hint(
                    ui.ctx(),
                    "Transform: click a component or vector to select it, then drag its \
                     handles to move, scale or rotate it",
                );
            }
            return;
        };
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        let camera = view3d::Camera::new(&surface, self.orbit, self.framing, fit, None);
        let plane = Plane3d {
            camera: &camera,
            origin: surface.origin,
            z: top_over(&surface, bounds),
        };
        let painter = ui.painter_at(area);
        let chosen = self
            .doc
            .selected_components
            .iter()
            .rev()
            .find(|id| self.doc.components.contains_key(id))
            .copied();
        if let Some(id) = chosen {
            self.component_icons_3d(ui, &painter, &plane, id);
        }
        self.transform_handles_3d(ui, &painter, &plane, bounds);
        if let Some(id) = chosen {
            self.handle_drag_3d(ui, fit, id);
        }
    }

    /// Aspire 12's component editing icons along the near edge of component `id`'s box, a
    /// button's height below it (the transform handles sit on the edge itself).
    fn component_icons_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        id: EntityId,
    ) {
        let extent = self.doc.components[&id].extent();
        let (near, far) = (
            plane.screen(extent.min),
            plane.screen(Point::new(extent.max.x, extent.min.y)),
        );
        // a button's height apart; an edge seen end-on runs to the right
        let edge = ui.spacing().interact_size.y;
        let mut along = (far - near).normalized();
        if !along.is_finite() || along == egui::Vec2::ZERO {
            along = egui::Vec2::X;
        }
        let mut below = along.rot90();
        if below.dot(near - plane.screen(extent.center())) < 0.0 {
            below = -below;
        }
        for (i, handle) in Handle3d::ALL.into_iter().enumerate() {
            #[expect(clippy::cast_precision_loss, reason = "four handles")]
            let centre = near + along * edge * (1.0 + 1.5 * i as f32) + below * edge;
            let rect = egui::Rect::from_center_size(centre, egui::Vec2::splat(edge));
            let response = ui
                .interact(
                    rect,
                    ui.scope_id().with(("handle_3d", i)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_hint(handle.hint());
            let look = ui.style().interact(&response);
            let fill = if response.hovered() || response.dragged() {
                look.bg_fill
            } else {
                ui.visuals().extreme_bg_color
            };
            painter.circle(centre, edge / 2.0, fill, look.bg_stroke);
            if let Some(icon) = handle.icon() {
                self.icons.image(icon).paint_at(ui, rect);
            }
            self.handles_3d.push((handle, centre));
            if response.drag_started()
                && let Some(origin) = ui.input(|i| i.pointer.press_origin())
            {
                self.drag_3d = Some((handle, origin));
            }
            if response.clicked() && handle == Handle3d::Settings {
                let params = cmd_component_properties::Params::of(id, &self.doc.components[&id]);
                self.settings_3d = Some(Settings3d {
                    component: id,
                    params,
                    dirty: false,
                    dragged: None,
                    at: rect.left_bottom(),
                    radios: vec![],
                    anchors: None,
                });
            }
        }
    }

    /// Aspire's 3D Transform Handles over the selection's box, level with the highest
    /// surface inside it so they sit on what is selected rather than under it: the 2D
    /// view's handles at the same places (a cross in the middle moves, white corners and
    /// black edges scale) and a rotation arrow on a stem above the far edge. Dragged as the
    /// 2D view's are, with the same keys, and applied once the pointer is let go (one
    /// undo); while dragging, the box as it will be. Escape cancels a drag.
    // ponytail: the manual's edit box for an exact value, and moving the rotation centre,
    // are not offered; the Move, Scale and Rotate forms take exact values
    fn transform_handles_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        bounds: cut_craft_core::geometry::Rect,
    ) {
        let visuals = ui.visuals();
        let (stroke, dark) = (visuals.selection.stroke, visuals.text_color());
        let edge = ui.spacing().interact_size.y;
        let half = edge * view2d::NODE_HANDLE_SHARE / 2.0;
        // the selection's box itself, so the handles read as its corners and sides
        let corners = box_corners(bounds).map(|c| plane.screen(c)).to_vec();
        painter.add(egui::Shape::dashed_line(
            &[corners.as_slice(), &corners[..1]].concat(),
            stroke,
            6.0,
            4.0,
        ));
        // the arrow a button's height out from the far edge's middle, on a stem
        let arrow = Handle::Rotate { x: 0, y: 1 };
        let far = plane.screen(handles::position(bounds, arrow));
        let mut out = (far - plane.screen(bounds.center())).normalized();
        if !out.is_finite() || out == egui::Vec2::ZERO {
            out = -egui::Vec2::Y;
        }
        let tip = far + out * edge;
        painter.line_segment([far, tip], stroke);
        let all = handles::handles(false).into_iter().chain([arrow]);
        for (i, handle) in all.enumerate() {
            let at = if handle == arrow {
                tip
            } else {
                plane.screen(handles::position(bounds, handle))
            };
            let square = egui::Rect::from_center_size(at, egui::Vec2::splat(half * 2.0));
            match handle {
                Handle::Move => {
                    for d in [egui::vec2(half, 0.0), egui::vec2(0.0, half)] {
                        painter.line_segment([at - d, at + d], stroke);
                    }
                }
                Handle::Scale { x, y } if x != 0 && y != 0 => {
                    painter.rect_filled(square, 0.0, visuals.extreme_bg_color);
                    painter.rect_stroke(square, 0, stroke, egui::StrokeKind::Middle);
                }
                Handle::Scale { .. } => {
                    painter.rect_filled(square, 0.0, dark);
                }
                Handle::Rotate { .. } => {
                    // a ring with an arrow head on top
                    painter.circle_stroke(at, half, egui::Stroke::new(stroke.width, dark));
                    let top = at - egui::vec2(0.0, half);
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            top + egui::vec2(half, 0.0),
                            top + egui::vec2(0.0, half / 2.0),
                            top - egui::vec2(0.0, half / 2.0),
                        ],
                        dark,
                        egui::Stroke::NONE,
                    ));
                }
            }
            // picked within the 2D view's reach of it: half a button's height
            let response = ui.interact(
                egui::Rect::from_center_size(at, egui::Vec2::splat(edge)),
                ui.scope_id().with(("transform_3d", i)),
                egui::Sense::click_and_drag(),
            );
            // the status bar says what it does and the keys that change it
            if response.hovered() || response.dragged() {
                cut_craft_bug_report::hint::hint(ui.ctx(), handle.hint());
            }
            self.handles_3d.push((Handle3d::Transform(handle), at));
            if response.drag_started()
                && let Some(from) = ui.input(|i| i.pointer.press_origin())
            {
                self.drag_3d = Some((Handle3d::Transform(handle), from));
            }
        }
        self.transform_drag_3d(ui, painter, plane, bounds);
    }

    /// A transform handle's drag in the 3D view: the box as it will be while dragging, the
    /// transform applied (one undo) when the pointer is let go over the handles' plane.
    fn transform_drag_3d(
        &mut self,
        ui: &egui::Ui,
        painter: &egui::Painter,
        plane: &Plane3d,
        bounds: cut_craft_core::geometry::Rect,
    ) {
        let Some((Handle3d::Transform(handle), from)) = self.drag_3d else {
            return;
        };
        let stroke = ui.visuals().selection.stroke;
        let (now, released, keys) = ui.input(|i| {
            (
                i.pointer.latest_pos(),
                i.pointer.primary_released(),
                handles::Keys {
                    alt: i.modifiers.alt,
                    shift: i.modifiers.shift,
                    ctrl: i.modifiers.ctrl || i.modifiers.command,
                },
            )
        });
        let Some((start, end)) = plane.back(from).zip(now.and_then(|at| plane.back(at))) else {
            // off the handles' plane (over the horizon in perspective, or the plane seen
            // edge-on): nothing to show, and letting go there changes nothing
            if released {
                self.drag_3d = None;
                self.status
                    .set_message("Let go off the material: nothing was changed");
            } else {
                self.status.set_message(
                    "The pointer is off the material: bring it back, or let go to cancel",
                );
            }
            return;
        };
        let t = handles::transform(bounds, handle, start, end, keys);
        let moved = box_corners(bounds)
            .map(|c| plane.screen(t.transform_point2(c)))
            .to_vec();
        painter.add(egui::Shape::closed_line(moved, stroke));
        if released {
            self.drag_3d = None;
            let change = Change::transform_selected(&self.doc, |_| t);
            self.record(change, "Transformed");
        }
    }

    /// Escape during a drag on a 3D view handle: the drag ends with nothing changed (a
    /// height already set as it went is undone), Transform mode stays.
    pub(super) fn cancel_drag_3d(&mut self) {
        if self.drag_3d.take().is_none() {
            return;
        }
        if let Some((.., Some(_))) = self.drag_base.take() {
            self.history.undo(&mut self.doc);
        }
        self.status.set_message("Cancelled: nothing was changed");
    }

    /// A height handle's drag: the component follows the pointer, its total, shape or base
    /// height set through Component Properties each frame the pointer moves (the last
    /// setting undone first, so the whole drag is one undo), its value in the status bar.
    /// A drag up a quarter of the view doubles, as the wheel's zoom does; the base height
    /// moves the material's thickness over that quarter.
    fn handle_drag_3d(&mut self, ui: &egui::Ui, fit: egui::Rect, id: EntityId) {
        let Some((handle, origin)) = self.drag_3d else {
            return;
        };
        if !matches!(handle, Handle3d::Total | Handle3d::Height | Handle3d::Base) {
            return;
        }
        let (now, released) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.primary_released()));
        let Some(by) = now.map(|now| now - origin) else {
            return;
        };
        let doc = &self.doc;
        let (base, top, applied) = self.drag_base.get_or_insert_with(|| {
            let k = &doc.components[&id];
            let top = k.height_range().map_or(0.0, |(_, top)| top);
            (cmd_component_properties::Params::of(id, k), top, None)
        });
        let quarter = fit.size() / VIEW_ZOOM_QUARTERS;
        let factor = |px: f32, over: f32| f64::from(2f32.powf(px / over));
        let mut p = base.clone();
        let message = match handle {
            Handle3d::Total => {
                let f = factor(-by.y, quarter.y);
                p.shape_height = base.shape_height * f;
                p.base_height = base.base_height * f;
                format!("Total height {:.2} mm", *top * f)
            }
            Handle3d::Height => {
                p.shape_height = base.shape_height * factor(-by.y, quarter.y);
                format!("Shape height {:.0}%", p.shape_height)
            }
            _ => {
                let per_pixel = doc.job.thickness / f64::from(quarter.y);
                p.base_height = base.base_height - f64::from(by.y) * per_pixel;
                format!("Base height {:.2} mm", p.base_height)
            }
        };
        let params = serde_json::to_value(&p).expect("component properties serialize");
        let (same, undo) = (applied.as_ref() == Some(&params), applied.is_some());
        if !same {
            if undo {
                self.history.undo(&mut self.doc);
            }
            let done = match self.apply("component_properties", &params) {
                Ok(()) => Some(params),
                Err(e) => {
                    self.status.set_message(&e);
                    None
                }
            };
            if let Some((.., applied)) = &mut self.drag_base {
                *applied = done;
            }
        }
        if same || self.drag_base.as_ref().is_some_and(|d| d.2.is_some()) {
            self.status.set_message(&message);
        }
        if released {
            self.drag_3d = None;
            self.drag_base = None;
        }
    }

    /// The Settings handle's popup (Aspire 12's): the combine mode, Fade and Tilt with
    /// their anchors' Set, Appearance and its colour, and Close; each change applied
    /// through Component Properties as it is made, a dragged value as it goes (the drag
    /// one undo).
    pub(super) fn settings_3d_window(&mut self, ctx: &egui::Context) {
        use cmd_component_properties::Appearance;
        let Some(s) = &mut self.settings_3d else {
            return;
        };
        if !self.doc.components.contains_key(&s.component) {
            self.settings_3d = None;
            return;
        }
        let mut close = false;
        egui::Window::new("component_settings_3d")
            .title_bar(false)
            .resizable(false)
            .fixed_pos(s.at)
            .show(ctx, |ui| {
                ui.strong("Combine with other components ...");
                combine_radios_3d(ui, &self.icons, s);
                egui::Grid::new("component_settings_3d").show(ui, |ui| {
                    let p = &mut s.params;
                    s.dirty |= ui.checkbox(&mut p.fade.on, "Fade").changed();
                    // Set: then two clicks in the 3D view, as Aspire's help says
                    let setting = s.anchors.map(|(which, _)| which);
                    let set_fade = egui::Button::new("Set").selected(setting == Some(Anchor::Fade));
                    if ui
                        .add_enabled(p.fade.on, set_fade)
                        .on_hover_hint(Anchor::Fade.prompt(false))
                        .clicked()
                    {
                        s.anchors = Some((Anchor::Fade, None));
                    }
                    let percent = egui::DragValue::new(&mut p.fade.percent)
                        .range(0.0..=100.0)
                        .suffix(" %");
                    s.dirty |= ui.add_enabled(p.fade.on, percent).changed();
                    ui.end_row();
                    s.dirty |= ui.checkbox(&mut p.tilt.on, "Tilt").changed();
                    let set_tilt = egui::Button::new("Set").selected(setting == Some(Anchor::Tilt));
                    if ui
                        .add_enabled(p.tilt.on, set_tilt)
                        .on_hover_hint(Anchor::Tilt.prompt(false))
                        .clicked()
                    {
                        s.anchors = Some((Anchor::Tilt, None));
                    }
                    let angle = egui::DragValue::new(&mut p.tilt.angle)
                        .range(-90.0..=90.0)
                        .suffix(" °");
                    s.dirty |= ui.add_enabled(p.tilt.on, angle).changed();
                    ui.end_row();
                    ui.label("Appearance");
                    egui::ComboBox::from_id_salt("appearance_3d")
                        .selected_text(p.appearance.label())
                        .show_ui(ui, |ui| {
                            for a in Appearance::ALL {
                                s.dirty |= ui
                                    .selectable_value(&mut p.appearance, a, a.label())
                                    .changed();
                            }
                        });
                    ui.end_row();
                    ui.label("");
                    ui.horizontal(|ui| {
                        let solid = p.appearance == Appearance::SolidColor;
                        s.dirty |= ui
                            .add_enabled_ui(solid, |ui| ui.color_edit_button_srgb(&mut p.colour))
                            .inner
                            .changed();
                        ui.label("Color...");
                    });
                    ui.end_row();
                });
                close = ui.button("Close").clicked();
            });
        let pressed = ctx.input(|i| i.pointer.any_down());
        if !pressed {
            s.dragged = None;
        }
        let apply = s.dirty.then(|| s.params.clone());
        let dragged = apply.as_ref().and(s.dragged.take());
        if close {
            self.settings_3d = None;
        } else if apply.is_some() {
            s.dirty = false;
        }
        if let Some(p) = apply {
            // the last frame's step of this drag taken back first, so the drag is one undo
            if dragged == Some(self.history.undo_depth()) {
                self.history.undo(&mut self.doc);
            }
            let before = self.history.undo_depth();
            let params = serde_json::to_value(&p).expect("component properties serialize");
            if let Err(e) = self.apply("component_properties", &params) {
                self.status.set_message(&e);
            }
            let depth = self.history.undo_depth();
            if let (true, Some(s)) = (pressed && depth > before, self.settings_3d.as_mut()) {
                s.dragged = Some(depth);
            }
        }
    }

    /// Takes away the previewed material's piece under `at` in the 3D view's picture `fit`
    /// (Aspire's double-click on waste); false when the click is off the material or on
    /// material cut right through.
    /// The 3D view's drops, sculpting and saved image: clipart dropped on it goes where it
    /// was dropped on the material, while sculpting a left drag is a stroke (true then), and
    /// Save Shaded Image asks for the window's picture once the view is drawn.
    fn view_3d_drop_and_sculpt(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        area: egui::Rect,
    ) -> bool {
        self.ask_for_shaded_image(ui.ctx());
        if let Some(path) = response.dnd_release_payload::<std::path::PathBuf>() {
            let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
            let at = ui
                .ctx()
                .pointer_latest_pos()
                .and_then(|at| self.point_on_surface(fit, at));
            self.import_clipart(&path, at, None);
        }
        let sculpting = self.form.as_ref().is_some_and(|f| f.id == "sculpt");
        if sculpting {
            self.sculpt_3d(response, area.shrink(ui.spacing().item_spacing.x * 2.0));
        }
        sculpting
    }

    /// View ▸ Save Shaded Image to `path`: the 3D view is shown, its window's picture asked
    /// for once it has been drawn, and the 3D view's part saved when it comes.
    pub(super) fn save_shaded_image(&mut self, path: std::path::PathBuf) {
        self.view = View::ThreeD;
        self.shaded_image = Some(path);
    }

    /// Asks for the window's picture once the 3D view has been drawn for Save Shaded Image;
    /// the 3D view's part of it is saved when it comes.
    fn ask_for_shaded_image(&mut self, ctx: &egui::Context) {
        let (Some(path), Some(area)) = (self.shaded_image.take(), self.view_3d_area) else {
            return;
        };
        let (saved, ctx2) = (std::sync::Arc::clone(&self.shaded_saved), ctx.clone());
        let pixels = area * ctx.pixels_per_point();
        let callback = egui::ScreenshotCallback::new(move |picture| {
            let said = save_part(&picture, pixels, &path).map_or_else(
                |e| e,
                |()| format!("Saved the shaded image as {}", path.display()),
            );
            if let Ok(mut slot) = saved.lock() {
                *slot = Some(said);
            }
            ctx2.request_repaint();
        });
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(callback));
    }

    /// What saving the shaded image said, once it has.
    pub(super) fn shaded_image_arrived(&mut self) {
        let said = self
            .shaded_saved
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        if let Some(said) = said {
            self.status.set_message(&said);
        }
    }

    /// Sculpting in the 3D view: a left drag over the model is a stroke, its points where
    /// the pointer met the surface, sculpted as one change when the button comes up.
    fn sculpt_3d(&mut self, response: &egui::Response, fit: egui::Rect) {
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.stroke.clear();
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(at) = response
                .interact_pointer_pos()
                .and_then(|at| self.point_on_surface(fit, at))
            && self.stroke.last() != Some(&at)
        {
            self.stroke.push(at);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let (Some(&start), Some(&end)) = (self.stroke.first(), self.stroke.last())
        {
            let drag = cut_craft_core::Drag {
                start,
                end,
                from_center: false,
                constrain: false,
            };
            self.draw("sculpt", drag);
        }
    }

    /// The job point of the 3D view's surface under `at` on the picture `fit`, if any.
    fn point_on_surface(
        &mut self,
        fit: egui::Rect,
        at: egui::Pos2,
    ) -> Option<cut_craft_core::geometry::Point> {
        let job = self.shown_job();
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d().map(Cow::into_owned)?;
        let wrap = wrap_for(&job, on, true, &surface);
        let (column, row) = view3d::pick(&surface, orbit, framing, fit, wrap, at)?;
        Some(surface.centre(column, row))
    }

    pub(super) fn remove_preview_piece(&mut self, fit: egui::Rect, at: egui::Pos2) -> bool {
        let job = self.shown_job();
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return false;
        };
        let wrap = wrap_for(&job, on, true, &surface);
        let Some((column, row)) = view3d::pick(&surface, orbit, framing, fit, wrap, at) else {
            return false;
        };
        let p = surface.centre(column, row);
        let before = self.simulation.clone();
        let removed = self.simulation.as_mut().map_or(0, |m| m.remove_piece(p));
        if removed > 0 {
            self.before_preview = Some(super::BeforePreview(before));
            (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
            self.status.set_message("Removed the piece cut free");
        }
        removed > 0
    }

    /// The topmost shown component at the point of the 3D view's picture `fit` under `at`.
    pub(super) fn component_at_3d(&mut self, fit: egui::Rect, at: egui::Pos2) -> Option<EntityId> {
        self.components_at_3d(fit, at).first().copied()
    }

    /// Aspire's rotation handles while Orientate 3D Model shows the model in a flat job: red,
    /// green and blue rings round it turn it about X, Y and Z, and the grey sphere in its
    /// middle about all three (help: Import 3D Model, Initial Orientation).
    fn rings_3d(&mut self, ui: &egui::Ui, area: egui::Rect, fit: egui::Rect) {
        use cut_craft_core::glam::DVec3;
        self.rings_3d = (vec![], None);
        let model = self
            .form
            .as_ref()
            .filter(|f| f.id == "orientate_3d_model" && f.model_whole)
            .and_then(|f| f.model.clone());
        let Some(model) = model.filter(|_| self.doc.job.rotary.is_none()) else {
            return;
        };
        let corners = model.iter().flat_map(|(t, _)| t);
        let (Some(lo), Some(hi)) = (
            corners.clone().copied().reduce(DVec3::min),
            corners.copied().reduce(DVec3::max),
        ) else {
            return;
        };
        let Some(surface) = self.surface_3d().map(Cow::into_owned) else {
            return;
        };
        let camera = view3d::Camera::new(&surface, self.orbit, self.framing, fit, None);
        let origin = DVec3::new(surface.origin.x, surface.origin.y, 0.0);
        let screen = |p: DVec3| camera.screen(p - origin);
        let (middle, reach) = ((lo + hi) / 2.0, (hi - lo).max_element() * RING_SHARE);
        let painter = ui.painter_at(area);
        let width = 2.0 * ui.visuals().widgets.noninteractive.fg_stroke.width;
        // a ring's points every few degrees, in the plane square to its axis
        let steps = 72;
        for (axis, colour) in [
            ("x", egui::Color32::RED),
            ("y", egui::Color32::GREEN),
            ("z", egui::Color32::BLUE),
        ] {
            let points: Vec<egui::Pos2> = (0..=steps)
                .map(|i| {
                    let a = std::f64::consts::TAU * f64::from(i) / f64::from(steps);
                    let (c, s) = (a.cos() * reach, a.sin() * reach);
                    middle
                        + match axis {
                            "x" => DVec3::new(0.0, c, s),
                            "y" => DVec3::new(c, 0.0, s),
                            _ => DVec3::new(c, s, 0.0),
                        }
                })
                .map(screen)
                .collect();
            let hot = self.turn_ring == Some(axis);
            let stroke = egui::Stroke::new(if hot { 2.0 * width } else { width }, colour);
            painter.add(egui::Shape::line(points.clone(), stroke));
            self.rings_3d.0.push((axis, points));
        }
        let (centre, radius) = (screen(middle), ui.spacing().interact_size.y / 2.0);
        painter.circle(
            centre,
            radius,
            ui.visuals().widgets.inactive.bg_fill,
            ui.visuals().widgets.inactive.bg_stroke,
        );
        self.rings_3d.1 = Some((centre, radius));
    }

    /// A left drag starting on a rotation handle takes it for the drag; let go, none.
    fn take_ring(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.drag_started_by(egui::PointerButton::Primary) {
            self.turn_ring = ui
                .input(|i| i.pointer.press_origin())
                .and_then(|at| self.ring_at(at, ui.spacing().interact_size.y / 2.0));
        } else if !response.dragged() {
            self.turn_ring = None;
        }
    }

    /// The rotation handle within `reach` of `at` as last drawn: the sphere (`xyz`) first,
    /// else the nearest ring.
    pub(super) fn ring_at(&self, at: egui::Pos2, reach: f32) -> Option<&'static str> {
        if let Some((centre, radius)) = self.rings_3d.1
            && centre.distance(at) <= radius
        {
            return Some("xyz");
        }
        let off = |points: &[egui::Pos2]| {
            points
                .windows(2)
                .map(|w| {
                    let (a, b) = (w[0], w[1]);
                    let along = (at - a).dot(b - a) / (b - a).length_sq().max(f32::EPSILON);
                    at.distance(a + (b - a) * along.clamp(0.0, 1.0))
                })
                .fold(f32::INFINITY, f32::min)
        };
        self.rings_3d
            .0
            .iter()
            .map(|(axis, points)| (*axis, off(points)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(axis, _)| axis)
    }

    /// Set in the settings popup: two clicks on the model place the fade's or tilt's
    /// anchors, as Aspire's help says the 3D view does; the status bar says which point to
    /// click, and the first is marked while the second is awaited.
    fn anchors_3d(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        area: egui::Rect,
        fit: egui::Rect,
    ) {
        let Some((which, first)) = self.settings_3d.as_ref().and_then(|s| s.anchors) else {
            return;
        };
        self.status.set_message(which.prompt(first.is_some()));
        if response.clicked()
            && let Some(at) = response.interact_pointer_pos()
            && let Some(p) = self.point_at_3d(fit, at)
        {
            self.place_anchor(which, first, p);
        }
        if let Some(first) = first
            && let Some(surface) = self.surface_3d().map(Cow::into_owned)
        {
            let screen = view3d::screen_of(&surface, self.orbit, self.framing, fit, None);
            let z = surface.at(first).unwrap_or(0.0);
            let at = screen(cut_craft_core::glam::DVec3::new(
                first.x - surface.origin.x,
                first.y - surface.origin.y,
                z,
            ));
            let r = ui.spacing().interact_size.y * view2d::NODE_HANDLE_SHARE / 2.0;
            ui.painter_at(area)
                .circle_filled(at, r, ui.visuals().selection.stroke.color);
        }
    }

    /// The job point on the drawn surface at the point of the 3D view's picture `fit` under
    /// `at`; `None` off the surface.
    pub(super) fn point_at_3d(&mut self, fit: egui::Rect, at: egui::Pos2) -> Option<Point> {
        let (job, previewing) = (self.shown_job(), self.simulation.is_some());
        let (orbit, framing, on) = (self.orbit, self.framing, self.display.auto_wrapping);
        let surface = self.surface_3d()?.into_owned();
        let wrap = wrap_for(&job, on, previewing, &surface);
        let (column, row) = view3d::pick(&surface, orbit, framing, fit, wrap, at)?;
        Some(surface.centre(column, row))
    }

    /// A click placing a fade's or tilt's anchor: the first is kept until the second, which
    /// turns the fade or tilt on between them through the settings popup.
    fn place_anchor(&mut self, which: Anchor, first: Option<Point>, p: Point) {
        use cut_craft_core::params::Xy;
        let Some(s) = &mut self.settings_3d else {
            return;
        };
        let Some(first) = first else {
            s.anchors = Some((which, Some(p)));
            return;
        };
        let (from, to) = (Xy::new(first.x, first.y), Xy::new(p.x, p.y));
        match which {
            Anchor::Fade => {
                s.params.fade.on = true;
                s.params.fade.from = from;
                s.params.fade.to = to;
            }
            Anchor::Tilt => {
                s.params.tilt.on = true;
                s.params.tilt.pivot = from;
                s.params.tilt.toward = to;
            }
        }
        s.anchors = None;
        s.dirty = true;
    }

    /// Every shown component at the point of the 3D view's picture `fit` under `at`,
    /// topmost first.
    pub(super) fn components_at_3d(&mut self, fit: egui::Rect, at: egui::Pos2) -> Vec<EntityId> {
        let Some(p) = self.point_at_3d(fit, at) else {
            return vec![];
        };
        self.doc
            .components
            .iter()
            .rev()
            .filter(|(_, k)| k.visible && k.height_at(p).is_some())
            .map(|(&id, _)| id)
            .collect()
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
    #[expect(clippy::too_many_arguments, reason = "one frame's view, taken apart")]
    fn paint_turned(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        surface: &cut_craft_core::simulation::Material,
        wrap: view3d::Wrap,
        camera: &view3d::Camera,
        look: &view3d::Look,
        gpu: Option<&mut Option<view3d::gpu::Draw>>,
    ) {
        use cut_craft_core::glam::DVec3;
        let Some(model) = self.form.as_ref().and_then(|f| f.model.clone()) else {
            return;
        };
        let painter = ui.painter_at(area);
        let screen = |p| camera.screen(p);
        // from the job's millimetres (the axis at y = z = 0 for an X axis) to the wrapped
        // view's: along from the material's start; across and up as they are, the model at
        // its true size inside the stock
        let start = if wrap.along_x {
            DVec3::new(-surface.origin.x, 0.0, 0.0)
        } else {
            DVec3::new(0.0, -surface.origin.y, 0.0)
        };
        let mark = ui.visuals().error_fg_color;
        if let Some(draw) = gpu {
            let geometry = self.model_geometry(&model);
            *draw = Some(view3d::gpu::Draw::new(
                geometry, camera, area, start, look, mark,
            ));
        } else {
            let shown: Vec<cmd_orientate_3d_model::Seen> = model
                .iter()
                .map(|(t, hidden)| (t.map(|p| p + start), *hidden))
                .collect();
            painter.add(view3d::triangles(&shown, screen, self.orbit, look, mark));
        }
        if !self.display.material_block {
            return;
        }
        let job = self.shown_job().area().size();
        let length = if wrap.along_x { job.x } else { job.y };
        let stroke = egui::Stroke::new(ui.visuals().widgets.noninteractive.fg_stroke.width, mark);
        for [a, b] in view3d::cylinder_edges(wrap.along_x, wrap.radius, 0.0, length) {
            painter.line_segment([screen(a), screen(b)], stroke);
        }
    }

    /// The imported model's triangles drawn for this view, kept from frame to frame while
    /// the model, the job and the view stay the same.
    fn model_mesh(
        &mut self,
        model: &std::sync::Arc<Vec<cmd_orientate_3d_model::Seen>>,
        view: (view3d::Orbit, view3d::Framing, egui::Rect, view3d::Look),
        mark: egui::Color32,
        screen: impl Fn(cut_craft_core::glam::DVec3) -> egui::Pos2,
    ) -> std::sync::Arc<egui::Mesh> {
        let (orbit, framing, fit, look) = view;
        let key = super::ModelKey {
            model: std::sync::Arc::as_ptr(model) as usize,
            job: self.shown_job(),
            orbit,
            framing,
            fit,
            look,
            mark,
        };
        if let Some((kept, mesh)) = &self.model_3d
            && *kept == key
        {
            return std::sync::Arc::clone(mesh);
        }
        let mesh = std::sync::Arc::new(view3d::triangles(model, screen, orbit, &look, mark));
        self.model_3d = Some((key, std::sync::Arc::clone(&mesh)));
        mesh
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
                    && f.model_whole
                    && f.params["model_type"] == "full"
            });
        let wrap = wrap_for(&job, shown.auto_wrapping || turned, previewing, &surface);
        let look = view3d::Look { wrap, ..look };
        // the modeling plane a model being imported sits on, under the material so it shows
        // round it and through what the material leaves open
        let importing = self.form.as_ref().is_some_and(|f| f.model.is_some());
        if importing && wrap.is_none() {
            let screen = view3d::screen_of(&surface, orbit, framing, fit, None);
            let area = job.area();
            let at = |p: cut_craft_core::geometry::Point| {
                cut_craft_core::glam::DVec3::new(
                    p.x - surface.origin.x,
                    p.y - surface.origin.y,
                    0.0,
                )
            };
            painter.add(modeling_plane(ui, &screen, at(area.min), at(area.max)));
        }
        // on the GPU when the window has one (the headless tests draw egui meshes): its
        // callback takes this place in the painter's order once the model is known too
        let camera = view3d::Camera::new(&surface, orbit, framing, fit, wrap);
        let mark = ui.visuals().error_fg_color;
        let slot = self
            .gpu_3d
            .is_some()
            .then(|| painter.add(egui::Shape::Noop));
        let (mut surface_draw, mut model_draw) = (None, None);
        if slot.is_some() {
            let geometry = self.surface_geometry(look.wrap, look.sides);
            let at = cut_craft_core::glam::DVec3::ZERO;
            surface_draw = Some(view3d::gpu::Draw::new(
                geometry, &camera, area, at, &look, mark,
            ));
        } else {
            painter.add(self.surface_mesh(ui, &surface, (orbit, framing), look, fit));
        }
        let gpu = slot.is_some().then_some(&mut model_draw);
        if let Some(wrap) = wrap {
            // ponytail: the overlays are drawn flat, so they are left off the wrapped view
            if turned {
                self.paint_turned(ui, area, &surface, wrap, &camera, &look, gpu);
            }
        } else {
            self.paint_flat(ui, area, &surface, &camera, look, tool_at, gpu);
        }
        // the model's body, under its surface: also while a model is imported, whose own
        // faces (all of them, or past the face limit those out of reach) are drawn as well
        let body_draw = (slot.is_some() && self.simulation.is_none())
            .then(|| self.solid_gpu.clone())
            .flatten()
            .map(|geometry| {
                let at = cut_craft_core::glam::DVec3::ZERO;
                view3d::gpu::Draw::new(geometry, &camera, area, at, &look, mark)
            });
        if let (Some(slot), Some(gpu)) = (slot, &mut self.gpu_3d) {
            painter.set(slot, gpu.paint(area, surface_draw, model_draw, body_draw));
        }
    }

    /// Over the unwrapped surface: the model being imported in the material's outline, a
    /// playing preview's tool, and the overlays.
    #[expect(clippy::too_many_arguments, reason = "one frame's view, taken apart")]
    fn paint_flat(
        &mut self,
        ui: &egui::Ui,
        area: egui::Rect,
        surface: &Material,
        camera: &view3d::Camera,
        look: view3d::Look,
        tool_at: Option<(cut_craft_core::glam::DVec3, f64)>,
        gpu: Option<&mut Option<view3d::gpu::Draw>>,
    ) {
        let (job, painter) = (self.shown_job(), ui.painter_at(area));
        let (orbit, framing) = (self.orbit, self.framing);
        let fit = area.shrink(ui.spacing().item_spacing.x * 2.0);
        let screen = |p| camera.screen(p);
        let local = |p: cut_craft_core::geometry::Point, z: f64| {
            cut_craft_core::glam::DVec3::new(p.x - surface.origin.x, p.y - surface.origin.y, z)
        };
        // an imported model whole, what a cutter cannot reach marked, inside the material's
        // outline to judge its size by, over the modeling plane it is placed on
        if let Some(model) = self.form.as_ref().and_then(|f| f.model.clone()) {
            let origin = local(cut_craft_core::geometry::Point::new(0.0, 0.0), 0.0);
            let mark = ui.visuals().error_fg_color;
            if let Some(draw) = gpu {
                let geometry = self.model_geometry(&model);
                *draw = Some(view3d::gpu::Draw::new(
                    geometry, camera, area, origin, &look, mark,
                ));
            } else {
                let view = (orbit, framing, fit, look);
                painter.add(self.model_mesh(&model, view, mark, |p| screen(p + origin)));
            }
            // the material's outline, unless View ▸ Draw Material Block is off
            if self.display.material_block {
                let job_area = job.area();
                let (lo, hi) = (local(job_area.min, 0.0), local(job_area.max, job.thickness));
                let stroke = ui.visuals().widgets.noninteractive.fg_stroke;
                for [a, b] in view3d::box_edges(lo, hi) {
                    painter.line_segment([screen(a), screen(b)], stroke);
                }
            }
        }
        // a playing preview's tool, an upright cylinder at the tip
        // ponytail: drawn whole, over the surface it is cutting into
        if let Some((tip, diameter)) = tool_at {
            let at = local(tip.truncate(), tip.z);
            let outline =
                view3d::cylinder(screen, at, diameter / 2.0, diameter * TOOL_HEIGHT_DIAMETERS);
            let selection = ui.visuals().selection;
            painter.add(egui::Shape::convex_polygon(
                outline,
                selection.bg_fill,
                selection.stroke,
            ));
        }
        self.overlays_3d(ui, &painter, surface, (orbit, framing), fit);
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
        // the keep-out zones in red, closed, over whatever they cover, as Aspire shows them
        let danger = egui::Stroke::new(2.0 * stroke.width, ui.visuals().error_fg_color);
        for zone in self.doc.keep_out_zones() {
            let mut points = zone.flatten(cut_craft_core::polygon::TOLERANCE);
            points.extend(points.first().copied());
            let z = view3d::vector_plane(surface, &points);
            let line: Vec<_> = points.iter().map(|&p| local(p, z)).collect();
            for run in in_sight(&line) {
                painter.add(egui::Shape::line(run, danger));
            }
        }
        let size = view3d::plan_size(surface);
        // the plane's outline, and always while a model is imported onto it
        if shown.modeling_plane || self.form.as_ref().is_some_and(|f| f.model.is_some()) {
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
        // the toolpath list in its own bottom half, as Aspire's, so it never scrolls away;
        // the strip over it drags it taller or shorter
        let row = ui.spacing().interact_size.y;
        let room = ui.available_height();
        let strip = ui.spacing().item_spacing.y * 2.0;
        let list = self
            .list_height
            .unwrap_or(room / 2.0)
            .clamp(row * 3.0, (room - row * 3.0).max(row * 3.0));
        let width = ui.available_width();
        ui.allocate_ui(egui::vec2(width, room - list - strip), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    self.tools_or_favourites(ui, &groups, Page::Toolpaths);
                });
        });
        let (rect, divider) = ui.allocate_exact_size(egui::vec2(width, strip), egui::Sense::drag());
        if divider.dragged() {
            self.list_height = Some(list - divider.drag_delta().y);
        }
        if divider.hovered() || divider.dragged() {
            ui.set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        let line = if divider.hovered() || divider.dragged() {
            ui.visuals().widgets.active.bg_stroke
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().hline(rect.x_range(), rect.center().y, line);
        self.toolpath_list(ui);
    }

    /// The Toolpath List: its buttons, then each toolpath, the selected one highlighted and
    /// a right-click on one opening its menu.
    /// The up and down arrows over the toolpath list: the selected toolpath moves through the
    /// cut order and stays selected.
    fn toolpath_order_arrows(&mut self, ui: &mut egui::Ui) {
        let mut moved = None;
        let mut new_group = false;
        ui.horizontal(|ui| {
            use cmd_reorder_toolpath::{Direction, neighbour};
            for (direction, arrow, tip) in [
                (Direction::Up, "Move Up", "Move Toolpath Up (cut it sooner)"),
                (
                    Direction::Down,
                    "Move Down",
                    "Move Toolpath Down (cut it later)",
                ),
            ] {
                let can = self
                    .selected_toolpath
                    .is_some_and(|id| neighbour(&self.doc, id, direction).is_some());
                if ui
                    .add_enabled(can, egui::Button::new(arrow))
                    .on_hover_hint(tip)
                    .clicked()
                {
                    moved = Some(direction);
                }
            }
            new_group = ui
                .button("New Group")
                .on_hover_hint("Create Empty Group: toolpaths made while it is picked join it")
                .clicked();
        });
        if new_group {
            let params = serde_json::json!({
                "toolpaths": [],
                "group": self.doc.next_toolpath_group_name(),
            });
            if let Err(e) = self.apply("group_toolpaths", &params) {
                self.status.set_message(&e);
            }
        }
        if let (Some(direction), Some(id)) = (moved, self.selected_toolpath) {
            let to = cmd_reorder_toolpath::neighbour(&self.doc, id, direction);
            let params = serde_json::json!({ "toolpath": id, "direction": direction });
            match self.apply("reorder_toolpath", &params) {
                Ok(()) => self.selected_toolpath = to,
                Err(e) => self.status.set_message(&e),
            }
        }
    }

    /// The toolpath list's tick showing or hiding a toolpath, or its swatch colouring its
    /// preview, as a Toolpath Properties edit.
    fn list_properties(
        &mut self,
        shown: Option<(EntityId, bool)>,
        coloured: Option<(EntityId, [u8; 3])>,
    ) {
        // a tick shows or hides a toolpath, a swatch colours its preview
        let properties = shown
            .and_then(|(id, visible)| Some((id, self.doc.toolpaths.get(&id)?, visible, None)))
            .or_else(|| {
                let (id, rgb) = coloured?;
                let t = self.doc.toolpaths.get(&id)?;
                Some((id, t, t.visible, Some(serde_json::json!({ "colour": rgb }))))
            })
            .map(|(id, t, visible, preview)| {
                let mut params =
                    serde_json::json!({ "toolpath": id, "name": t.name, "visible": visible });
                if let Some(preview) = preview {
                    params["preview"] = preview;
                }
                params
            });
        if let Some(params) = properties
            && let Err(e) = self.apply("toolpath_properties", &params)
        {
            self.status.set_message(&e);
        }
    }

    /// The toolpath list's rows: each group, its toolpaths under it (named with it), then
    /// the toolpaths in no group; and each toolpath's place in the cut order.
    fn toolpath_rows(&self) -> (Vec<(Option<EntityId>, String)>, Places) {
        let mut rows = vec![];
        for group in self.doc.toolpath_groups_in_use() {
            rows.push((None, group.clone()));
            let members = self.doc.toolpaths_in_group(&group);
            rows.extend(members.into_iter().map(|id| (Some(id), group.clone())));
        }
        let ungrouped = self.doc.toolpaths.iter().filter(|(_, t)| t.group.is_none());
        rows.extend(ungrouped.map(|(&id, _)| (Some(id), String::new())));
        let places = self
            .doc
            .toolpaths
            .keys()
            .enumerate()
            .map(|(n, &id)| (id, n))
            .collect();
        (rows, places)
    }

    /// A group's row in the toolpath list: a click picks it (again: unpicks), a right-click
    /// picks it and opens its menu. Whether it was picked, and the menu action chosen.
    fn group_row(&self, ui: &mut egui::Ui, group: &str) -> (Option<bool>, Option<String>) {
        let picked = self.selected_group.as_deref() == Some(group);
        let name = egui::RichText::new(group).strong();
        let label = ui
            .selectable_label(picked, name)
            .on_hover_hint("A toolpath group: toolpaths made while it is picked join it");
        let pick = if label.clicked() {
            Some(!picked)
        } else if label.secondary_clicked() {
            Some(true)
        } else {
            None
        };
        (pick, self.popup(&label, || gui::toolpath_group_menu(group)))
    }

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
                "preview_visible_toolpaths",
                "undo_last_preview",
                "reset_preview",
            ],
        }];
        self.tool_groups(ui, &actions, Page::Toolpaths);
        if self.doc.toolpaths.is_empty() {
            ui.weak("No toolpaths yet");
        }
        self.toolpath_order_arrows(ui);
        let mut chosen = None;
        let mut shown = None;
        let mut edit = None;
        let mut coloured = None;
        let (rows, places) = self.toolpath_rows();
        let mut group_picked = None;
        // each toolpath's colour in the preview, when the machined areas take them
        let swatches = self.doc.job.machined_area == MachinedArea::ToolpathColor;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, group) in &rows {
                let Some(id) = *id else {
                    let (picked, action) = self.group_row(ui, group);
                    if let Some(on) = picked {
                        group_picked = Some((group.clone(), on));
                    }
                    if action.is_some() {
                        chosen = action;
                    }
                    continue;
                };
                let toolpath = &self.doc.toolpaths[&id];
                let selected = self.selected_toolpath == Some(id);
                let swatch = swatches.then_some((places[&id], self.doc.job.colour));
                let (label, ticked, rgb) =
                    toolpath_row(ui, toolpath, selected, swatch, !group.is_empty());
                if let Some(visible) = ticked {
                    shown = Some((id, visible));
                }
                if let Some(rgb) = rgb {
                    coloured = Some((id, rgb));
                }
                if label.clicked() {
                    self.selected_toolpath = if selected { None } else { Some(id) };
                    self.selected_group = None;
                }
                // a double-click opens the toolpath's form, as Aspire's list does
                if label.double_clicked() {
                    edit = Some(id);
                }
                // Aspire's toolpath menu acts on the row right-clicked, so it selects it
                if label.secondary_clicked() {
                    self.selected_toolpath = Some(id);
                    self.selected_group = None;
                }
                if let Some(action) =
                    self.popup(&label, || gui::toolpath_menu(&self.doc, id, toolpath))
                {
                    chosen = Some(action);
                }
            }
        });
        if let Some((group, on)) = group_picked {
            self.selected_group = on.then_some(group);
            self.selected_toolpath = None;
        }
        if let Some((id, visible)) = shown
            && let Some(t) = self.doc.toolpaths.get(&id)
        {
            let params = serde_json::json!({ "toolpath": id, "name": t.name, "visible": visible });
            if let Err(e) = self.apply("toolpath_properties", &params) {
                self.status.set_message(&e);
            }
        }
        self.list_properties(shown, coloured);
        if let Some(id) = edit {
            self.selected_toolpath = Some(id);
            self.invoke(ui.ctx(), "edit_toolpath");
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Status bar");
        let mut swap = false;
        ui.horizontal(|ui| {
            // what the pointer is over or the next step, else the last message
            let hint = cut_craft_bug_report::hint::current(ui.ctx());
            ui.label(hint.as_deref().unwrap_or(&self.status.message));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // the version, at the far right, as Help ▸ About gives it
                ui.weak(concat!("v", env!("CARGO_PKG_VERSION")))
                    .on_hover_hint("Cut Craft's version (Help ▸ About)");
                ui.separator();
                if self.screen == Screen::Job {
                    // the job's units, and the one-click way to swap them
                    let (unit, other) = match self.doc.job.units {
                        Units::Mm => ("mm", "inches"),
                        Units::Inch => ("in", "millimetres"),
                    };
                    swap = ui
                        .small_button(unit)
                        .on_hover_hint(format!("Switch to {other} (Edit ▸ Units)"))
                        .clicked();
                    if let Some((figures, _)) = self.coordinates() {
                        ui.monospace(figures);
                    }
                    ui.separator();
                }
                ui.label(format!("Tool: {}", self.status.current_tool));
            });
        });
        if swap {
            let to = match self.doc.job.units {
                Units::Mm => "units_inch",
                Units::Inch => "units_mm",
            };
            self.set_units(to);
        }
    }
}

/// Each toolpath's place in the cut order.
type Places = std::collections::BTreeMap<EntityId, usize>;

/// A toolpath's row in the list: its tick, its preview swatch (given its place and the
/// material's colour, when the preview takes toolpath colours) and its name, indented in a
/// group. The name's response, the tick's new state if changed, and the colour picked if any.
fn toolpath_row(
    ui: &mut egui::Ui,
    toolpath: &cut_craft_core::toolpath::Toolpath,
    selected: bool,
    swatch: Option<(usize, [u8; 3])>,
    indented: bool,
) -> (egui::Response, Option<bool>, Option<[u8; 3]>) {
    let (mut ticked, mut rgb) = (None, None);
    let label = ui
        .horizontal(|ui| {
            if indented {
                ui.add_space(ui.spacing().indent);
            }
            let mut visible = toolpath.visible;
            if ui
                .checkbox(&mut visible, "")
                .on_hover_hint("Show or hide this toolpath")
                .changed()
            {
                ticked = Some(visible);
            }
            if let Some((place, material)) = swatch {
                rgb = preview_swatch(ui, toolpath, place, material);
            }
            if toolpath.stale {
                let name = egui::RichText::new(format!("⚠ {}", toolpath.name))
                    .color(ui.visuals().warn_fg_color);
                ui.selectable_label(selected, name).on_hover_hint(format!(
                    "{}\nOut of date: what it was made from has changed since. Recalculate it.",
                    toolpath.tool.name
                ))
            } else {
                ui.selectable_label(selected, &toolpath.name)
                    .on_hover_hint(&toolpath.tool.name)
            }
        })
        .inner;
    (label, ticked, rgb)
}

/// The part of `picture` inside `area` (in its pixels) saved as a PNG at `path`.
fn save_part(
    picture: &egui::ColorImage,
    area: egui::Rect,
    path: &std::path::Path,
) -> Result<(), String> {
    let [width, height] = picture.size;
    let clamp = |v: f32, most: usize| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "pixels"
        )]
        let v = v.round().max(0.0) as usize;
        v.min(most)
    };
    let (x0, y0) = (clamp(area.min.x, width), clamp(area.min.y, height));
    let (x1, y1) = (clamp(area.max.x, width), clamp(area.max.y, height));
    if x1 <= x0 || y1 <= y0 {
        return Err("the 3D view was not on screen to save".into());
    }
    let mut out = image::RgbaImage::new(
        u32::try_from(x1 - x0).map_err(|e| e.to_string())?,
        u32::try_from(y1 - y0).map_err(|e| e.to_string())?,
    );
    for (y, row) in (y0..y1).enumerate() {
        for (x, column) in (x0..x1).enumerate() {
            let c = picture.pixels[row * width + column];
            let (x, y) = (u32::try_from(x).unwrap_or(0), u32::try_from(y).unwrap_or(0));
            out.put_pixel(x, y, image::Rgba(c.to_srgba_unmultiplied()));
        }
    }
    out.save(path)
        .map_err(|e| format!("could not save {}: {e}", path.display()))
}

/// Whether `path` is a mesh, imported through Orientate 3D Model.
/// Asks before Delete from Disk, naming what goes; whether to go ahead.
fn confirm_delete(doomed: &[std::path::PathBuf]) -> bool {
    const LISTED: usize = 8;
    let mut names: Vec<String> = (doomed.iter().take(LISTED))
        .map(|f| format!("  {}", file_name(f)))
        .collect();
    if doomed.len() > LISTED {
        names.push(format!("  and {} more", doomed.len() - LISTED));
    }
    let what = if doomed.len() == 1 {
        "this file".to_owned()
    } else {
        format!("these {} files", doomed.len())
    };
    let answer = super::message_dialog()
        .set_title("Delete from Disk")
        .set_description(format!(
            "Delete {what} from your disk?\n\n{}\n\nThis can't be undone: they don't go \
             to the bin.",
            names.join("\n")
        ))
        .set_level(rfd::MessageLevel::Warning)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            "Delete".into(),
            "Cancel".into(),
        ))
        .show();
    matches!(answer, rfd::MessageDialogResult::Custom(c) if c == "Delete")
}

fn is_model(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("stl") || e.eq_ignore_ascii_case("obj"))
}

/// `change` (an import) with what it adds moved to be centred on `at`.
fn centred_on(mut change: Change, at: cut_craft_core::geometry::Point) -> Change {
    use cut_craft_core::Edit;
    use cut_craft_core::geometry::Rect;
    let boxes = change.edits.iter().filter_map(|e| match e {
        Edit::Add { path, .. } => Some(path.bounds()),
        Edit::AddComponent { component, .. } => Some(component.extent()),
        Edit::AddBitmap { bitmap, .. } => Some(bitmap.rect),
        _ => None,
    });
    let Some(all) = boxes.reduce(Rect::union) else {
        return change;
    };
    let by = at - all.center();
    let t = cut_craft_core::glam::DAffine2::from_translation(by);
    for edit in &mut change.edits {
        match edit {
            Edit::Add { path, .. } => *path = path.transformed(&t),
            Edit::AddComponent { component, .. } => **component = component.transformed(&t),
            Edit::AddBitmap { bitmap, .. } => {
                bitmap.rect = Rect {
                    min: bitmap.rect.min + by,
                    max: bitmap.rect.max + by,
                };
            }
            _ => {}
        }
    }
    change
}

/// A toolpath's colour in a Toolpath Color preview, as a swatch beside it in the toolpath
/// list (the material's `colour` when it has no fill); the colour picked, if one was.
fn preview_swatch(
    ui: &mut egui::Ui,
    toolpath: &cut_craft_core::toolpath::Toolpath,
    place: usize,
    colour: [u8; 3],
) -> Option<[u8; 3]> {
    let given = toolpath.preview_colour(place);
    let mut rgb = given.unwrap_or(colour);
    let tip = if given.is_some() {
        "Its colour in the preview (right-click it for No Fill)"
    } else {
        "No Fill: previewed in the material's colour"
    };
    // ponytail: a drag in the picker is an edit a frame; coalesce them if the undo list
    // gets long
    ui.color_edit_button_srgb(&mut rgb)
        .on_hover_hint(tip)
        .changed()
        .then_some(rgb)
}

/// The modeling plane as a sheet of light blue (the zero plane a model is placed on), a
/// little past the material on every side so it shows under the model.
fn modeling_plane(
    ui: &egui::Ui,
    screen: &dyn Fn(cut_craft_core::glam::DVec3) -> egui::Pos2,
    lo: cut_craft_core::glam::DVec3,
    hi: cut_craft_core::glam::DVec3,
) -> egui::Mesh {
    use cut_craft_core::glam::DVec3;
    let margin = (hi - lo).max_element() * MODELING_PLANE_MARGIN;
    let (lo, hi) = (lo - DVec3::splat(margin), hi + DVec3::splat(margin));
    let fill = ui
        .visuals()
        .selection
        .bg_fill
        .gamma_multiply(MODELING_PLANE_ALPHA);
    let mut mesh = egui::Mesh::default();
    for corner in [
        DVec3::new(lo.x, lo.y, 0.0),
        DVec3::new(hi.x, lo.y, 0.0),
        DVec3::new(hi.x, hi.y, 0.0),
        DVec3::new(lo.x, hi.y, 0.0),
    ] {
        mesh.colored_vertex(screen(corner), fill);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    mesh
}

/// The combine modes as Aspire's small shape icons, the chosen one pressed and the names on
/// hover; where each is drawn goes to `s.radios`.
fn combine_radios_3d(ui: &mut egui::Ui, icons: &crate::icons::Icons, s: &mut Settings3d) {
    let edge = ui.spacing().interact_size.y;
    s.radios.clear();
    ui.horizontal(|ui| {
        for (name, mode, icon) in gui::COMBINE_MODES {
            let image = icons.image(icon).fit_to_exact_size(egui::Vec2::splat(edge));
            let radio = egui::Button::image(image)
                .selected(s.params.combine == mode)
                .min_size(egui::Vec2::splat(edge));
            let response = ui.add(radio).on_hover_hint(name);
            if response.clicked() {
                s.params.combine = mode;
                s.dirty = true;
            }
            s.radios.push((mode, response.rect.center()));
        }
    });
}

/// The level plane the 3D view's transform handles sit on, at height `z` in the material.
struct Plane3d<'a> {
    camera: &'a view3d::Camera,
    origin: Point,
    z: f64,
}

impl Plane3d<'_> {
    /// Where the job's point `p` on this plane shows.
    fn screen(&self, p: Point) -> egui::Pos2 {
        use cut_craft_core::glam::DVec3;
        self.camera
            .screen(DVec3::new(p.x - self.origin.x, p.y - self.origin.y, self.z))
    }

    /// The job's point on this plane under `at` on screen, if the plane is there.
    fn back(&self, at: egui::Pos2) -> Option<Point> {
        self.camera.on_plane(at, self.z).map(|p| p + self.origin)
    }
}

/// The highest surface over `bounds` (sampled on a 9 by 9 grid), or the material's top
/// when that is higher or the surface is not there.
fn top_over(surface: &Material, bounds: cut_craft_core::geometry::Rect) -> f64 {
    const SAMPLES: u32 = 9;
    let size = bounds.size();
    (0..SAMPLES)
        .flat_map(|i| (0..SAMPLES).map(move |j| (i, j)))
        .filter_map(|(i, j)| {
            let along = |k: u32| f64::from(k) / f64::from(SAMPLES - 1);
            surface.at(bounds.min + Point::new(size.x * along(i), size.y * along(j)))
        })
        .fold(0.0, f64::max)
}

/// The corners of `bounds`, round from its minimum.
fn box_corners(bounds: cut_craft_core::geometry::Rect) -> [Point; 4] {
    [
        bounds.min,
        Point::new(bounds.max.x, bounds.min.y),
        bounds.max,
        Point::new(bounds.min.x, bounds.max.y),
    ]
}
