//! The manager window, laid out as Aspire's Tool Database: the tool tree with its buttons
//! on the left, the picked group's or tool's settings on the right, OK / Cancel under both.
//! Edits are made to a copy; only Apply and OK hand it back.

use crate::tree;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::document::Units;
use cut_craft_core::toolpath::{CuttingData, RateUnits, Tool, ToolDatabase, ToolShape};
use eframe::egui;
use std::collections::BTreeMap;

/// The window's share of the screen when it first opens.
/// The window's own id (its title's hashing is egui's).
fn tool_window() -> egui::Id {
    egui::Id::unique("tool_database_window")
}
const WINDOW_SHARE: f32 = 0.7;
/// The tree's share of the window's width when it first opens.
const TREE_SHARE: f32 = 0.35;
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
    /// Keep the edited database, close the window and cut with this tool (by name): the
    /// window was opened from a toolpath form's Select….
    Select(String),
}

/// How the window draws: lengths in inches or millimetres, and its pictures.
pub struct Look<'a> {
    /// The picture at an icon path (`tool_database/end_mill.png`), if there is one.
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
    /// The job's selected open vector, which a form tool can take its shape from.
    pub profile: Option<&'a cut_craft_core::geometry::Path>,
}

/// The open window: the database being edited and what is picked in it.
pub struct Manager {
    pub draft: ToolDatabase,
    pub pick: Option<Pick>,
    /// The active machine's tool changer, its tools loaded here; kept with the tools.
    pub changer: Option<Changer>,
    /// The name typed for a new material.
    pub new_material: String,
    /// Duplicate Material copies every tool's cutting data for the material too.
    pub copy_cutting_data: bool,
    /// The tree leaves out the tools with no cutting data for the active material (else
    /// they are greyed); the tree's right-click Hide Unset Tools.
    pub hide_unset: bool,
    /// What is typed in the search box over the tree: tools are found by name.
    pub search: String,
    /// Opened from a toolpath form's Select…: titled Select Tool, with Select for the tool
    /// shown in place of OK.
    pub selecting: bool,
    index: crate::search::SearchIndex,
}

/// A machine's automatic tool changer: the tool in each slot by name, slot 1 first.
#[derive(Clone, Debug, PartialEq)]
pub struct Changer {
    pub machine: String,
    pub slots: Vec<Option<String>>,
}

impl Changer {
    /// The slot `tool` is loaded in, counted from 1.
    pub fn slot_of(&self, tool: &str) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_deref() == Some(tool))
            .map(|i| i + 1)
    }

    /// Loads `tool` into `slot` (counted from 1), taking out what was there and the tool
    /// from any other slot; `None` unloads it.
    pub fn load(&mut self, tool: &str, slot: Option<usize>) {
        for s in &mut self.slots {
            if s.as_deref() == Some(tool) {
                *s = None;
            }
        }
        if let Some(s) = slot.and_then(|n| self.slots.get_mut(n.wrapping_sub(1))) {
            *s = Some(tool.to_owned());
        }
    }

    /// A tool renamed keeps its slot.
    pub fn rename(&mut self, from: &str, to: &str) {
        for s in &mut self.slots {
            if s.as_deref() == Some(from) {
                *s = Some(to.to_owned());
            }
        }
    }

    /// Slots whose tool is no longer in `db` are emptied.
    pub fn unload_missing(&mut self, db: &ToolDatabase) {
        for s in &mut self.slots {
            if s.as_deref().is_some_and(|t| db.get(t).is_none()) {
                *s = None;
            }
        }
    }
}

/// The picture of a tool's shape.
pub fn picture_path(tool: &Tool) -> &'static str {
    match tool.shape {
        ToolShape::BallNose if tapered_ball(tool) => "tool_database/tapered_ball_nose.png",
        ToolShape::EndMill => "tool_database/end_mill.png",
        ToolShape::BallNose => "tool_database/ball_nose.png",
        ToolShape::VBit => "tool_database/v_bit.png",
        ToolShape::RadiusedEndMill => "tool_database/radiused_end_mill.png",
        // ponytail: no thread mill or form tool picture is drawn yet, so none shows
        ToolShape::ThreadMill => "tool_database/thread_mill.png",
        ToolShape::FormTool => "tool_database/form_tool.png",
    }
}

const SHAPES: [(ToolShape, &str); 6] = [
    (ToolShape::EndMill, "End Mill"),
    (ToolShape::BallNose, "Ball Nose"),
    (ToolShape::VBit, "V-Bit"),
    (ToolShape::RadiusedEndMill, "Radiused End Mill"),
    (ToolShape::ThreadMill, "Thread Mill"),
    (ToolShape::FormTool, "Form Tool"),
];

/// A tapered ball nose (a TBN or cone ball nose): a ball nose whose sides open out to its
/// shank at its taper angle. The editor lists it as a type of its own.
fn tapered_ball(tool: &Tool) -> bool {
    tool.shape == ToolShape::BallNose && tool.taper > 0.0
}

/// The Tool Type list: the shapes, with Tapered Ball Nose under Ball Nose.
fn tool_type(ui: &mut egui::Ui, tool: &mut Tool) {
    let tapered = tapered_ball(tool);
    let shown = if tapered {
        "Tapered Ball Nose"
    } else {
        shape_name(tool.shape)
    };
    egui::ComboBox::from_id_salt("tool_type")
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (shape, name) in SHAPES {
                if ui
                    .selectable_label(tool.shape == shape && !tapered, name)
                    .clicked()
                {
                    tool.shape = shape;
                    if shape == ToolShape::BallNose {
                        tool.taper = 0.0;
                    }
                }
                if shape == ToolShape::BallNose
                    && ui.selectable_label(tapered, "Tapered Ball Nose").clicked()
                    && !tapered
                {
                    make_tapered_ball(tool);
                }
            }
        });
}

