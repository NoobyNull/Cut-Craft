//! The Machine menu's windows: Machine Configuration Management (the
//! machines, each one's details and associated posts; kept on Apply or OK) and Post-Processor
//! Management (the post library: view, customise, edit, install, delete).

use crate::library::{Entry, Library, Source};
use crate::machine::{Machine, Machines};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use eframe::egui;
use std::path::PathBuf;

/// The windows' share of the screen when they first open.
/// The Machine Configuration Management window's own id (its title's hashing is egui's).
fn machine_window() -> egui::Id {
    egui::Id::unique("machine_configuration_management")
}
const WINDOW_SHARE: f32 = 0.4;
/// A post's text in View, in rows of the theme's interactive height.
const VIEW_ROWS: f32 = 20.0;
/// Millimetres to an inch.
const INCH: f64 = 25.4;

/// The pictures the windows' buttons show, by icon path.
pub struct Look<'a> {
    pub picture: &'a dyn Fn(&str) -> Option<egui::Image<'static>>,
}

/// An icon button, or its words when the icon is missing; whether it was clicked.
fn icon_button(ui: &mut egui::Ui, look: &Look, icon: &str, label: &str, enabled: bool) -> bool {
    let edge = ui.spacing().interact_size.y;
    let button = match (look.picture)(icon) {
        Some(image) => egui::Button::image(image.fit_to_exact_size(egui::vec2(edge, edge))),
        None => egui::Button::new(label),
    };
    ui.add_enabled(enabled, button)
        .on_hover_hint(label)
        .on_disabled_hover_text(label)
        .clicked()
}

/// How big the windows open: a share of the main window.
fn dialog_size(ctx: &egui::Context) -> egui::Vec2 {
    ctx.content_rect().size() * egui::vec2(WINDOW_SHARE, cut_craft_bug_report::DIALOG_HEIGHT_SHARE)
}

/// Read-only text about a post, under its title: its instructions (Post-Processor Content)
/// or its change log.
fn viewer(ctx: &egui::Context, viewing: &mut Option<(String, String)>) {
    let Some((title, text)) = viewing else { return };
    let mut open = true;
    let id = egui::Id::unique("post_processor_content");
    cut_craft_bug_report::dialog(ctx, id, title, dialog_size(ctx), &mut open, |ui| {
        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(text.clone());
        }
        let height = ui.spacing().interact_size.y * VIEW_ROWS;
        egui::ScrollArea::both().max_height(height).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text.as_str())
                    .code_editor()
                    .desired_width(f32::INFINITY),
            );
        });
    });
    if !open {
        *viewing = None;
    }
}

fn view(entry: &Entry) -> Option<(String, String)> {
    let title = format!("Post-Processor Content: {}", entry.name);
    Some((title, Library::text(entry).ok()?))
}

/// The post's change log (Post-Processor Change Log).
fn change_log(entry: &Entry) -> Option<(String, String)> {
    let log = Library::change_log(&Library::text(entry).ok()?);
    let text = if log.is_empty() {
        "This post-processor keeps no change log (a \"+ History\" comment).".to_owned()
    } else {
        log.join("\n")
    };
    Some((format!("Post-Processor Change Log: {}", entry.name), text))
}

/// What Machine Configuration Management asks of the app.
#[derive(Debug, PartialEq)]
pub enum MachineOutcome {
    /// Keep the edited machines, the window staying open.
    Apply,
    /// Keep them and close.
    Ok,
    /// Close, dropping the edits.
    Cancel,
    /// Open Post-Processor Management.
    ManagePosts,
    /// Pick a folder for the machine being edited (the window's draft's active one).
    PickFolder(FolderFor),
    /// Open Machine Files on the machine being edited.
    ManageFiles,
}

/// Which of a machine's folders is being picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderFor {
    /// Where its G-code programs are saved.
    Programs,
    /// The machine's own files (Send to Machine).
    Machine,
}

/// The open Machine Configuration Management window: the machines being edited.
pub struct MachineManager {
    pub draft: Machines,
    /// The associated post picked in the list.
    picked: Option<usize>,
    viewing: Option<(String, String)>,
}

impl MachineManager {
    pub fn new(machines: &Machines) -> Self {
        Self {
            draft: machines.clone(),
            picked: None,
            viewing: None,
        }
    }

