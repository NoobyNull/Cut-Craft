//! First-time setup: three questions asked on the first run (which machine, millimetres or
//! inches, the material usually cut) that add the machine, set the job new files start with
//! and put the starter tools in those units first. Cut Craft's own: Aspire has none (see
//! `re/SPEC.md`, First-time setup). Help ▸ Run First-Time Setup Again asks them again.

use super::CutCraftApp;
use cut_craft_core::document::{Job, Units};
use cut_craft_core::toolpath::ToolDatabase;
use cut_craft_machines::{Controller, Machine, Machines};
use eframe::egui;

const MM_PER_INCH: f64 = 25.4;
/// The thickness offered to start with: 18 mm, or 3/4 inch (common plywood and MDF).
const THICKNESS_MM: f64 = 18.0;
const THICKNESS_INCH: f64 = 0.75 * MM_PER_INCH;
/// The starter tool groups, by units.
const METRIC_TOOLS: &str = "Metric Tools";
const IMPERIAL_TOOLS: &str = "Imperial Tools";
/// What a machine not in the list is called.
const MY_MACHINE: &str = "My Machine";

/// Whether to ask: never asked (nor skipped) before, with no machine and no default job kept,
/// so someone who set Cut Craft up before this setup existed is not asked.
pub(super) fn wanted(options: &crate::options::Options, machines: &Machines) -> bool {
    !options.first_run_done && machines.machines.is_empty() && options.default_job.is_none()
}

/// The machine's maker, as picked.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Maker {
    Unpicked,
    Named(String),
    /// "Mine is not listed": the bed and controller are asked for instead.
    NotListed,
}

/// The answers so far.
#[derive(Clone, Debug)]
pub(super) struct Setup {
    /// The built-in machines to pick from.
    pub catalogue: Vec<Machine>,
    pub maker: Maker,
    /// The machine picked from the catalogue, by index.
    pub model: Option<usize>,
    /// A machine not listed: its working area, mm, and controller.
    pub bed: [f64; 2],
    pub controller: Controller,
    pub inches: bool,
    /// The material usually cut: width, length and thickness, mm.
    pub sheet: [f64; 3],
}

impl Setup {
    pub fn new() -> Self {
        let job = Job::default();
        Self {
            catalogue: cut_craft_machines::catalogue(),
            maker: Maker::Unpicked,
            model: None,
            bed: [job.width, job.height],
            controller: Controller::default(),
            inches: false,
            sheet: [job.width, job.height, THICKNESS_MM],
        }
    }

    /// The models of the maker picked, by catalogue index.
    pub fn models(&self) -> Vec<usize> {
        match &self.maker {
            Maker::Named(maker) => (0..self.catalogue.len())
                .filter(|&i| &self.catalogue[i].manufacturer == maker)
                .collect(),
            _ => vec![],
        }
    }

    /// Picks a maker; one with a single model has it picked too.
    pub fn pick_maker(&mut self, maker: Maker) {
        self.maker = maker;
        self.model = None;
        match self.models().as_slice() {
            [only] => self.pick_model(*only),
            _ if self.maker == Maker::NotListed => self.fit_sheet(self.bed),
            _ => {}
        }
    }

    /// Picks a machine from the catalogue: its units, and material the size of its bed.
    pub fn pick_model(&mut self, index: usize) {
        let Some(machine) = self.catalogue.get(index) else {
            return;
        };
        self.model = Some(index);
        let bed = [machine.width, machine.height];
        self.set_inches(machine.inches);
        self.fit_sheet(bed);
    }

    /// Millimetres or inches; a thickness still the one offered becomes the one offered in
    /// these units.
    pub fn set_inches(&mut self, inches: bool) {
        self.inches = inches;
        let thickness = &mut self.sheet[2];
        if [THICKNESS_MM, THICKNESS_INCH].contains(thickness) {
            *thickness = if inches { THICKNESS_INCH } else { THICKNESS_MM };
        }
    }

    /// The material's width and length: the machine's bed, when it has one.
    fn fit_sheet(&mut self, bed: [f64; 2]) {
        if bed[0] > 0.0 && bed[1] > 0.0 {
            self.sheet[0] = bed[0];
            self.sheet[1] = bed[1];
        }
    }

    /// Whether the machine question is answered, so setup can finish.
    pub fn ready(&self) -> bool {
        match self.maker {
            Maker::Unpicked => false,
            Maker::Named(_) => self.model.is_some(),
            Maker::NotListed => self.bed.iter().all(|&s| s > 0.0),
        }
    }