/// Makes the tool a tapered ball nose, keeping its taper and shank when it has them, else a
/// usual one's: 3° a side on a shank twice its tip (1/8 in at the least).
fn make_tapered_ball(tool: &mut Tool) {
    tool.shape = ToolShape::BallNose;
    if tool.taper <= 0.0 {
        tool.taper = 3.0;
    }
    if tool.shank_diameter <= tool.diameter {
        tool.shank_diameter = (tool.diameter * 2.0).max(3.175);
    }
}

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
            changer: None,
            new_material: String::new(),
            copy_cutting_data: true,
            hide_unset: false,
            search: String::new(),
            selecting: false,
            index: crate::search::SearchIndex::default(),
        }
    }

    /// Opens on a copy of `db` to pick a toolpath's tool, on `current` when it is listed.
    pub fn selecting(db: &ToolDatabase, current: &str) -> Self {
        let mut manager = Self::new(db);
        manager.selecting = true;
        manager.pick = db
            .tools
            .iter()
            .position(|t| t.name == current)
            .map(Pick::Tool);
        manager
    }

    /// The name of the tool shown, when one is.
    pub fn shown_tool(&self) -> Option<&str> {
        match self.pick {
            Some(Pick::Tool(i)) => self.draft.tools.get(i).map(|t| t.name.as_str()),
            _ => None,
        }
    }

    /// Select: what the window asks for, when a tool is shown.
    pub fn select(&self) -> Option<Outcome> {
        self.shown_tool()
            .map(|name| Outcome::Select(name.to_owned()))
    }

    /// Loads tool `i` into the tool changer's `slot` (counted from 1; `None` takes it out),
    /// the slot becoming its tool number so the number shown is the holder it is in.
    pub fn load_into(&mut self, i: usize, slot: Option<usize>) {
        let (Some(changer), Some(tool)) = (&mut self.changer, self.draft.tools.get(i)) else {
            return;
        };
        changer.load(&tool.name, slot);
        self.draft.number_by_slots(&changer.slots);
    }

    /// Aspire's Duplicate Material: the active material copied under the typed name, or
    /// its own with " copy", with every tool's cutting data for it when asked, and made
    /// active.
    pub fn duplicate_material(&mut self) {
        let from = self.draft.material.clone();
        if from.is_empty() {
            return;
        }
        let name = std::mem::take(&mut self.new_material);
        self.draft
            .duplicate_material(&from, &name, self.copy_cutting_data);
    }

    /// Aspire's Add Material: the typed name joins the list and becomes active (the one
    /// already listed, if it is there under another case).
    pub fn add_material(&mut self) {
        let name = self.new_material.trim().to_owned();
        if name.is_empty() {
            return;
        }
        self.draft.add_material(&name);
        if let Some(listed) = self
            .draft
            .materials
            .iter()
            .find(|m| m.eq_ignore_ascii_case(&name))
        {
            self.draft.material.clone_from(listed);
        }
        self.new_material.clear();
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
        let size = ctx.content_rect().size() * WINDOW_SHARE;
        cut_craft_bug_report::dialog_with_panels(
            ctx,
            tool_window(),
            if self.selecting {
                "Select Tool"
            } else {
                "Tool Database"
            },
            size,
            &mut open,
            |ui| {
                cut_craft_bug_report::mark(ui, "Tool Database window");
                egui::Panel::bottom("tool_database_buttons").show(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Cancel").clicked() {
                            outcome = Some(Outcome::Cancel);
                        }
                        if !self.selecting {
                            if ui.button("OK").clicked() {
                                outcome = Some(Outcome::Ok);
                            }
                            return;
                        }
                        let select = ui
                            .add_enabled(self.shown_tool().is_some(), egui::Button::new("Select"))
                            .on_hover_hint(
                                "Cut with the tool shown, with its saved feeds, speeds and \
                                 pass depth (any edits made here are kept)",
                            )
                            .on_disabled_hover_text("Pick a tool in the list first");
                        if select.clicked() {
                            outcome = self.select();
                        }
                        match self.shown_tool() {
                            Some(name) => ui.label(name),
                            None => ui.weak("Pick a tool in the list"),
                        };
                    });
                });
                egui::Panel::left("tool_database_tree")
                    .resizable(true)
                    .default_size(size.x * TREE_SHARE)
                    .show(ui, |ui| {
                        egui::Panel::bottom("tool_database_tree_buttons").show(ui, |ui| {
                            if let Some(action) = self.tree_buttons(ui, look) {
                                outcome = self.act(action).or(outcome.take());
                            }
                        });
                        // the active material is the whole database's, so it sits by the tree
                        egui::Panel::bottom("tool_database_material")
                            .show(ui, |ui| self.materials(ui));
                        egui::ScrollArea::both().show(ui, |ui| self.tree(ui, look));
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Panel::bottom("tool_database_apply").show(ui, |ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Apply").clicked() {
                                outcome = Some(Outcome::Apply);
                            }
                        });
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| self.details(ui, look));
                });
            },
        );
        if !open {
            outcome = Some(Outcome::Cancel);
        }
        outcome
    }

    /// The tree: groups that open and close, each over its groups and tools.
    fn tree(&mut self, ui: &mut egui::Ui, look: &Look) {
        cut_craft_bug_report::mark(ui, "Tool Database tree");
        self.changer_list(ui);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Search tool names")
                .desired_width(f32::INFINITY),
        );
        if self.search.trim().is_empty() {
            let all = tree::groups(&self.draft);
            self.branch(ui, look, &all, "");
        } else {
            self.found(ui, look);
        }
    }

    /// The tools the search finds, each with its group, instead of the tree; a click picks one.
    fn found(&mut self, ui: &mut egui::Ui, look: &Look) {
        if self.index.is_stale(&self.draft) {
            self.index = crate::search::SearchIndex::new(&self.draft);
        }
        let found = self.index.find(&self.search);
        if found.is_empty() {
            ui.weak("No tool has that name");
        }
        let row = ui.spacing().interact_size.y;
        for i in found {
            let tool = &self.draft.tools[i];
            ui.horizontal(|ui| {
                if let Some(image) = (look.picture)(picture_path(tool)) {
                    ui.add(image.fit_to_exact_size(egui::vec2(row * PICTURE_ASPECT, row)));
                }
                let picked = self.pick == Some(Pick::Tool(i));
                if ui.selectable_label(picked, &tool.name).clicked() {
                    self.pick = Some(Pick::Tool(i));
                }
                if !tool.group.is_empty() {
                    ui.weak(&tool.group);
                }
            });
        }
    }

    /// The tool changer's slots above the tree; a click picks the tool in it.
    fn changer_list(&mut self, ui: &mut egui::Ui) {
        let Some(changer) = &self.changer else { return };
        let mut picked = None;
        egui::CollapsingHeader::new(format!("Tool changer: {}", changer.machine))
            .default_open(true)
            .show(ui, |ui| {
                for (i, slot) in changer.slots.iter().enumerate() {
                    let text = format!("T{}  {}", i + 1, slot.as_deref().unwrap_or("(empty)"));
                    let at = slot
                        .as_deref()
                        .and_then(|t| self.draft.tools.iter().position(|x| x.name == t));
                    let on = at.is_some() && self.pick == at.map(Pick::Tool);
                    if ui.selectable_label(on, text).clicked() {
                        picked = at;
                    }
                }
            });
        if let Some(i) = picked {
            self.pick = Some(Pick::Tool(i));
        }
        ui.separator();
    }

    fn branch(&mut self, ui: &mut egui::Ui, look: &Look, all: &[String], group: &str) {
        let row = ui.spacing().interact_size.y;
        for sub in tree::subgroups(all, group) {
            let id = ui.make_persistent_id(("tool_group", sub.as_str()));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
                .show_header(ui, |ui| {
                    if let Some(image) = (look.picture)("common/tool_database.png") {
                        ui.add(image.fit_to_exact_size(egui::vec2(row, row)));
                    }
                    let picked = self.pick == Some(Pick::Group(sub.clone()));
                    let label = egui::RichText::new(tree::leaf(sub)).strong();
                    let label = ui.selectable_label(picked, label);
                    cut_craft_bug_report::context_menu(&label, |ui| {
                        ui.checkbox(&mut self.hide_unset, "Hide Unset Tools");
                    });
                    if label.clicked() {
                        self.pick = Some(Pick::Group(sub.clone()));
                    }
                })
                .body(|ui| self.branch(ui, look, all, sub));
        }
        for i in tree::tools_in(&self.draft, group) {
            let tool = &self.draft.tools[i];
            let unset = !tool.has_data_for(&self.draft.material);
            if unset && self.hide_unset {
                continue;
            }
            ui.horizontal(|ui| {
                if let Some(image) = (look.picture)(picture_path(tool)) {
                    ui.add(image.fit_to_exact_size(egui::vec2(row * PICTURE_ASPECT, row)));
                }
                let picked = self.pick == Some(Pick::Tool(i));
                let mut name = egui::RichText::new(&tool.name);
                if unset {
                    name = name.color(ui.visuals().weak_text_color());
                }
                let label = ui.selectable_label(picked, name);
                if unset {
                    label.clone().on_hover_hint(format!(
                        "No cutting data for {}: it cuts with its default data",
                        self.draft.material
                    ));
                }
                cut_craft_bug_report::context_menu(&label, |ui| {
                    ui.checkbox(&mut self.hide_unset, "Hide Unset Tools");
                });
                if label.clicked() {
                    self.pick = Some(Pick::Tool(i));
                }
                if let Some(n) = self.changer.as_ref().and_then(|c| c.slot_of(&tool.name)) {
                    ui.weak(format!("T{n}"))
                        .on_hover_hint("Loaded in the tool changer");
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
                ("common/add.png", "New Tool", true, TreeAction::NewTool),
                (
                    "common/add_tool.png",
                    "New Tool Group",
                    true,
                    TreeAction::NewGroup,
                ),
                (
                    "common/copy_duplicate.png",
                    "Copy Tool",
                    tool,
                    TreeAction::Copy,
                ),
                ("common/open.png", "Import Tools", true, TreeAction::Import),
                (
                    "common/save.png",
                    "Export Tools",
                    picked,
                    TreeAction::Export,
                ),
                ("common/delete.png", "Delete", picked, TreeAction::Delete),
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
                    .on_hover_hint(label)
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
        let outcome = self.do_action(action, &group);
        // a tool deleted leaves its slot
        if let Some(changer) = &mut self.changer {
            changer.unload_missing(&self.draft);
        }
        outcome
    }

    fn do_action(&mut self, action: TreeAction, group: &str) -> Option<Outcome> {
        let group = group.to_owned();
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
                    Some(Pick::Tool(i)) => {
                        let tool = self.draft.tools[*i].clone();
                        let materials = tool.materials.iter().map(|m| m.material.clone());
                        Some(Outcome::Export(ToolDatabase {
                            materials: materials.collect(),
                            tools: vec![tool],
                            ..ToolDatabase::default()
                        }))
                    }
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
                let before = self.draft.tools[i].name.clone();
                let material = self.draft.material.clone();
                let ToolDatabase {
                    tools,
                    name_formats,
                    ..
                } = &mut self.draft;
                let loaded = self
                    .changer
                    .as_ref()
                    .and_then(|c| c.slot_of(&before).map(|slot| (c.machine.as_str(), slot)));
                tool_details(
                    ui,
                    &mut tools[i],
                    (&groups, name_formats),
                    (look, loaded),
                    &material,
                );
                let name = self.draft.tools[i].name.clone();
                let mut chosen = None;
                if let Some(changer) = &mut self.changer {
                    changer.rename(&before, &name);
                    section(ui, "Tool Changer", |ui| {
                        chosen = load_slot(ui, changer, &name);
                    });
                }
                if let Some(slot) = chosen {
                    self.load_into(i, (slot > 0).then_some(slot));
                }
            }
            _ => {
                ui.weak("Pick a group or tool in the tree");
            }
        }
    }

    /// Aspire's Material combo and Material Management in one row: the active material,
    /// whose cutting data tools show and toolpaths get; adding one, and deleting the active
    /// one with every tool's data for it.
    // ponytail: no renaming materials, and no machine combo (the data is per material only)
    fn materials(&mut self, ui: &mut egui::Ui) {
        const DEFAULT: &str = "(default)";
        ui.horizontal_wrapped(|ui| {
            ui.label("Material");
            let active = &mut self.draft.material;
            let shown = if active.is_empty() {
                DEFAULT.to_owned()
            } else {
                active.clone()
            };
            egui::ComboBox::from_id_salt("tool_material")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    ui.selectable_value(active, String::new(), DEFAULT);
                    for m in &self.draft.materials {
                        ui.selectable_value(active, m.clone(), m);
                    }
                });
            ui.add(egui::TextEdit::singleline(&mut self.new_material).hint_text("New material"));
            let typed = !self.new_material.trim().is_empty();
            if ui
                .add_enabled(typed, egui::Button::new("Add Material"))
                .clicked()
            {
                self.add_material();
            }
            if ui
                .add_enabled(
                    !self.draft.material.is_empty(),
                    egui::Button::new("Delete Material"),
                )
                .on_hover_hint("Deletes every tool's cutting data for it")
                .clicked()
            {
                let gone = self.draft.material.clone();
                self.draft.delete_material(&gone);
            }
            if ui
                .add_enabled(
                    !self.draft.material.is_empty(),
                    egui::Button::new("Duplicate Material"),
                )
                .on_hover_hint("A copy of the active material, named as typed (or \"… copy\")")
                .clicked()
            {
                self.duplicate_material();
            }
            ui.checkbox(&mut self.copy_cutting_data, "with cutting data")
                .on_hover_hint("The copy gets every tool's cutting data for the material");
        });
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

/// Which slot of the tool changer `tool` is loaded in, to change: the slot picked (0 for
/// taking it out), if one was.
fn load_slot(ui: &mut egui::Ui, changer: &Changer, tool: &str) -> Option<usize> {
    let now = changer.slot_of(tool);
    let shown = now.map_or_else(|| "(not loaded)".to_owned(), |n| format!("T{n}"));
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(format!("Loaded in {}", changer.machine));
        egui::ComboBox::from_id_salt("tool_changer_slot")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                if ui.selectable_label(now.is_none(), "(not loaded)").clicked() {
                    chosen = Some(0);
                }
                for (i, slot) in changer.slots.iter().enumerate() {
                    let holding = slot
                        .as_deref()
                        .map_or_else(|| "empty".to_owned(), str::to_owned);
                    if ui
                        .selectable_label(now == Some(i + 1), format!("T{} ({holding})", i + 1))
                        .clicked()
                    {
                        chosen = Some(i + 1);
                    }
                }
            });
    });
    ui.weak("The slot it is loaded in becomes its tool number");
    chosen
}

