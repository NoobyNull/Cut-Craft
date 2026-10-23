//! Main window: Aspire-style panel layout drawn with egui on wgpu.
//! Layout follows re/SPEC.md (P2): menu bar, Design panel (left), Toolpaths panel (right),
//! 2D/3D view tabs (centre), status bar.

use crate::gui;
use eframe::egui::{self, Align, Layout, Rect, Sense, Shape, pos2, vec2};

/// Material block size (width X, height Y) in mm for a new job.
// ponytail: fixed until the Job Setup form exists, then it comes from the job file
const DEFAULT_JOB_SIZE_MM: [f32; 2] = [600.0, 400.0];

const DESIGN_TABS: [&str; 5] = ["Drawing", "Modeling", "Sheets", "Layers", "Clipart"];
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

#[derive(PartialEq)]
enum View {
    TwoD,
    ThreeD,
}

struct AspireApp {
    menus: gui::MenuSystem,
    toolbar: gui::Toolbar,
    properties: gui::PropertyPanel,
    status: gui::StatusBar,
    design_tab: usize,
    view: View,
    job_size: egui::Vec2,
    /// Cursor position in job coordinates (mm), when over the 2D view.
    cursor_mm: Option<egui::Vec2>,
}

impl AspireApp {
    fn new() -> Self {
        Self {
            menus: gui::MenuSystem::new(),
            toolbar: gui::Toolbar::new(),
            properties: gui::PropertyPanel::new(),
            status: gui::StatusBar::new(),
            design_tab: 0,
            view: View::TwoD,
            job_size: DEFAULT_JOB_SIZE_MM.into(),
            cursor_mm: None,
        }
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut clicked = None;
        egui::menu::bar(ui, |ui| {
            for menu in &self.menus.menus {
                ui.menu_button(&menu.name, |ui| menu_items(ui, &menu.items, &mut clicked));
            }
        });
        match clicked.as_deref() {
            Some("exit") => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            Some(action) => self
                .status
                .set_message(&format!("{action}: not implemented yet")),
            None => {}
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
            if DESIGN_TABS[self.design_tab] == "Drawing" {
                for group in &self.toolbar.groups {
                    egui::CollapsingHeader::new(&group.name)
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for tool in &group.tools {
                                    let active = self.status.current_tool == *tool;
                                    if ui.selectable_label(active, tool).clicked() {
                                        self.status.current_tool.clone_from(tool);
                                    }
                                }
                            });
                        });
                }
            } else {
                ui.weak(format!(
                    "{} tools not implemented yet",
                    DESIGN_TABS[self.design_tab]
                ));
            }
            ui.separator();
            ui.strong("Properties");
            egui::Grid::new("properties")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    for p in &mut self.properties.properties {
                        ui.label(&p.name);
                        ui.text_edit_singleline(&mut p.value);
                        ui.end_row();
                    }
                });
        });
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
                if let Some(p) = self.cursor_mm {
                    ui.monospace(format!("X {:8.2}  Y {:8.2} mm", p.x, p.y));
                    ui.separator();
                }
                ui.label(format!("Tool: {}", self.status.current_tool));
            });
        });
    }

    /// Material block fitted to the view, with dashed X0/Y0 datum lines (lower-left datum).
    fn view_2d(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let painter = ui.painter_at(rect);
        let visuals = ui.visuals();
        let spacing = ui.spacing();

        let fit = rect.shrink(spacing.interact_size.y);
        let scale = (fit.width() / self.job_size.x).min(fit.height() / self.job_size.y);
        let job = Rect::from_center_size(fit.center(), self.job_size * scale);
        painter.rect_filled(job, 0.0, visuals.widgets.inactive.bg_fill);
        painter.rect_stroke(job, 0.0, visuals.widgets.noninteractive.bg_stroke);

        let origin = job.left_bottom();
        let stroke = visuals.widgets.noninteractive.fg_stroke;
        let (dash, gap) = (spacing.item_spacing.x, spacing.item_spacing.x / 2.0);
        for line in [
            [pos2(rect.left(), origin.y), pos2(rect.right(), origin.y)],
            [pos2(origin.x, rect.top()), pos2(origin.x, rect.bottom())],
        ] {
            painter.extend(Shape::dashed_line(&line, stroke, dash, gap));
        }

        self.cursor_mm = response
            .hover_pos()
            .map(|p| vec2(p.x - origin.x, origin.y - p.y) / scale);
    }
}

/// Menu entries, recursing into submenus. Sets `clicked` to the chosen item's action.
fn menu_items(ui: &mut egui::Ui, items: &[gui::MenuItem], clicked: &mut Option<String>) {
    for item in items {
        if item.is_separator() {
            ui.separator();
        } else if !item.children.is_empty() {
            ui.menu_button(&item.name, |ui| menu_items(ui, &item.children, clicked));
        } else {
            let button = egui::Button::new(&item.name)
                .shortcut_text(&item.shortcut)
                .selected(item.is_checked);
            if ui.add_enabled(item.is_enabled, button).clicked() {
                *clicked = Some(item.action.clone());
                ui.close_menu();
            }
        }
    }
}

impl eframe::App for AspireApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::F2) {
                self.view = View::TwoD;
            }
            if i.key_pressed(egui::Key::F3) {
                self.view = View::ThreeD;
            }
        });

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
                    self.cursor_mm = None;
                    ui.centered_and_justified(|ui| {
                        ui.weak("3D view: wgpu viewport not implemented yet")
                    });
                }
            }
        });
    }
}

/// Open the main window and run until it is closed.
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
            if let Some(rs) = &cc.wgpu_render_state {
                let info = rs.adapter.get_info();
                println!(
                    "GPU: {} ({:?}, driver: {})",
                    info.name, info.backend, info.driver
                );
            }
            Box::new(AspireApp::new())
        }),
    )
    .expect("event loop error");
}
