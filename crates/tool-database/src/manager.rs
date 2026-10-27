//! The manager window, laid out as Aspire's Tool Database: the tool tree with its buttons
//! on the left, the picked group's or tool's settings on the right, OK / Cancel under both.
//! Edits are made to a copy; only Apply and OK hand it back.

use crate::tree;
use cut_craft_core::toolpath::{Tool, ToolDatabase, ToolShape};
use eframe::egui;

/// The window's share of the screen when it first opens.
const WINDOW_SHARE: f32 = 0.6;
/// The tree's share of the window's width when it first opens.
const TREE_SHARE: f32 = 0.35;
/// A tool's picture beside its geometry, in rows of the theme's interactive height.
const PICTURE_ROWS: f32 = 6.0;
/// Aspire's tool pictures are 90 × 130: their width for a height.
const PICTURE_ASPECT: f32 = 90.0 / 130.0;
/// Lines of the notes boxes.
const NOTES_ROWS: usize = 4;
/// Millimetres to an inch.
const INCH: f64 = 25.4;

/// What is picked in the tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    Group(String),
    Tool(usize),
}

/// What the window asks of the app.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    /// Keep the edited database and leave the window open.
    Apply,
    /// Keep it and close the window.
    Ok,
    /// Close the window, dropping the edits.
    Cancel,
    /// Pick a tool database file and hand it to [`Manager::import`].
    Import,
    /// Pick where to save these tools.
    Export(ToolDatabase),
}

/// How the window draws: lengths in inches or millimetres, and its pictures.
pub struct Look<'a> {
    pub inch: bool,
    /// The picture at an icon path (`tool_database/end_mill.bmp`), if there is one.
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
}

/// The open window: the database being edited and what is picked in it.
pub struct Manager {
    pub draft: ToolDatabase,
    pub pick: Option<Pick>,
}

/// The picture of a tool shape, Aspire's own.
pub fn picture_path(shape: ToolShape) -> &'static str {
    match shape {
        ToolShape::EndMill => "tool_database/end_mill.bmp",
        ToolShape::BallNose => "tool_database/ball_nose.bmp",
        ToolShape::VBit => "tool_database/v_bit.bmp",
        ToolShape::RadiusedEndMill => "tool_database/radiused_end_mill.bmp",
    }
}

const SHAPES: [(ToolShape, &str); 4] = [
    (ToolShape::EndMill, "End Mill"),
    (ToolShape::BallNose, "Ball Nose"),
    (ToolShape::VBit, "V-Bit"),
    (ToolShape::RadiusedEndMill, "Radiused End Mill"),
];

fn shape_name(shape: ToolShape) -> &'static str {
    SHAPES
        .iter()
        .find(|(s, _)| *s == shape)
        .map_or("", |(_, n)| n)
}

/// What a tree button asked for.
#[derive(Clone, Copy)]
enum TreeAction {
    NewTool,
    NewGroup,
    Copy,
    Delete,
    Import,
    Export,
}

impl Manager {
    /// Opens on a copy of `db`, nothing picked.
    pub fn new(db: &ToolDatabase) -> Self {
        Self {
            draft: db.clone(),
            pick: None,
        }
    }

    /// Adds the tools of a picked file (named `name`) to the copy, in a group of their own.
    pub fn import(&mut self, incoming: ToolDatabase, name: &str) {
        let group = tree::import(&mut self.draft, incoming, name);
        self.pick = Some(Pick::Group(group));
    }