/// A length (mm) shown in the tool's units; whether it changed.
pub(crate) fn length(ui: &mut egui::Ui, value: &mut f64, units: Units) -> bool {
    let (per, unit) = if units == Units::Inch {
        (INCH, "in")
    } else {
        (1.0, "mm")
    };
    number(ui, value, per, unit)
}

/// A rate (mm/min) shown in the tool's rate units; whether it changed.
fn rate(ui: &mut egui::Ui, value: &mut f64, units: RateUnits) -> bool {
    number(ui, value, units.mm_per_min(), units.label())
}

/// `value` shown as so many of `per`, with its unit; whether it changed.
fn number(ui: &mut egui::Ui, value: &mut f64, per: f64, unit: &str) -> bool {
    let mut shown = *value / per;
    // ponytail: drag speed is a hundredth of the value, so small and large tools both drag well
    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
    let changed = ui
        .add(
            egui::DragValue::new(&mut shown)
                .speed(speed)
                .range(0.0..=f64::MAX)
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

fn tool_details(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    (groups, formats): (&[String], &mut BTreeMap<String, String>),
    (look, loaded): (&Look, Option<(&str, usize)>),
    material: &str,
) {
    section(ui, "Tool Info", |ui| {
        tool_info(ui, tool, (groups, formats), look, loaded);
    });
    // the cutting data shown is the active material's, as Aspire's
    let own = tool
        .materials
        .iter()
        .any(|m| m.material.eq_ignore_ascii_case(material));
    if !material.is_empty() && !own {
        ui.weak(format!(
            "No cutting data for {material} yet: the default's is shown, and a change \
             keeps it as {material}'s own"
        ));
    }
    copy_settings_from(ui, tool, material);
    let mut shown = tool.for_material(material);
    section(ui, "Cutting Parameters", |ui| cutting(ui, &mut shown));
    section(ui, "Feeds and Speeds", |ui| feeds(ui, &mut shown));
    keep_cutting(tool, &shown, material);
    tool.renamed(material);
}

/// Aspire's Copy Settings From: the tool's cutting data for another material (or its
/// default) copied as its data for `material`, where it has any to copy.
fn copy_settings_from(ui: &mut egui::Ui, tool: &mut Tool, material: &str) {
    let others: Vec<String> = std::iter::once(String::new())
        .chain(tool.materials.iter().map(|m| m.material.clone()))
        .filter(|m| !m.eq_ignore_ascii_case(material))
        .collect();
    if others.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.label("Copy settings from");
        egui::ComboBox::from_id_salt("copy_cutting_from")
            .selected_text("another material…")
            .show_ui(ui, |ui| {
                for from in &others {
                    let label = if from.is_empty() { "(default)" } else { from };
                    if ui.selectable_label(false, label).clicked() {
                        tool.copy_cutting(from, material);
                    }
                }
            });
    });
}