    /// Add Machine: opens on a new machine.
    pub fn adding(machines: &Machines) -> Self {
        let mut m = Self::new(machines);
        m.draft.add();
        m
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        look: &Look,
    ) -> Option<MachineOutcome> {
        let mut open = true;
        let mut outcome = None;
        let (id, title) = (machine_window(), "Machine Configuration Management");
        cut_craft_bug_report::dialog(ctx, id, title, dialog_size(ctx), &mut open, |ui| {
            cut_craft_bug_report::mark(ui, "Machine Configuration Management");
            self.machine_row(ui, look);
            ui.separator();
            let names_clash = self.draft.names_clash();
            let Some(machine) = self.draft.active_mut() else {
                ui.weak("No machines yet: Add Machine makes one");
                outcome = buttons(ui, false);
                return;
            };
            details(ui, machine, library);
            limits(ui, machine);
            tool_changer(ui, machine, None);
            keep_out(ui, machine);
            drill_bank(ui, machine);
            if let Some(asked) = g_code_folders(ui, machine) {
                outcome = Some(asked);
            }
            if names_clash {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    "Two machines cannot have the same name",
                );
            }
            let entries = library.entries();
            if let Some(asked) = posts_section(
                ui,
                look,
                machine,
                &entries,
                &mut self.picked,
                &mut self.viewing,
            ) {
                outcome = Some(asked);
            }
            outcome = buttons(ui, !names_clash).or(outcome.take());
        });
        viewer(ctx, &mut self.viewing);
        if !open {
            outcome = Some(MachineOutcome::Cancel);
        }
        outcome
    }

    /// The machine drop-down with Add / Duplicate / Delete, and the online ones Cut Craft
    /// has no service for.
    fn machine_row(&mut self, ui: &mut egui::Ui, look: &Look) {
        ui.label("Machine Configuration");
        ui.horizontal(|ui| {
            let shown = self
                .draft
                .active()
                .map_or("(none)", |m| m.name.as_str())
                .to_owned();
            egui::ComboBox::from_id_salt("machine_configuration")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for (i, m) in self.draft.machines.iter().enumerate() {
                        ui.selectable_value(&mut self.draft.active, i, &m.name);
                    }
                });
            let some = self.draft.active().is_some();
            if icon_button(ui, look, "common/add.png", "Add Machine", true) {
                self.draft.add();
                self.picked = None;
            }
            if icon_button(
                ui,
                look,
                "common/copy_duplicate.png",
                "Duplicate Machine",
                some,
            ) {
                self.draft.duplicate();
            }
            if icon_button(ui, look, "common/delete.png", "Delete Machine", some) {
                self.draft.delete();
                self.picked = None;
            }
            // ponytail: machine packages come from a vendor's online service, which Cut Craft
            // has none of; machines are made by hand
            icon_button(
                ui,
                look,
                "common/open.png",
                "Install Offline Package (needs an online machine service)",
                false,
            );
        });
    }
}

/// Name, maker, model, controller, bed size, Z travel and 4th axis, in a framed block.
fn details(ui: &mut egui::Ui, machine: &mut Machine, library: &Library) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new("machine_details")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, value) in [
                    ("Name", &mut machine.name),
                    ("Manufacturer", &mut machine.manufacturer),
                    ("Model", &mut machine.model),
                    ("Controller", &mut machine.controller),
                ] {
                    ui.label(label);
                    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
                    ui.end_row();
                }
                let (per, unit) = if machine.inches {
                    (INCH, " in")
                } else {
                    (1.0, " mm")
                };
                for (label, value) in [
                    ("Width", &mut machine.width),
                    ("Height", &mut machine.height),
                    ("Z travel", &mut machine.depth),
                ] {
                    ui.label(label);
                    measure(ui, value, per, unit);
                    ui.end_row();
                }
                ui.label("Units");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut machine.inches, false, "mm");
                    ui.radio_value(&mut machine.inches, true, "inches");
                });
                ui.end_row();
                ui.label("4th axis");
                let ticked = ui
                    .checkbox(&mut machine.rotary, "A, turning the material (rotary jobs)")
                    .on_hover_hint(
                        "Rotary jobs are saved through the machine's post that wraps their side \
                         onto A; ticking this adds Cut Craft's for its posts",
                    )
                    .changed();
                if ticked && machine.rotary {
                    let names: Vec<String> =
                        library.entries().into_iter().map(|e| e.name).collect();
                    machine.add_rotary_posts(&names);
                }
                ui.end_row();
            });
    });
}

/// A length (or a rate of one) kept in mm and shown `per` to the unit: 0 reads as not given.
fn measure(ui: &mut egui::Ui, value: &mut f64, per: f64, unit: &str) {
    let mut shown = *value / per;
    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
    let drag = Number::new(&mut shown)
        .speed(speed)
        .range(0.0..=f64::MAX)
        .max_decimals(3)
        .suffix(unit);
    if ui.add(drag).changed() {
        *value = shown * per;
    }
}

/// How fast the machine may move and turn: what the time estimates and the checks before
/// cutting go by. 0 is not given.
fn limits(ui: &mut egui::Ui, machine: &mut Machine) {
    let (per, length) = if machine.inches {
        (INCH, "in")
    } else {
        (1.0, "mm")
    };
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Limits").on_hover_hint(
            "From the controller's settings (GRBL $110-$112 and $120-$122); 0 is not given",
        );
        egui::Grid::new("machine_limits")
            .num_columns(4)
            .show(ui, |ui| {
                ui.label("");
                for axis in ["X", "Y", "Z"] {
                    ui.label(axis);
                }
                ui.end_row();
                let rows = [
                    ("Max rate", &mut machine.max_rate, format!(" {length}/min")),
                    (
                        "Acceleration",
                        &mut machine.acceleration,
                        format!(" {length}/s²"),
                    ),
                ];
                for (label, values, unit) in rows {
                    ui.label(label);
                    for value in values.iter_mut() {
                        measure(ui, value, per, &unit);
                    }
                    ui.end_row();
                }
                ui.label("Spindle");
                for value in &mut machine.spindle {
                    ui.add(
                        Number::new(value)
                            .speed(100.0)
                            .range(0.0..=f64::MAX)
                            .suffix(" rpm"),
                    );
                }
                ui.end_row();
            });
    });
}