    /// Draws the window; what it asks of the app, if anything.
    pub fn show(&mut self, ctx: &egui::Context, look: &Look) -> Option<Outcome> {
        let mut open = true;
        let mut outcome = None;
        let size = ctx.screen_rect().size() * WINDOW_SHARE;
        egui::Window::new("Tool Database")
            .open(&mut open)
            .default_size(size)
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Tool Database window");
                egui::TopBottomPanel::bottom("tool_database_buttons").show_inside(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = Some(Outcome::Cancel);
                        }
                        if ui.button("OK").clicked() {
                            outcome = Some(Outcome::Ok);
                        }
                    });
                });
                egui::SidePanel::left("tool_database_tree")
                    .resizable(true)
                    .default_width(size.x * TREE_SHARE)
                    .show_inside(ui, |ui| {
                        egui::TopBottomPanel::bottom("tool_database_tree_buttons").show_inside(
                            ui,
                            |ui| {
                                if let Some(action) = self.tree_buttons(ui, look) {
                                    outcome = self.act(action).or(outcome.take());
                                }
                            },
                        );
                        egui::ScrollArea::both().show(ui, |ui| self.tree(ui, look));
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::TopBottomPanel::bottom("tool_database_apply").show_inside(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Apply").clicked() {
                                outcome = Some(Outcome::Apply);
                            }
                        });
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| self.details(ui, look));
                });
            });
        if !open {
            outcome = Some(Outcome::Cancel);
        }
        outcome
    }

    /// The tree: groups that open and close, each over its groups and tools.
    fn tree(&mut self, ui: &mut egui::Ui, look: &Look) {
        cut_craft_bug_report::mark(ui, "Tool Database tree");
        let all = tree::groups(&self.draft);
        self.branch(ui, look, &all, "");
    }

    fn branch(&mut self, ui: &mut egui::Ui, look: &Look, all: &[String], group: &str) {
        let row = ui.spacing().interact_size.y;
        for sub in tree::subgroups(all, group) {
            let id = ui.make_persistent_id(("tool_group", sub.as_str()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    if let Some(image) = (look.picture)("common/tool_database.svg") {
                        ui.add(image.fit_to_exact_size(egui::vec2(row, row)));
                    }
                    let picked = self.pick == Some(Pick::Group(sub.clone()));
                    let label = egui::RichText::new(tree::leaf(sub)).strong();
                    if ui.selectable_label(picked, label).clicked() {
                        self.pick = Some(Pick::Group(sub.clone()));
                    }
                })
                .body(|ui| self.branch(ui, look, all, sub));
        }
        for i in tree::tools_in(&self.draft, group) {
            let tool = &self.draft.tools[i];
            ui.horizontal(|ui| {
                if let Some(image) = (look.picture)(picture_path(tool.shape)) {
                    ui.add(image.fit_to_exact_size(egui::vec2(row * PICTURE_ASPECT, row)));
                }
                let picked = self.pick == Some(Pick::Tool(i));
                if ui.selectable_label(picked, &tool.name).clicked() {
                    self.pick = Some(Pick::Tool(i));
                }
            });
        }
    }

    /// Aspire's row under the tree: new tool, new group, copy, import, export, delete.
    fn tree_buttons(&self, ui: &mut egui::Ui, look: &Look) -> Option<TreeAction> {
        let edge = ui.spacing().interact_size.y;
        let picked = self.pick.is_some();
        let tool = matches!(self.pick, Some(Pick::Tool(_)));
        let mut action = None;
        ui.horizontal(|ui| {
            let buttons = [
                ("common/add.svg", "New Tool", true, TreeAction::NewTool),
                (
                    "common/add_tool.svg",
                    "New Tool Group",
                    true,
                    TreeAction::NewGroup,
                ),
                (
                    "common/copy_duplicate.svg",
                    "Copy Tool",
                    tool,
                    TreeAction::Copy,
                ),
                ("common/open.svg", "Import Tools", true, TreeAction::Import),
                (
                    "common/save.svg",
                    "Export Tools",
                    picked,
                    TreeAction::Export,
                ),
                ("common/delete.svg", "Delete", picked, TreeAction::Delete),
            ];
            for (icon, label, enabled, asked) in buttons {
                let button = match (look.picture)(icon) {
                    Some(image) => {
                        egui::Button::image(image.fit_to_exact_size(egui::vec2(edge, edge)))
                    }
                    None => egui::Button::new(label),
                };
                if ui
                    .add_enabled(enabled, button)
                    .on_hover_text(label)
                    .clicked()
                {
                    action = Some(asked);
                }
            }
        });
        action
    }

    fn act(&mut self, action: TreeAction) -> Option<Outcome> {
        // new things go in the picked group, or the picked tool's
        let group = match &self.pick {
            Some(Pick::Group(g)) => g.clone(),
            Some(Pick::Tool(i)) => self.draft.tools[*i].group.clone(),
            None => String::new(),
        };
        match action {
            TreeAction::NewTool => {
                self.pick = Some(Pick::Tool(tree::add_tool(&mut self.draft, &group)));
            }
            TreeAction::NewGroup => {
                self.pick = Some(Pick::Group(tree::add_group(&mut self.draft, &group)));
            }
            TreeAction::Copy => {
                if let Some(Pick::Tool(i)) = self.pick {
                    self.pick = Some(Pick::Tool(tree::copy_tool(&mut self.draft, i)));
                }
            }
            TreeAction::Delete => match self.pick.take() {
                Some(Pick::Tool(i)) => {
                    self.draft.tools.remove(i);
                }
                Some(Pick::Group(g)) => tree::delete_group(&mut self.draft, &g),
                None => {}
            },
            TreeAction::Import => return Some(Outcome::Import),
            TreeAction::Export => {
                return match &self.pick {
                    Some(Pick::Tool(i)) => Some(Outcome::Export(ToolDatabase {
                        tools: vec![self.draft.tools[*i].clone()],
                        groups: vec![],
                    })),
                    Some(Pick::Group(g)) => {
                        Some(Outcome::Export(tree::export_group(&self.draft, g)))
                    }
                    None => None,
                };
            }
        }
        None
    }

    /// The picked group's name and notes, or the picked tool's settings.
    fn details(&mut self, ui: &mut egui::Ui, look: &Look) {
        cut_craft_bug_report::mark(ui, "Tool Database details");
        match self.pick.clone() {
            Some(Pick::Group(group)) => self.group_details(ui, &group),
            Some(Pick::Tool(i)) if i < self.draft.tools.len() => {
                let groups = tree::groups(&self.draft);
                tool_details(ui, &mut self.draft.tools[i], &groups, look);
            }
            _ => {
                ui.weak("Pick a group or tool in the tree");
            }
        }
    }

    fn group_details(&mut self, ui: &mut egui::Ui, group: &str) {
        egui::Grid::new("tool_group_details")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Name");
                let mut name = tree::leaf(group).to_owned();
                let edit =
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
                // a group's name is its path's last level, so an empty or `/` name is refused
                if edit.changed() && !name.is_empty() && !name.contains('/') {
                    let renamed = tree::rename_group(&mut self.draft, group, &name);
                    self.pick = Some(Pick::Group(renamed));
                }
                ui.end_row();
                ui.label("Notes");
                let path = match &self.pick {
                    Some(Pick::Group(g)) => g.clone(),
                    _ => group.to_owned(),
                };
                ui.add(
                    egui::TextEdit::multiline(tree::notes_mut(&mut self.draft, &path))
                        .desired_rows(NOTES_ROWS)
                        .desired_width(f32::INFINITY),
                );
                ui.end_row();
            });
    }
}