/// Keeps what was edited in `shown` (the tool as it cuts `material`) as the tool's data for
/// that material, or its default for none; rate units are the tool's whatever the material.
fn keep_cutting(tool: &mut Tool, shown: &Tool, material: &str) {
    tool.rate_units = shown.rate_units;
    let edited = CuttingData {
        material: material.to_owned(),
        ..shown.cutting("")
    };
    if edited != tool.cutting(material) {
        tool.set_cutting(edited);
    }
}

/// The tool's name format (Aspire's Name Format): `{Variable|Modifier}` filled in from the
/// tool, a variable picked from the list put on the end; Set as Default makes it its type's
/// for new tools, Restore puts the type's back. A tool with a format is named by it.
fn name_format(ui: &mut egui::Ui, tool: &mut Tool, formats: &mut BTreeMap<String, String>) {
    use cut_craft_core::tool_names::type_key;
    let key = type_key(tool.shape);
    ui.vertical(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut tool.name_format)
                .hint_text("{Tool Type} {Diameter|F}")
                .desired_width(f32::INFINITY),
        )
        .on_hover_hint(
            "The name made from the tool's own sizes: leave empty to type the name. \
             Modifiers: |U upper case, |L lower case, |0.00 decimals, |F inches as a \
             fraction",
        );
        ui.horizontal(|ui| name_format_buttons(ui, tool, formats, &key));
    });
}

