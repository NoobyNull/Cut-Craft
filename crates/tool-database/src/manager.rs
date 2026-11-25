//! The manager window, laid out as the Tool Database: the tool tree with its buttons
//! on the left, the picked group's or tool's settings on the right, OK / Cancel under both.
//! Edits are made to a copy; only Apply and OK hand it back.

use crate::tree;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use cut_craft_core::document::Units;
use cut_craft_core::toolpath::{CATALOGUES, CuttingData, RateUnits, Tool, ToolDatabase, ToolShape};
use eframe::egui;
use std::collections::BTreeMap;
mod form;
pub use form::*;

/// The window's share of the screen when it first opens.
/// The window's own id (its title's hashing is egui's).
fn tool_window() -> egui::Id {
    egui::Id::unique("tool_database_window")
}
const WINDOW_SHARE: f32 = 0.7;
/// The tree's share of the window's width when it first opens.
const TREE_SHARE: f32 = 0.35;
/// The lines of text the questions' area under the tree keeps room for.
const QUESTION_ROWS: f32 = 5.0;
/// The tool pictures are 90 × 130: their width for a height.
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
    /// Open the Materials Library (the edits so far kept, as Apply).
    MaterialsLibrary,
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "the window's own tick boxes and whether it is asking to close"
)]
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
    /// What a toolpath form's Select… may pick: `Some(true)` only lasers (a laser
    /// toolpath's), `Some(false)` only bits; `None` every tool.
    pub lasers: Option<bool>,
    index: crate::search::SearchIndex,
    /// The Materials Library: its materials are offered as the active material, and a
    /// bit's feeds for their class are placed by their hardness.
    pub library: cut_craft_core::material_library::MaterialLibrary,
    /// What a button inside the window asked for this frame (Materials Library…).
    pending: Option<Outcome>,
    /// The tools renamed since it opened, first name to the name now.
    renames: Vec<(String, String)>,
    /// The picked tool and the maker ▸ type group it had last drawn: while its group is
    /// still that one, the group follows its maker and type.
    followed: Option<(usize, String)>,
    /// What the window is asking before it goes on, if anything.
    asking: Option<Question>,
    /// The database as last kept (opened, or Apply): what the edits since would delete.
    saved: ToolDatabase,
    /// The tool changer as last kept (the app loads it after opening, then calls
    /// [`Self::applied`]): edits to it wait for Apply too.
    saved_changer: Option<Changer>,
    /// Closed with changes not applied: it asks Apply or Discard first.
    closing: bool,
    /// The tool each of the open job's toolpaths cuts with, one name per toolpath: Delete
    /// says how many of them cut with the tools it would take.
    pub job_tools: Vec<String>,
    /// Where the tree was last drawn: a question showing never moves it.
    #[cfg(test)]
    tree_rect: egui::Rect,
}

/// Apply at the right, greyed out until something changes, with whether changes wait
/// beside it. Whether Apply was pressed.
fn apply_row(ui: &mut egui::Ui, dirty: bool) -> bool {
    use cut_craft_bug_report::unapplied;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let pressed = unapplied::apply_button(ui, dirty);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            unapplied::note(ui, dirty);
        });
        pressed
    })
    .inner
}

/// A question the window asks before it goes on.
#[derive(Clone, Debug, PartialEq)]
enum Question {
    /// Delete on this group, waiting for Delete or Keep under the tree.
    Delete(String),
    /// Materials Library…, waiting for Keep and Open or Not Now, as opening it would keep
    /// deletions that Cancel could then not bring back.
    Library,
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
        ToolShape::EndMill | ToolShape::SurfacingBit => "tool_database/end_mill.png",
        ToolShape::BallNose => "tool_database/ball_nose.png",
        ToolShape::VBit => "tool_database/v_bit.png",
        ToolShape::RadiusedEndMill => "tool_database/radiused_end_mill.png",
        // ponytail: no thread mill or form tool picture is drawn yet, so none shows
        ToolShape::ThreadMill => "tool_database/thread_mill.png",
        ToolShape::FormTool => "tool_database/form_tool.png",
        // ponytail: no laser picture is drawn yet, so none shows
        ToolShape::Laser => "tool_database/laser.png",
    }
}