/// The most slots a tool changer can be given here.
// ponytail: a generous ceiling for router carousels; raise it for big machining centres
const MOST_SLOTS: usize = 99;

/// Automatic Tool Changer: how many slots, and what is loaded in them. Given the tool
/// database's tools (name and tool number), each slot's Select… asks for its tool to be picked
/// in the Tool Database, × empties it, and Load by Tool Number puts each numbered tool in its
/// holder; else the slots are only listed. Whether anything changed, and the holder (from 1)
/// whose Select… was clicked.
pub fn tool_changer(
    ui: &mut egui::Ui,
    machine: &mut Machine,
    tools: Option<&[(String, u32)]>,
) -> (bool, Option<usize>) {
    let before = machine.slots.clone();
    let mut select = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Automatic Tool Changer");
        let mut has = !machine.slots.is_empty();
        if ui
            .checkbox(&mut has, "This machine has a tool changer")
            .changed()
        {
            machine.set_slot_count(usize::from(has));
        }
        if !has {
            ui.weak("Tool changes are by hand; tools are numbered in the order they are used");
            return;
        }
        let mut count = machine.slots.len();
        ui.horizontal(|ui| {
            ui.label("Slots");
            if ui
                .add(Number::new(&mut count).range(1..=MOST_SLOTS))
                .changed()
            {
                machine.set_slot_count(count);
            }
        });
        let Some(tools) = tools else {
            egui::Grid::new("atc_slots").num_columns(2).show(ui, |ui| {
                for (i, tool) in machine.slots.iter().enumerate() {
                    ui.label(format!("T{}", i + 1));
                    match tool {
                        Some(name) => ui.label(name),
                        None => ui.weak("(empty)"),
                    };
                    ui.end_row();
                }
            });
            ui.weak("Load the holders in Machine ▸ Tool Changer…");
            return;
        };
        if ui
            .button("Load by Tool Number")
            .on_hover_hint("Each tool with a tool number goes in that holder")
            .clicked()
        {
            for (name, number) in tools {
                let slot = usize::try_from(*number).unwrap_or(0);
                if (1..=machine.slots.len()).contains(&slot) {
                    machine.load_tool(name, slot);
                }
            }
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("atc_slots").num_columns(3).show(ui, |ui| {
                for i in 0..machine.slots.len() {
                    ui.label(format!("T{}", i + 1));
                    match &machine.slots[i] {
                        Some(name) => ui.label(name),
                        None => ui.weak("(empty)"),
                    };
                    ui.horizontal(|ui| {
                        if ui
                            .button("Select…")
                            .on_hover_hint("Pick the tool for this holder in the Tool Database")
                            .clicked()
                        {
                            select = Some(i + 1);
                        }
                        if machine.slots[i].is_some()
                            && ui
                                .small_button("×")
                                .on_hover_hint("Empty this holder")
                                .clicked()
                        {
                            machine.slots[i] = None;
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });
    (machine.slots != before, select)
}

/// Keep-Out Zones: rectangles of the bed no toolpath of any job may pass over, such as
/// clamps fixed to it; rapids go round them and toolpaths that cannot are not saved.
fn keep_out(ui: &mut egui::Ui, machine: &mut Machine) {
    let (per, length) = if machine.inches {
        (INCH, "in")
    } else {
        (1.0, "mm")
    };
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Keep-Out Zones").on_hover_hint(
            "Clamps fixed to the bed: no toolpath of any job passes over them. X and Y are \
             from the job's XY origin, as the G-code is",
        );
        let mut gone = None;
        egui::Grid::new("machine_keep_out")
            .num_columns(6)
            .show(ui, |ui| {
                for heading in ["Name", "X", "Y", "Width", "Height"] {
                    ui.label(heading);
                }
                ui.end_row();
                for (i, zone) in machine.keep_out.iter_mut().enumerate() {
                    // what its label in the views says, after "Keep-out: "
                    let hint = zone.name_at(i);
                    ui.add(
                        egui::TextEdit::singleline(&mut zone.name)
                            .hint_text(hint)
                            .desired_width(ui.spacing().text_edit_width / 2.0),
                    )
                    .on_hover_hint("What its label in the 2D and 3D views calls it");
                    // the corner may be either side of the origin
                    for value in [&mut zone.x, &mut zone.y] {
                        let mut shown = *value / per;
                        let drag = Number::new(&mut shown)
                            .max_decimals(3)
                            .suffix(format!(" {length}"));
                        if ui.add(drag).changed() {
                            *value = shown * per;
                        }
                    }
                    for value in [&mut zone.width, &mut zone.height] {
                        measure(ui, value, per, &format!(" {length}"));
                    }
                    if ui.button("Remove").clicked() {
                        gone = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = gone {
            machine.keep_out.remove(i);
        }
        if ui.button("Add Zone").clicked() {
            machine.keep_out.push(crate::machine::Zone::default());
        }
    });
}

/// G-code Files: where the machine's programs are saved on this computer, and the folder
/// the machine's own files are in when the computer can see it (a wireless controller's shared
/// card, or its card or stick plugged in here), which Send to Machine saves straight to.
fn g_code_folders(ui: &mut egui::Ui, machine: &mut Machine) -> Option<MachineOutcome> {
    let mut asked = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("G-code Files").on_hover_hint(
            "Each project's G-code goes in a folder of its own, named after the project",
        );
        egui::Grid::new("machine_g_code_folders")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, value, hint, which, tip) in [
                    (
                        "Save programs to",
                        &mut machine.send_to.programs,
                        "Wherever you saved last",
                        FolderFor::Programs,
                        "Save Toolpaths' file dialog opens here, in the project's own folder",
                    ),
                    (
                        "Machine's files",
                        &mut machine.send_to.folder,
                        "A folder, or its address such as http://fluidnc.local/sd/",
                        FolderFor::Machine,
                        "The machine's card: a folder this computer sees it as (a network \
                         share, or its SD card or USB stick plugged in here), or the address \
                         a WiFi controller serves it at (WebDAV; FluidNC's is \
                         http://fluidnc.local/sd/). Send to Machine saves straight to it",
                    ),
                ] {
                    ui.label(label).on_hover_hint(tip);
                    ui.horizontal(|ui| {
                        let browse = ui.button("Browse…");
                        let width = ui.available_width();
                        ui.add(
                            egui::TextEdit::singleline(value)
                                .hint_text(hint)
                                .desired_width(width),
                        )
                        .on_hover_hint(tip);
                        if browse.clicked() {
                            asked = Some(MachineOutcome::PickFolder(which));
                        }
                    });
                    ui.end_row();
                }
                ui.label("");
                ui.checkbox(
                    &mut machine.send_to.short_names,
                    "Short names (8 letters) for controllers that need them",
                )
                .on_hover_hint(
                    "Some older controllers only read names of up to 8 letters off their card. \
                     Leave this off for FluidNC and most WiFi controllers",
                );
                ui.end_row();
            });
        let can = machine.send_to.is_set();
        let manage = ui
            .add_enabled(can, egui::Button::new("Machine Files…"))
            .on_hover_hint("See, upload, rename and delete the files on the machine")
            .on_disabled_hover_text("Set the machine's files folder first");
        if manage.clicked() {
            asked = Some(MachineOutcome::ManageFiles);
        }
    });
    asked
}

/// Drill Bank: whether the head has a bank of drills that drop down together, and if so
/// its arms, pitch and each drill's number and size, with a picture of it.
fn drill_bank(ui: &mut egui::Ui, machine: &mut Machine) {
    use cut_craft_core::drill_bank::DrillBank;
    let (per, length) = if machine.inches {
        (INCH, "in")
    } else {
        (1.0, "mm")
    };
    // one line, folded, until wanted: most machines have none, and it is set once
    let summary = machine.drill_bank.as_ref().map_or_else(
        || "Drill Bank: none".to_owned(),
        |bank| {
            format!(
                "Drill Bank: {} drills, {} {length} apart",
                bank.drills().len(),
                bank.pitch / per
            )
        },
    );
    egui::CollapsingHeader::new(summary)
        .id_salt("drill_bank")
        .show(ui, |ui| {
            let mut has = machine.drill_bank.is_some();
            if ui
                .checkbox(&mut has, "This machine has a drill bank")
                .on_hover_hint(
                    "A head with several drills side by side that drop down together, so a \
                     row of shelf-pin or dowel holes is drilled in one plunge. Most hobby \
                     machines have none",
                )
                .changed()
            {
                // a common start: five 5 mm drills 32 mm apart (the 32 mm cabinet system)
                machine.drill_bank = has.then(|| DrillBank::new(5, 0, 32.0, 5.0));
            }
            if let Some(bank) = machine.drill_bank.as_mut() {
                bank_layout(ui, bank, per, length);
            }
        });
    if let Some(problem) = machine.drill_bank.as_ref().and_then(DrillBank::problem) {
        ui.colored_label(ui.visuals().error_fg_color, problem);
    }
}

/// The bank's arms, pitch and drills, and a picture of it.
fn bank_layout(
    ui: &mut egui::Ui,
    bank: &mut cut_craft_core::drill_bank::DrillBank,
    per: f64,
    length: &str,
) {
    {
        egui::Grid::new("drill_bank_layout")
            .num_columns(2)
            .show(ui, |ui| {
                let (mut across, mut along) = (bank.horizontal.len(), bank.vertical.len());
                ui.label("Drills along X")
                    .on_hover_hint("How many drills are on the arm running left to right");
                let a = ui.add(Number::new(&mut across).range(0..=32));
                ui.end_row();
                ui.label("Drills along Y")
                    .on_hover_hint("How many drills are on the arm running front to back; 0 for a single row");
                let b = ui.add(Number::new(&mut along).range(0..=32));
                ui.end_row();
                if a.changed() || b.changed() {
                    bank.set_counts(across, along);
                }
                ui.label("Pitch")
                    .on_hover_hint("The distance between neighbouring drills, centre to centre");
                measure(ui, &mut bank.pitch, per, &format!(" {length}"));
                ui.end_row();
                if !bank.horizontal.is_empty() && !bank.vertical.is_empty() {
                    ui.label("Arms cross at").on_hover_hint(
                        "The drill both arms share: its place along X, then along Y, counted from 1",
                    );
                    ui.horizontal(|ui| {
                        let limits = [bank.horizontal.len(), bank.vertical.len()];
                        for (place, most) in bank.central.iter_mut().zip(limits) {
                            let mut shown = *place + 1;
                            if ui
                                .add(Number::new(&mut shown).range(1..=most))
                                .changed()
                            {
                                *place = shown - 1;
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        let drills = bank.drills();
        bank_drills(ui, bank, &drills, per, length);
        bank_picture(ui, bank, &drills);
    }
}

/// Each drill of the bank: its number for the controller, its size, and whether it is the
/// one the machine positions by.
fn bank_drills(
    ui: &mut egui::Ui,
    bank: &mut cut_craft_core::drill_bank::DrillBank,
    drills: &[(
        cut_craft_core::drill_bank::DrillId,
        cut_craft_core::drill_bank::BankDrill,
    )],
    per: f64,
    length: &str,
) {
    use cut_craft_core::drill_bank::Arm;
    egui::Grid::new("drill_bank_drills")
        .num_columns(4)
        .striped(true)
        .show(ui, |ui| {
            for heading in ["Drill", "Number", "Diameter", "Reference"] {
                ui.label(heading);
            }
            ui.end_row();
            for (id, _) in drills {
                let name = match id.arm {
                    Arm::Horizontal => format!("X{}", id.index + 1),
                    Arm::Vertical => format!("Y{}", id.index + 1),
                };
                let crossing = bank.is_crossing(*id);
                ui.label(if crossing {
                    format!("{name} (both arms)")
                } else {
                    name
                });
                ui.horizontal(|ui| {
                    let drill = match id.arm {
                        Arm::Horizontal => &mut bank.horizontal[id.index],
                        Arm::Vertical => &mut bank.vertical[id.index],
                    };
                    ui.add(Number::new(&mut drill.number)).on_hover_hint(
                        "The number the controller picks this drill by; controllers that \
                             take a bitmask expect 1, 2, 4, 8...",
                    );
                    if crossing {
                        let y = &mut bank.vertical[bank.central[1]].number;
                        ui.add(Number::new(y).prefix("Y ")).on_hover_hint(
                            "Its number on the Y arm, for controllers that number the \
                                 arms apart",
                        );
                    }
                });
                let drill = match id.arm {
                    Arm::Horizontal => &mut bank.horizontal[id.index],
                    Arm::Vertical => &mut bank.vertical[id.index],
                };
                measure(ui, &mut drill.diameter, per, &format!(" {length}"));
                let is_reference = bank.same(bank.reference, *id);
                if ui
                    .radio(is_reference, "")
                    .on_hover_hint(
                        "The drill the machine positions by: the G-code's X and Y are where \
                     this drill is",
                    )
                    .clicked()
                {
                    bank.reference = *id;
                }
                ui.end_row();
            }
        });
}

/// The bank from above: each drill as a circle its size, the reference drill crossed.
fn bank_picture(
    ui: &mut egui::Ui,
    bank: &cut_craft_core::drill_bank::DrillBank,
    drills: &[(
        cut_craft_core::drill_bank::DrillId,
        cut_craft_core::drill_bank::BankDrill,
    )],
) {
    if drills.is_empty() || bank.pitch <= 0.0 {
        return;
    }
    let places: Vec<(egui::Vec2, f64)> = drills
        .iter()
        .map(|(id, d)| {
            let p = bank.position(*id);
            #[expect(clippy::cast_possible_truncation, reason = "screen points")]
            let at = egui::vec2(p.x as f32, -p.y as f32);
            (at, d.diameter)
        })
        .collect();
    let (mut lo, mut hi) = (places[0].0, places[0].0);
    for (p, _) in &places {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    #[expect(clippy::cast_possible_truncation, reason = "screen points")]
    let pitch = bank.pitch as f32;
    let span = (hi - lo) + egui::vec2(pitch, pitch);
    let width = ui.available_width().min(320.0);
    let scale = (width / span.x).min(140.0 / span.y);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, span.y * scale), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let middle = (lo + hi) / 2.0;
    let stroke = egui::Stroke::new(1.5, ui.visuals().text_color());
    for ((id, _), (at, diameter)) in drills.iter().zip(&places) {
        let centre = rect.center() + (*at - middle) * scale;
        #[expect(clippy::cast_possible_truncation, reason = "screen points")]
        let radius = (diameter / 2.0) as f32 * scale;
        painter.circle_stroke(centre, radius.max(2.0), stroke);
        if bank.same(bank.reference, *id) {
            let r = radius.max(2.0) + 3.0;
            painter.line_segment(
                [centre - egui::vec2(r, 0.0), centre + egui::vec2(r, 0.0)],
                stroke,
            );
            painter.line_segment(
                [centre - egui::vec2(0.0, r), centre + egui::vec2(0.0, r)],
                stroke,
            );
        }
    }
}

/// Associated Post-Processors: the machine's posts (the default marked), Add and Remove, and
/// a right-click for View and Make Default.
fn posts_section(
    ui: &mut egui::Ui,
    look: &Look,
    machine: &mut Machine,
    entries: &[Entry],
    picked: &mut Option<usize>,
    viewing: &mut Option<(String, String)>,
) -> Option<MachineOutcome> {
    let mut asked = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong("Associated Post-Processors");
        if machine.posts.is_empty() {
            ui.weak("None: toolpaths are saved through Cut Craft's own FluidNC post");
        }
        let default = machine.post().map(str::to_owned);
        let mut make_default = None;
        for (i, name) in machine.posts.iter().enumerate() {
            let missing = !entries.iter().any(|e| &e.name == name);
            let mut text = egui::RichText::new(name);
            if default.as_deref() == Some(name) {
                text = text.strong();
            }
            let row = ui.selectable_label(*picked == Some(i), text);
            let row = if missing {
                row.on_hover_hint("Not in the post library")
            } else {
                row
            };
            if row.clicked() {
                *picked = Some(i);
            }
            cut_craft_bug_report::context_menu(&row, |ui| {
                if ui.button("View").clicked() {
                    *viewing = entries.iter().find(|e| &e.name == name).and_then(view);
                    ui.close();
                }
                if ui.button("Make Default").clicked() {
                    make_default = Some(name.clone());
                    ui.close();
                }
            });
        }
        if let Some(name) = make_default {
            machine.default_post = Some(name);
        }
        ui.horizontal(|ui| {
            let unused: Vec<&Entry> = entries
                .iter()
                .filter(|e| e.broken.is_none() && !machine.posts.contains(&e.name))
                .collect();
            egui::ComboBox::from_id_salt("add_post")
                .selected_text("Add a post-processor")
                .show_ui(ui, |ui| {
                    for e in unused {
                        if ui.selectable_label(false, &e.name).clicked() {
                            machine.posts.push(e.name.clone());
                        }
                    }
                });
            if icon_button(ui, look, "common/delete.png", "Remove", picked.is_some())
                && let Some(i) = picked.take().filter(|&i| i < machine.posts.len())
            {
                machine.posts.remove(i);
            }
            if ui.button("Post-Processor Management…").clicked() {
                asked = Some(MachineOutcome::ManagePosts);
            }
        });
        ui.weak("The default (bold) saves toolpaths; right-click a post for more");
    });
    asked
}

/// Apply on its own line, then OK and Cancel.
fn buttons(ui: &mut egui::Ui, can_keep: bool) -> Option<MachineOutcome> {
    let mut outcome = None;
    // in a row of its own: a right-to-left layout on its own takes all the height left, and
    // the window grew down the screen with it
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(can_keep, egui::Button::new("Apply"))
                .clicked()
            {
                outcome = Some(MachineOutcome::Apply);
            }
        });
    });
    ui.separator();
    ui.horizontal(|ui| {
        if ui.add_enabled(can_keep, egui::Button::new("OK")).clicked() {
            outcome = Some(MachineOutcome::Ok);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Cancel").clicked() {
                outcome = Some(MachineOutcome::Cancel);
            }
        });
    });
    outcome
}