    /// The machine the answers describe.
    pub fn machine(&self) -> Option<Machine> {
        match self.maker {
            Maker::Unpicked => None,
            Maker::Named(_) => self.model.and_then(|i| self.catalogue.get(i)).cloned(),
            Maker::NotListed => {
                let (posts, default) = self.controller.posts(self.inches);
                Some(Machine {
                    name: MY_MACHINE.into(),
                    controller: self.controller.label().into(),
                    width: self.bed[0],
                    height: self.bed[1],
                    inches: self.inches,
                    posts,
                    default_post: Some(default),
                    ..Machine::default()
                })
            }
        }
    }

    /// The job new files start with.
    pub fn job(&self) -> Job {
        Job {
            width: self.sheet[0],
            height: self.sheet[1],
            thickness: self.sheet[2],
            units: if self.inches { Units::Inch } else { Units::Mm },
            ..Job::default()
        }
    }
}

/// Puts the starter tool group in these units first in the tree; whether it moved.
pub(super) fn units_first(tools: &mut ToolDatabase, inches: bool) -> bool {
    let (first, second) = if inches {
        (IMPERIAL_TOOLS, METRIC_TOOLS)
    } else {
        (METRIC_TOOLS, IMPERIAL_TOOLS)
    };
    let at = |path: &str| tools.groups.iter().position(|g| g.path == path);
    let (Some(a), Some(b)) = (at(first), at(second)) else {
        return false;
    };
    if a < b {
        return false;
    }
    let group = tools.groups.remove(a);
    tools.groups.insert(b, group);
    true
}

/// A length in mm, shown and typed in the units picked.
fn length(ui: &mut egui::Ui, mm: &mut f64, inches: bool) -> egui::Response {
    let scale = if inches { MM_PER_INCH } else { 1.0 };
    let mut shown = *mm / scale;
    let response = ui.add(
        egui::DragValue::new(&mut shown)
            .range(0.01..=100_000.0)
            .speed(if inches { 0.05 } else { 1.0 })
            .max_decimals(if inches { 3 } else { 1 })
            .suffix(if inches { " in" } else { " mm" }),
    );
    if response.changed() {
        *mm = shown * scale;
    }
    response
}

/// What the setup window's buttons asked for.
enum Finish {
    Done,
    Skipped,
}

impl CutCraftApp {
    /// The options kept between runs, read at start (after the machines, which decide
    /// whether first-time setup is shown).
    pub(super) fn load_options(&mut self) {
        let (options, tampered) = crate::options::Options::load_checked();
        self.options = options;
        self.options.advanced.apply();
        self.playback_speed = self.options.toolpaths.preview_speed();
        if tampered {
            self.status.set_message(crate::options::TAMPERED);
        }
        self.options_path = crate::options::Options::path();
        self.options.apply_to(&mut self.display);
        if wanted(&self.options, &self.machines) {
            self.open_first_run();
        }
    }

    /// Help ▸ Run First-Time Setup Again, or the first run: the questions shown.
    pub(super) fn open_first_run(&mut self) {
        self.first_run = Some(Setup::new());
    }

    /// The setup window, while it is open.
    pub(super) fn first_run_window(&mut self, ctx: &egui::Context) {
        let Some(mut setup) = self.first_run.take() else {
            return;
        };
        let mut finish = None;
        egui::Modal::new(egui::Id::unique("first_run")).show(ctx, |ui| {
            cut_craft_bug_report::mark(ui, "First-time setup");
            ui.set_width(ui.spacing().interact_size.x * 9.0);
            finish = Self::first_run_questions(ui, &mut setup);
        });
        match finish {
            Some(Finish::Done) => self.finish_first_run(&setup),
            Some(Finish::Skipped) => self.skip_first_run(),
            None => self.first_run = Some(setup),
        }
    }

