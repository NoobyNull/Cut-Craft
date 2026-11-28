//! The Machine menu's windows: Machine Configuration Management (the
//! machines, each one's details and associated posts; kept on Apply or OK) and Post-Processor
//! Management (the post library: view, customise, edit, install, delete).

use crate::capabilities::{Capabilities, CapabilityDb, Family, Filter, Moves};
use crate::library::{Entry, Library, Source};
use crate::machine::{ChangePosition, Firmware, HandChanges, Machine, Machines, NO_A_AXIS};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_bug_report::number::Number;
use eframe::egui;
use std::path::PathBuf;
mod posts;
pub use posts::*;

/// The windows' share of the screen when they first open.
/// The Machine Configuration Management window's own id (its title's hashing is egui's).
fn machine_window() -> egui::Id {
    egui::Id::unique("machine_configuration_management")
}
const WINDOW_SHARE: f32 = 0.4;
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
    cut_craft_bug_report::dialog_with_panels(ctx, id, title, dialog_size(ctx), &mut open, |ui| {
        if ui.button(cut_craft_i18n::t!("Copy")).clicked() {
            ui.ctx().copy_text(text.clone());
        }
        text_view(ui, &mut text.as_str());
    });
    if !open {
        *viewing = None;
    }
}

/// A post's text filling the rest of the window, scrolling when it is longer; where it shows.
fn text_view(ui: &mut egui::Ui, text: &mut dyn egui::TextBuffer) -> egui::Rect {
    egui::ScrollArea::both()
        .auto_shrink(false)
        .show(ui, |ui| {
            let fill = ui.available_size();
            ui.add(
                egui::TextEdit::multiline(text)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .min_size(fill),
            );
        })
        .inner_rect
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
    /// Install Offline Package: pick a machine package file and add its machines here.
    InstallPackage,
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
    /// The machines as last kept (opened, or Apply): the draft differs while changes wait.
    applied: Machines,
    /// Closed with changes not applied: it asks Apply or Discard first.
    closing: bool,
    /// The associated post picked in the list.
    picked: Option<usize>,
    viewing: Option<(String, String)>,
    /// Add a post-processor's filters: set afresh from the machine each time it opens.
    post_filter: Filter,
    /// Whether Add a post-processor's list was open last frame.
    adding_post: bool,
    /// The 4th axis warning's link was clicked: show and focus the Firmware choice.
    find_firmware: bool,
}

impl MachineManager {
    pub fn new(machines: &Machines) -> Self {
        Self {
            draft: machines.clone(),
            applied: machines.clone(),
            closing: false,
            picked: None,
            viewing: None,
            post_filter: Filter::default(),
            adding_post: false,
            find_firmware: false,
        }
    }

    /// Add Machine: opens on a new machine.
    pub fn adding(machines: &Machines) -> Self {
        let mut m = Self::new(machines);
        m.draft.add();
        m
    }

    /// There are edits Apply or OK hasn't kept yet.
    pub fn dirty(&self) -> bool {
        self.draft != self.applied
    }

    /// The edits were kept (Apply): the window has none waiting from here.
    pub fn applied(&mut self) {
        self.applied = self.draft.clone();
        self.closing = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        library: &Library,
        look: &Look,
    ) -> Option<MachineOutcome> {
        use cut_craft_bug_report::unapplied::{self, Answer};
        let mut open = true;
        let (mut outcome, mut row) = (None, None);
        let dirty = self.dirty();
        let (id, title) = (machine_window(), "Machine Configuration Management");
        cut_craft_bug_report::dialog(ctx, id, title, dialog_size(ctx), &mut open, |ui| {
            cut_craft_bug_report::mark(ui, "Machine Configuration Management");
            unapplied::hold_close(ui, dirty);
            if self.closing {
                match unapplied::question(ui.ctx(), "machines_apply_question") {
                    Some(Answer::ApplyAndClose) => outcome = Some(MachineOutcome::Ok),
                    Some(Answer::Discard) => outcome = Some(MachineOutcome::Cancel),
                    Some(Answer::KeepEditing) => self.closing = false,
                    None => {}
                }
            }
            row = self.machine_row(ui, look);
            ui.separator();
            let names_clash = self.draft.names_clash();
            let Some(machine) = self.draft.active_mut() else {
                ui.weak(cut_craft_i18n::t!("No machines yet: Add Machine makes one"));
                outcome = buttons(ui, false, dirty).or(outcome.take());
                return;
            };
            details(ui, machine, library, &mut self.find_firmware);
            rotary(ui, machine);
            limits(ui, machine);
            holder(ui, machine);
            tool_changer(ui, machine, None);
            bit_changes(ui, machine);
            keep_out(ui, machine);
            protect(ui, machine);
            drill_bank(ui, machine);
            if let Some(asked) = g_code_folders(ui, machine) {
                outcome = Some(asked);
            }
            if names_clash {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    cut_craft_i18n::t!("Two machines cannot have the same name"),
                );
            }
            let entries = library.entries();
            // read each frame (a small file), so corrections made in Post-Processor
            // Management show here at once
            let db = CapabilityDb::load(library);
            if let Some(asked) = posts_section(
                ui,
                look,
                machine,
                &entries,
                (&db, &mut self.post_filter, &mut self.adding_post),
                &mut self.picked,
                &mut self.viewing,
            ) {
                outcome = Some(asked);
            }
            outcome = buttons(ui, !names_clash, dirty).or(outcome.take());
        });
        viewer(ctx, &mut self.viewing);
        // the close box drops the edits only once asked, while some wait
        if !open {
            if dirty {
                self.closing = true;
            } else {
                outcome = Some(MachineOutcome::Cancel);
            }
        }
        outcome.or(row)
    }

    /// The machine drop-down with Add / Duplicate / Delete and Install Offline Package;
    /// what was asked for.
    fn machine_row(&mut self, ui: &mut egui::Ui, look: &Look) -> Option<MachineOutcome> {
        let mut asked = None;
        ui.label(cut_craft_i18n::t!("Machine Configuration"));
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
            if icon_button(
                ui,
                look,
                "common/open.png",
                "Install Offline Package: add the machines in a package file you downloaded",
                true,
            ) {
                asked = Some(MachineOutcome::InstallPackage);
            }
        });
        asked
    }
}

