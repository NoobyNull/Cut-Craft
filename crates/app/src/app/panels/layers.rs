//! The Layers, Library and Sheets panels, with importing into the job.

use super::{
    Change, CutCraftApp, EXPORT_LEVEL, HoverHint, LayerId, Menus, Value, View, centred_on,
    confirm_delete, egui, files, gui, importing, is_model, menu_items, sheet_length, tr,
};

impl CutCraftApp {
    /// Layers, top first: visibility and lock toggles, the current layer highlighted, and
    /// The layer menu on a right-click.
    /// The arrows over the Layers List: the active layer one place up or down the list.
    pub(super) fn layer_order_arrows(&mut self, ui: &mut egui::Ui) {
        use cmd_reorder_layer::{Direction, moved_to};
        let layer = self.doc.current_layer;
        let mut moved = None;
        let arrows = ui.horizontal(|ui| {
            for (direction, arrow, tip) in [
                (
                    Direction::Up,
                    tr!("panels-move-up"),
                    "Move the active layer up the list",
                ),
                (
                    Direction::Down,
                    tr!("panels-move-down"),
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
        crate::marks::note(ui, "tool:reorder_layer", arrows.response.rect);
        ui.checkbox(
            &mut self.layers_on_sheet,
            tr!("panels-show-only-layers-used-on"),
        )
        .on_hover_hint(tr!("panels-leave-out-the-layers-with"));
        if let Some(direction) = moved {
            let params = serde_json::json!({ "layer": layer, "direction": direction });
            if let Err(e) = self.apply("reorder_layer", &params) {
                self.status.set_message(&e);
            }
        }
    }

    pub(in super::super) fn layers_panel(&mut self, ui: &mut egui::Ui) {
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
                // the colour its vectors are drawn in; a click opens Layer Properties to change it
                let usual = self.options.appearance.lines(ui.visuals().dark_mode).vector;
                let (swatch, chip) =
                    ui.allocate_exact_size(egui::vec2(edge, edge), egui::Sense::click());
                let inner = swatch.shrink(edge * 0.2);
                ui.painter().rect(
                    inner,
                    2.0,
                    crate::view2d::colour32(layer.colour.unwrap_or(usual)),
                    ui.visuals().widgets.noninteractive.fg_stroke,
                    egui::StrokeKind::Inside,
                );
                let tip = if layer.colour.is_some() {
                    "The layer's own colour: click to change it"
                } else {
                    "The usual vector colour: click to give the layer its own"
                };
                if chip.on_hover_hint(tip).clicked() {
                    clicked = Some((layer.id, "layer_properties"));
                }
                let count = self.doc.vectors_on(layer.id).count();
                let row = ui
                    .selectable_label(self.doc.current_layer == layer.id, &layer.name)
                    .on_hover_hint(tr!("panels-layer-vectors", count = count));
                crate::marks::note(ui, &format!("layer:{}", layer.name), row.rect);
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
        let add = ui.button(label("add_layer"));
        crate::marks::note(ui, "tool:add_layer", add.rect);
        let add = add.clicked();
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
    pub(in super::super) fn level_action(&mut self, id: &str) {
        let (how, level) = id.split_once(':').unwrap_or_default();
        if let Ok(level) = level.parse() {
            self.doc.selected_components = self.doc.level_components(level).into_iter().collect();
            if format!("{how}:") == EXPORT_LEVEL {
                self.export("export_as_3d_clipart");
            }
        }
    }

    /// The Library tab: its folders, and the chosen folder's clipart to add to the project.
    pub(in super::super) fn clipart_panel(&mut self, ui: &mut egui::Ui) {
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
                    self.status.set_message(&cut_craft_i18n::tr!(
                        "status-could-not-show",
                        path = folder.display().to_string(),
                        error = e.to_string()
                    ));
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
    pub(in super::super) fn add_from_library(
        &mut self,
        files: Vec<std::path::PathBuf>,
        level: Option<crate::clipart::ToLevel>,
    ) {
        self.import_queue
            .extend(files.into_iter().map(|f| (f, level)));
        self.next_import();
    }

    /// The next file waiting to be added from the Library, when no form is open.
    pub(in super::super) fn next_import(&mut self) {
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
                self.status.set_message(&cut_craft_i18n::tr!(
                    "status-file-gone",
                    path = path.display().to_string()
                ));
            }
        }
    }

    /// Brings clipart `path` into the job as a double-click does, then (dropped on a view)
    /// centred on `at`, and (Import to Level) on `level`: one of the job's, or a new level
    /// named after the file. A model goes through Orientate 3D Model, placed and
    /// levelled as it is made.
    pub(in super::super) fn import_clipart(
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
        // dropped at a point, other units are scaled about it
        let pivot = at.unwrap_or(cut_craft_core::geometry::Point::ZERO);
        self.record_import(change, &format!("Imported {}", path.display()), pivot);
    }

    /// Brings `path` into the job: meshes through Orientate 3D Model
    /// does; anything else through its importer.
    pub(in super::super) fn import_file(&mut self, path: &std::path::Path) {
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
        // a big drawing or picture is read off the window's thread, its progress shown
        let (registry, doc, reading) = (
            std::sync::Arc::clone(&self.registry),
            self.doc.clone(),
            path.to_owned(),
        );
        let done = format!("Imported {}", path.display());
        self.in_background(
            &format!("Importing {}", super::super::file_name(path)),
            &importing(path),
            move || files::import(&registry, &doc, &reading),
            move |app, change| {
                let change = change.map_err(cut_craft_core::CommandError::Failed);
                app.record_import(change, &done, cut_craft_core::geometry::Point::ZERO);
            },
        );
    }

    /// The Sheets tab: the Sheets List (each sheet's name and size, the active one
    /// marked) and its Add New Sheet button. A click activates a sheet; a right-click has its
    /// menu.
    pub(in super::super) fn sheets_panel(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Sheets tab");
        ui.strong(tr!("panels-sheets-list"));
        let units = self.doc.job.units;
        let (mut activate, mut chosen) = (None, None);
        egui::Grid::new("sheets_list").striped(true).show(ui, |ui| {
            for heading in ["Name", "X", "Y", "Z"] {
                // each axis's size in its colour, as everywhere
                match crate::axes::Axis::named(heading) {
                    Some(axis) => ui.label(crate::axes::label(heading, axis)),
                    None => ui.strong(heading),
                };
            }
            ui.end_row();
            for (i, sheet) in self.doc.sheets.iter().enumerate() {
                let job = self.doc.sheet_job(i).unwrap_or(sheet.job);
                let active = i == self.doc.active_sheet;
                let row = ui.radio(active, &sheet.name);
                crate::marks::note(ui, &format!("sheet:{}", sheet.name), row.rect);
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
        let add = ui.button(tr!("panels-add-new-sheet"));
        crate::marks::note(ui, "tool:add_sheet", add.rect);
        let add = add.clicked();
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
    pub(in super::super) fn popup(
        &self,
        response: &egui::Response,
        menu: impl FnOnce() -> Vec<gui::MenuItem>,
    ) -> Option<String> {
        let mut clicked = None;
        cut_craft_bug_report::context_menu(response, |ui| {
            let menus = Menus {
                available: &|id| self.available(id),
                why: &|id| self.unavailable_why(id),
                checked: &|_| None,
                shortcut_of: &|id| self.shortcut_text(id),
                recent: &[],
                image: &|path| self.icons.find(path, "").map(|p| self.icons.image(p)),
            };
            menu_items(ui, &menu(), &menus, &mut clicked);
        });
        clicked
    }

    /// A layer-panel action: the toggles are undoable; choosing a layer is view state like the
    /// selection, so it is not.
    pub(in super::super) fn layer_action(
        &mut self,
        ctx: &egui::Context,
        layer: LayerId,
        action: &str,
    ) {
        let Some(l) = self.doc.layer(layer).cloned() else {
            return;
        };
        // start from the whole layer, so the eye and lock change only what they show
        let properties = |visible: bool, locked: bool| {
            serde_json::to_value(cmd_layer_properties::Params {
                visible,
                locked,
                ..l.clone().into()
            })
            .expect("layer properties serialize")
        };
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
}
