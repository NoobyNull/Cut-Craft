//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

use crate::files::{self, Session};
use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::view2d::{Canvas, Gesture, Overlay, ViewTransform};
use crate::{commands, shortcut, view3d};
use aspire_core::document::{Document, EntityId, LayerId};
use aspire_core::file::JOB_EXTENSION;
use aspire_core::geometry::{Path, Point, Rect};
use aspire_core::simulation::Material;
use aspire_core::toolpath::{Tool, ToolDatabase};
use aspire_core::{Change, Drag, History, Registry};
use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers};
use serde_json::Value;
use std::f64::consts::{FRAC_PI_2, PI};

const DESIGN_TABS: [&str; 5] = ["Drawing", "Modeling", "Sheets", "Layers", "Clipart"];
/// Actions the app handles itself rather than through a command.
const BUILT_INS: [&str; 17] = [
    "undo",
    "redo",
    "exit",
    "new",
    "open",
    "save",
    "save_as",
    "import_vectors_bitmap",
    "recalculate_toolpath",
    "recalculate_all_toolpaths",
    "delete_toolpath",
    "save_toolpaths",
    "install_post_processor",
    "tool_database",
    "preview_toolpath",
    "preview_all_toolpaths",
    "reset_preview",
];
/// Simulation cells along the longer side of the job.
// ponytail: one quality until the Preview Simulation Quality menu is wired
const PREVIEW_CELLS: usize = 1000;
/// 3D view mesh cells along the longer side of the job.
// ponytail: fixed until the 3D view has a detail setting
const MESH_CELLS: usize = 250;
const APP_NAME: &str = "Aspire Clone";
/// Tool icon edge, in rows of the theme's interactive height.
// ponytail: fixed ratio until a preferences dialog exists
const TOOL_ICON_ROWS: f32 = 1.75;

#[derive(PartialEq)]
enum View {
    TwoD,
    ThreeD,
}

/// A command whose settings form is open.
struct OpenForm {
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    fields: Vec<Field>,
    params: Value,
    error: Option<String>,
}

struct AspireApp {
    doc: Document,
    history: History,
    registry: Registry,
    icons: Icons,
    menus: gui::MenuSystem,
    toolbar: gui::Toolbar,
    status: gui::StatusBar,
    design_tab: usize,
    view: View,
    cursor: Option<Point>,
    form: Option<OpenForm>,
    /// How the 2D view mapped the job last frame.
    view_transform: Option<ViewTransform>,
    session: Session,
    /// Toolpath picked in the Toolpaths panel.
    selected_toolpath: Option<EntityId>,
    /// Installed post-processor; without one, toolpaths save as generic G-code.
    post: Option<aspire_post::Post>,
    tools: ToolDatabase,
    /// Where `tools` is saved; `None` keeps it in memory (tests, or a file that failed to load).
    tools_path: Option<std::path::PathBuf>,
    tool_window: bool,
    /// The tool shown in the Tool Database window.
    tool_selected: usize,
    /// The material as cut by the previewed toolpaths, and its picture once drawn.
    simulation: Option<Material>,
    /// `simulation` at the 3D view's mesh resolution.
    simulation_coarse: Option<Material>,
    orbit: view3d::Orbit,
    /// Window title last sent to the viewport.
    title: String,
}

impl AspireApp {
    fn new() -> Self {
        Self {
            doc: Document::default(),
            history: History::default(),
            registry: commands::registry(),
            icons: Icons::load(&Icons::default_dir()),
            menus: gui::MenuSystem::new(),
            toolbar: gui::Toolbar::new(),
            status: gui::StatusBar::new(),
            design_tab: 0,
            view: View::TwoD,
            cursor: None,
            form: None,
            view_transform: None,
            session: Session::default(),
            selected_toolpath: None,
            post: None,
            tools: ToolDatabase::starter(),
            tools_path: None,
            tool_window: false,
            tool_selected: 0,
            simulation: None,
            simulation_coarse: None,
            orbit: view3d::Orbit::default(),
            title: String::new(),
        }
    }