/// Name, maker, model, controller, firmware, bed size, Z travel and 4th axis, in a framed
/// block; no 4th axis on standard GRBL, which has none. `find_firmware`: the warning's link
/// was clicked, so the Firmware choice is scrolled to and focused.
fn details(ui: &mut egui::Ui, machine: &mut Machine, library: &Library, find_firmware: &mut bool) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        egui::Grid::new("machine_details")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, value) in [
                    ("Name", &mut machine.name),
                    ("Manufacturer", &mut machine.manufacturer),
                    ("Model", &mut machine.model),
                ] {
                    ui.label(label);
                    ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
                    ui.end_row();
                }
                // the controller (board) and the firmware it runs, side by side
                ui.label(cut_craft_i18n::t!("Controller"));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut machine.controller)
                            .desired_width((ui.available_width() - 290.0).max(120.0)),
                    );
                    ui.label(cut_craft_i18n::t!("Firmware"));
                    firmware(ui, machine, find_firmware);
                });
                ui.end_row();
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
                    measure(ui, label, value, per, unit);
                    ui.end_row();
                }
                ui.label(cut_craft_i18n::t!("Units"));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut machine.inches, false, cut_craft_i18n::t!("mm"));
                    ui.radio_value(&mut machine.inches, true, cut_craft_i18n::t!("inches"));
                });
                ui.end_row();
                // standard GRBL has no A axis: no 4th axis to offer at all
                if machine.firmware().has_a_axis() {
                    ui.label(cut_craft_i18n::t!("4th axis"));
                    let mut ticked = machine.rotary;
                    if ui
                        .checkbox(
                            &mut ticked,
                            cut_craft_i18n::t!("A, turning the material (rotary jobs)"),
                        )
                        .on_hover_hint(cut_craft_i18n::t!(
                            "Rotary jobs are saved through the machine's post that turns their \
                             side into A; ticking this adds PowerWhittle's for its posts"
                        ))
                        .changed()
                    {
                        machine.rotary = ticked;
                        if ticked {
                            let names: Vec<String> =
                                library.entries().into_iter().map(|e| e.name).collect();
                            machine.add_rotary_posts(&names);
                        }
                    }
                    ui.end_row();
                }
                ui.label(cut_craft_i18n::t!("Laser"));
                ui.checkbox(&mut machine.laser, cut_craft_i18n::t!("Laser fitted"))
                    .on_hover_hint(cut_craft_i18n::t!(
                        "Tick if a laser is fitted to the head: only then are the laser \
                         toolpaths offered on the Toolpaths page"
                    ));
                ui.end_row();
            });
        // a 4th axis ticked before, that its firmware cannot drive (below the grid, so it
        // wraps to the window)
        if machine.rotary && !machine.firmware().has_a_axis() {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    format!("⚠ {}", cut_craft_i18n::t!(NO_A_AXIS)),
                );
                if ui.link("Pick the firmware").clicked() {
                    *find_firmware = true;
                }
            });
        }
    });
}

/// The Firmware choice: the one picked, else the one the Controller text names, else
/// standard GRBL; each says what it is on hover.
fn firmware(ui: &mut egui::Ui, machine: &mut Machine, find: &mut bool) {
    let now = machine.firmware();
    let combo = egui::ComboBox::from_id_salt("machine_firmware")
        .selected_text(now.label())
        .width(180.0)
        .show_ui(ui, |ui| {
            for f in Firmware::ALL {
                if ui
                    .selectable_label(now == f && machine.firmware.is_some(), f.label())
                    .on_hover_hint(f.hint())
                    .clicked()
                {
                    machine.firmware = Some(f);
                }
            }
        });
    let response = combo.response.on_hover_hint(cut_craft_i18n::t!(
        "Which firmware the controller runs. It decides whether the machine can have a 4th axis: \
         standard GRBL has none. Not picked, it is read from Controller, and taken for standard \
         GRBL when Controller does not name one"
    ));
    if std::mem::take(find) {
        response.scroll_to_me(Some(egui::Align::Center));
        response.request_focus();
        response.highlight();
    }
}