/// Add variable, Set as Default and Restore, under the name format.
fn name_format_buttons(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    formats: &mut BTreeMap<String, String>,
    key: &str,
) {
    use cut_craft_core::tool_names::VARIABLES;
    egui::ComboBox::from_id_salt("name_variable")
        .selected_text("Add variable")
        .show_ui(ui, |ui| {
            for v in VARIABLES {
                if ui.selectable_label(false, v).clicked() {
                    tool.name_format.extend(["{", v, "}"]);
                }
            }
        });
    if ui
        .button("Set as Default")
        .on_hover_hint("New tools of this type are named by this format")
        .clicked()
    {
        formats.insert(key.to_owned(), tool.name_format.clone());
    }
    if let Some(default) = formats.get(key).filter(|d| **d != tool.name_format)
        && ui
            .button("Restore")
            .on_hover_hint(format!("Back to this type's default: {default}"))
            .clicked()
    {
        tool.name_format.clone_from(default);
    }
}

/// A form tool's row in Tool Info: Use Selected Vector takes its shape from the job's
/// selected open vector, and how many points its profile has.
fn form_row(ui: &mut egui::Ui, tool: &mut Tool, look: &Look) {
    if tool.shape != ToolShape::FormTool {
        return;
    }
    ui.label("Form");
    ui.horizontal(|ui| {
        let use_it = ui
            .add_enabled(
                look.profile.is_some(),
                egui::Button::new("Use Selected Vector"),
            )
            .on_hover_hint(
                "Half the tool's section, drawn upright: the axis at its left, the \
                 tip at its lowest point",
            )
            .on_disabled_hover_text("Select one open vector in the job first");
        if use_it.clicked()
            && let Some(path) = look.profile
        {
            tool.set_form(path);
        }
        ui.weak(if tool.form.is_empty() {
            "no profile yet".to_owned()
        } else {
            format!("{} points", tool.form.len())
        });
    });
    ui.end_row();
}

/// Name, type, group and notes, then the geometry beside the tool's picture.
fn tool_info(
    ui: &mut egui::Ui,
    tool: &mut Tool,
    (groups, formats): (&[String], &mut BTreeMap<String, String>),
    look: &Look,
    loaded: Option<(&str, usize)>,
) {
    egui::Grid::new("tool_info").num_columns(2).show(ui, |ui| {
        ui.label("Name");
        // a name made from its name format is not typed
        ui.add_enabled(
            tool.name_format.is_empty(),
            egui::TextEdit::singleline(&mut tool.name).desired_width(f32::INFINITY),
        );
        ui.end_row();
        // under the name it makes (Aspire keeps it in a dialog of its own)
        ui.label("Name Format");
        name_format(ui, tool, formats);
        ui.end_row();
        ui.label("Manufacturer")
            .on_hover_hint("Who makes the bit, for the name format's {Manufacturer}; leave it empty if you don't know");
        ui.add(
            egui::TextEdit::singleline(&mut tool.manufacturer)
                .hint_text("e.g. Amana, Whiteside")
                .desired_width(f32::INFINITY),
        );
        ui.end_row();
        ui.label("Tool Type");
        ui.horizontal(|ui| {
            tool_type(ui, tool);
            // the tool's own units beside it, whatever the job's: a 6 mm end mill reads
            // 0.2362 in
            egui::ComboBox::from_id_salt("tool_units")
                .selected_text(if tool.units == Units::Inch {
                    "inches"
                } else {
                    "mm"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.units, Units::Mm, "mm");
                    ui.selectable_value(&mut tool.units, Units::Inch, "inches");
                })
                .response
                .on_hover_hint("Units: how this tool's lengths read; the tool stays the same size");
        });
        ui.end_row();
        // stands in for dragging a tool between groups
        ui.label("Group");
        egui::ComboBox::from_id_salt("tool_group")
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
        ui.label("Tool Number")
            .on_hover_hint("The post's [T] for this tool without a tool changer, and its holder for Load by Tool Number; 0 for none");
        // a tool in a tool changer is numbered by its slot: change the slot, not the number
        match loaded {
            Some((machine, slot)) => {
                ui.horizontal(|ui| {
                    ui.add_enabled(false, egui::DragValue::new(&mut tool.number));
                    ui.weak(format!("from slot T{slot} of {machine}'s tool changer"));
                });
            }
            None => {
                ui.add(egui::DragValue::new(&mut tool.number).range(0..=999));
            }
        }
        ui.end_row();
        form_row(ui, tool, look);
        ui.label("Notes");
        ui.add(
            egui::TextEdit::multiline(&mut tool.notes)
                .desired_rows(NOTES_ROWS)
                .desired_width(f32::INFINITY),
        );
        ui.end_row();
    });
    ui.separator();
    geometry(ui, tool);
}