    fn first_run_questions(ui: &mut egui::Ui, setup: &mut Setup) -> Option<Finish> {
        ui.heading("Welcome to Cut Craft");
        ui.label(
            "Three quick questions so your first job matches your machine. You can change \
             any of this later.",
        );
        ui.add_space(ui.spacing().item_spacing.y);
        ui.strong("1. Which machine do you have?");
        Self::machine_question(ui, setup);
        ui.add_space(ui.spacing().item_spacing.y);
        ui.strong("2. Millimetres or inches?");
        ui.horizontal(|ui| {
            let mut inches = setup.inches;
            ui.radio_value(&mut inches, false, "Millimetres");
            ui.radio_value(&mut inches, true, "Inches");
            if inches != setup.inches {
                setup.set_inches(inches);
            }
        });
        ui.weak("Picked to match your machine. Choose whichever your rulers and bits use.");
        ui.add_space(ui.spacing().item_spacing.y);
        ui.strong("3. What size material do you usually cut?");
        egui::Grid::new("first_run_sheet")
            .num_columns(2)
            .show(ui, |ui| {
                let inches = setup.inches;
                let [width, length_, thickness] = &mut setup.sheet;
                ui.label("Width (X)");
                length(ui, width, inches);
                ui.end_row();
                ui.label("Length (Y)");
                length(ui, length_, inches);
                ui.end_row();
                ui.label("Thickness");
                length(ui, thickness, inches);
                ui.end_row();
            });
        ui.weak("New jobs start this size. Each job can be changed in Job Setup.");
        ui.separator();
        let mut finish = None;
        ui.horizontal(|ui| {
            let start = ui
                .add_enabled(setup.ready(), egui::Button::new("Start using Cut Craft"))
                .on_disabled_hover_text("Pick your machine first, or Mine is not listed");
            if start.clicked() {
                finish = Some(Finish::Done);
            }
            if ui
                .button("Skip for now")
                .on_hover_text(
                    "Start with Cut Craft's usual 600 × 400 × 20 mm job and no machine. \
                     Help ▸ Run First-Time Setup Again asks again.",
                )
                .clicked()
            {
                finish = Some(Finish::Skipped);
            }
        });
        finish
    }

    fn machine_question(ui: &mut egui::Ui, setup: &mut Setup) {
        let makers = cut_craft_machines::makers(&setup.catalogue);
        let shown = match &setup.maker {
            Maker::Unpicked => "Pick the maker…".to_owned(),
            Maker::Named(m) => m.clone(),
            Maker::NotListed => "Mine is not listed".to_owned(),
        };
        let mut picked = None;
        egui::Grid::new("first_run_machine")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Maker");
                egui::ComboBox::from_id_salt("first_run_maker")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for maker in makers {
                            if ui.selectable_label(false, &maker).clicked() {
                                picked = Some(Maker::Named(maker));
                            }
                        }
                        ui.separator();
                        if ui.selectable_label(false, "Mine is not listed").clicked() {
                            picked = Some(Maker::NotListed);
                        }
                    });
                ui.end_row();
                if let Maker::Named(_) = setup.maker {
                    let models = setup.models();
                    let shown = setup
                        .model
                        .map_or("Pick the model…", |i| &setup.catalogue[i].model)
                        .to_owned();
                    let mut model = None;
                    ui.label("Model");
                    egui::ComboBox::from_id_salt("first_run_model")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for i in models {
                                let name = &setup.catalogue[i].model;
                                if ui.selectable_label(setup.model == Some(i), name).clicked() {
                                    model = Some(i);
                                }
                            }
                        });
                    ui.end_row();
                    if let Some(i) = model {
                        setup.pick_model(i);
                    }
                }
                if setup.maker == Maker::NotListed {
                    let inches = setup.inches;
                    let [width, height] = &mut setup.bed;
                    let mut moved = false;
                    ui.label("Working area width (X)");
                    moved |= length(ui, width, inches).changed();
                    ui.end_row();
                    ui.label("Working area length (Y)");
                    moved |= length(ui, height, inches).changed();
                    ui.end_row();
                    if moved {
                        setup.sheet[0] = setup.bed[0];
                        setup.sheet[1] = setup.bed[1];
                    }
                    ui.label("Controller");
                    egui::ComboBox::from_id_salt("first_run_controller")
                        .selected_text(setup.controller.label())
                        .show_ui(ui, |ui| {
                            for c in Controller::ALL {
                                ui.selectable_value(&mut setup.controller, c, c.label());
                            }
                        });
                    ui.end_row();
                }
            });
        if setup.maker == Maker::NotListed {
            ui.weak(
                "The controller is the board or program that runs the machine. Not sure? Most \
                 hobby routers run grbl.",
            );
        }
        if let Some(maker) = picked {
            setup.pick_maker(maker);
        }
    }

    /// Start using Cut Craft: the machine added and made active, new jobs set to start as
    /// answered, and the starter tools in those units first.
    pub(super) fn finish_first_run(&mut self, setup: &Setup) {
        let job = setup.job();
        let mut name = None;
        if let Some(machine) = setup.machine() {
            name = Some(machine.name.clone());
            self.add_set_up_machine(machine);
        }
        if units_first(&mut self.tools, setup.inches) {
            self.tools_changed("Starter tools in your units shown first");
        }
        self.options.default_job = Some(job);
        self.options.first_run_done = true;
        self.save_options();
        let units = if setup.inches { "in" } else { "mm" };
        let scale = if setup.inches { MM_PER_INCH } else { 1.0 };
        let size = format!(
            "{} × {} × {} {units}",
            round(job.width / scale),
            round(job.height / scale),
            round(job.thickness / scale)
        );
        let machine = name.map_or_else(String::new, |n| format!("{n}, "));
        self.status.set_message(&format!(
            "Set up: {machine}new jobs {size}. Press Create a new file to start, or Try a \
             sample project. Change these any time in Machine ▸ Machine Configuration and Job \
             Setup."
        ));
    }

    /// The machine set up, made active: added, or the one of that name already here.
    fn add_set_up_machine(&mut self, machine: Machine) {
        let at = self
            .machines
            .machines
            .iter()
            .position(|m| m.name == machine.name);
        self.machines.active = at.unwrap_or_else(|| {
            self.machines.machines.push(machine);
            self.machines.machines.len() - 1
        });
        self.machines_changed("Machine set up");
    }

    /// Skip for now: today's start (no machine, Cut Craft's usual job), not asked again.
    pub(super) fn skip_first_run(&mut self) {
        self.options.first_run_done = true;
        self.save_options();
        self.status.set_message(
            "Setup skipped. Help ▸ Run First-Time Setup Again asks the three questions again.",
        );
    }
}