/// A length (or a rate of one) kept in mm and shown `per` to the unit: 0 reads as not given.
/// `name` is what a screen reader calls the box: its label's words.
fn measure(ui: &mut egui::Ui, name: &str, value: &mut f64, per: f64, unit: &str) {
    let mut shown = *value / per;
    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
    let drag = Number::new(&mut shown)
        .speed(speed)
        .range(0.0..=f64::MAX)
        .max_decimals(3)
        .suffix(unit)
        .name(name);
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
        ui.strong(cut_craft_i18n::t!("Limits"))
            .on_hover_hint(cut_craft_i18n::t!(
                "From the controller's settings (GRBL $110-$112 and $120-$122); 0 is not given"
            ));
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
                    for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values.iter_mut()) {
                        measure(ui, &format!("{label} {axis}"), value, per, &unit);
                    }
                    ui.end_row();
                }
                ui.label(cut_craft_i18n::t!("Cornering"))
                    .on_hover_hint(cut_craft_i18n::t!(
                        "How far the controller lets the bit stray from a sharp corner to keep \
                     its speed through it (junction deviation, GRBL $11); 0 is the usual \
                     0.01 mm. The cutting time estimates use it"
                    ));
                let mut shown = machine.junction_deviation / per;
                let corner = Number::new(&mut shown)
                    .speed(0.001)
                    .range(0.0..=f64::MAX)
                    .max_decimals(4)
                    .suffix(format!(" {length}"))
                    .name("Cornering");
                if ui.add(corner).changed() {
                    machine.junction_deviation = shown * per;
                }
                ui.end_row();
                ui.label(cut_craft_i18n::t!("Spindle"));
                for (end, value) in ["slowest", "fastest"].into_iter().zip(&mut machine.spindle) {
                    ui.add(
                        Number::new(value)
                            .speed(100.0)
                            .range(0.0..=f64::MAX)
                            .suffix(" rpm")
                            .name(format!("Spindle {end}")),
                    );
                }
                ui.end_row();
            });
    });
}

/// What holds the bits: the collet nut and the spindle above it, which the checks before
/// cutting look for hitting the material on deep cuts, and Holder per bit.
fn holder(ui: &mut egui::Ui, machine: &mut Machine) {
    let (per, length) = if machine.inches {
        (INCH, " in")
    } else {
        (1.0, " mm")
    };
    // one line, folded, until wanted: it is set once, and the usual sizes are there already
    let holder = machine.holder;
    let summary = format!(
        "Spindle and Collet Nut: nut {} across, spindle {} across{}",
        shown(holder.nut_diameter / per, length),
        shown(holder.spindle_diameter / per, length),
        if machine.holder_per_bit {
            ", holder per bit"
        } else {
            ""
        }
    );
    egui::CollapsingHeader::new(summary)
        .id_salt("machine_holder")
        .show(ui, |ui| {
            ui.weak(cut_craft_i18n::t!(
                "Only the bit's flutes cut. On a deep cut the collet nut or the spindle above \
                 it can run into material still standing beside the cut: the preview and Save \
                 Toolpaths warn of it. Measure across the nut and the spindle with a ruler"
            ));
            egui::Grid::new("machine_holder_sizes")
                .num_columns(2)
                .show(ui, |ui| {
                    let holder = &mut machine.holder;
                    let rows = [
                        (
                            "Collet nut across",
                            &mut holder.nut_diameter,
                            "Across the nut that tightens the collet (an ER20 nut is about \
                             35 mm, a trim router's about 25 mm)",
                        ),
                        (
                            "Collet nut height",
                            &mut holder.nut_length,
                            "From the nut's bottom face up to where the spindle (or router) \
                             body starts",
                        ),
                        (
                            "Spindle across",
                            &mut holder.spindle_diameter,
                            "Across the spindle or router body above the nut (65 mm or 80 \
                             mm for most spindles)",
                        ),
                    ];
                    for (label, value, hint) in rows {
                        ui.label(label).on_hover_hint(hint);
                        measure(ui, label, value, per, length);
                        ui.end_row();
                    }
                });
            ui.checkbox(
                &mut machine.holder_per_bit,
                cut_craft_i18n::t!("Holder per bit"),
            )
            .on_hover_hint(cut_craft_i18n::t!(
                "Tick for a tool changer whose bits each sit in a holder of their own: \
                     each bit can then have its own collet nut and spindle sizes in the Bit \
                     Library. A bit without them uses the sizes above"
            ));
        });
}