/// What Post-Processor Management asks of the app.
#[derive(Debug, PartialEq)]
pub enum PostOutcome {
    /// Pick a `.pp` file to install.
    Install,
    /// Use this post with the active machine.
    Select(String),
    /// Show the custom posts' folder.
    OpenFolder(PathBuf),
    /// Something to say in the status bar.
    Message(String),
    Close,
}

/// The open Post-Processor Management window.
#[derive(Default)]
pub struct PostManager {
    picked: Option<String>,
    /// A custom post asked to be deleted, waiting for the user to say so again.
    deleting: Option<Entry>,
    viewing: Option<(String, String)>,
    /// A custom post open in Edit.
    editing: Option<Editing>,
}

/// A custom post being edited: its file, the text as last saved, and as it is now.
#[derive(Clone, Debug, Default)]
struct Editing {
    name: String,
    path: PathBuf,
    saved: String,
    text: String,
}

impl Editing {
    fn of(entry: &Entry) -> Option<Self> {
        let Source::Custom(path) = &entry.source else {
            return None;
        };
        let text = Library::text(entry).ok()?;
        Some(Self {
            name: entry.name.clone(),
            path: path.clone(),
            saved: text.clone(),
            text,
        })
    }

    fn save(&mut self) -> Result<(), String> {
        Library::save(&self.path, &self.text)?;
        self.saved.clone_from(&self.text);
        Ok(())
    }
}