/// A size as the message gives it: up to three decimals, no trailing zeros.
fn round(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(setup: &Setup, name: &str) -> usize {
        setup
            .catalogue
            .iter()
            .position(|m| m.name == name)
            .unwrap_or_else(|| panic!("no {name}"))
    }

    #[test]
    fn asked_only_with_no_machine_no_default_job_and_not_asked_before() {
        let options = crate::options::Options::default();
        let none = Machines::default();
        assert!(wanted(&options, &none), "a fresh profile");
        let mut one = Machines::default();
        one.add();
        assert!(!wanted(&options, &one), "a machine set up already");
        let kept = crate::options::Options {
            default_job: Some(Job::default()),
            ..options.clone()
        };
        assert!(!wanted(&kept, &none), "Set as Default pressed before");
        let done = crate::options::Options {
            first_run_done: true,
            ..options
        };
        assert!(!wanted(&done, &none), "answered or skipped once");
    }

    #[test]
    fn picking_a_longmill_gives_its_bed_in_inches_and_its_grbl_inch_post() {
        let mut setup = Setup::new();
        assert!(!setup.ready());
        setup.pick_maker(Maker::Named("Sienci".into()));
        assert!(!setup.ready(), "Sienci makes more than one");
        let i = named(&setup, "Sienci LongMill MK2 30x30");
        setup.pick_model(i);
        assert!(setup.ready() && setup.inches);
        let job = setup.job();
        assert_eq!(job.units, Units::Inch);
        assert!(job.width <= 762.0 && job.height <= 762.0, "{job:?}");
        assert!((job.thickness - 19.05).abs() < 1e-9, "3/4 in to start");
        let machine = setup.machine().unwrap();
        assert_eq!(machine.post(), Some("Cut Craft GRBL Arcs (inch) (*.nc)"));
    }

    #[test]
    fn a_machine_not_listed_takes_its_bed_and_controller() {
        let mut setup = Setup::new();
        setup.pick_maker(Maker::NotListed);
        setup.bed = [500.0, 300.0];
        setup.controller = Controller::Mach3;
        setup.set_inches(false);
        assert!(setup.ready());
        let machine = setup.machine().unwrap();
        assert_eq!((machine.width, machine.height), (500.0, 300.0));
        assert_eq!(machine.post(), Some("Cut Craft Mach3 Arcs (mm) (*.tap)"));
        assert_eq!(machine.name, MY_MACHINE);
        setup.bed[0] = 0.0;
        assert!(!setup.ready(), "a working area is needed");
    }

    #[test]
    fn a_typed_thickness_is_kept_when_the_units_change() {
        let mut setup = Setup::new();
        setup.set_inches(true);
        assert!((setup.sheet[2] - THICKNESS_INCH).abs() < 1e-9);
        setup.sheet[2] = 12.0;
        setup.set_inches(false);
        assert!((setup.sheet[2] - 12.0).abs() < 1e-9);
    }

    #[test]
    fn the_starter_tools_in_the_units_picked_come_first() {
        let mut tools = ToolDatabase::starter();
        assert_eq!(tools.groups[0].path, METRIC_TOOLS);
        assert!(!units_first(&mut tools, false), "already first");
        assert!(units_first(&mut tools, true));
        assert_eq!(tools.groups[0].path, IMPERIAL_TOOLS);
        assert_eq!(tools.groups[1].path, METRIC_TOOLS);
        let mut none = ToolDatabase::default();
        assert!(
            !units_first(&mut none, true),
            "a database with neither group"
        );
    }

    #[test]
    fn sizes_lose_trailing_zeros() {
        assert_eq!(round(30.0), "30");
        assert_eq!(round(0.75), "0.75");
    }
}