/// Rotary (4th axis): where it lies, its centre line, which way and how fast A turns, and its
/// chuck and tailstock. Folded to one line, and shown only on a machine with a 4th axis its
/// firmware can drive.
fn rotary(ui: &mut egui::Ui, machine: &mut Machine) {
    use cut_craft_core::document::Along;
    if !machine.has_rotary() {
        return;
    }
    let (per, length) = if machine.inches {
        (INCH, " in")
    } else {
        (1.0, " mm")
    };
    let axis = &mut machine.rotary_axis;
    let summary = format!(
        "Rotary (4th axis): along {}, {}",
        axis.along.letter(),
        if axis.centre_set {
            "centre line found"
        } else {
            "centre line not found yet"
        }
    );
    // open while there is something still to set
    egui::CollapsingHeader::new(summary)
        .id_salt("machine_rotary")
        .default_open(!axis.centre_set)
        .show(ui, |ui| {
            ui.weak(cut_craft_i18n::t!(
                "Where the rotary sits and which way it turns. Rotary jobs cut round its \
                 turning axis, so the bit must sit exactly over it: a centre line out by half a \
                 millimetre makes a round carving lumpy"
            ));
            egui::Grid::new("machine_rotary_axis")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label(cut_craft_i18n::t!("Lies along"))
                        .on_hover_hint(cut_craft_i18n::t!(
                            "The direction of the turning axis on the bed: along X (left to right, \
                         most kits) or along Y (front to back)"
                        ));
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut axis.along, Along::X, cut_craft_i18n::t!("X"));
                        ui.radio_value(&mut axis.along, Along::Y, cut_craft_i18n::t!("Y"));
                    });
                    ui.end_row();
                    let across = if axis.along == Along::X { "Y" } else { "X" };
                    ui.label(cut_craft_i18n::tr!("machines-centre-line", across = across))
                        .on_hover_hint(cut_craft_i18n::tr!(
                            "machines-centre-line-hint",
                            across = across
                        ));
                    let mut v = axis.centre_across / per;
                    if ui
                        .add(
                            Number::new(&mut v)
                                .speed(0.01)
                                .max_decimals(3)
                                .suffix(length)
                                .name(format!("Centre line {across}")),
                        )
                        .changed()
                    {
                        axis.centre_across = v * per;
                    }
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(
                        &mut axis.centre_set,
                        cut_craft_i18n::t!("Centre line found"),
                    )
                    .on_hover_hint(cut_craft_i18n::t!(
                        "Tick once you have found the centre line on the machine and set \
                             the value above (Help: The 4th Axis and Your Firmware shows \
                             how). Until then, the checks before cutting ask about it"
                    ));
                    ui.end_row();
                    turning(ui, axis, per, length);
                    winding(ui, axis);
                    room(ui, axis, per, length);
                });
        });
}

/// The rotary's turning rows: which way +A turns, its top speed, and its chuck and
/// tailstock (lengths shown `per` to the `length` unit).
fn turning(
    ui: &mut egui::Ui,
    axis: &mut cut_craft_core::rotary_axis::RotaryAxis,
    per: f64,
    length: &str,
) {
    ui.label(cut_craft_i18n::t!("+A turns"))
        .on_hover_hint(cut_craft_i18n::t!(
            "Seen from the chuck end, looking along the axis. Jog A by +10° and \
         watch which way the chuck turns"
        ));
    ui.horizontal(|ui| {
        ui.radio_value(&mut axis.clockwise, true, cut_craft_i18n::t!("Clockwise"));
        ui.radio_value(
            &mut axis.clockwise,
            false,
            cut_craft_i18n::t!("Anticlockwise"),
        );
    });
    ui.end_row();
    ui.label(cut_craft_i18n::t!("A top speed"))
        .on_hover_hint(cut_craft_i18n::t!(
            "The fastest A turns, from the controller's settings (grblHAL's $113, \
         FluidNC's max_rate_mm_per_min for A). For cutting times and to keep \
         feeds within it"
        ));
    ui.add(
        Number::new(&mut axis.max_rate)
            .range(0.0..=f64::MAX)
            .speed(10.0)
            .max_decimals(0)
            .suffix(" °/min")
            .name("A top speed"),
    );
    ui.end_row();
    for (label, value, hint) in [
        (
            "Chuck",
            &mut axis.chuck,
            "How far the chuck and its jaws reach along the axis from where the \
             material starts. Cuts are kept out of it",
        ),
        (
            "Tailstock",
            &mut axis.tailstock,
            "How far the tailstock reaches back along the axis from where the \
             material ends",
        ),
    ] {
        ui.label(label).on_hover_hint(hint);
        measure(ui, label, value, per, length);
        ui.end_row();
    }
}

/// How much room material has to swing round the axis: down to the spoilboard (or the bed
/// with it out) and up to the bit lifted all the way (lengths shown `per` to the `length`
/// unit). 0 is not measured yet.
fn room(
    ui: &mut egui::Ui,
    axis: &mut cut_craft_core::rotary_axis::RotaryAxis,
    per: f64,
    length: &str,
) {
    ui.label(cut_craft_i18n::t!("Axis height"))
        .on_hover_hint(cut_craft_i18n::t!(
            "From the top of the spoilboard up to the turning axis: the middle of the chuck. \
         Material turning on the rotary can swing out no further than this below it. 0 until \
         you measure it"
        ));
    measure(ui, "Axis height", &mut axis.axis_height, per, length);
    ui.end_row();
    ui.label(cut_craft_i18n::t!("Spoilboard"))
        .on_hover_hint(cut_craft_i18n::t!(
            "How thick the spoilboard under the rotary is. Take it out and material can swing \
         down past where its top was, to the bed"
        ));
    ui.horizontal(|ui| {
        measure(ui, "Spoilboard", &mut axis.spoilboard, per, length);
        ui.checkbox(&mut axis.spoilboard_off, cut_craft_i18n::t!("Taken out"));
    });
    ui.end_row();
    ui.label(cut_craft_i18n::t!("Room above"))
        .on_hover_hint(cut_craft_i18n::t!(
            "With Z all the way up, from the turning axis up to the tip of the bit. Material can \
         turn under the bit only if it swings out less than this. 0 until you measure it"
        ));
    measure(ui, "Room above", &mut axis.room_above, per, length);
    ui.end_row();
}