/// The tool's shape: drawn from its sizes, each typed on its dimension line, then what the
/// drawing does not show (flutes, a thread mill's tooth).
fn geometry(ui: &mut egui::Ui, tool: &mut Tool) {
    crate::profile::editor(ui, tool);
    egui::Grid::new("tool_geometry")
        .num_columns(2)
        .show(ui, |ui| {
            if tool.shape == ToolShape::ThreadMill {
                thread_mill(ui, tool);
            }
            ui.label("Number of Flutes")
                .on_hover_hint("How many cutting edges the bit has, for the chip load");
            ui.add(egui::DragValue::new(&mut tool.flutes).range(1..=12));
            ui.end_row();
        });
}

/// A single-point thread mill's tooth: size, height (which changes the size, the angle
/// staying, as Aspire's form does) and offset.
fn thread_mill(ui: &mut egui::Ui, tool: &mut Tool) {
    ui.label("Tooth Size");
    length(ui, &mut tool.tooth_size, tool.units);
    ui.end_row();
    ui.label("Tooth Height");
    let mut height = tool.tooth_height();
    if length(ui, &mut height, tool.units) {
        tool.set_tooth_height(height);
    }
    ui.end_row();
    ui.label("Tooth Offset")
        .on_hover_hint("From the bottom of the tool up to the tip of the tooth");
    length(ui, &mut tool.tooth_offset, tool.units);
    ui.end_row();
}

fn cutting(ui: &mut egui::Ui, tool: &mut Tool) {
    egui::Grid::new("tool_cutting")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Pass Depth");
            length(ui, &mut tool.pass_depth, tool.units);
            ui.end_row();
            ui.label("Clearing Stepover").on_hover_hint(
                "Stepover is how far the tool moves sideways between passes, as a share of \
                 its width. Clearing (roughing) removes the bulk of the wood, in pockets, \
                 flat bottoms and 3D roughing: a big step is faster. 40% suits most end mills",
            );
            percent(ui, &mut tool.stepover);
            ui.end_row();
            ui.label("Finishing Stepover").on_hover_hint(
                "How far the tool moves sideways between finishing passes, which leave the \
                 surface you see: 3D finishing, V-carved edges, chamfers and mouldings. A \
                 smaller step leaves finer ridges but takes longer; 8-10% suits a ball nose",
            );
            ui.horizontal(|ui| {
                let mut finish = tool.finishing_stepover();
                if percent(ui, &mut finish) {
                    tool.finish_stepover = finish;
                }
                if tool.finish_stepover <= 0.0 {
                    ui.weak("same as clearing until changed");
                }
            });
            ui.end_row();
        });
}

/// A stepover (a fraction of the diameter) shown as a percentage; whether it changed.
fn percent(ui: &mut egui::Ui, fraction: &mut f64) -> bool {
    let mut percent = *fraction * 100.0;
    let changed = ui
        .add(
            egui::DragValue::new(&mut percent)
                .range(0.0..=100.0)
                .max_decimals(2)
                .suffix(" %"),
        )
        .changed();
    if changed {
        *fraction = percent / 100.0;
    }
    changed
}

