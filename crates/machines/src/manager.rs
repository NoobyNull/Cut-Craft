//! The Machine menu's windows, laid out as Aspire's: Machine Configuration Management (the
//! machines, each one's details and associated posts; kept on Apply or OK) and Post-Processor
//! Management (the post library: view, customise, install, delete).

use crate::library::{Entry, Library, Source};
use crate::machine::{Machine, Machines};
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
        .on_hover_text(label)
        .on_disabled_hover_text(label)
        .clicked()
}

/// A post's instructions, read-only, as Aspire's Post-Processor Content.
fn viewer(ctx: &egui::Context, viewing: &mut Option<(String, String)>) {
    let Some((name, text)) = viewing else { return };
    let mut open = true;
    egui::Window::new(format!("Post-Processor Content: {name}"))
        .id(egui::Id::unique("post_processor_content"))
        .open(&mut open)
        .show(ctx, |ui| {
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
    Some((entry.name.clone(), Library::text(entry).ok()?))
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
        egui::Window::new("Machine Configuration Management")
            .id(machine_window())
            .open(&mut open)
            .default_width(ctx.content_rect().width() * WINDOW_SHARE)
            .collapsible(false)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Machine Configuration Management");
                self.machine_row(ui, look);
                ui.separator();
                let names_clash = self.draft.names_clash();
                let Some(machine) = self.draft.active_mut() else {
                    ui.weak("No machines yet: Add Machine makes one");
                    outcome = buttons(ui, false);
                    return;
                };
                details(ui, machine);
                tool_changer(ui, machine);
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
            if icon_button(ui, look, "common/add.svg", "Add Machine", true) {
                self.draft.add();
                self.picked = None;
            }
            if icon_button(
                ui,
                look,
                "common/copy_duplicate.svg",
                "Duplicate Machine",
                some,
            ) {
                self.draft.duplicate();
            }
            if icon_button(ui, look, "common/delete.svg", "Delete Machine", some) {
                self.draft.delete();
                self.picked = None;
            }
            // ponytail: machine packages come from Vectric's online service, which Cut Craft
            // has none of; machines are made by hand
            icon_button(
                ui,
                look,
                "common/open.svg",
                "Install Offline Package (needs an online machine service)",
                false,
            );
        });
    }
}

/// Name, maker, model, controller and bed size, in a framed block.
fn details(ui: &mut egui::Ui, machine: &mut Machine) {
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
                ] {
                    ui.label(label);
                    let mut shown = *value / per;
                    let speed = (shown.abs() / 100.0).max(f64::EPSILON);
                    let drag = egui::DragValue::new(&mut shown)
                        .speed(speed)
                        .range(0.0..=f64::MAX)
                        .max_decimals(3)
                        .suffix(unit);
                    if ui.add(drag).changed() {
                        *value = shown * per;
                    }
                    ui.end_row();
                }
                ui.label("Units");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut machine.inches, false, "mm");
                    ui.radio_value(&mut machine.inches, true, "inches");
                });
                ui.end_row();
            });
    });
}

/// The most slots a tool changer can be given here.
// ponytail: a generous ceiling for router carousels; raise it for big machining centres
const MOST_SLOTS: usize = 99;

/// Automatic Tool Changer: how many slots, and what is loaded in them (tools are loaded in
/// the Tool Database).
fn tool_changer(ui: &mut egui::Ui, machine: &mut Machine) {
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
                .add(egui::DragValue::new(&mut count).range(1..=MOST_SLOTS))
                .changed()
            {
                machine.set_slot_count(count);
            }
        });
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
        ui.weak("Load tools into the slots in the Tool Database");
    });
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
                row.on_hover_text("Not in the post library")
            } else {
                row
            };
            if row.clicked() {
                *picked = Some(i);
            }
            row.context_menu(|ui| {
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
            if icon_button(ui, look, "common/delete.svg", "Remove", picked.is_some())
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

/// Apply on its own line, then OK and Cancel, as Aspire's form has them.
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
        egui::Window::new("Post-Processor Management")
            .open(&mut open)
            .default_width(ctx.content_rect().width() * WINDOW_SHARE)
            .collapsible(false)
            .show(ctx, |ui| {
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
                    if icon_button(ui, look, "common/open.svg", "Install Post-Processor", true) {
                        outcome = Some(PostOutcome::Install);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            outcome = Some(PostOutcome::Close);
                        }
                        let picked = self.picked.clone();
                        let select = ui
                            .add_enabled(picked.is_some(), egui::Button::new("Select"))
                            .on_hover_text("Use it with the active machine");
                        if select.clicked()
                            && let Some(name) = picked
                        {
                            outcome = Some(PostOutcome::Select(name));
                        }
                    });
                });
            });
        viewer(ctx, &mut self.viewing);
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
            match (entry.is_custom(), (look.picture)("common/edit.svg")) {
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
            if let Some(why) = &entry.broken {
                ui.colored_label(ui.visuals().error_fg_color, "⚠")
                    .on_hover_text(why);
            }
            // Aspire's Inspect icon at the row's end, and the same menu on a right-click
            let button = ui.menu_button("Inspect ▾", |ui| self.inspect(ui, library, entry));
            if let Some(Some(a)) = button.inner {
                asked = Some(a);
            }
            label.context_menu(|ui| {
                if let Some(a) = self.inspect(ui, library, entry) {
                    asked = Some(a);
                }
            });
        });
        asked
    }

    /// Inspect: View and Customize for Cut Craft's posts; View, Open File Location and
    /// Delete for custom ones.
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
        match &entry.source {
            Source::BuiltIn(_) => {
                if ui.button("Customize").clicked() {
                    asked = Some(match library.customize(entry) {
                        Ok(copy) => {
                            self.picked = Some(copy.name.clone());
                            PostOutcome::Message(format!("Customised as {}", copy.name))
                        }
                        Err(e) => PostOutcome::Message(e),
                    });
                    ui.close();
                }
            }
            Source::Custom(path) => {
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
                egui::vec2(1200.0, 800.0),
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
            window.height() < 600.0,
            "fits its contents, not the screen: {window:?}"
        );
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