/// How far A may wind one way, what happens at the limit, and its backlash.
fn winding(ui: &mut egui::Ui, axis: &mut cut_craft_core::rotary_axis::RotaryAxis) {
    use cut_craft_core::rotary_axis::{AtLimit, Turning};
    ui.label(cut_craft_i18n::t!("A can turn"))
        .on_hover_hint(cut_craft_i18n::t!(
            "Freely: A keeps turning one way as long as the job needs, like a lathe. Choose a \
         number of turns when a cable, a hose or a limit switch on the rotary stops it \
         winding on and on"
        ));
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut axis.turning,
            Turning::Free,
            cut_craft_i18n::t!("Freely"),
        );
        ui.radio_value(
            &mut axis.turning,
            Turning::Unwind,
            cut_craft_i18n::t!("Up to"),
        );
        ui.add_enabled(
            axis.turning == Turning::Unwind,
            Number::new(&mut axis.turns)
                .range(0.5..=1000.0)
                .speed(0.5)
                .max_decimals(1)
                .suffix(" turns each way")
                .name("Up to"),
        );
    });
    ui.end_row();
    ui.label(cut_craft_i18n::t!("At the limit"))
        .on_hover_hint(cut_craft_i18n::t!(
            "When the next pass would turn A past its turns. Cut both ways: the next pass is \
         cut the other way round, so A comes back as it cuts. Lift and spin back: the bit \
         lifts clear and A spins back whole turns before the next pass; spinning back to \
         the start also shows up any steps A has lost"
        ));
    ui.add_enabled_ui(axis.turning == Turning::Unwind, |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut axis.at_limit,
                AtLimit::CutBothWays,
                cut_craft_i18n::t!("Cut both ways"),
            );
            ui.radio_value(
                &mut axis.at_limit,
                AtLimit::SpinBack,
                cut_craft_i18n::t!("Lift and spin back"),
            );
        });
    });
    ui.end_row();
    ui.label(cut_craft_i18n::t!("Backlash"))
        .on_hover_hint(cut_craft_i18n::t!(
            "How far A turns before the material moves when it changes direction (a little \
         play in its gears or belt). The file turns that much further each time A changes \
         direction. 0 for none"
        ));
    ui.add(
        Number::new(&mut axis.backlash)
            .range(0.0..=10.0)
            .speed(0.01)
            .max_decimals(3)
            .suffix(" °")
            .name("Backlash"),
    );
    ui.end_row();
}

/// A length for a summary line: no more decimals than it needs.
fn shown(value: f64, unit: &str) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}{unit}")
}

/// The most slots a tool changer can be given here.
// ponytail: a generous ceiling for router carousels; raise it for big machining centres
const MOST_SLOTS: usize = 99;