fn feeds(ui: &mut egui::Ui, tool: &mut Tool) {
    egui::Grid::new("tool_feeds").num_columns(2).show(ui, |ui| {
        ui.label("Spindle Speed");
        ui.add(
            egui::DragValue::new(&mut tool.spindle_speed)
                .range(0.0..=f64::MAX)
                .suffix(" r.p.m."),
        );
        ui.end_row();
        // the rate units beside the feed rate, as Aspire's form has them
        ui.label("Feed Rate");
        ui.horizontal(|ui| {
            rate(ui, &mut tool.feed_rate, tool.rate_units);
            egui::ComboBox::from_id_salt("tool_rate_units")
                .selected_text(tool.rate_units.label())
                .show_ui(ui, |ui| {
                    for units in RateUnits::ALL {
                        ui.selectable_value(&mut tool.rate_units, units, units.label());
                    }
                })
                .response
                .on_hover_hint("Rate Units");
        });
        ui.end_row();
        ui.label("Plunge Rate");
        rate(ui, &mut tool.plunge_rate, tool.rate_units);
        ui.end_row();
    });
    // Aspire shows the feed per tooth, to check against the cutter maker's figure
    if let Some(load) = tool.chip_load() {
        let (per, unit, places) = if tool.units == Units::Inch {
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
            picture: &|_| None,
            profile: None,
        };
        let mut outcome = None;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        // no renderer takes a headless frame's texture changes
        ctx.run_pass(input, |ctx| outcome = manager.show(ctx, &look))
            .textures_delta
            .clear();
        outcome
    }

    #[test]
    fn a_tapered_ball_nose_is_a_ball_nose_with_a_taper_and_a_wider_shank() {
        let mut tool = Tool {
            shape: ToolShape::EndMill,
            diameter: 0.5,
            ..Tool::default()
        };
        make_tapered_ball(&mut tool);
        assert_eq!(tool.shape, ToolShape::BallNose);
        assert!(tapered_ball(&tool));
        assert!(tool.tapered().is_some(), "it cuts as a tapered tool");
        assert_eq!(picture_path(&tool), "tool_database/tapered_ball_nose.png");
        // its own taper and shank are kept
        tool.taper = 4.7;
        tool.shank_diameter = 6.0;
        make_tapered_ball(&mut tool);
        assert!((tool.taper - 4.7).abs() < 1e-9 && (tool.shank_diameter - 6.0).abs() < 1e-9);
    }

    #[test]
    fn drawing_each_tool_keeps_none_of_the_drawing_s_guesses() {
        let ctx = egui::Context::default();
        let db = ToolDatabase::starter();
        let mut m = Manager::new(&db);
        for i in 0..db.tools.len() {
            m.pick = Some(Pick::Tool(i));
            frame(&mut m, &ctx);
            frame(&mut m, &ctx);
        }
        // the starter tools give no flute or shank length; drawn, they are still unset
        assert_eq!(m.draft.tools, db.tools);
    }

    #[test]
    fn the_window_draws_groups_tools_and_holds_its_size() {
        let ctx = egui::Context::default();
        let mut m = Manager::new(&ToolDatabase::starter());
        frame(&mut m, &ctx);
        m.pick = Some(Pick::Tool(0));
        frame(&mut m, &ctx);
        let first = ctx.memory(|mem| mem.area_rect(tool_window()));
        assert!(first.is_some(), "the window is drawn");
        for _ in 0..3 {
            frame(&mut m, &ctx);
        }
        let later = ctx.memory(|mem| mem.area_rect(tool_window()));
        assert_eq!(first, later, "panels inside the window do not grow it");
        m.pick = Some(Pick::Group("Metric Tools".into()));
        assert_eq!(frame(&mut m, &ctx), None);
    }

    #[test]
    fn a_material_added_becomes_active_and_the_window_draws_its_data() {
        let ctx = egui::Context::default();
        let mut m = Manager::new(&ToolDatabase::starter());
        m.draft.add_material("Oak");
        m.new_material = " oak ".into();
        m.add_material();
        assert_eq!(
            (m.draft.materials.len(), m.draft.material.as_str()),
            (1, "Oak"),
            "the listed one, whatever the case"
        );
        assert_eq!(m.new_material, "");
        let thread_mill = m
            .draft
            .tools
            .iter()
            .position(|t| t.shape == ToolShape::ThreadMill)
            .unwrap();
        m.pick = Some(Pick::Tool(thread_mill));
        frame(&mut m, &ctx);
        assert!(
            m.draft.tools[thread_mill].materials.is_empty(),
            "only drawing it makes no data"
        );
    }

    #[test]
    fn a_material_is_duplicated_with_its_cutting_data_and_settings_copy_between_materials() {
        let mut m = Manager::new(&ToolDatabase::starter());
        m.draft.add_material("Oak");
        m.draft.material = "Oak".into();
        let mut oak = m.draft.tools[0].cutting("");
        oak.material = "Oak".into();
        oak.feed_rate = 777.0;
        m.draft.tools[0].set_cutting(oak);
        m.new_material = " Ash ".into();
        m.duplicate_material();
        assert_eq!(m.draft.materials, ["Oak", "Ash"]);
        assert_eq!(
            (m.draft.material.as_str(), m.new_material.as_str()),
            ("Ash", "")
        );
        assert!((m.draft.tools[0].cutting("Ash").feed_rate - 777.0).abs() < 1e-9);
        assert!(
            !m.draft.tools[1].has_data_for("Ash"),
            "no Oak data, none copied"
        );
        // without the data, named after the original, and made unique
        m.draft.material = "Oak".into();
        m.copy_cutting_data = false;
        m.duplicate_material();
        assert_eq!(m.draft.materials, ["Oak", "Ash", "Oak copy"]);
        assert!(!m.draft.tools[0].has_data_for("Oak copy"));
        m.draft.material = "Oak".into();
        m.duplicate_material();
        assert_eq!(m.draft.material, "Oak copy 2");
        // Copy Settings From another material, or from the default
        let tool = &mut m.draft.tools[0];
        tool.copy_cutting("Oak", "Oak copy");
        assert!((tool.cutting("Oak copy").feed_rate - 777.0).abs() < 1e-9);
        tool.copy_cutting("", "Oak");
        assert!((tool.cutting("Oak").feed_rate - tool.feed_rate).abs() < 1e-9);
    }

    /// Every piece of text one frame draws.
    fn drawn(manager: &mut Manager, ctx: &egui::Context) -> Vec<String> {
        let look = Look {
            picture: &|_| None,
            profile: None,
        };
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_pass(input, |ctx| {
            manager.show(ctx, &look);
        });
        output.textures_delta.clear();
        output
            .shapes
            .iter()
            .filter_map(|c| match &c.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_search_box_lists_the_tools_it_finds_by_name() {
        let ctx = egui::Context::default();
        let mut m = Manager::new(&ToolDatabase::starter());
        let all: Vec<String> = m.draft.tools.iter().map(|t| t.name.clone()).collect();
        let ball: Vec<&String> = all.iter().filter(|n| n.contains("Ball")).collect();
        assert!(!ball.is_empty() && ball.len() < all.len());
        m.search = "ball".into();
        drawn(&mut m, &ctx);
        let shown = drawn(&mut m, &ctx);
        for name in &all {
            assert_eq!(shown.contains(name), name.contains("Ball"), "{name}");
        }
        // a renamed tool is found by its new name
        m.draft.tools[0].name = "Zebra Cutter".into();
        m.search = "zeb".into();
        assert!(drawn(&mut m, &ctx).contains(&"Zebra Cutter".to_owned()));
    }

    #[test]
    fn tools_without_data_for_the_material_are_greyed_or_hidden() {
        let ctx = egui::Context::default();
        let mut m = Manager::new(&ToolDatabase::starter());
        m.draft.add_material("Oak");
        m.draft.material = "Oak".into();
        let mut oak = m.draft.tools[0].cutting("");
        oak.material = "Oak".into();
        m.draft.tools[0].set_cutting(oak);
        let (set, unset) = (m.draft.tools[0].name.clone(), m.draft.tools[1].name.clone());
        assert!(m.draft.tools[0].has_data_for("Oak") && !m.draft.tools[1].has_data_for("oak"));
        assert!(
            m.draft.tools[1].has_data_for(""),
            "the default always has data"
        );
        drawn(&mut m, &ctx);
        let shown = drawn(&mut m, &ctx);
        assert!(
            shown.contains(&set) && shown.contains(&unset),
            "greyed, still listed"
        );
        m.hide_unset = true;
        let shown = drawn(&mut m, &ctx);
        assert!(shown.contains(&set) && !shown.contains(&unset), "hidden");
    }

    #[test]
    fn edits_shown_for_a_material_become_its_data_and_leave_the_default() {
        let mut tool = Tool::default();
        let mut shown = tool.for_material("Oak");
        keep_cutting(&mut tool, &shown, "Oak");
        assert!(tool.materials.is_empty(), "nothing changed, nothing kept");
        shown.feed_rate = 600.0;
        shown.rate_units = RateUnits::InchPerMin;
        keep_cutting(&mut tool, &shown, "Oak");
        assert!((tool.cutting("Oak").feed_rate - 600.0).abs() < 1e-9);
        assert!((tool.feed_rate - 1000.0).abs() < 1e-9, "the default stays");
        assert_eq!(
            tool.rate_units,
            RateUnits::InchPerMin,
            "units are the tool's"
        );
        let mut default = tool.for_material("");
        default.plunge_rate = 50.0;
        keep_cutting(&mut tool, &default, "");
        assert!((tool.plunge_rate - 50.0).abs() < 1e-9);
        assert_eq!(tool.materials.len(), 1);
    }

    #[test]
    fn select_tool_opens_on_the_forms_tool_and_hands_back_the_one_shown() {
        let db = ToolDatabase::starter();
        let name = db.tools[1].name.clone();
        let mut m = Manager::selecting(&db, &name);
        assert!(m.selecting);
        assert_eq!(m.pick, Some(Pick::Tool(1)), "shown first");
        assert_eq!(m.select(), Some(Outcome::Select(name)));
        m.pick = Some(Pick::Tool(0));
        assert_eq!(m.select(), Some(Outcome::Select(db.tools[0].name.clone())));
        // a group shown, nothing to select
        m.pick = Some(Pick::Group(String::new()));
        assert_eq!(m.select(), None);
        assert_eq!(Manager::selecting(&db, "no such tool").pick, None);
        assert!(!Manager::new(&db).selecting);
    }

    #[test]
    fn tools_load_into_one_slot_each_and_follow_renames_and_deletes() {
        let mut changer = Changer {
            machine: "Router".into(),
            slots: vec![None; 5],
        };
        changer.load("6 mm End Mill", Some(2));
        changer.load("3 mm End Mill", Some(2));
        assert_eq!(
            changer.slot_of("3 mm End Mill"),
            Some(2),
            "it takes the slot"
        );
        assert_eq!(
            changer.slot_of("6 mm End Mill"),
            None,
            "what was there comes out"
        );
        changer.load("3 mm End Mill", Some(5));
        assert_eq!(
            (changer.slots[1].clone(), changer.slot_of("3 mm End Mill")),
            (None, Some(5))
        );
        changer.load("3 mm End Mill", Some(9));
        assert_eq!(
            changer.slot_of("3 mm End Mill"),
            None,
            "no such slot: unloaded"
        );
        let mut m = Manager::new(&ToolDatabase::starter());
        let name = m.draft.tools[0].name.clone();
        changer.load(&name, Some(1));
        m.changer = Some(changer);
        m.pick = Some(Pick::Tool(0));
        m.act(TreeAction::Delete);
        assert_eq!(
            m.changer.as_ref().unwrap().slots[0],
            None,
            "a deleted tool leaves its slot"
        );
    }

    #[test]
    fn a_tool_loaded_into_a_slot_takes_the_slot_as_its_number() {
        let mut m = Manager::new(&ToolDatabase::starter());
        m.changer = Some(Changer {
            machine: "Router".into(),
            slots: vec![None; 6],
        });
        m.draft.tools[1].number = 9;
        m.load_into(1, Some(4));
        assert_eq!(m.draft.tools[1].number, 4, "T4 holds it: tool 4");
        assert_eq!(
            m.changer.as_ref().unwrap().slot_of(&m.draft.tools[1].name),
            Some(4)
        );
        m.load_into(1, Some(2));
        assert_eq!(m.draft.tools[1].number, 2, "moved to T2");
        m.load_into(1, None);
        assert_eq!(
            m.draft.tools[1].number, 2,
            "taken out, it keeps its last number"
        );
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