/// The Tool Type list: each type's trade name with what woodworkers call it, so either
/// one finds it.
const SHAPES: [(ToolShape, &str); 8] = [
    (ToolShape::EndMill, "End Mill (flat / straight bit)"),
    (ToolShape::BallNose, "Ball Nose (core box)"),
    (ToolShape::VBit, "V-Bit (engraving / chamfer)"),
    (
        ToolShape::RadiusedEndMill,
        "Radiused End Mill (bowl / bull nose)",
    ),
    (ToolShape::ThreadMill, "Thread Mill (cuts screw threads)"),
    (ToolShape::FormTool, "Form Tool (custom profile)"),
    (
        ToolShape::SurfacingBit,
        "Surfacing Bit (spoilboard bit / fly cutter)",
    ),
    (ToolShape::Laser, "Laser (for the laser toolpaths)"),
];
const TAPERED_BALL: &str = "Tapered Ball Nose (cone ball nose)";

/// A tapered ball nose (a TBN or cone ball nose): a ball nose whose sides open out to its
/// shank at its taper angle. The editor lists it as a type of its own.
fn tapered_ball(tool: &Tool) -> bool {
    tool.shape == ToolShape::BallNose && tool.taper > 0.0
}

/// The Tool Type list: the shapes, with Tapered Ball Nose under Ball Nose.
fn tool_type(ui: &mut egui::Ui, tool: &mut Tool) {
    let tapered = tapered_ball(tool);
    let shown = if tapered {
        TAPERED_BALL
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
                    && ui.selectable_label(tapered, TAPERED_BALL).clicked()
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

/// The Delete button's hover hint.
const DELETE_HINT: &str = "Delete the picked tool, or the picked group with every tool and \
                           group in it (it asks first, counting tools the tree hides)";

/// What a tree button asked for.
#[derive(Clone, Copy, PartialEq)]
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
            lasers: None,
            index: crate::search::SearchIndex::default(),
            library: cut_craft_core::material_library::MaterialLibrary::starter(),
            pending: None,
            followed: None,
            renames: vec![],
            asking: None,
            saved: db.clone(),
            saved_changer: None,
            closing: false,
            job_tools: vec![],
            #[cfg(test)]
            tree_rect: egui::Rect::NOTHING,
        }
    }

    /// The tool named `before` is now `name`.
    pub fn note_rename(&mut self, before: &str, name: &str) {
        if before == name {
            return;
        }
        match self.renames.iter_mut().find(|(_, now)| now == before) {
            Some((_, now)) => name.clone_into(now),
            None => self.renames.push((before.to_owned(), name.to_owned())),
        }
    }

    /// The tools renamed since it opened or this was last asked, as (first name, name
    /// now); the open job's toolpaths follow them, so they still find the tool's tool
    /// changer slot.
    pub fn take_renamed(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.renames)
            .into_iter()
            .filter(|(first, now)| first != now)
            .collect()
    }

    /// Opens on a copy of `db` to pick a toolpath's tool, on `current` when it is listed.
    pub fn selecting(db: &ToolDatabase, current: &str) -> Self {
        let mut manager = Self::new(db);
        manager.selecting = true;
        manager.pick = db.index_of(current).map(Pick::Tool);
        manager
    }

    /// Whether `tool` is one the form asking may pick ([`Self::lasers`]).
    fn may_pick(&self, tool: &Tool) -> bool {
        self.lasers.is_none_or(|lasers| tool.is_laser() == lasers)
    }

    /// The name of the tool shown, when one is.
    pub fn shown_tool(&self) -> Option<&str> {
        match self.pick {
            Some(Pick::Tool(i)) => self.draft.tools.get(i).map(|t| t.name.as_str()),
            _ => None,
        }
    }

    /// Whether the tool shown is a read-only catalogue's.
    fn shown_is_catalogue(&self) -> bool {
        match self.pick {
            Some(Pick::Tool(i)) => self
                .draft
                .tools
                .get(i)
                .is_some_and(|t| self.draft.is_catalogue(t)),
            _ => false,
        }
    }

    /// Copies the catalogue tool shown into My Tools and shows the copy.
    fn copy_shown_to_mine(&mut self) {
        if let Some(Pick::Tool(i)) = self.pick
            && self.shown_is_catalogue()
        {
            self.pick = Some(Pick::Tool(tree::copy_to_mine(&mut self.draft, i)));
        }
    }

    /// Select: what the window asks for, when a tool is shown. A catalogue's tool is
    /// copied into My Tools first, toolpaths cutting with My Tools.
    pub fn select(&mut self) -> Option<Outcome> {
        self.copy_shown_to_mine();
        self.shown_tool()
            .map(|name| Outcome::Select(name.to_owned()))
    }

    /// Loads tool `i` into the tool changer's `slot` (counted from 1; `None` takes it out),
    /// the slot becoming its tool number so the number shown is the holder it is in.
    pub fn load_into(&mut self, i: usize, slot: Option<usize>) {
        let (Some(changer), Some(tool)) = (&mut self.changer, self.draft.tools.get(i)) else {
            return;
        };
        // a tool changer holds My Tools only
        if self.draft.is_catalogue(tool) {
            return;
        }
        changer.load(&tool.name, slot);
        self.draft.number_by_slots(&changer.slots);
    }

    /// The Duplicate Material: the active material copied under the typed name, or
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

    /// The Add Material: the typed name joins the list and becomes active (the one
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
    /// A bit the file lists once per material becomes one bit
    /// ([`ToolDatabase::join_material_copies`]); what was not kept, if anything.
    pub fn import(&mut self, incoming: ToolDatabase, name: &str) -> Vec<String> {
        let group = tree::import(&mut self.draft, incoming, name);
        let report = self.draft.join_material_copies();
        self.pick = Some(Pick::Group(group));
        report
    }

    /// There are edits Apply or OK hasn't kept yet.
    pub fn dirty(&self) -> bool {
        self.draft != self.saved || self.saved_changer != self.changer
    }

    /// Its close box pressed: closes as Cancel, or asks first while changes wait.
    fn close_box(&mut self, dirty: bool) -> Option<Outcome> {
        if dirty {
            self.closing = true;
            None
        } else {
            Some(Outcome::Cancel)
        }
    }

    /// Closed with changes waiting: asks Apply or Discard; what the answer asks of the app.
    fn close_question(&mut self, ctx: &egui::Context) -> Option<Outcome> {
        use cut_craft_bug_report::unapplied::{Answer, question};
        if !self.closing {
            return None;
        }
        match question(ctx, "tool_database_apply_question")? {
            Answer::ApplyAndClose => Some(Outcome::Ok),
            Answer::Discard => Some(Outcome::Cancel),
            Answer::KeepEditing => {
                self.closing = false;
                None
            }
        }
    }

    /// Draws the window; what it asks of the app, if anything.
    pub fn show(&mut self, ctx: &egui::Context, look: &Look) -> Option<Outcome> {
        use cut_craft_bug_report::unapplied;
        let mut open = true;
        let mut outcome = None;
        let dirty = self.dirty();
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
                unapplied::hold_close(ui, dirty);
                outcome = self.close_question(ui.ctx());
                egui::Panel::bottom("tool_database_buttons").show(ui, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(cut_craft_i18n::t!("Cancel")).clicked() {
                            outcome = Some(Outcome::Cancel);
                        }
                        if !self.selecting {
                            if ui.button(cut_craft_i18n::t!("OK")).clicked() {
                                outcome = Some(Outcome::Ok);
                            }
                            return;
                        }
                        let text = if self.shown_is_catalogue() {
                            "Copy to My Tools and Select"
                        } else {
                            "Select"
                        };
                        let select = ui
                            .add_enabled(self.shown_tool().is_some(), egui::Button::new(text))
                            .on_hover_hint(cut_craft_i18n::t!(
                                "Cut with the tool shown, with its saved feeds, speeds and \
                                 pass depth (any edits made here are kept); a catalogue's \
                                 tool is copied into My Tools first"
                            ))
                            .on_disabled_hover_text(cut_craft_i18n::t!(
                                "Pick a tool in the list first"
                            ));
                        if select.clicked() {
                            outcome = self.select();
                        }
                        match self.shown_tool() {
                            Some(name) => ui.label(name),
                            None => ui.weak(cut_craft_i18n::t!("Pick a tool in the list")),
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
                        // the questions' own area, kept whether one shows or not, so one
                        // showing never moves the tree or the buttons
                        let rows = ui.text_style_height(&egui::TextStyle::Body) * QUESTION_ROWS;
                        let height =
                            rows + ui.spacing().interact_size.y + ui.spacing().item_spacing.y * 2.0;
                        egui::Panel::bottom("tool_database_question")
                            .resizable(false)
                            .exact_size(height)
                            .show(ui, |ui| self.question(ui));
                        #[cfg(test)]
                        {
                            self.tree_rect = ui.available_rect_before_wrap();
                        }
                        egui::ScrollArea::both().show(ui, |ui| self.tree(ui, look));
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    egui::Panel::bottom("tool_database_apply").show(ui, |ui| {
                        if apply_row(ui, dirty) {
                            outcome = Some(Outcome::Apply);
                        }
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| self.details(ui, look));
                });
            },
        );
        // the close box drops the edits only once asked, while some wait
        if !open {
            outcome = self.close_box(dirty);
        }
        if let Some(asked) = self.pending.take() {
            outcome = outcome.or(Some(asked));
        }
        if matches!(outcome, Some(Outcome::Apply)) {
            self.applied();
        }
        outcome
    }

    /// The edits so far are kept (Apply): Cancel goes back to here from now on.
    pub fn applied(&mut self) {
        self.saved = self.draft.clone();
        self.saved_changer.clone_from(&self.changer);
        self.closing = false;
    }

    /// Materials Library…: opens it, keeping the edits so far as Apply does; when they
    /// delete tools or materials, it asks first, as Cancel could not bring those back.
    pub fn open_materials_library(&mut self) {
        if self.deletions().is_empty() {
            self.pending = Some(Outcome::MaterialsLibrary);
        } else {
            self.asking = Some(Question::Library);
        }
    }

    /// Keep and Open (`yes`) or Not Now, for Materials Library….
    pub fn answer_library(&mut self, yes: bool) {
        if self.asking == Some(Question::Library) {
            self.asking = None;
        }
        if yes {
            self.pending = Some(Outcome::MaterialsLibrary);
        }
    }

    /// What deleting since the last Apply took: "2 deleted tools (A, B)", and so on. A tool
    /// renamed is not deleted.
    fn deletions(&self) -> Vec<String> {
        let renamed = |name: &str| {
            self.renames
                .iter()
                .any(|(first, now)| first == name && self.draft.get(now).is_some())
        };
        let tools: Vec<&str> = self
            .saved
            .tools
            .iter()
            .filter(|t| self.draft.get(&t.name).is_none() && !renamed(&t.name))
            .map(|t| t.name.as_str())
            .collect();
        let materials: Vec<&str> = self
            .saved
            .materials
            .iter()
            .filter(|m| {
                !self
                    .draft
                    .materials
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(m))
            })
            .map(String::as_str)
            .collect();
        let named = |names: &[&str], one: &str| {
            const SHOWN: usize = 3;
            let mut list = names[..names.len().min(SHOWN)].join(", ");
            if names.len() > SHOWN {
                list.push_str(", …");
            }
            match names.len() {
                1 => format!("1 deleted {one} ({list})"),
                n => format!("{n} deleted {one}s ({list})"),
            }
        };
        let mut out = vec![];
        if !tools.is_empty() {
            out.push(named(&tools, "tool"));
        }
        if !materials.is_empty() {
            out.push(named(&materials, "material"));
        }
        out
    }

    /// What Materials Library… asks while it waits for an answer.
    pub fn library_question(&self) -> Option<String> {
        let deletions = self.deletions();
        if self.asking != Some(Question::Library) || deletions.is_empty() {
            return None;
        }
        Some(format!(
            "Opening the Materials Library keeps your changes here first, as Apply does, so \
             Cancel can no longer bring back {}. Open it now?",
            deletions.join(" and ")
        ))
    }

    /// The tree: groups that open and close, each over its groups and tools.
    fn tree(&mut self, ui: &mut egui::Ui, look: &Look) {
        cut_craft_bug_report::mark(ui, "Tool Database tree");
        self.changer_list(ui);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(cut_craft_i18n::t!("Search tool names"))
                .desired_width(f32::INFINITY),
        );
        if self.search.trim().is_empty() {
            let all = tree::groups(&self.draft);
            ui.strong(cut_craft_i18n::t!("My Tools"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "Your own tools: the ones toolpaths and the tool changer use"
                ));
            for sub in tree::subgroups(&all, "") {
                if sub != CATALOGUES {
                    self.group_row(ui, look, &all, sub);
                }
            }
            self.tool_rows(ui, look, "");
            if all.iter().any(|g| g == CATALOGUES) {
                ui.separator();
                self.group_row(ui, look, &all, CATALOGUES);
            }
        } else {
            self.found(ui, look);
        }
    }

    /// The tools the search finds, each with its group, instead of the tree; a click picks one.
    fn found(&mut self, ui: &mut egui::Ui, look: &Look) {
        if self.index.is_stale(&self.draft) {
            self.index = crate::search::SearchIndex::new(&self.draft);
        }
        let mut found = self.index.find(&self.search);
        found.retain(|&i| self.may_pick(&self.draft.tools[i]));
        if found.is_empty() {
            ui.weak(cut_craft_i18n::t!("No tool has that name"));
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
        egui::CollapsingHeader::new(cut_craft_i18n::tr!(
            "tool-database-tool-changer",
            machine = changer.machine.clone()
        ))
        .default_open(true)
        .show(ui, |ui| {
            for (i, slot) in changer.slots.iter().enumerate() {
                let text = format!("T{}  {}", i + 1, slot.as_deref().unwrap_or("(empty)"));
                let at = slot.as_deref().and_then(|t| self.draft.index_of(t));
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
        for sub in tree::subgroups(all, group) {
            self.group_row(ui, look, all, sub);
        }
        self.tool_rows(ui, look, group);
    }

    /// The group `sub`, opening onto its groups and tools.
    fn group_row(&mut self, ui: &mut egui::Ui, look: &Look, all: &[String], sub: &str) {
        let row = ui.spacing().interact_size.y;
        let id = ui.make_persistent_id(("tool_group", sub));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true)
            .show_header(ui, |ui| {
                if let Some(image) = (look.picture)("common/tool_database.png") {
                    ui.add(image.fit_to_exact_size(egui::vec2(row, row)));
                }
                let picked = self.pick == Some(Pick::Group(sub.to_owned()));
                let text = if sub == CATALOGUES {
                    "Catalogues (read only)"
                } else {
                    tree::leaf(sub)
                };
                let label = ui.selectable_label(picked, egui::RichText::new(text).strong());
                if sub == CATALOGUES {
                    label.clone().on_hover_hint(cut_craft_i18n::t!(
                        "Tool databases imported from makers, kept as they came: copy a tool \
                         into My Tools to edit it, cut with it or load it in the tool changer"
                    ));
                }
                cut_craft_bug_report::context_menu(&label, |ui| {
                    ui.checkbox(&mut self.hide_unset, cut_craft_i18n::t!("Hide Unset Tools"));
                });
                if label.clicked() {
                    self.pick = Some(Pick::Group(sub.to_owned()));
                }
            })
            .body(|ui| self.branch(ui, look, all, sub));
    }

    /// The tools straight inside `group`.
    fn tool_rows(&mut self, ui: &mut egui::Ui, look: &Look, group: &str) {
        let row = ui.spacing().interact_size.y;
        for i in tree::tools_in(&self.draft, group) {
            let tool = &self.draft.tools[i];
            let unset = !tool.has_data_for(&self.draft.material);
            if (unset && self.hide_unset) || !self.may_pick(tool) {
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
                    label.clone().on_hover_hint(cut_craft_i18n::tr!(
                        "tool-database-no-cutting-data",
                        material = self.draft.material.clone()
                    ));
                }
                cut_craft_bug_report::context_menu(&label, |ui| {
                    ui.checkbox(&mut self.hide_unset, cut_craft_i18n::t!("Hide Unset Tools"));
                });
                if label.clicked() {
                    self.pick = Some(Pick::Tool(i));
                }
                if let Some(n) = self.changer.as_ref().and_then(|c| c.slot_of(&tool.name)) {
                    ui.weak(format!("T{n}"))
                        .on_hover_hint(cut_craft_i18n::t!("Loaded in the tool changer"));
                }
            });
        }
    }

    /// The row under the tree: new tool, new group, copy, import, export, delete.
    fn tree_buttons(&self, ui: &mut egui::Ui, look: &Look) -> Option<TreeAction> {
        let edge = ui.spacing().interact_size.y;
        let picked = self.pick.is_some();
        let tool = matches!(self.pick, Some(Pick::Tool(_)));
        let deletable = self.deletable();
        let copy = if self.shown_is_catalogue() {
            "Copy to My Tools"
        } else {
            "Copy Tool"
        };
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
                ("common/copy_duplicate.png", copy, tool, TreeAction::Copy),
                ("common/open.png", "Import Tools", true, TreeAction::Import),
                (
                    "common/save.png",
                    "Export Tools",
                    picked,
                    TreeAction::Export,
                ),
                ("common/delete.png", "Delete", deletable, TreeAction::Delete),
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
                    .on_hover_hint(if asked == TreeAction::Delete {
                        DELETE_HINT
                    } else {
                        label
                    })
                    .clicked()
                {
                    action = Some(asked);
                }
            }
        });
        action
    }

    /// Whether the pick can be deleted: anything in My Tools, or a whole catalogue (not
    /// a catalogue's tools or groups one by one).
    fn deletable(&self) -> bool {
        match &self.pick {
            Some(Pick::Tool(_)) => !self.shown_is_catalogue(),
            Some(Pick::Group(g)) => {
                !self.draft.is_catalogue_group(g) || self.draft.catalogues.contains(g)
            }
            None => false,
        }
    }

    /// Whether `group` is the catalogues' top group or inside it.
    fn in_catalogues(&self, group: &str) -> bool {
        tree::within(group, CATALOGUES) || self.draft.is_catalogue_group(group)
    }

    fn act(&mut self, action: TreeAction) -> Option<Outcome> {
        // new things go in the picked group, or the picked tool's; in My Tools, a new
        // tool picked in a catalogue going in its maker ▸ type group
        let group = match &self.pick {
            Some(Pick::Group(g)) => g.clone(),
            Some(Pick::Tool(i)) => self.draft.tools[*i].group.clone(),
            None => String::new(),
        };
        let group = match action {
            TreeAction::NewTool if self.in_catalogues(&group) => Tool::default().own_group(),
            TreeAction::NewGroup if self.in_catalogues(&group) => String::new(),
            _ => group,
        };
        if action == TreeAction::Delete && !self.deletable() {
            return None;
        }
        // a whole group asks first: it can hold tools the tree hides
        if let (TreeAction::Delete, Some(Pick::Group(g))) = (action, &self.pick) {
            self.asking = Some(Question::Delete(g.clone()));
            return None;
        }
        let outcome = self.do_action(action, &group);
        // a tool deleted leaves its slot
        if let Some(changer) = &mut self.changer {
            changer.unload_missing(&self.draft);
        }
        outcome
    }

    /// What Delete asks before a group goes, while it waits for an answer: the group, how
    /// many tools and groups go with it, and how many of those the tree hides or the tool
    /// changer holds.
    pub fn delete_question(&self) -> Option<String> {
        let Some(Question::Delete(group)) = &self.asking else {
            return None;
        };
        let inside: Vec<&Tool> = self
            .draft
            .tools
            .iter()
            .filter(|t| tree::within(&t.group, group))
            .collect();
        let hidden = inside
            .iter()
            .filter(|t| self.hide_unset && !t.has_data_for(&self.draft.material))
            .count();
        let loaded = inside
            .iter()
            .filter(|t| {
                self.changer
                    .as_ref()
                    .and_then(|c| c.slot_of(&t.name))
                    .is_some()
            })
            .count();
        let groups = tree::groups(&self.draft)
            .iter()
            .filter(|g| g.as_str() != group && tree::within(g, group))
            .count();
        let plural = |n: usize, one: &str| {
            if n == 1 {
                format!("1 {one}")
            } else {
                format!("{n} {one}s")
            }
        };
        let mut what = plural(inside.len(), "tool");
        if groups > 0 {
            what = format!("{what} in {}", plural(groups, "group"));
        }
        let mut notes = vec![];
        if hidden > 0 {
            notes.push(format!("{hidden} hidden by Hide Unset Tools"));
        }
        if loaded > 0 {
            notes.push(format!(
                "{loaded} loaded in the tool changer, emptying their slots"
            ));
        }
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" ({})", notes.join("; "))
        };
        let cutting = self
            .job_tools
            .iter()
            .filter(|name| inside.iter().any(|t| t.name == **name))
            .count();
        let toolpaths = match cutting {
            0 => String::new(),
            n => format!(
                " {} in this job cut with them: they keep their tool, but it can no longer \
                 be picked here or loaded in the tool changer.",
                plural(n, "toolpath")
            ),
        };
        Some(format!(
            "Delete the group \"{}\" and its {what}{notes}?{toolpaths} Cancel still brings \
             them back until OK.",
            tree::leaf(group)
        ))
    }

    /// Delete (`yes`) or Keep, for the group Delete asked about.
    pub fn answer_delete(&mut self, yes: bool) {
        let Some(Question::Delete(group)) = self.asking.take() else {
            return;
        };
        if yes {
            tree::delete_group(&mut self.draft, &group);
            self.pick = None;
            if let Some(changer) = &mut self.changer {
                changer.unload_missing(&self.draft);
            }
        }
    }

    /// The question the window is asking, in its own area under the tree: Delete's, with
    /// Delete and Keep (picking something else drops it), or Materials Library…'s, with Keep
    /// and Open and Not Now. The text scrolls when it is longer than the area.
    fn question(&mut self, ui: &mut egui::Ui) {
        if let Some(Question::Delete(group)) = &self.asking
            && self.pick.as_ref() != Some(&Pick::Group(group.clone()))
        {
            self.asking = None;
        }
        let (question, yes, no) = if let Some(q) = self.delete_question() {
            (q, "Delete", "Keep")
        } else if let Some(q) = self.library_question() {
            (q, "Keep and Open", "Not Now")
        } else {
            return;
        };
        let room =
            ui.available_height() - ui.spacing().interact_size.y - ui.spacing().item_spacing.y;
        egui::ScrollArea::vertical()
            .max_height(room.max(0.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.colored_label(ui.visuals().warn_fg_color, question)
            });
        let delete = matches!(self.asking, Some(Question::Delete(_)));
        ui.horizontal(|ui| {
            let answer = if ui.button(yes).clicked() {
                Some(true)
            } else if ui.button(no).clicked() {
                Some(false)
            } else {
                None
            };
            match answer {
                Some(a) if delete => self.answer_delete(a),
                Some(a) => self.answer_library(a),
                None => {}
            }
        });
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
            TreeAction::Copy if self.shown_is_catalogue() => self.copy_shown_to_mine(),
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
            Some(Pick::Tool(i)) if i < self.draft.tools.len() && self.shown_is_catalogue() => {
                self.catalogue_tool(ui, look, i);
            }
            Some(Pick::Tool(i)) if i < self.draft.tools.len() => {
                self.follow_own_group(i);
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
                let taken = sharing_number(tools, i);
                tool_details(
                    ui,
                    &mut tools[i],
                    (&groups, name_formats),
                    (look, loaded, &taken),
                    &material,
                    Some(&self.library),
                );
                let name = self.draft.tools[i].name.clone();
                self.note_rename(&before, &name);
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
                ui.weak(cut_craft_i18n::t!("Pick a group or tool in the tree"));
            }
        }
    }

    /// The Material combo and Material Management in one row: the active material,
    /// whose cutting data tools show and toolpaths get; adding one, and deleting the active
    /// one with every tool's data for it.
    // ponytail: no renaming materials, and no machine combo (the data is per material only)
    fn materials(&mut self, ui: &mut egui::Ui) {
        const DEFAULT: &str = "(default)";
        ui.horizontal_wrapped(|ui| {
            ui.label(cut_craft_i18n::t!("Material"));
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
                    // the library's, by class: picking one and changing the feeds shown
                    // gives the bit feeds of its own for it
                    for class in cut_craft_core::material_class::MaterialClass::ALL {
                        let listed = self.library.in_class(class);
                        let new: Vec<_> = listed
                            .iter()
                            .filter(|m| {
                                !self
                                    .draft
                                    .materials
                                    .iter()
                                    .any(|d| d.eq_ignore_ascii_case(&m.name))
                            })
                            .collect();
                        if new.is_empty() {
                            continue;
                        }
                        ui.separator();
                        ui.weak(class.name());
                        for m in new {
                            ui.selectable_value(active, m.name.clone(), &m.name);
                        }
                    }
                });
            if ui
                .button(cut_craft_i18n::t!("Materials Library…"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "The materials by class and hardness, and each one's bits with their \
                     feeds. Your changes here are kept first, as Apply does (it asks first \
                     when they delete tools or materials)"
                ))
                .clicked()
            {
                self.open_materials_library();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.new_material)
                    .hint_text(cut_craft_i18n::t!("New material")),
            );
            let typed = !self.new_material.trim().is_empty();
            if ui
                .add_enabled(typed, egui::Button::new(cut_craft_i18n::t!("Add Material")))
                .clicked()
            {
                self.add_material();
            }
            if ui
                .add_enabled(
                    !self.draft.material.is_empty(),
                    egui::Button::new(cut_craft_i18n::t!("Delete Material")),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "Deletes every tool's cutting data for it"
                ))
                .clicked()
            {
                let gone = self.draft.material.clone();
                self.draft.delete_material(&gone);
            }
            if ui
                .add_enabled(
                    !self.draft.material.is_empty(),
                    egui::Button::new(cut_craft_i18n::t!("Duplicate Material")),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "A copy of the active material, named as typed (or \"… copy\")"
                ))
                .clicked()
            {
                self.duplicate_material();
            }
            ui.checkbox(
                &mut self.copy_cutting_data,
                cut_craft_i18n::t!("with cutting data"),
            )
            .on_hover_hint(cut_craft_i18n::t!(
                "The copy gets every tool's cutting data for the material"
            ));
        });
    }

    /// A catalogue's tool, read only, with Copy to My Tools.
    fn catalogue_tool(&mut self, ui: &mut egui::Ui, look: &Look, i: usize) {
        let mut copy = false;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(cut_craft_i18n::t!(
                    "This tool is in a maker's catalogue, which stays as it came. Copy it into \
                     My Tools to change it, cut with it or load it in the tool changer."
                ));
                copy = ui.button(cut_craft_i18n::t!("Copy to My Tools")).clicked();
            });
        });
        let groups = tree::groups(&self.draft);
        let material = self.draft.material.clone();
        // drawn from a copy: nothing typed here reaches the catalogue
        let mut shown = self.draft.tools[i].clone();
        let mut formats = self.draft.name_formats.clone();
        ui.add_enabled_ui(false, |ui| {
            tool_details(
                ui,
                &mut shown,
                (&groups, &mut formats),
                (look, None, &[]),
                &material,
                Some(&self.library),
            );
        });
        if copy {
            self.copy_shown_to_mine();
        }
    }

    /// Copies every catalogue tool in `group` into My Tools (each once).
    pub fn copy_group_to_mine(&mut self, group: &str) {
        let inside: Vec<usize> = (0..self.draft.tools.len())
            .filter(|&i| tree::within(&self.draft.tools[i].group, group))
            .collect();
        for i in inside {
            tree::copy_to_mine(&mut self.draft, i);
        }
    }

    /// A My Tools tool's group follows its maker and type while it is still the group they
    /// gave it (the user moving it to another group keeps it there).
    fn follow_own_group(&mut self, i: usize) {
        let tool = &mut self.draft.tools[i];
        let own = tool.own_group();
        if let Some((at, last)) = &self.followed
            && *at == i
            && tool.group == *last
        {
            tool.group.clone_from(&own);
        }
        self.followed = Some((i, own));
    }

    fn group_details(&mut self, ui: &mut egui::Ui, group: &str) {
        if self.in_catalogues(group) {
            ui.label(egui::RichText::new(tree::leaf(group)).strong());
            ui.weak(cut_craft_i18n::t!(
                "A maker's catalogue, kept as it came: copy its tools into My Tools to edit \
                 them. Delete takes a whole catalogue out (copies in My Tools stay)."
            ));
            if let Some(notes) = self.draft.groups.iter().find(|g| g.path == group)
                && !notes.notes.is_empty()
            {
                ui.label(&notes.notes);
            }
            if ui
                .button(cut_craft_i18n::t!("Copy All to My Tools"))
                .on_hover_hint(cut_craft_i18n::t!(
                    "Every tool in this group, each into its maker ▸ type group"
                ))
                .clicked()
            {
                self.copy_group_to_mine(group);
            }
            return;
        }
        egui::Grid::new("tool_group_details")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label(cut_craft_i18n::t!("Name"));
                let mut name = tree::leaf(group).to_owned();
                let edit =
                    ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
                // a group's name is its path's last level, so an empty or `/` name is refused
                if edit.changed() && !name.is_empty() && !name.contains('/') {
                    let renamed = tree::rename_group(&mut self.draft, group, &name);
                    self.pick = Some(Pick::Group(renamed));
                }
                ui.end_row();
                ui.label(cut_craft_i18n::t!("Notes"));
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

#[cfg(test)]
mod tests;