/// Automatic Tool Changer: how many slots, and what is loaded in them. Given the tool
/// database's tools (name and bit number), each slot's Select… asks for its tool to be picked
/// in the Bit Library, × empties it, and Load by Bit Number puts each numbered tool in its
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
        ui.strong(cut_craft_i18n::t!("Automatic Tool Changer"));
        let mut has = !machine.slots.is_empty();
        if ui
            .checkbox(
                &mut has,
                cut_craft_i18n::t!("This machine has a tool changer"),
            )
            .changed()
        {
            machine.set_slot_count(usize::from(has));
        }
        if !has {
            ui.weak(cut_craft_i18n::t!(
                "Bit changes are by hand; bits are numbered in the order they are used"
            ));
            return;
        }
        let mut count = machine.slots.len();
        ui.horizontal(|ui| {
            ui.label(cut_craft_i18n::t!("Slots"));
            if ui
                .add(Number::new(&mut count).range(1..=MOST_SLOTS).name("Slots"))
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
                        None => ui.weak(cut_craft_i18n::t!("(empty)")),
                    };
                    ui.end_row();
                }
            });
            ui.weak(cut_craft_i18n::t!(
                "Load the holders in Machine ▸ Tool Changer…"
            ));
            return;
        };
        if ui
            .button(cut_craft_i18n::t!("Load by Bit Number"))
            .on_hover_hint(cut_craft_i18n::t!(
                "Each bit with a bit number goes in that holder"
            ))
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
                        None => ui.weak(cut_craft_i18n::t!("(empty)")),
                    };
                    ui.horizontal(|ui| {
                        if ui
                            .button(cut_craft_i18n::t!("Select…"))
                            .on_hover_hint(cut_craft_i18n::t!(
                                "Pick the bit for this holder in the Bit Library"
                            ))
                            .clicked()
                        {
                            select = Some(i + 1);
                        }
                        if machine.slots[i].is_some()
                            && ui
                                .small_button("×")
                                .on_hover_hint(cut_craft_i18n::t!("Empty this holder"))
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

/// Bit Changes: how a job with several bits is saved without a tool changer (a file for
/// each bit, or one file that pauses), where a pause takes the head and how long the
/// spindle gets to come up to speed, and Group by bit, which cuts each bit's toolpaths
/// together. Folded to one line saying how bits are changed.
fn bit_changes(ui: &mut egui::Ui, machine: &mut Machine) {
    let how = match (machine.slots.is_empty(), machine.hand_changes) {
        (false, _) => "by the tool changer",
        (true, HandChanges::SeparateFiles) => "a separate file for each bit",
        (true, HandChanges::Pause) => "one file that pauses",
    };
    let grouped = if machine.group_by_bit { "yes" } else { "no" };
    egui::CollapsingHeader::new(cut_craft_i18n::tr!(
        "machines-bit-changes",
        how = how,
        grouped = grouped
    ))
    .id_salt("bit_changes")
    .show(ui, |ui| {
        if machine.slots.is_empty() {
            hand_changes(ui, machine);
        } else {
            ui.weak(cut_craft_i18n::t!(
                "The tool changer changes the bits; a pause chosen in Save Toolpaths uses these:"
            ));
        }
        change_position(ui, machine);
        ui.horizontal(|ui| {
            ui.label(cut_craft_i18n::t!("Spindle spin-up after a pause"));
            ui.add(
                Number::new(&mut machine.spin_up)
                    .range(0.0..=120.0)
                    .max_decimals(1)
                    .suffix(" s")
                    .name("Spindle spin-up"),
            )
            .on_hover_hint(cut_craft_i18n::t!(
                "How long the machine waits for the spindle to come up to speed before \
                     cutting again; 0 keeps the post's own wait (3 s in PowerWhittle's posts). \
                     It is written in the units the post gives its dwells"
            ));
        });
        ui.checkbox(
            &mut machine.group_by_bit,
            cut_craft_i18n::t!("Group by bit"),
        )
        .on_hover_hint(cut_craft_i18n::t!(
            "Off (safest): toolpaths are cut in the order they are listed. On: each \
                     bit's toolpaths are cut together, for fewer bit changes, which changes \
                     the order. The checks before cutting show the new order to look over"
        ));
    });
}

/// What Save Toolpaths starts on for a job with several bits on a machine without a tool
/// changer: a file for each bit, or one file that pauses (warned on standard GRBL).
fn hand_changes(ui: &mut egui::Ui, machine: &mut Machine) {
    ui.label(cut_craft_i18n::t!(
        "When a job uses more than one bit, save it as:"
    ));
    ui.radio_value(
        &mut machine.hand_changes,
        HandChanges::SeparateFiles,
        cut_craft_i18n::t!("A separate file for each bit"),
    )
    .on_hover_hint(cut_craft_i18n::t!(
        "The safest: run the files in number order, changing the bit and setting Z zero \
         between them. Save Toolpaths starts on this choice"
    ));
    ui.radio_value(
        &mut machine.hand_changes,
        HandChanges::Pause,
        cut_craft_i18n::t!("One file that pauses at each bit change"),
    )
    .on_hover_hint(cut_craft_i18n::t!(
        "The machine stops (M0) at each change; change the bit, set Z zero, then resume. \
         Standard GRBL cannot jog while paused, so use separate files there"
    ));
    if machine.hand_changes == HandChanges::Pause && machine.cannot_jog_when_paused() {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            cut_craft_i18n::t!(
                "⚠ Standard GRBL cannot jog while paused, so setting Z zero at a pause is hard. \
             Pick the firmware above if yours is not standard GRBL"
            ),
        );
    }
}

/// Go to a bit change position at each pause, and where: X and Y from the job's X0 Y0, Z
/// above the material.
fn change_position(ui: &mut egui::Ui, machine: &mut Machine) {
    let (per, unit) = if machine.inches {
        (INCH, " in")
    } else {
        (1.0, " mm")
    };
    let mut moving = machine.change_position.is_some();
    if ui
        .checkbox(
            &mut moving,
            cut_craft_i18n::t!("Go to a bit change position at each pause"),
        )
        .on_hover_hint(cut_craft_i18n::t!(
            "Off: the bit lifts straight up to the safe Z, often over the work. On: the head \
             also goes to this spot, out of the way, to change the bit"
        ))
        .changed()
    {
        machine.change_position = moving.then(|| ChangePosition {
            z: 50.0,
            ..ChangePosition::default()
        });
    }
    let Some(position) = machine.change_position.as_mut() else {
        return;
    };
    egui::Grid::new("bit_change_position")
        .num_columns(2)
        .show(ui, |ui| {
            let across = "From the job's X0 Y0";
            let up = "Above the top of the material; never lower than the safe Z";
            for (label, value, hint) in [
                ("X", &mut position.x, across),
                ("Y", &mut position.y, across),
                ("Z", &mut position.z, up),
            ] {
                ui.label(label).on_hover_hint(hint);
                let mut shown = *value / per;
                let field = Number::new(&mut shown)
                    .speed(1.0 / per)
                    .max_decimals(3)
                    .suffix(unit)
                    .name(label);
                if ui.add(field).on_hover_hint(hint).changed() {
                    *value = shown * per;
                }
                ui.end_row();
            }
        });
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
        ui.strong(cut_craft_i18n::t!("Keep-Out Zones"))
            .on_hover_hint(cut_craft_i18n::t!(
                "Clamps fixed to the bed: no toolpath of any job passes over them. X and Y are \
             from the job's XY origin, as the G-code is"
            ));
        let mut gone = None;
        egui::Grid::new("machine_keep_out")
            .num_columns(7)
            .show(ui, |ui| {
                for heading in ["Name", "X", "Y", "Width", "Height"] {
                    ui.label(heading);
                }
                ui.label(cut_craft_i18n::t!("Tall"))
                    .on_hover_hint(cut_craft_i18n::t!(
                        "How tall the clamp stands above the top of the material, so the checks \
                     see the collet nut or spindle coming down over it beside a deep cut; 0 \
                     takes 20 mm"
                    ));
                ui.end_row();
                for (i, zone) in machine.keep_out.iter_mut().enumerate() {
                    // what its label in the views says, after "Keep-out: "
                    let hint = zone.name_at(i);
                    ui.add(
                        egui::TextEdit::singleline(&mut zone.name)
                            .hint_text(hint.clone())
                            .desired_width(ui.spacing().text_edit_width / 2.0),
                    )
                    .on_hover_hint(cut_craft_i18n::t!(
                        "What its label in the 2D and 3D views calls it"
                    ));
                    // the corner may be either side of the origin
                    let zone_name = if zone.name.is_empty() {
                        hint.clone()
                    } else {
                        zone.name.clone()
                    };
                    for (axis, value) in [("X", &mut zone.x), ("Y", &mut zone.y)] {
                        let mut shown = *value / per;
                        let drag = Number::new(&mut shown)
                            .max_decimals(3)
                            .suffix(format!(" {length}"))
                            .name(format!("{zone_name} {axis}"));
                        if ui.add(drag).changed() {
                            *value = shown * per;
                        }
                    }
                    for (what, value) in [
                        ("Width", &mut zone.width),
                        ("Height", &mut zone.height),
                        ("Tall", &mut zone.stands),
                    ] {
                        let name = format!("{zone_name} {what}");
                        measure(ui, &name, value, per, &format!(" {length}"));
                    }
                    if ui.button(cut_craft_i18n::t!("Remove")).clicked() {
                        gone = Some(i);
                    }
                    ui.end_row();
                }
            });
        if let Some(i) = gone {
            machine.keep_out.remove(i);
        }
        if ui.button(cut_craft_i18n::t!("Add Zone")).clicked() {
            machine.keep_out.push(crate::machine::Zone::default());
        }
    });
}

