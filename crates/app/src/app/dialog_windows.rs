//! The app's smaller windows: Tool Changer, Save Toolpaths, Job Sheet, Part Labels, Clearing, Save
//! STL, previews and saving straight away.

use super::views::{TICK_TO_SAVE, files_section, post_combo, tool_changes_section};
use super::{CutCraftApp, Drag, EntityId, Info, Path, dialog_size, egui, saving_path};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// Machine ▸ Tool Changer…: the active machine's tool changer, each holder loaded with a
    /// tool from the Tool Database, or all at once by their tool numbers.
    pub(super) fn tool_changer_window(&mut self, ctx: &egui::Context) {
        if !self.tool_changer_window {
            return;
        }
        // a tool changer holds My Tools only, not a catalogue's
        let tools: Vec<(String, u32)> = self
            .tools
            .my_tools()
            .map(|t| (t.name.clone(), t.number))
            .collect();
        let mut open = true;
        let (mut changed, mut select) = (false, None);
        egui::Window::new(tr!("views-tool-changer"))
            .id(egui::Id::unique("tool_changer"))
            .collapsible(false)
            .open(&mut open)
            .show(ctx, |ui| {
                cut_craft_bug_report::mark(ui, "Tool Changer");
                let Some(machine) = self.machines.active_mut() else {
                    ui.weak(tr!("views-no-machine-yet-add-one"));
                    return;
                };
                ui.strong(&machine.name);
                (changed, select) = cut_craft_machines::tool_changer(ui, machine, Some(&tools));
            });
        self.tool_changer_window = open;
        if changed {
            self.tool_changer_changed();
        }
        // a holder's Select…: Select Tool, on the tool in it; its Select loads the holder
        if let Some(slot) = select {
            let current = self
                .machines
                .active()
                .and_then(|m| m.slots.get(slot - 1).cloned().flatten())
                .unwrap_or_default();
            self.select_tool(&current);
            self.picking = Some(super::PickFor::Holder(slot));
        }
    }

    /// Paths `id` would create for `drag`, by running the command itself.
    pub(super) fn preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        let (Some(cmd), Some(form)) = (self.registry.get(id), &self.form) else {
            return vec![];
        };
        let mut paths: Vec<Path> = cmd
            .drag_params(form.way.apply(drag), &self.with_stroke(id, &form.params))
            .and_then(|params| cmd.run_json(&self.doc, &params))
            .map_or_default(|change| change.added().cloned().collect());
        // Freehand's stabilizing circle, round the path's end: the pointer pulls from its edge
        if id == "draw_freehand"
            && let Some(radius) = form.params["stabilize"].as_f64().filter(|r| *r > 0.0)
            && let Some(&end) = cmd_draw_freehand::stabilized(&self.stroke, radius, false).last()
        {
            paths.push(Path::circle(end, radius));
        }
        paths
    }

    /// The quick engrave toolpath Immediate Output saves: the one the form is editing, else
    /// the newest.
    pub(super) fn immediate_output_toolpath(&self) -> Option<EntityId> {
        let editing = self.form.as_ref().and_then(|f| f.editing);
        editing.or_else(|| {
            self.doc
                .toolpaths
                .iter()
                .rev()
                .find(|(_, t)| t.strategy == "quick_engrave_toolpath")
                .map(|(&id, _)| id)
        })
    }

    /// Quick Engrave's Save Toolpath Now: that toolpath alone, through the post picked on
    /// the form, with Save Toolpaths' checks and file dialog.
    pub(super) fn immediate_save(&mut self) {
        let Some(id) = self.immediate_output_toolpath() else {
            return;
        };
        self.selected_toolpath = Some(id);
        self.selected_group = None;
        let form = self.immediate_output.clone();
        self.save_toolpaths(&form);
    }

    /// The Help windows, Toolpath Summary and Snapping Options.
    pub(super) fn open_window(&mut self, id: &str) {
        self.info = match id {
            "keyboard_shortcuts" => Some(Info::Shortcuts),
            "third_party_licenses" => Some(Info::Licenses),
            "toolpath_summary" => Some(Info::Summary),
            "about_cut_craft" => Some(Info::About),
            "help_contents" => Some(Info::Help("contents")),
            _ => {
                self.snap_window = true;
                return;
            }
        };
    }

    /// Save Toolpaths: the machine and post, which toolpaths go in which file, and Save.
    pub(super) fn save_toolpaths_window(&mut self, ctx: &egui::Context) {
        use crate::files::SaveMode;
        let Some(mut form) = self.save_form.clone() else {
            return;
        };
        let mut open = true;
        let mut pressed = (false, false, None, false);
        let mut machine_changed = None;
        let choices = self.post_choices();
        let default = self.output_post().map(|p| p.name);
        let post = self.form_post(&form);
        let plan = post
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|p| self.plan(&form, p));
        let split = match &post {
            Ok(p) if super::output::split_for(&form, p) => Some(if p.changes_tools() {
                "Each tool gets a file of its own. Run them in order, changing the tool \
                 between files."
            } else {
                "This post-processor cannot change tools, so each tool gets a file of its own. \
                 Run them in order, changing the tool between files."
            }),
            _ => None,
        };
        let changer = self.has_tool_changer();
        // `None` while the checks run, or once they were cancelled
        let found = match &plan {
            Ok(plan) => self.save_checks(plan, form.changes).map(<[_]>::to_vec),
            Err(_) => Some(vec![]),
        };
        let skipped = plan
            .as_ref()
            .map_or_else(|_| vec![], |p| self.not_checked(p, form.changes));
        let mut again = false;
        let (id, size) = (egui::Id::unique("save_toolpaths"), dialog_size(ctx));
        cut_craft_bug_report::dialog(ctx, id, "Save Toolpaths", size, &mut open, |ui| {
            ui.strong(tr!("views-machine-and-post-processor"));
            if !self.machines.machines.is_empty() {
                let active = self.machines.active;
                let shown = self.machines.active().map_or("", |m| m.name.as_str());
                egui::ComboBox::from_label("Machine")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (n, m) in self.machines.machines.iter().enumerate() {
                            if ui.selectable_label(n == active, &m.name).clicked() && n != active {
                                machine_changed = Some(n);
                            }
                        }
                    });
            }
            post_combo(ui, "save_post", &mut form.post, &default, &choices);
            ui.separator();
            ui.strong(tr!("views-toolpaths-to-save"));
            for (mode, label) in [
                (SaveMode::Selected, "Selected toolpath"),
                (SaveMode::VisibleToOneFile, "Visible toolpaths to one file"),
                (
                    SaveMode::VisibleToMultipleFiles,
                    "Visible toolpaths to multiple files",
                ),
            ] {
                ui.radio_value(&mut form.mode, mode, label);
            }
            ui.add_enabled(
                form.mode == SaveMode::VisibleToMultipleFiles,
                egui::Checkbox::new(&mut form.group, tr!("views-merge-toolpaths-in-a-row")),
            )
            .on_hover_hint(tr!("views-toolpaths-next-to-each-other"));
            tool_changes_section(ui, &mut form.changes, changer);
            if self.doc.job.double_sided.is_some() {
                ui.checkbox(&mut form.add_side, tr!("views-add-side-to-toolpath-name"));
            }
            ui.checkbox(&mut form.job_sheet, tr!("views-save-a-job-sheet-with"))
                .on_hover_hint(tr!("views-the-project-s-sheet-to"));
            ui.separator();
            files_section(ui, &self.doc, &plan, split);
            ui.separator();
            again = self.check_list(ui, found.as_deref(), &skipped);
            ui.add_space(ui.spacing().item_spacing.y);
            // ticked in the list this frame counts at once
            let unticked = found.as_deref().is_some_and(|f| !self.all_acknowledged(f));
            let ready = plan.is_ok() && found.is_some() && !unticked;
            pressed = self.save_buttons(ui, ready, unticked);
        });
        if again {
            self.check_again();
        }
        if let Some(n) = machine_changed {
            self.machines.active = n;
            form.post = None;
            form.changes = self.default_changes();
            self.machines_changed("Machine changed");
        }
        self.pressed(&form, pressed);
        self.save_form = (open && self.save_after.is_none()).then_some(form);
    }

    /// Does what Save Toolpaths' buttons asked (as [`Self::save_buttons`] says).
    fn pressed(
        &mut self,
        form: &super::SaveForm,
        (save, send, manage, view): (
            bool,
            bool,
            Option<(String, cut_craft_machines::send::SendTo)>,
            bool,
        ),
    ) {
        if view {
            self.view_gcode(form.clone());
        }
        if save || send {
            // the form is a window of its own: it closes first, so the file dialog and the
            // questions saving asks are not hidden behind it, and comes back once they are
            // answered (a 0.6.11 user found the save dialog behind it)
            self.save_after = Some(super::gcode_view::SaveAfter::new(form.clone(), send));
        }
        if let Some((name, send_to)) = manage {
            self.open_machine_files(&name, &send_to);
        }
    }

    /// Save Toolpaths' buttons: Save Toolpaths, then Send to Machine (greyed until the
    /// machine has a files folder) and Machine Files; which was pressed, Machine Files with
    /// the machine's name and folders.
    fn save_buttons(
        &self,
        ui: &mut egui::Ui,
        ready: bool,
        unticked: bool,
    ) -> (
        bool,
        bool,
        Option<(String, cut_craft_machines::send::SendTo)>,
        bool,
    ) {
        let (mut save, mut send, mut manage, mut view) = (false, false, None, false);
        ui.horizontal(|ui| {
            let response = ui.add_enabled(ready, egui::Button::new(tr!("views-save-toolpaths")));
            save = if unticked {
                response.on_disabled_hover_text(TICK_TO_SAVE).clicked()
            } else {
                response.clicked()
            };
            let machine = self.machines.active();
            let reachable = machine.is_some_and(|m| m.send_to.is_set());
            let response = ui
                .add_enabled(
                    ready && reachable,
                    egui::Button::new(tr!("views-send-to-machine")),
                )
                .on_hover_hint(tr!("views-save-the-files-straight-onto"));
            send = if reachable && unticked {
                response.on_disabled_hover_text(TICK_TO_SAVE).clicked()
            } else if reachable {
                response.clicked()
            } else {
                response
                    .on_disabled_hover_text(super::send::NO_MACHINE_FOLDER)
                    .clicked()
            };
            if reachable && ui.button(tr!("views-machine-files")).clicked() {
                manage = machine.map(|m| (m.name.clone(), m.send_to.clone()));
            }
            view = ui
                .button(super::panels::VIEW_GCODE)
                .on_hover_hint(super::panels::VIEW_GCODE_ABOUT)
                .clicked();
        });
        (save, send, manage, view)
    }

    /// Gadgets ▸ Job Setup Sheet Editor: the job sheets' title, colours, logo and sections,
    /// kept with the options.
    pub(super) fn job_sheet_window(&mut self, ctx: &egui::Context) {
        if !self.job_sheet_window {
            return;
        }
        let mut open = true;
        let schema = serde_json::to_value(schemars::schema_for!(fmt_create_job_sheet::Style))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(&self.options.job_sheet).expect("a style serialises");
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("job_sheet_editor"), dialog_size(ctx));
        let mut changed = None;
        let mut pick_logo = false;
        cut_craft_bug_report::dialog(ctx, id, "Job Setup Sheet Editor", size, &mut open, |ui| {
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
                group_end: None,
                materials: None,
                pattern: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(style) = serde_json::from_value(values)
            {
                changed = Some(style);
            }
            ui.horizontal(|ui| {
                pick_logo = ui.button(tr!("views-change-logo")).clicked();
                let has_logo = !self.options.job_sheet.logo.is_empty();
                if ui
                    .add_enabled(has_logo, egui::Button::new(tr!("views-remove-logo")))
                    .clicked()
                {
                    let mut style = self.options.job_sheet.clone();
                    style.logo.clear();
                    changed = Some(style);
                }
                if ui.button(tr!("views-restore-default-sheet")).clicked() {
                    changed = Some(fmt_create_job_sheet::Style::default());
                }
            });
            ui.weak(tr!("views-a-logo-is-scaled-to"));
        });
        if pick_logo
            && let Some(path) =
                super::filtered_dialog(super::Dialog::JobSheetLogo, &self.registry).pick_file()
        {
            let ext = path
                .extension()
                .map_or_default(|e| e.to_string_lossy().into_owned());
            match std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| fmt_create_job_sheet::logo(&bytes, &ext))
            {
                Ok(logo) => {
                    let mut style = self.options.job_sheet.clone();
                    style.logo = logo;
                    changed = Some(style);
                }
                Err(e) => self.status.set_message(&e),
            }
        }
        if let Some(style) = changed {
            self.options.job_sheet = style;
            self.save_options();
        }
        self.job_sheet_window = open;
    }

    /// Toolpaths ▸ Print Part Labels: the labels' size and what they show (kept with the
    /// options), how many parts there are, and Open Labels to Print.
    pub(super) fn part_labels_window(&mut self, ctx: &egui::Context) {
        if !self.part_labels_window {
            return;
        }
        let mut open = true;
        let schema = serde_json::to_value(schemars::schema_for!(fmt_part_labels::Style))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values =
            serde_json::to_value(&self.options.part_labels).expect("a style serialises");
        let units = self.doc.job.units;
        let count = fmt_cut_list::parts(&self.doc).len();
        let (id, size) = (egui::Id::unique("part_labels"), dialog_size(ctx));
        let (mut changed, mut print) = (None, false);
        cut_craft_bug_report::dialog(ctx, id, "Print Part Labels", size, &mut open, |ui| {
            ui.label(tr!("views-a-label-for-each-part"));
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
                group_end: None,
                materials: None,
                pattern: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(style) = serde_json::from_value(values)
            {
                changed = Some(style);
            }
            ui.separator();
            if count == 0 {
                ui.label(tr!("views-the-job-has-no-parts"));
            } else {
                ui.label(tr!("views-labels-count", count = count));
            }
            print = ui
                .add_enabled(
                    count > 0,
                    egui::Button::new(tr!("views-open-labels-to-print")),
                )
                .on_hover_text(tr!("views-opens-the-labels-in-your"))
                .clicked();
            ui.weak(tr!("views-on-a-label-printer-turn"));
        });
        if let Some(style) = changed {
            self.options.part_labels = style;
            self.save_options();
        }
        if print {
            self.open_part_labels();
        }
        self.part_labels_window = open;
    }

    /// Toolpaths ▸ Clearing Settings (or Clearing Settings… in a Cutting window): the job's
    /// light step, loops and way back, with pictures. A change is an edit of the job, so it
    /// can be undone and marks the toolpaths it shapes out of date; Use as My Default keeps
    /// it for new jobs, Reset to My Defaults takes that back.
    pub(super) fn clearing_window(&mut self, ctx: &egui::Context) {
        let asked = super::cutting::clearing_window_id();
        if ctx
            .data_mut(|d| d.remove_temp::<bool>(asked))
            .unwrap_or(false)
        {
            self.clearing_window = true;
        }
        if !self.clearing_window {
            return;
        }
        let mut open = true;
        // the open form's bit, else a common 1/4" one, for the sizes and pictures
        let diameter = self
            .form
            .as_ref()
            .and_then(|f| f.params["tool"]["diameter"].as_f64())
            .filter(|d| *d > 0.0)
            .unwrap_or(6.35);
        let units = self.doc.job.units;
        let mut clearing = self.doc.job.clearing;
        let mine = self.options.default_job.map_or_default(|j| j.clearing);
        let (mut keep, mut reset) = (false, false);
        let id = egui::Id::unique("clearing_settings");
        cut_craft_bug_report::dialog(
            ctx,
            id,
            "Clearing Settings",
            dialog_size(ctx),
            &mut open,
            |ui| {
                ui.label(tr!("views-how-the-light-cut-clearing"));
                // where they are seen without scrolling
                ui.horizontal(|ui| {
                    keep = ui
                        .add_enabled(
                            clearing != mine,
                            egui::Button::new(tr!("views-use-as-my-default")),
                        )
                        .on_hover_hint(tr!("views-new-jobs-start-with-these"))
                        .clicked();
                    reset = ui
                        .add_enabled(
                            clearing != mine,
                            egui::Button::new(tr!("views-reset-to-my-defaults")),
                        )
                        .on_hover_hint(tr!("views-back-to-the-settings-new"))
                        .clicked();
                });
                ui.separator();
                super::cutting::clearing_settings(ui, &mut clearing, diameter, units);
            },
        );
        if keep {
            self.options
                .default_job
                .get_or_insert_with(cut_craft_core::document::Job::default)
                .clearing = clearing;
            self.save_options();
        }
        if reset {
            clearing = mine;
        }
        // shown as a slider is dragged, made one edit when it is let go
        let dragging = ctx.input(|i| i.pointer.any_down()) && !reset;
        if clearing != self.doc.job.clearing {
            if dragging {
                self.clearing_before.get_or_insert(self.doc.job.clearing);
                self.doc.job.clearing = clearing;
            } else {
                self.set_clearing(clearing);
            }
        }
        if !dragging && let Some(before) = self.clearing_before.take() {
            let now = std::mem::replace(&mut self.doc.job.clearing, before);
            self.set_clearing(now);
        }
        self.clearing_window = open;
    }

    /// The job's Clearing Settings become `clearing`, as an edit that can be undone.
    pub(super) fn set_clearing(&mut self, clearing: cut_craft_core::entry::ClearingSettings) {
        if clearing == self.doc.job.clearing {
            return;
        }
        let mut job = self.doc.job;
        job.clearing = clearing;
        let mut c = cut_craft_core::Change::builder(&self.doc);
        c.set_job(job);
        let change = c.finish();
        if let Err(e) = self.history.apply(&mut self.doc, change) {
            self.status.set_message(&e.to_string());
        }
    }

    /// Export as STL: how the model is triangulated (kept with the options), Triangulate to
    /// see the triangle count and error, and Save STL… to write it.
    pub(super) fn stl_window(&mut self, ctx: &egui::Context) {
        if !self.stl_window {
            return;
        }
        let result = self.stl_result;
        let mut open = true;
        let schema = serde_json::to_value(schemars::schema_for!(fmt_export_as_stl::Settings))
            .expect("a schema serialises");
        let fields = crate::form::fields(&schema);
        let mut values = serde_json::to_value(self.options.stl).expect("settings serialise");
        let units = self.doc.job.units;
        let (id, size) = (egui::Id::unique("export_as_stl"), dialog_size(ctx));
        let (mut changed, mut triangulate, mut save) = (None, false, false);
        cut_craft_bug_report::dialog(ctx, id, "Export Model as STL", size, &mut open, |ui| {
            let picture = |_: &str| None;
            let look = crate::form::Look {
                units,
                picture: &picture,
                material: crate::form::material(&self.doc, units),
                tool: None,
                group_end: None,
                materials: None,
                pattern: None,
            };
            if crate::form::show(ui, &fields, &mut values, &look)
                && let Ok(settings) = serde_json::from_value(values)
            {
                changed = Some(settings);
            }
            ui.horizontal(|ui| {
                triangulate = ui.button(tr!("views-triangulate")).clicked();
                save = ui.button(tr!("views-save-stl")).clicked();
            });
            if let Some((count, error)) = result {
                ui.label(tr!("views-triangles-made", count = count));
                ui.label(tr!(
                    "views-maximum-error",
                    error = super::length(error, units)
                ));
            }
        });
        if let Some(settings) = changed {
            self.options.stl = settings;
            self.stl_result = None;
            self.save_options();
        }
        if triangulate || save {
            match fmt_export_as_stl::triangulate(&self.doc, &self.options.stl) {
                Ok(mesh) => {
                    self.stl_result = Some((mesh.triangles.len(), mesh.error));
                    if save {
                        self.save_stl(&mesh.triangles);
                    }
                }
                Err(e) => self.status.set_message(&e.to_string()),
            }
        }
        self.stl_window = open;
    }

    fn save_stl(&mut self, triangles: &[[cut_craft_core::glam::DVec3; 3]]) {
        let Some(path) = super::filtered_dialog(super::Dialog::SaveStl, &self.registry)
            .save_file()
            .and_then(|path| saving_path(&path, &["stl"]))
        else {
            return;
        };
        let result = std::fs::write(&path, fmt_export_as_stl::binary_stl(triangles));
        self.status.set_message(&result.map_or_else(
            |e| format!("{}: {e}", path.display()),
            |()| format!("Exported {}", path.display()),
        ));
    }
}
