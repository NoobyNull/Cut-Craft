//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

use crate::files::{self, Session};
use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::view2d::{Canvas, Gesture, Overlay, ViewTransform};
use crate::{commands, shortcut};
use aspire_core::document::{Document, EntityId};
use aspire_core::file::JOB_EXTENSION;
use aspire_core::geometry::{Path, Point, Rect};
use aspire_core::{Change, Drag, History, Registry};
use eframe::egui::{self, Align, Key, KeyboardShortcut, Layout, Modifiers};
use serde_json::Value;

const DESIGN_TABS: [&str; 5] = ["Drawing", "Modeling", "Sheets", "Layers", "Clipart"];
// ponytail: labels only until toolpath commands exist; then they come from the registry
const TOOLPATH_STRATEGIES: [&str; 12] = [
    "Profile",
    "Pocket",
    "Drilling",
    "V-Carve",
    "Quick Engrave",
    "Fluting",
    "Prism",
    "Inlay",
    "Texture",
    "Rough Machining",
    "Finish Machining",
    "Moulding",
];
/// Actions the app handles itself rather than through a command.
const BUILT_INS: [&str; 8] = [
    "undo",
    "redo",
    "exit",
    "new",
    "open",
    "save",
    "save_as",
    "import_vectors_bitmap",
];
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
            title: String::new(),
        }
    }

    fn available(&self, id: &str) -> bool {
        match id {
            "undo" => self.history.can_undo(),
            "redo" => self.history.can_redo(),
            _ => {
                BUILT_INS.contains(&id)
                    || self.registry.get(id).is_some()
                    || self.registry.exporter(id).is_some()
            }
        }
    }

    /// Menu items, tool buttons and shortcuts all end up here.
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
                None => self.invoke(ctx, "save_as"),
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
                    let result =
                        files::import(&self.registry, &self.doc, &path).and_then(|change| {
                            self.history
                                .apply(&mut self.doc, change)
                                .map_err(|e| e.to_string())
                        });
                    self.status.set_message(
                        &result.map_or_else(|e| e, |()| format!("Imported {}", path.display())),
                    );
                }
            }
            id if self.registry.exporter(id).is_some() => self.export(id),
            _ => {
                let Some(cmd) = self.registry.get(id) else {
                    self.status
                        .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
                    return;
                };
                let (meta, fields, params) = (
                    cmd.meta(),
                    form::fields(&cmd.schema()),
                    cmd.default_params(),
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
                self.tool_groups(ui);
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
    }

    fn tool_groups(&mut self, ui: &mut egui::Ui) {
        let edge = TOOL_ICON_ROWS * ui.spacing().interact_size.y;
        let mut clicked = None;
        for group in &self.toolbar.groups {
            egui::CollapsingHeader::new(group.name)
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for &id in group.tools {
                            let cmd = self.registry.get(id);
                            let label = cmd
                                .map_or_else(|| gui::title_case(id), |c| c.meta().label.to_owned());
                            let explicit = cmd.map_or("", |c| c.meta().icon);
                            let button = match self.icons.find(explicit, id) {
                                Some(path) => egui::Button::image(
                                    Icons::image(path).fit_to_exact_size(egui::vec2(edge, edge)),
                                ),
                                None => egui::Button::new(&label),
                            }
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

    fn toolpaths_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("Toolpath Operations");
        ui.horizontal_wrapped(|ui| {
            for strategy in TOOLPATH_STRATEGIES {
                if ui.button(strategy).clicked() {
                    self.status
                        .set_message(&format!("{strategy} toolpath: not implemented yet"));
                }
            }
        });
        ui.separator();
        ui.strong("Toolpath List");
        ui.weak("No toolpaths yet");
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.view, View::TwoD, "2D View (F2)");
                ui.selectable_value(&mut self.view, View::ThreeD, "3D View (F3)");
            });
            ui.separator();
            match self.view {
                View::TwoD => self.view_2d(ui),
                View::ThreeD => {
                    ui.centered_and_justified(|ui| ui.weak("3D view: not implemented yet"));
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
        canvas.paint(ui, &self.doc, &overlay);
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
            Box::new(AspireApp::new())
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
}