/// Protect the Machine: the checks that refuse a job that could damage the machine, each on
/// unless switched off here, and how deep a cut may go into the spoilboard. Folded to one
/// line saying which are on.
fn protect(ui: &mut egui::Ui, machine: &mut Machine) {
    let (per, length) = if machine.inches {
        (INCH, " in")
    } else {
        (1.0, " mm")
    };
    let off = machine.protect.switched_off();
    let summary = if off.is_empty() {
        "Protect the Machine: every check on".to_owned()
    } else {
        format!("Protect the Machine: switched off: {}", off.join(", "))
    };
    egui::CollapsingHeader::new(summary)
        .id_salt("machine_protect")
        .show(ui, |ui| {
            ui.weak(
                "Save Toolpaths, Send to Machine and the Web API refuse a job these checks \
                 find, with no Save Anyway: it could damage the machine. Switching one off \
                 here is the only way past it, and the G-code then says it was off",
            );
            let protect = &mut machine.protect;
            let rows = [
                (
                    &mut protect.keep_out,
                    "Keep-out zones",
                    "Refuse a cut into a keep-out zone, the job's or the machine's",
                ),
                (
                    &mut protect.bed,
                    "Bed size",
                    "Refuse toolpaths bigger than the bed (each tile of a tiled job on its own)",
                ),
                (
                    &mut protect.z_travel,
                    "Z travel",
                    "Refuse toolpaths going further up and down than the Z axis travels",
                ),
                (
                    &mut protect.spoilboard,
                    "Spoilboard depth",
                    "Refuse a cut deeper into the spoilboard than allowed below",
                ),
            ];
            for (on, label, hint) in rows {
                ui.checkbox(on, label).on_hover_hint(hint);
            }
            ui.horizontal(|ui| {
                let label = "Allowed into spoilboard";
                ui.label(label).on_hover_hint(
                    "How far below the material a cut may go into the spoilboard; deeper is \
                     refused while Spoilboard depth is ticked",
                );
                measure(ui, label, &mut protect.spoilboard_depth, per, length);
            });
            ui.checkbox(
                &mut protect.warn_spoilboard,
                "Warn about cuts more than 1 mm into the spoilboard",
            )
            .on_hover_hint(
                "Within the depth allowed, a cut more than 1 mm below the material is a \
                 warning to read and tick: a wrong material thickness shows first this way",
            );
        });
}