/// Edit: a custom post's instructions, saved to its file, with what Cut Craft makes of them
/// (a post that does not read shows why, and is marked ⚠ in the list until it does).
fn editor(ctx: &egui::Context, editing: &mut Option<Editing>) -> Option<PostOutcome> {
    let edit = editing.as_mut()?;
    let mut open = true;
    let mut outcome = None;
    let (id, title) = (
        egui::Id::unique("post_processor_edit"),
        format!("Edit Post-Processor: {}", edit.name),
    );
    cut_craft_bug_report::dialog(ctx, id, &title, dialog_size(ctx), &mut open, |ui| {
        cut_craft_bug_report::mark(ui, "Edit Post-Processor");
        match cut_craft_post::Post::parse(&edit.text) {
            Ok(post) if post.name == edit.name => {
                ui.weak("Reads as a post-processor");
            }
            // machines know a post by its name, so a new one leaves them looking for the old
            Ok(post) => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    format!(
                        "Renamed {}: machines using {} need it chosen again",
                        post.name, edit.name
                    ),
                );
            }
            Err(why) => {
                ui.colored_label(ui.visuals().error_fg_color, format!("Will not load: {why}"));
            }
        }
        let changed = edit.text != edit.saved;
        ui.horizontal(|ui| {
            if ui.add_enabled(changed, egui::Button::new("Save")).clicked() {
                outcome = Some(PostOutcome::Message(match edit.save() {
                    Ok(()) => format!("Saved {}", edit.name),
                    Err(e) => e,
                }));
            }
            if ui
                .add_enabled(changed, egui::Button::new("Revert"))
                .clicked()
            {
                edit.text.clone_from(&edit.saved);
            }
            if changed {
                ui.weak("Unsaved changes are lost when this closes");
            }
        });
        let height = ui.spacing().interact_size.y * VIEW_ROWS;
        egui::ScrollArea::both().max_height(height).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut edit.text)
                    .code_editor()
                    .desired_width(f32::INFINITY),
            );
        });
    });
    if !open {
        *editing = None;
    }
    outcome
}