/// A length (mm) shown in the job's units; whether it changed.
fn length(ui: &mut egui::Ui, value: &mut f64, look: &Look, per_minute: bool) -> bool {
    let (per, unit) = match (look.inch, per_minute) {
        (true, false) => (INCH, "in"),
        (true, true) => (INCH, "in/min"),
        (false, false) => (1.0, "mm"),
        (false, true) => (1.0, "mm/min"),
    };
    let mut shown = *value / per;
    // ponytail: drag speed is a hundredth of the value, so small and large tools both drag well
    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
    let changed = ui
        .add(
            egui::DragValue::new(&mut shown)
                .speed(speed)
                .clamp_range(0.0..=f64::MAX)
                .max_decimals(4)
                .suffix(format!(" {unit}")),
        )
        .changed();
    if changed {
        *value = shown * per;
    }
    changed
}

/// A framed block with a heading, as Aspire's Tool Info, Cutting Parameters and Feeds and
/// Speeds are.
fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(title);
        body(ui);
    });
}

fn tool_details(ui: &mut egui::Ui, tool: &mut Tool, groups: &[String], look: &Look) {
    section(ui, "Tool Info", |ui| tool_info(ui, tool, groups, look));
    section(ui, "Cutting Parameters", |ui| cutting(ui, tool, look));
    section(ui, "Feeds and Speeds", |ui| feeds(ui, tool, look));
}

/// Name, type, group and notes, then the geometry beside the tool's picture.
fn tool_info(ui: &mut egui::Ui, tool: &mut Tool, groups: &[String], look: &Look) {
    egui::Grid::new("tool_info").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut tool.name).desired_width(f32::INFINITY));
        ui.end_row();
        ui.label("Tool Type");
        egui::ComboBox::from_id_source("tool_type")
            .selected_text(shape_name(tool.shape))
            .show_ui(ui, |ui| {
                for (shape, name) in SHAPES {
                    ui.selectable_value(&mut tool.shape, shape, name);
                }
            });
        ui.end_row();
        // stands in for dragging a tool between groups
        ui.label("Group");
        egui::ComboBox::from_id_source("tool_group")
            .selected_text(if tool.group.is_empty() {
                "(top)"
            } else {
                &tool.group
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut tool.group, String::new(), "(top)");
                for g in groups {
                    ui.selectable_value(&mut tool.group, g.clone(), g);
                }
            });
        ui.end_row();
        ui.label("Notes");
        ui.add(
            egui::TextEdit::multiline(&mut tool.notes)
                .desired_rows(NOTES_ROWS)
                .desired_width(f32::INFINITY),
        );
        ui.end_row();
    });
    ui.separator();
    ui.horizontal(|ui| {
        egui::Grid::new("tool_geometry")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Diameter (D)");
                length(ui, &mut tool.diameter, look, false);
                ui.end_row();
                if tool.shape == ToolShape::VBit {
                    ui.label("Included Angle (A)");
                    ui.add(
                        egui::DragValue::new(&mut tool.angle)
                            .clamp_range(1.0..=179.0)
                            .suffix("°"),
                    );
                    ui.end_row();
                }
                if tool.shape == ToolShape::RadiusedEndMill {
                    ui.label("Tip Radius (R)");
                    length(ui, &mut tool.tip_radius, look, false);
                    ui.end_row();
                }
                ui.label("Number of Flutes");
                ui.add(egui::DragValue::new(&mut tool.flutes).clamp_range(1..=12));
                ui.end_row();
            });
        if let Some(image) = (look.picture)(picture_path(tool.shape)) {
            let height = ui.spacing().interact_size.y * PICTURE_ROWS;
            ui.add(image.fit_to_exact_size(egui::vec2(height * PICTURE_ASPECT, height)));
        }
    });
}