/// G-code Files: where the machine's programs are saved on this computer, and the folder
/// the machine's own files are in when the computer can see it (a wireless controller's shared
/// card, or its card or stick plugged in here), which Send to Machine saves straight to.
fn g_code_folders(ui: &mut egui::Ui, machine: &mut Machine) -> Option<MachineOutcome> {
    let mut asked = None;
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.strong(cut_craft_i18n::t!("G-code Files"))
            .on_hover_hint(cut_craft_i18n::t!(
                "Each project's G-code goes in a folder of its own, named after the project"
            ));
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
                        let browse = ui.button(cut_craft_i18n::t!("Browse…"));
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
                    cut_craft_i18n::t!("Short names (8 letters) for controllers that need them"),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "Some older controllers only read names of up to 8 letters off their card. \
                     Files that would end up with the same 8 letters get a number instead \
                     (Oaksign2.nc), so none overwrites another. Leave this off for FluidNC and \
                     most WiFi controllers"
                ));
                ui.end_row();
            });
        let can = machine.send_to.is_set();
        let manage = ui
            .add_enabled(can, egui::Button::new(cut_craft_i18n::t!("Machine Files…")))
            .on_hover_hint(cut_craft_i18n::t!(
                "See, upload, rename and delete the files on the machine"
            ))
            .on_disabled_hover_text(cut_craft_i18n::t!("Set the machine's files folder first"));
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
                .checkbox(
                    &mut has,
                    cut_craft_i18n::t!("This machine has a drill bank"),
                )
                .on_hover_hint(cut_craft_i18n::t!(
                    "A head with several drills side by side that drop down together, so a \
                     row of shelf-pin or dowel holes is drilled in one plunge. Most hobby \
                     machines have none"
                ))
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
                ui.label(cut_craft_i18n::t!("Drills along X"))
                    .on_hover_hint(cut_craft_i18n::t!("How many drills are on the arm running left to right"));
                let a = ui.add(Number::new(&mut across).range(0..=32).name("Drills along X"));
                ui.end_row();
                ui.label(cut_craft_i18n::t!("Drills along Y"))
                    .on_hover_hint(cut_craft_i18n::t!("How many drills are on the arm running front to back; 0 for a single row"));
                let b = ui.add(Number::new(&mut along).range(0..=32).name("Drills along Y"));
                ui.end_row();
                if a.changed() || b.changed() {
                    bank.set_counts(across, along);
                }
                ui.label(cut_craft_i18n::t!("Pitch"))
                    .on_hover_hint(cut_craft_i18n::t!("The distance between neighbouring drills, centre to centre"));
                measure(ui, "Pitch", &mut bank.pitch, per, &format!(" {length}"));
                ui.end_row();
                if !bank.horizontal.is_empty() && !bank.vertical.is_empty() {
                    ui.label(cut_craft_i18n::t!("Arms cross at")).on_hover_hint(cut_craft_i18n::t!("The drill both arms share: its place along X, then along Y, counted from 1"),
                    );
                    ui.horizontal(|ui| {
                        let limits = [bank.horizontal.len(), bank.vertical.len()];
                        let arms = ["Arms cross at X", "Arms cross at Y"];
                        for ((place, most), arm) in bank.central.iter_mut().zip(limits).zip(arms) {
                            let mut shown = *place + 1;
                            if ui
                                .add(Number::new(&mut shown).range(1..=most).name(arm))
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
                    cut_craft_i18n::tr!("machines-both-arms", name = name.clone())
                } else {
                    name.clone()
                });
                ui.horizontal(|ui| {
                    let drill = match id.arm {
                        Arm::Horizontal => &mut bank.horizontal[id.index],
                        Arm::Vertical => &mut bank.vertical[id.index],
                    };
                    ui.add(Number::new(&mut drill.number).name(format!("Drill {name} number")))
                        .on_hover_hint(cut_craft_i18n::t!(
                            "The number the controller picks this drill by; controllers that \
                             take a bitmask expect 1, 2, 4, 8..."
                        ));
                    if crossing {
                        let y = &mut bank.vertical[bank.central[1]].number;
                        ui.add(
                            Number::new(y)
                                .prefix("Y ")
                                .name(format!("Drill {name} Y arm number")),
                        )
                        .on_hover_hint(cut_craft_i18n::t!(
                            "Its number on the Y arm, for controllers that number the \
                                 arms apart"
                        ));
                    }
                });
                let drill = match id.arm {
                    Arm::Horizontal => &mut bank.horizontal[id.index],
                    Arm::Vertical => &mut bank.vertical[id.index],
                };
                measure(
                    ui,
                    &format!("Drill {name} diameter"),
                    &mut drill.diameter,
                    per,
                    &format!(" {length}"),
                );
                let is_reference = bank.same(bank.reference, *id);
                if ui
                    .radio(is_reference, "")
                    .on_hover_hint(cut_craft_i18n::t!(
                        "The drill the machine positions by: the G-code's X and Y are where \
                     this drill is"
                    ))
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

#[cfg(test)]
mod tests;
