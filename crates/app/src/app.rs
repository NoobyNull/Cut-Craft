//! The main window: menus, Design and Toolpaths panels, and the 2D/3D views over the
//! document, its undo history and the command registry.

use crate::form::{self, Field};
use crate::gui;
use crate::icons::Icons;
use crate::{commands, shortcut, view2d};
use aspire_core::document::Document;
use aspire_core::geometry::Point;
use aspire_core::{History, Registry};
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
const BUILT_INS: [&str; 3] = ["undo", "redo", "exit"];
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
        }
    }

    fn available(&self, id: &str) -> bool {
        match id {
            "undo" => self.history.can_undo(),
            "redo" => self.history.can_redo(),
            _ => BUILT_INS.contains(&id) || self.registry.get(id).is_some(),
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
            "exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
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
                                .add_enabled(cmd.is_some(), button)
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
            self.cursor = match self.view {
                View::TwoD => view2d::paint(ui, &self.doc),
                View::ThreeD => {
                    ui.centered_and_justified(|ui| ui.weak("3D view: not implemented yet"));
                    None
                }
            };
        });
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