fn cutting(ui: &mut egui::Ui, tool: &mut Tool, look: &Look) {
    egui::Grid::new("tool_cutting")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Pass Depth");
            length(ui, &mut tool.pass_depth, look, false);
            ui.end_row();
            ui.label("Stepover");
            let mut percent = tool.stepover * 100.0;
            if ui
                .add(
                    egui::DragValue::new(&mut percent)
                        .clamp_range(0.0..=100.0)
                        .max_decimals(2)
                        .suffix(" %"),
                )
                .changed()
            {
                tool.stepover = percent / 100.0;
            }
            ui.end_row();
        });
}

fn feeds(ui: &mut egui::Ui, tool: &mut Tool, look: &Look) {
    egui::Grid::new("tool_feeds").num_columns(2).show(ui, |ui| {
        ui.label("Spindle Speed");
        ui.add(
            egui::DragValue::new(&mut tool.spindle_speed)
                .clamp_range(0.0..=f64::MAX)
                .suffix(" r.p.m."),
        );
        ui.end_row();
        ui.label("Feed Rate");
        length(ui, &mut tool.feed_rate, look, true);
        ui.end_row();
        ui.label("Plunge Rate");
        length(ui, &mut tool.plunge_rate, look, true);
        ui.end_row();
    });
    // Aspire shows the feed per tooth, to check against the cutter maker's figure
    if let Some(load) = tool.chip_load() {
        let (per, unit, places) = if look.inch {
            (INCH, "in", 4)
        } else {
            (1.0, "mm", 3)
        };
        ui.label(format!("Chip load: {:.places$} {unit}", load / per));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(manager: &mut Manager, ctx: &egui::Context) -> Option<Outcome> {
        let look = Look {
            inch: false,
            picture: &|_| None,
        };
        let mut outcome = None;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| outcome = manager.show(ctx, &look));
        outcome
    }

    #[test]
    fn the_window_draws_groups_tools_and_holds_its_size() {
        let ctx = egui::Context::default();
        let mut m = Manager::new(&ToolDatabase::starter());
        frame(&mut m, &ctx);
        m.pick = Some(Pick::Tool(0));
        frame(&mut m, &ctx);
        let first = ctx.memory(|mem| mem.area_rect(egui::Id::new("Tool Database")));
        for _ in 0..3 {
            frame(&mut m, &ctx);
        }
        let later = ctx.memory(|mem| mem.area_rect(egui::Id::new("Tool Database")));
        assert_eq!(first, later, "panels inside the window do not grow it");
        m.pick = Some(Pick::Group("Metric Tools".into()));
        assert_eq!(frame(&mut m, &ctx), None);
    }

    #[test]
    fn tree_buttons_add_copy_and_delete_in_the_copy_only() {
        let db = ToolDatabase::starter();
        let mut m = Manager::new(&db);
        m.pick = Some(Pick::Group("Metric Tools/End Mills".into()));
        m.act(TreeAction::NewTool);
        let Some(Pick::Tool(i)) = m.pick else {
            panic!("the new tool is picked")
        };
        assert_eq!(m.draft.tools[i].group, "Metric Tools/End Mills");
        m.act(TreeAction::Copy);
        assert_eq!(m.draft.tools.len(), db.tools.len() + 2);
        m.act(TreeAction::Delete);
        assert_eq!(m.draft.tools.len(), db.tools.len() + 1);
        m.pick = Some(Pick::Group("Imperial Tools".into()));
        let Some(Outcome::Export(out)) = m.act(TreeAction::Export) else {
            panic!("exports")
        };
        assert!(!out.tools.is_empty() && out.tools.iter().all(|t| t.group.starts_with("Imperial")));
        assert_eq!(m.act(TreeAction::Import), Some(Outcome::Import));
        m.import(out, "Vendor");
        assert_eq!(m.pick, Some(Pick::Group("Vendor".into())));
        assert_eq!(
            ToolDatabase::starter(),
            db,
            "the database itself is untouched"
        );
    }
}