    fn available(&self, id: &str) -> bool {
        match id {
            "undo" => self.history.can_undo(),
            "redo" => self.history.can_redo(),
            "recalculate_toolpath" | "delete_toolpath" | "preview_toolpath" => {
                self.selected_toolpath.is_some()
            }
            "recalculate_all_toolpaths" | "preview_all_toolpaths" => !self.doc.toolpaths.is_empty(),
            "reset_preview" => self.simulation.is_some(),
            "save_toolpaths" => {
                !self.doc.toolpaths.is_empty()
                    && (self.post.is_some() || self.registry.exporter("export_gcode").is_some())
            }
            _ => {
                BUILT_INS.contains(&id)
                    || self.registry.get(id).is_some()
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Menu items, tool buttons and shortcuts all end up here: built-in actions first, then
    /// the command registry.
    fn invoke(&mut self, ctx: &egui::Context, id: &str) {
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
            | "save"
            | "save_as"
            | "import_vectors_bitmap"
            | "install_post_processor" => self.file_action(id),
            "tool_database" => self.tool_window = true,
            "recalculate_toolpath"
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
            // the cascade: start again from uncut material, then preview each toolpath
            "preview_all_toolpaths" => {
                self.simulation = None;
                for id in self.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
                    self.preview_toolpath(id);
                }
            }
            "reset_preview" => (self.simulation, self.simulation_coarse) = (None, None),
            id if self.registry.exporter(id).is_some() => self.export(id),
            _ => self.start_command(id),
        }
    }

    fn file_action(&mut self, id: &str) {
        match id {
            "new" => {
                if self.confirm_discard() {
                    (self.doc, self.history, self.session, self.form) = Default::default();
                }
            }
            "open" => {
                if self.confirm_discard()
                    && let Some(path) = job_dialog().pick_file()
                {
                    match self.session.open(&path) {
                        Ok((doc, history)) => {
                            (self.doc, self.history, self.form) = (doc, history, None);
                        }
                        Err(e) => self.status.set_message(&e),
                    }
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
            "import_vectors_bitmap" => {
                let mut dialog = rfd::FileDialog::new();
                for importer in self.registry.importers() {
                    dialog = dialog.add_filter(importer.format.label, importer.format.extensions);
                }
                if let Some(path) = dialog.pick_file() {
                    let change = files::import(&self.registry, &self.doc, &path)
                        .map_err(aspire_core::CommandError::Failed);
                    self.record(change, &format!("Imported {}", path.display()));
                }
            }
            "install_post_processor" => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Post-processor", &["pp"])
                    .pick_file()
                {
                    match files::load_post(&path) {
                        Ok(post) => {
                            self.status
                                .set_message(&format!("Post-processor: {}", post.name));
                            self.post = Some(post);
                        }
                        Err(e) => self.status.set_message(&e),
                    }
                }
            }
            _ => unreachable!("not a file action: {id}"),
        }
    }

    fn toolpath_action(&mut self, id: &str) {
        match id {
            "recalculate_toolpath" => {
                if let Some(id) = self.selected_toolpath {
                    let result = self.registry.recalculate(&self.doc, id);
                    self.record(result, "Toolpath recalculated");
                }
            }
            "recalculate_all_toolpaths" => {
                let result = self.registry.recalculate_all(&self.doc);
                self.record(result, "All toolpaths recalculated");
            }
            "delete_toolpath" => {
                if let Some(id) = self.selected_toolpath.take() {
                    let mut c = Change::builder(&self.doc);
                    c.remove_toolpath(id);
                    let change = c.finish();
                    self.record(Ok(change), "Toolpath deleted");
                }
            }
            "save_toolpaths" => match &self.post {
                Some(post) => {
                    let extension = post.extension.clone();
                    let dialog =
                        rfd::FileDialog::new().add_filter(&post.name, &[extension.as_str()]);
                    if let Some(path) = dialog.save_file() {
                        let path = path.with_extension(&extension);
                        let result = files::save_through(post, &self.doc, &path);
                        self.status.set_message(&result.map_or_else(
                            |e| e,
                            |()| format!("Saved toolpaths to {}", path.display()),
                        ));
                    }
                }
                None => self.export("export_gcode"),
            },
            _ => unreachable!("not a toolpath action: {id}"),
        }
    }

    /// Runs a registered command: straight away if it has no settings, else opens its form.
    fn start_command(&mut self, id: &str) {
        let Some(cmd) = self.registry.get(id) else {
            self.status
                .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
            return;
        };
        let (meta, fields, params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        meta.label.clone_into(&mut self.status.current_tool);
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
            });
        }
    }

    fn save_to(&mut self, path: &std::path::Path) {
        match self.session.save(&self.doc, &self.history, path) {
            Ok(()) => self
                .status
                .set_message(&format!("Saved {}", path.display())),
            Err(e) => self.status.set_message(&e),
        }
    }

    fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = rfd::FileDialog::new().add_filter(format.label, format.extensions);
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
    fn confirm_discard(&mut self) -> bool {
        if !self.session.is_dirty(&self.history) {
            return true;
        }
        let answer = rfd::MessageDialog::new()
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
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    /// Applies a computed change for undo, reporting how it went in the status bar.
    fn record(&mut self, change: Result<Change, aspire_core::CommandError>, done: &str) {
        let result = change.map_err(|e| e.to_string()).and_then(|c| {
            self.history
                .apply(&mut self.doc, c)
                .map_err(|e| e.to_string())
        });
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Runs a command and records it for undo.
    fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = cmd.run_json(&self.doc, params).map_err(|e| e.to_string())?;
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status
            .set_message(&format!("{} applied", cmd.meta().label));
        Ok(())
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        // single-letter shortcuts must not fire while typing in a form field
        if !ctx.wants_keyboard_input() {
            let fired: Vec<String> = self
                .bindings()
                .into_iter()
                .filter(|(_, keys)| ctx.input_mut(|i| i.consume_shortcut(keys)))
                .map(|(id, _)| id)
                .collect();
            for id in fired {
                self.invoke(ctx, &id);
            }
        }
        if ctx
            .input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape)))
        {
            self.form = None;
        }
        ctx.input(|i| {
            if i.key_pressed(Key::F2) {
                self.view = View::TwoD;
            }
            if i.key_pressed(Key::F3) {
                self.view = View::ThreeD;
            }
        });
    }

    /// Every key binding: each command's own shortcut, plus the built-ins as the menus spell them.
    fn bindings(&self) -> Vec<(String, KeyboardShortcut)> {
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
        commands.chain(built_ins).collect()
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut clicked = None;
        egui::menu::bar(ui, |ui| {
            for menu in &self.menus.menus {
                ui.menu_button(&menu.name, |ui| {
                    menu_items(
                        ui,
                        &menu.items,
                        &|id| self.available(id),
                        &|id| self.registry.get(id).map(|c| c.meta().shortcut),
                        &mut clicked,
                    );
                });
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    fn design_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (i, tab) in DESIGN_TABS.iter().enumerate() {
                ui.selectable_value(&mut self.design_tab, i, *tab);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.command_form(ui);
            if DESIGN_TABS[self.design_tab] == "Drawing" {
                let groups = self.toolbar.groups.clone();
                self.tool_groups(ui, &groups);
            } else if DESIGN_TABS[self.design_tab] == "Layers" {
                self.layers_panel(ui);
            } else {
                ui.weak(format!(
                    "{} tools not implemented yet",
                    DESIGN_TABS[self.design_tab]
                ));
            }
        });
    }

    fn command_form(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &mut self.form else { return };
        let mut apply = false;
        let mut close = false;
        ui.horizontal(|ui| {
            if let Some(path) = self.icons.find(open.icon, open.id) {
                let edge = ui.spacing().interact_size.y;
                ui.add(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)));
            }
            ui.strong(open.label);
        });
        let (mut chosen, mut save_tool) = (None, false);
        if open.params.get("tool").is_some_and(Value::is_object) {
            let current = open.params["tool"]["name"].as_str().unwrap_or_default();
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Tool database")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for tool in &self.tools.tools {
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
        form::show(ui, &open.fields, &mut open.params);
        if let Some(error) = &open.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.horizontal(|ui| {
            apply = ui.button("Apply").clicked();
            close = ui.button("Close").clicked();
        });
        ui.separator();
        if apply {
            let (id, params) = (open.id, open.params.clone());
            let result = self.apply(id, &params);
            if let Some(open) = &mut self.form {
                open.error = result.err();
            }
        }
        if close {
            self.form = None;
        }
        if let Some(name) = chosen {
            self.choose_tool(&name);
        }
        if save_tool {
            self.save_form_tool();
        }
    }

    /// Reads the user's tool database; a broken one is reported and left untouched on disk.
    fn load_tools(&mut self) {
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

    /// Copies a database tool into the open form.
    fn choose_tool(&mut self, name: &str) {
        if let (Some(form), Some(tool)) = (&mut self.form, self.tools.get(name)) {
            form.params["tool"] = serde_json::to_value(tool).expect("tools serialize");
        }
    }

    /// Stores the open form's tool in the database, replacing one of the same name.
    fn save_form_tool(&mut self) {
        let tool = self
            .form
            .as_ref()
            .and_then(|f| serde_json::from_value::<Tool>(f.params["tool"].clone()).ok());
        match tool {
            Some(tool) => {
                let done = format!("Saved {} to the tool database", tool.name);
                self.tools.put(tool);
                self.tools_changed(&done);
            }
            None => self
                .status
                .set_message("the form has no valid tool to save"),
        }
    }

    fn delete_tool(&mut self, index: usize) {
        if index < self.tools.tools.len() {
            let tool = self.tools.tools.remove(index);
            self.tools_changed(&format!("Deleted {} from the tool database", tool.name));
        }
    }

    /// Writes the tool database, if it has a file, and reports `done` or the failure.
    fn tools_changed(&mut self, done: &str) {
        let result = self
            .tools_path
            .as_deref()
            .map_or(Ok(()), |path| files::save_tools(&self.tools, path));
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
    }

    /// Pick a tool to edit or delete; edits are saved as they are made.
    fn tool_database_window(&mut self, ctx: &egui::Context) {
        let (mut open, mut delete, mut edited) = (self.tool_window, None, false);
        egui::Window::new("Tool Database")
            .open(&mut open)
            .show(ctx, |ui| {
                let Some(last) = self.tools.tools.len().checked_sub(1) else {
                    ui.weak("No tools: save one from a toolpath form");
                    return;
                };
                self.tool_selected = self.tool_selected.min(last);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_source("tool_database_tools")
                        .selected_text(&self.tools.tools[self.tool_selected].name)
                        .show_ui(ui, |ui| {
                            for (i, tool) in self.tools.tools.iter().enumerate() {
                                ui.selectable_value(&mut self.tool_selected, i, &tool.name);
                            }
                        });
                    if ui.button("Delete").clicked() {
                        delete = Some(self.tool_selected);
                    }
                });
                let fields = form::fields(
                    &serde_json::to_value(schemars::schema_for!(Tool)).expect("schemas serialize"),
                );
                let tool = &mut self.tools.tools[self.tool_selected];
                let mut value = serde_json::to_value(&*tool).expect("tools serialize");
                if form::show(ui, &fields, &mut value)
                    && let Ok(changed) = serde_json::from_value(value)
                {
                    *tool = changed;
                    edited = true;
                }
            });
        self.tool_window = open;
        if let Some(index) = delete {
            self.delete_tool(index);
        } else if edited {
            self.tools_changed("Tool database saved");
        }
    }

    /// Icon buttons for groups of command ids: implemented ones run, the rest are greyed out.
    fn tool_groups(&mut self, ui: &mut egui::Ui, groups: &[gui::ToolGroup]) {
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        for group in groups {
            egui::CollapsingHeader::new(group.name)
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &id in group.tools {
                            let cmd = self.registry.get(id);
                            let label = cmd
                                .map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
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
                        }
                    });
                });
        }
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), id);
        }
    }

    /// An icon button, or a text one when the icon is missing.
    fn icon_button(&self, icon: &str, id: &str, label: &str, edge: f32) -> egui::Button<'static> {
        match self.icons.find(icon, id) {
            Some(path) => {
                egui::Button::image(Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)))
            }
            None => egui::Button::new(label.to_owned()),
        }
    }

    /// Layers, top first: visibility and lock toggles, the current layer highlighted, and a
    /// context menu of layer commands.
    fn layers_panel(&mut self, ui: &mut egui::Ui) {
        let edge = ui.spacing().interact_size.y;
        let label = |id: &str| self.registry.get(id).map_or("", |c| c.meta().label);
        let mut clicked: Option<(LayerId, &str)> = None;
        for layer in self.doc.layers.iter().rev() {
            ui.horizontal(|ui| {
                let (eye, eye_tip) = if layer.visible {
                    ("layers/visible.svg", "Hide")
                } else {
                    ("layers/hidden.svg", "Show")
                };
                let (lock, lock_tip) = if layer.locked {
                    ("layers/locked.svg", "Unlock")
                } else {
                    ("layers/unlocked.svg", "Lock")
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
                row.context_menu(|ui| {
                    for id in [
                        "layer_properties",
                        "select_layer_vectors",
                        "move_to_layer",
                        "delete_layer",
                    ] {
                        if ui.button(label(id)).clicked() {
                            clicked = Some((layer.id, id));
                            ui.close_menu();
                        }
                    }
                });
            });
        }
        let add = ui.button(label("add_layer")).clicked();
        if let Some((layer, action)) = clicked {
            self.layer_action(ui.ctx(), layer, action);
        }
        if add {
            self.invoke(ui.ctx(), "add_layer");
        }
    }

    /// A layer-panel action: the toggles and menu commands are undoable; choosing a layer is
    /// view state like the selection, so it is not.
    fn layer_action(&mut self, ctx: &egui::Context, layer: LayerId, action: &str) {
        let Some(l) = self.doc.layer(layer).cloned() else {
            return;
        };
        let properties = |visible: bool, locked: bool| serde_json::json!({ "layer": l.id, "name": l.name, "visible": visible, "locked": locked });
        let just_layer = serde_json::json!({ "layer": layer });
        let result = match action {
            "toggle_visible" => self.apply("layer_properties", &properties(!l.visible, l.locked)),
            "toggle_locked" => self.apply("layer_properties", &properties(l.visible, !l.locked)),
            "set_current_layer" => self
                .registry
                .get(action)
                .expect("set_current_layer is registered")
                .run_json(&self.doc, &just_layer)
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
            _ => self.apply(action, &just_layer),
        };
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Cuts the toolpath from the previewed material (uncut material if there is none yet).
    fn preview_toolpath(&mut self, id: EntityId) {
        let Some(toolpath) = self.doc.toolpaths.get(&id) else {
            return;
        };
        self.simulation
            .get_or_insert_with(|| Material::new(&self.doc.job, PREVIEW_CELLS))
            .cut(toolpath);
        self.simulation_coarse = None;
        self.view = View::ThreeD;
        self.status
            .set_message(&format!("Previewed {}", toolpath.name));
    }

    /// The previewed material, turned by dragging sideways and tilted by dragging up/down.
    fn view_3d(&mut self, ui: &mut egui::Ui) {
        let Some(material) = &self.simulation else {
            ui.centered_and_justified(|ui| ui.weak("No preview yet: preview the toolpaths"));
            return;
        };
        let (area, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
        // dragging across the whole view turns half a turn
        let delta = response.drag_delta();
        self.orbit.yaw += f64::from(delta.x / area.width().max(1.0)) * PI;
        self.orbit.pitch = (self.orbit.pitch + f64::from(delta.y / area.height().max(1.0)) * PI)
            .clamp(0.0, FRAC_PI_2);
        // ponytail: rebuilt every frame; cache it with the orbit if frames get slow
        let shown = self
            .simulation_coarse
            .get_or_insert_with(|| view3d::coarse(material, MESH_CELLS));
        let margin = ui.spacing().item_spacing.x * 2.0;
        let mesh = view3d::mesh(
            shown,
            self.orbit,
            ui.visuals().warn_fg_color,
            area.shrink(margin),
        );
        ui.painter_at(area).add(mesh);
    }

    fn toolpaths_panel(&mut self, ui: &mut egui::Ui) {
        let groups = self.toolbar.toolpath_groups.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.tool_groups(ui, &groups);
            ui.separator();
            ui.strong("Toolpath List");
            if self.doc.toolpaths.is_empty() {
                ui.weak("No toolpaths yet");
            }
            for (&id, toolpath) in &self.doc.toolpaths {
                let selected = self.selected_toolpath == Some(id);
                let label = ui
                    .selectable_label(selected, &toolpath.name)
                    .on_hover_text(&toolpath.tool.name);
                if label.clicked() {
                    self.selected_toolpath = if selected { None } else { Some(id) };
                }
            }
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
            self.tool_groups(ui, &actions);
        });
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
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

fn job_dialog() -> rfd::FileDialog {
    rfd::FileDialog::new().add_filter("Aspire Clone job", &[JOB_EXTENSION])
}

/// Menu entries, recursing into submenus. Sets `clicked` to the chosen item's action.
fn menu_items(
    ui: &mut egui::Ui,
    items: &[gui::MenuItem],
    available: &dyn Fn(&str) -> bool,
    shortcut_of: &dyn Fn(&str) -> Option<&'static str>,
    clicked: &mut Option<String>,
) {
    for item in items {
        if item.is_separator() {
            ui.separator();
        } else if !item.children.is_empty() {
            ui.menu_button(&item.name, |ui| {
                menu_items(ui, &item.children, available, shortcut_of, clicked);
            });
        } else {
            let button = egui::Button::new(&item.name)
                .shortcut_text(shortcut_of(&item.action).unwrap_or(&item.shortcut))
                .selected(item.is_checked);
            let response = ui
                .add_enabled(item.is_enabled && available(&item.action), button)
                .on_disabled_hover_text("Not implemented yet");
            if response.clicked() {
                *clicked = Some(item.action.clone());
                ui.close_menu();
            }
        }
    }
}

impl eframe::App for AspireApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl AspireApp {
    /// One frame of the whole window; separate from `update` so tests can drive it headlessly.
    fn ui(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        let title = self.session.title(APP_NAME, &self.history);
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        self.shortcuts(ctx);
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| self.menu_bar(ui));
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| self.status_bar(ui));
        egui::SidePanel::left("design_panel").show(ctx, |ui| self.design_panel(ui));
        egui::SidePanel::right("toolpaths_panel").show(ctx, |ui| self.toolpaths_panel(ui));
        self.tool_database_window(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view, View::TwoD, "2D View (F2)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D View (F3)");
            });
            ui.separator();
            match self.view {
                View::TwoD => self.view_2d(ui),
                View::ThreeD => {
                    self.view_3d(ui);
                    self.cursor = None;
                }
            }
        });
    }

    fn view_2d(&mut self, ui: &mut egui::Ui) {
        let canvas = Canvas::new(ui, &self.doc.job);
        let gesture = canvas.gesture(ui);
        let overlay = match gesture {
            Some(Gesture::Dragging(drag)) => match self.drawing_tool() {
                Some(id) => Overlay::Preview(self.preview(id, drag)),
                None => Overlay::Window {
                    start: drag.start,
                    end: drag.end,
                    crossing: drag.end.x < drag.start.x,
                },
            },
            _ => Overlay::None,
        };
        canvas.paint(ui, &self.doc, &overlay, self.selected_toolpath);
        self.view_transform = Some(canvas.transform);
        self.cursor = canvas.cursor();
        match (gesture, self.drawing_tool()) {
            (Some(Gesture::Dropped { drag, .. }), Some(id)) => self.draw(id, drag),
            (Some(Gesture::Dropped { drag, shift }), None) => {
                let ids = self
                    .doc
                    .window(Rect::new(drag.start, drag.end), drag.end.x < drag.start.x);
                self.select(ids, shift);
            }
            (Some(Gesture::Click { at, shift }), None) => {
                let hit = self.doc.pick(at, canvas.pick_tolerance(ui));
                if shift {
                    let mut ids = self.doc.selection.clone();
                    if let Some(id) = hit
                        && !ids.remove(&id)
                    {
                        ids.insert(id);
                    }
                    self.select(ids, false);
                } else {
                    self.select(hit, false);
                }
            }
            _ => {}
        }
    }

    /// The open form's command, if it can be drawn with the mouse.
    fn drawing_tool(&self) -> Option<&'static str> {
        let form = self.form.as_ref()?;
        self.registry
            .get(form.id)
            .filter(|c| c.interactive())
            .map(aspire_core::DynCommand::id)
    }

    /// Paths `id` would create for `drag`, by running the command itself.
    fn preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        let (Some(cmd), Some(form)) = (self.registry.get(id), &self.form) else {
            return vec![];
        };
        cmd.drag_params(drag, &form.params)
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map(|change| change.added().cloned().collect())
            .unwrap_or_default()
    }

    /// Finishes a drag with a drawing tool: the form shows the dragged values, then they apply.
    fn draw(&mut self, id: &'static str, drag: Drag) {
        let Some(cmd) = self.registry.get(id) else {
            return;
        };
        let Some(current) = self.form.as_ref().map(|f| f.params.clone()) else {
            return;
        };
        let result = cmd
            .drag_params(drag, &current)
            .map_err(|e| e.to_string())
            .and_then(|params| {
                if let Some(form) = &mut self.form {
                    form.params.clone_from(&params);
                }
                self.apply(id, &params)
            });
        if let Some(form) = &mut self.form {
            form.error = result.err();
        }
    }

    /// Selection is view state: applied directly, never recorded for undo.
    fn select(&mut self, ids: impl IntoIterator<Item = EntityId>, add: bool) {
        let mut c = Change::builder(&self.doc);
        if add {
            c.select(self.doc.selection.iter().copied().chain(ids));
        } else {
            c.select(ids);
        }
        self.doc
            .apply(&c.finish())
            .expect("picked ids come from the document");
    }
}