impl PostManager {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        look: &Look,
    ) -> Option<PostOutcome> {
        let mut open = true;
        let mut outcome = None;
        let (id, title) = (
            egui::Id::unique("post_processor_management"),
            "Post-Processor Management",
        );
        cut_craft_bug_report::dialog(ctx, id, title, dialog_size(ctx), &mut open, |ui| {
            cut_craft_bug_report::mark(ui, "Post-Processor Management");
            let height = ui.spacing().interact_size.y * VIEW_ROWS;
            egui::ScrollArea::vertical()
                .max_height(height)
                .show(ui, |ui| {
                    for entry in library.entries() {
                        if let Some(asked) = self.row(ui, look, library, &entry) {
                            outcome = Some(asked);
                        }
                    }
                });
            if let Some(asked) = self.confirm_delete(ui) {
                outcome = Some(asked);
            }
            ui.separator();
            ui.horizontal(|ui| {
                if icon_button(ui, look, "common/open.png", "Install Post-Processor", true) {
                    outcome = Some(PostOutcome::Install);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        outcome = Some(PostOutcome::Close);
                    }
                    let picked = self.picked.clone();
                    let select = ui
                        .add_enabled(picked.is_some(), egui::Button::new("Select"))
                        .on_hover_hint("Use it with the active machine");
                    if select.clicked()
                        && let Some(name) = picked
                    {
                        outcome = Some(PostOutcome::Select(name));
                    }
                });
            });
        });
        viewer(ctx, &mut self.viewing);
        if let Some(said) = editor(ctx, &mut self.editing) {
            outcome = Some(said);
        }
        if !open {
            outcome = Some(PostOutcome::Close);
        }
        outcome
    }

    /// One post: a pencil for a custom one, its name, and its Inspect menu (also on a
    /// right-click).
    fn row(
        &mut self,
        ui: &mut egui::Ui,
        look: &Look,
        library: &Library,
        entry: &Entry,
    ) -> Option<PostOutcome> {
        let mut asked = None;
        ui.horizontal(|ui| {
            let edge = ui.spacing().interact_size.y;
            match (entry.is_custom(), (look.picture)("common/edit.png")) {
                (true, Some(pencil)) => {
                    ui.add(pencil.fit_to_exact_size(egui::vec2(edge, edge)));
                }
                _ => {
                    ui.add_space(edge + ui.spacing().item_spacing.x);
                }
            }
            let picked = self.picked.as_ref() == Some(&entry.name);
            let label = ui.selectable_label(picked, &entry.name);
            if label.clicked() {
                self.picked = Some(entry.name.clone());
            }
            if let Some(version) = Library::text(entry).ok().and_then(|t| Library::version(&t)) {
                ui.weak(version)
                    .on_hover_hint("Version: the date of its last change");
            }
            if let Some(why) = &entry.broken {
                ui.colored_label(ui.visuals().error_fg_color, "⚠")
                    .on_hover_hint(why);
            }
            // The Inspect icon at the row's end, and the same menu on a right-click
            let button = ui.menu_button("Inspect ▾", |ui| self.inspect(ui, library, entry));
            if let Some(Some(a)) = button.inner {
                asked = Some(a);
            }
            cut_craft_bug_report::context_menu(&label, |ui| {
                if let Some(a) = self.inspect(ui, library, entry) {
                    asked = Some(a);
                }
            });
        });
        asked
    }

    /// Inspect: View, Changelog and Customize (which opens the copy in Edit) for Cut Craft's posts;
    /// View, Changelog, Edit, Open File Location and Delete for custom ones.
    fn inspect(
        &mut self,
        ui: &mut egui::Ui,
        library: &Library,
        entry: &Entry,
    ) -> Option<PostOutcome> {
        let mut asked = None;
        if ui.button("View").clicked() {
            self.viewing = view(entry);
            ui.close();
        }
        if ui.button("Changelog").clicked() {
            self.viewing = change_log(entry);
            ui.close();
        }
        match &entry.source {
            Source::BuiltIn(_) => {
                if ui.button("Customize").clicked() {
                    asked = Some(match library.customize(entry) {
                        Ok(copy) => {
                            self.picked = Some(copy.name.clone());
                            self.editing = Editing::of(&copy);
                            PostOutcome::Message(format!("Customised as {}", copy.name))
                        }
                        Err(e) => PostOutcome::Message(e),
                    });
                    ui.close();
                }
            }
            Source::Custom(path) => {
                if ui.button("Edit…").clicked() {
                    self.editing = Editing::of(entry);
                    ui.close();
                }
                if ui.button("Open File Location…").clicked() {
                    asked = path
                        .parent()
                        .map(|d| PostOutcome::OpenFolder(d.to_path_buf()));
                    ui.close();
                }
                if ui.button("Delete…").clicked() {
                    self.deleting = Some(entry.clone());
                    ui.close();
                }
            }
        }
        asked
    }

    /// Delete asks again, as it cannot be undone.
    fn confirm_delete(&mut self, ui: &mut egui::Ui) -> Option<PostOutcome> {
        let entry = self.deleting.clone()?;
        let mut asked = None;
        ui.group(|ui| {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "Delete {}? Its file is removed, which cannot be undone.",
                    entry.name
                ),
            );
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    self.deleting = None;
                    asked = Some(PostOutcome::Message(match Library::delete(&entry) {
                        Ok(()) => format!("Deleted {}", entry.name),
                        Err(e) => e,
                    }));
                }
                if ui.button("Keep").clicked() {
                    self.deleting = None;
                }
            });
        });
        asked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, draw: impl FnMut(&egui::Context)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                // tall enough for the whole Machine Configuration, so a window sized to the
                // screen and not its contents shows
                egui::vec2(1200.0, 1000.0),
            )),
            ..Default::default()
        };
        // no renderer takes a headless frame's texture changes
        ctx.run_pass(input, draw).textures_delta.clear();
    }

    #[test]
    fn the_windows_draw_with_and_without_machines() {
        let ctx = egui::Context::default();
        let library = Library::new(None);
        let look = Look { picture: &|_| None };
        let mut none = MachineManager::new(&Machines::default());
        let mut one = MachineManager::adding(&Machines::default());
        one.draft.machines[0].posts = vec![library.entries()[0].name.clone(), "Gone (*.nc)".into()];
        // a drill bank folds away to one line
        one.draft.machines[0].drill_bank =
            Some(cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0));
        let mut posts = PostManager::default();
        for _ in 0..10 {
            frame(&ctx, |ctx| {
                assert_eq!(none.show(ctx, &library, &look), None);
                assert_eq!(one.show(ctx, &library, &look), None);
                assert_eq!(posts.show(ctx, &library, &look), None);
            });
        }
        assert_eq!(one.draft.machines[0].name, "My Machine");
        let window = ctx
            .memory(|m| m.area_rect(machine_window()))
            .expect("drawn");
        assert!(
            window.height() < ctx.content_rect().height() - 2.0 * window.top(),
            "fits its contents, not the screen: {window:?}"
        );
    }

    #[test]
    fn a_drill_banks_layout_draws_on_both_arms_and_in_inches() {
        let ctx = egui::Context::default();
        let mut bank = cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0);
        for (per, length) in [(1.0, "mm"), (INCH, "in")] {
            frame(&ctx, |ctx| {
                egui::Window::new("bank").show(ctx, |ui| {
                    bank_layout(ui, &mut bank, per, length);
                });
            });
        }
        assert_eq!(
            bank,
            cut_craft_core::drill_bank::DrillBank::new(5, 3, 32.0, 5.0)
        );
    }

    #[test]
    fn changelog_shows_a_posts_history_under_its_name() {
        let entry = Library::new(None).entries()[0].clone();
        let (title, text) = change_log(&entry).unwrap();
        assert_eq!(title, format!("Post-Processor Change Log: {}", entry.name));
        assert!(text.contains("2026-10-31"), "{text}");
    }

    #[test]
    fn a_custom_post_is_edited_and_saved_to_its_file() {
        let dir = std::env::temp_dir().join(format!("cut-craft-edit-{}", std::process::id()));
        let library = Library::new(Some(dir.clone()));
        let copy = library.customize(&library.entries()[0]).unwrap();
        let own = library.entries().into_iter().last().unwrap();
        assert!(
            Editing::of(&own).is_none(),
            "Cut Craft's own are customised, not edited"
        );
        let mut edit = Editing::of(&copy).expect("a custom post opens");
        edit.text.push_str("\n+ my change\n");
        edit.save().unwrap();
        assert!(Library::text(&copy).unwrap().ends_with("+ my change\n"));
        assert_eq!(edit.saved, edit.text);
        let mut editing = Some(edit);
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, |ctx| assert_eq!(editor(ctx, &mut editing), None));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_post_is_deleted_only_when_asked_twice() {
        let dir = std::env::temp_dir().join(format!("cut-craft-pm-{}", std::process::id()));
        let library = Library::new(Some(dir.clone()));
        let copy = library.customize(&library.entries()[0]).unwrap();
        let mut posts = PostManager {
            deleting: Some(copy.clone()),
            ..PostManager::default()
        };
        let ctx = egui::Context::default();
        frame(&ctx, |ctx| {
            ctx.root_ui(|ui| {
                egui::CentralPanel::default()
                    .show(ui, |ui| assert_eq!(posts.confirm_delete(ui), None));
            });
        });
        assert!(
            library.find(&copy.name).is_some(),
            "still there until Delete is pressed"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