/// Opens the main window and runs until it is closed.
pub fn run(title: &str) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(title),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        title,
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                log::info!(
                    "GPU: {} ({:?}, driver: {})",
                    info.name,
                    info.backend,
                    info.driver
                );
            }
            let mut app = AspireApp::new();
            app.load_tools();
            Box::new(app)
        }),
    )
    .expect("event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Event, PointerButton, Pos2, RawInput, Rect, vec2};

    /// Drives the real UI headlessly, one frame per call, with synthetic pointer input.
    struct Harness {
        ctx: egui::Context,
        app: AspireApp,
        time: f64,
    }

    impl Harness {
        fn new() -> Self {
            let mut h = Self {
                ctx: egui::Context::default(),
                app: AspireApp::new(),
                time: 0.0,
            };
            h.frame(vec![]);
            h.frame(vec![]);
            h
        }

        fn frame(&mut self, events: Vec<Event>) {
            self.frame_holding(Modifiers::NONE, events);
        }

        /// One frame with `modifiers` held, as the windowing layer reports them every frame.
        fn frame_holding(&mut self, modifiers: Modifiers, events: Vec<Event>) {
            self.time += 0.1;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0))),
                time: Some(self.time),
                modifiers,
                events,
                ..Default::default()
            };
            let _ = self.ctx.run(input, |ctx| self.app.ui(ctx));
        }

        /// Screen position of a job coordinate in the 2D view.
        fn at(&self, x: f64, y: f64) -> Pos2 {
            self.app
                .view_transform
                .expect("the 2D view has been drawn")
                .to_screen(Point::new(x, y))
        }

        fn press(&mut self, pos: Pos2, pressed: bool, modifiers: Modifiers) {
            let button = Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers,
            };
            self.frame_holding(modifiers, vec![button]);
        }

        fn drag(&mut self, from: Pos2, to: Pos2) {
            self.frame(vec![Event::PointerMoved(from)]);
            self.press(from, true, Modifiers::NONE);
            self.frame(vec![Event::PointerMoved(from.lerp(to, 0.5))]);
            self.frame(vec![Event::PointerMoved(to)]);
            self.press(to, false, Modifiers::NONE);
            self.frame(vec![]);
        }

        /// A click with `modifiers` held for the whole gesture, as a held key would be.
        fn click(&mut self, at: Pos2, modifiers: Modifiers) {
            self.frame_holding(modifiers, vec![Event::PointerMoved(at)]);
            self.press(at, true, modifiers);
            self.press(at, false, modifiers);
            self.frame_holding(modifiers, vec![]);
        }

        /// Draws a rectangle through the command, then closes its form.
        fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) {
            let params = serde_json::json!({ "x": x, "y": y, "width": width, "height": height, "corners": "square", "radius": 0.0 });
            self.app.apply("draw_rectangle", &params).unwrap();
            self.app.form = None;
            self.frame(vec![]);
        }
    }

    fn near(a: Point, b: Point) -> bool {
        // one screen pixel is under a millimetre at this window size
        a.distance(b) < 2.0
    }

    #[test]
    fn dragging_with_the_rectangle_tool_draws_that_rectangle() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "draw_rectangle");
        h.frame(vec![]);
        let (from, to) = (h.at(100.0, 100.0), h.at(300.0, 200.0));
        h.drag(from, to);
        assert_eq!(h.app.doc.vectors.len(), 1);
        let b = h.app.doc.vectors.values().next().unwrap().bounds();
        assert!(
            near(b.min, Point::new(100.0, 100.0)) && near(b.max, Point::new(300.0, 200.0)),
            "{b:?}"
        );
        assert!(h.app.history.can_undo());
        let width = h.app.form.as_ref().unwrap().params["width"]
            .as_f64()
            .unwrap();
        assert!(
            (width - 200.0).abs() < 2.0,
            "the form shows the dragged size, got {width}"
        );
    }

    #[test]
    fn clicking_selects_without_adding_undo_steps() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.click(h.at(500.0, 300.0), Modifiers::NONE);
        assert!(
            h.app.doc.selection.is_empty(),
            "clicking empty space clears the selection"
        );
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "clicking the right edge selects it"
        );
        assert!(h.app.history.undo(&mut h.app.doc));
        assert!(
            h.app.doc.vectors.is_empty(),
            "the last undo step is the draw, not a click"
        );
    }

    #[test]
    fn shift_click_adds_and_removes() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.click(h.at(10.0, 35.0), Modifiers::NONE);
        h.click(h.at(150.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 2);
        h.click(h.at(10.0, 35.0), Modifiers::SHIFT);
        assert_eq!(h.app.doc.selection.len(), 1);
    }

    #[test]
    fn left_to_right_windows_and_right_to_left_crosses() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.rectangle(100.0, 10.0, 50.0, 50.0);
        h.drag(h.at(0.0, 0.0), h.at(120.0, 80.0));
        assert_eq!(
            h.app.doc.selection.len(),
            1,
            "only the first rectangle is fully inside"
        );
        h.drag(h.at(120.0, 80.0), h.at(0.0, 0.0));
        assert_eq!(
            h.app.doc.selection.len(),
            2,
            "crossing also takes the one it touches"
        );
    }

    #[test]
    fn forms_open_with_values_from_the_document() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 20.0);
        h.app.invoke(&h.ctx, "set_size");
        let params = &h.app.form.as_ref().unwrap().params;
        assert_eq!(
            (params["width"].as_f64(), params["height"].as_f64()),
            (Some(50.0), Some(20.0))
        );
    }

    fn cut_min_x(app: &AspireApp) -> f64 {
        aspire_core::testing::cut_bounds(app.doc.toolpaths.values().next().unwrap())
            .min
            .x
    }

    #[test]
    fn toolpaths_follow_their_vectors_when_recalculated() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "profile_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("profile_toolpath", &params).unwrap();
        let before = cut_min_x(&h.app);
        let shift =
            serde_json::json!({ "relative": true, "x": 100.0, "y": 0.0, "anchor": "bottom_left" });
        h.app.apply("move", &shift).unwrap();
        h.app.invoke(&h.ctx, "recalculate_all_toolpaths");
        assert!((cut_min_x(&h.app) - before - 100.0).abs() < 1e-6);
        h.app.invoke(&h.ctx, "undo");
        assert!(
            (cut_min_x(&h.app) - before).abs() < 1e-6,
            "recalculating is one undo step"
        );
    }

    #[test]
    fn hiding_a_layer_from_the_panel_is_undoable_and_stops_picking() {
        let mut h = Harness::new();
        h.app.design_tab = DESIGN_TABS.iter().position(|&t| t == "Layers").unwrap();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.layer_action(&h.ctx, LayerId(1), "toggle_visible");
        h.frame(vec![]);
        h.click(h.at(60.0, 35.0), Modifiers::NONE);
        assert!(h.app.doc.selection.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert!(h.app.doc.layers[0].visible);
    }

    #[test]
    fn choosing_a_layer_makes_it_current_without_an_undo_step() {
        let mut h = Harness::new();
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.layer_action(&h.ctx, LayerId(1), "set_current_layer");
        assert_eq!(h.app.doc.current_layer, LayerId(1));
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(
            h.app.doc.layers.len(),
            1,
            "the last undo step added the layer"
        );
    }

    #[test]
    fn layer_menu_actions_act_on_that_layer() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app
            .apply("add_layer", &serde_json::json!({ "name": "Top" }))
            .unwrap();
        h.app.layer_action(&h.ctx, LayerId(2), "move_to_layer");
        assert!(
            h.app
                .doc
                .vectors
                .keys()
                .all(|&id| h.app.doc.layer_of(id) == LayerId(2))
        );
        h.app.layer_action(&h.ctx, LayerId(1), "delete_layer");
        assert_eq!(h.app.doc.layers.len(), 1);
        assert_eq!(
            h.app.doc.vectors.len(),
            1,
            "the vector was not on the deleted layer"
        );
    }

    #[test]
    fn toolpath_forms_take_tools_from_the_database_and_save_them_back() {
        let mut h = Harness::new();
        let path =
            std::env::temp_dir().join(format!("aspire-tools-{}/tools.json", std::process::id()));
        h.app.tools_path = Some(path.clone());
        h.app.invoke(&h.ctx, "pocket_toolpath");
        h.app.choose_tool("3 mm End Mill");
        h.frame(vec![]);
        let params = &mut h.app.form.as_mut().unwrap().params;
        assert_eq!(params["tool"]["diameter"], 3.0);
        params["tool"]["name"] = "Mine".into();
        h.app.save_form_tool();
        assert_eq!(h.app.tools.get("Mine").map(|t| t.diameter), Some(3.0));
        assert_eq!(files::load_tools(&path).unwrap(), h.app.tools);
    }

    #[test]
    fn the_tool_database_window_lists_and_deletes_tools() {
        let mut h = Harness::new();
        h.app.invoke(&h.ctx, "tool_database");
        h.frame(vec![]);
        assert!(h.app.tool_window);
        let count = h.app.tools.tools.len();
        h.app.delete_tool(0);
        h.frame(vec![]);
        assert_eq!(h.app.tools.tools.len(), count - 1);
    }

    #[test]
    fn previewing_all_toolpaths_is_previewing_each_in_turn() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        for id in ["drilling_toolpath", "profile_toolpath"] {
            h.app.invoke(&h.ctx, id);
            let params = h.app.form.as_ref().unwrap().params.clone();
            h.app.apply(id, &params).unwrap();
        }
        h.app.invoke(&h.ctx, "preview_all_toolpaths");
        assert!(h.app.view == View::ThreeD, "previewing shows the 3D view");
        h.frame(vec![]);
        let all = h.app.simulation.clone().expect("a preview");
        h.app.invoke(&h.ctx, "reset_preview");
        assert!(h.app.simulation.is_none());
        for id in h.app.doc.toolpaths.keys().copied().collect::<Vec<_>>() {
            h.app.selected_toolpath = Some(id);
            h.app.invoke(&h.ctx, "preview_toolpath");
        }
        assert_eq!(h.app.simulation, Some(all));
    }

    #[test]
    fn deleting_the_selected_toolpath_is_undoable() {
        let mut h = Harness::new();
        h.rectangle(10.0, 10.0, 50.0, 50.0);
        h.app.invoke(&h.ctx, "drilling_toolpath");
        let params = h.app.form.as_ref().unwrap().params.clone();
        h.app.apply("drilling_toolpath", &params).unwrap();
        h.app.selected_toolpath = h.app.doc.toolpaths.keys().next().copied();
        h.app.invoke(&h.ctx, "delete_toolpath");
        assert!(h.app.doc.toolpaths.is_empty());
        h.app.invoke(&h.ctx, "undo");
        assert_eq!(h.app.doc.toolpaths.len(), 1);
    }
}
