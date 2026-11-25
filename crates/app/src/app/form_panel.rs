//! A tool form's frame: its buttons, saved settings, heading and the errors and warnings at its
//! foot.

use super::toolpath_output::KEEP_OUT_REFUSED;
use super::{
    Align, CutCraftApp, Layout, Page, Value, commit, egui, files, form, form_settings_dialog,
    saving_path,
};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::Form;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// The open form's commit button, its label and hint, if pressing it would do something:
    /// OK for a new job's setup, else named for what it does ([`commit`]).
    fn commit_button(&self) -> Option<(String, String)> {
        let open = self.form.as_ref()?;
        if open.new_job {
            return Some((tr!("job-setup-ok"), tr!("job-setup-ok.tooltip")));
        }
        let drawing = self.drawing_tool().is_some();
        let made = self.drawn_shape().is_some() || self.remade_last().is_some();
        // opened on something it made (Embellish on its embellishment, E): pressing it
        // changes that one
        let reopened = self
            .registry
            .get(open.id)
            .is_some_and(|c| c.reopens(&self.doc));
        commit::commit(open.kind, open.page, drawing, made || reopened).map(|c| {
            let again = open.editing.is_some() || open.made.is_some();
            (c.label(), commit::hint(c, drawing, again))
        })
    }

    /// The open form's buttons: OK and Cancel for a new job's setup; else its commit button
    /// when pressing it would do something (named for what it does, [`commit`]), Close and
    /// Cancel.
    pub(super) fn form_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        if open.id == "orientate_3d_model" {
            return self.import_buttons(ui);
        }
        let new_job = open.new_job;
        let committing = self.commit_button();
        let drawing = self.drawing_tool().is_some();
        let made = self.drawn_shape().is_some() || self.remade_last().is_some();
        let close_key = if new_job {
            "job-setup-cancel"
        } else {
            "form-close"
        };
        let job_setup = open.id == "job_setup";
        // what the form has done that Cancel would take back
        let done = self.history.undo_depth() > open.undo_from + open.list_steps.len();
        let (mut apply, mut close, mut default, mut cancel) = (false, false, false, false);
        let mut new_shape = false;
        let as_new_offered = self.offers_calculate_as_new();
        let (save, load) = if open.page == Page::Toolpaths {
            settings_file_row(ui)
        } else {
            (false, false)
        };
        if committing.is_none() {
            new_shape = live_row(ui, open.kind, drawing, made);
        }
        let as_new = as_new_offered && calculate_as_new_row(ui);
        let mut reset = false;
        ui.horizontal(|ui| {
            if let Some((label, hint)) = &committing {
                apply = marked_commit_button(ui, label, hint);
            }
            reset = reset_button(ui, open.id);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                close = ui
                    .button(tr!(close_key))
                    .on_hover_hint(tr!(&format!("{close_key}.tooltip")))
                    .clicked();
                if !new_job {
                    cancel = ui
                        .add_enabled(done, egui::Button::new(tr!("form-cancel")))
                        .on_hover_hint(tr!("form-cancel.tooltip"))
                        .on_disabled_hover_text(tr!("form-cancel.disabled"))
                        .clicked();
                }
                if job_setup {
                    default = ui
                        .button(tr!("job-setup-set-as-default"))
                        .on_hover_hint(tr!("job-setup-set-as-default.tooltip"))
                        .clicked();
                }
            });
        });
        if new_shape {
            if let Some(form) = self.form.as_mut() {
                (form.drawn, form.applied) = (None, None);
            }
            return;
        }
        if as_new {
            self.calculate_as_new();
            return;
        }
        if reset {
            self.reset_form();
            return;
        }
        if default {
            self.job_setup_default();
            return;
        }
        if cancel {
            self.cancel_form();
            return;
        }
        if save || load {
            self.form_settings_file(save);
            return;
        }
        match (new_job, apply, close) {
            (true, true, _) => self.job_setup_ok(),
            (true, _, true) => self.job_setup_cancel(),
            (false, true, _) => {
                let (id, params) = (open.id, open.params.clone());
                self.calculate_checked(id, &params);
            }
            (false, _, true) => {
                let page = open.page;
                self.form = None;
                self.pages.done_with(page);
            }
            _ => {}
        }
    }

    /// Save Settings… (or Load Settings… when not `save`) on a toolpath's form.
    fn form_settings_file(&mut self, save: bool) {
        let Some(form) = &self.form else { return };
        let dialog = form_settings_dialog();
        if save {
            let Some(path) = dialog
                .set_file_name(format!("{}.{}", form.label, files::FORM_SETTINGS_EXTENSION))
                .save_file()
                .and_then(|path| saving_path(&path, &[files::FORM_SETTINGS_EXTENSION]))
            else {
                return;
            };
            self.save_form_settings(&path);
        } else if let Some(path) = dialog.pick_file() {
            self.load_form_settings(&path);
        }
    }

    /// The open form's settings saved to `path`, saying so in the status bar.
    pub(super) fn save_form_settings(&mut self, path: &std::path::Path) {
        let Some(form) = &self.form else { return };
        let bytes = files::form_settings(form.id, form.label, &form.params);
        let message = match cut_craft_core::file::write_atomic(path, &bytes) {
            Ok(()) => format!("Saved the settings as {}", path.display()),
            Err(e) => format!("Could not save the settings to {}: {e}", path.display()),
        };
        self.status.set_message(&message);
    }

    /// The open form filled from the settings saved at `path`, saying so in the status bar.
    pub(super) fn load_form_settings(&mut self, path: &std::path::Path) {
        let Some(form) = &mut self.form else { return };
        let loaded = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                files::load_form_settings(form.id, form.label, &mut form.params, &bytes)
            });
        let message = match loaded {
            Ok(()) => {
                form.dirty = true;
                format!("Loaded the settings from {}", path.display())
            }
            Err(e) => format!("Could not load {}: {e}", path.display()),
        };
        self.status.set_message(&message);
    }

    /// Import 3D Model's buttons: Import keeps the model as placed and closes; Cancel takes
    /// back what the form made and closes.
    fn import_buttons(&mut self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        let page = open.page;
        // only a model that was placed can be kept: after a failed placement the form stays,
        // saying what went wrong, for new settings or Cancel
        let placed = open.error.is_none()
            && open
                .drawn
                .is_some_and(|id| self.doc.components.contains_key(&id));
        let (mut import, mut cancel) = (false, false);
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                cancel = ui
                    .button(tr!("import-model-cancel"))
                    .on_hover_hint(tr!("import-model-cancel.tooltip"))
                    .clicked();
                import = ui
                    .add_enabled(placed, egui::Button::new(tr!("import-model-import")))
                    .on_hover_hint(tr!("import-model-import.tooltip"))
                    .on_disabled_hover_text(tr!("import-model-import.waiting"))
                    .clicked();
            });
        });
        if cancel {
            self.cancel_form();
        } else if import && placed {
            self.form = None;
            self.pages.done_with(page);
            self.status
                .set_message(&cut_craft_i18n::tr!("status-model-imported"));
        }
    }

    /// Reset on a form: every setting back to its default, and what it made (or is editing)
    /// made again with them, as if it were new.
    pub(super) fn reset_form(&mut self) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let Some(command) = self.registry.get(form.id) else {
            return;
        };
        let mut fresh = command.default_params();
        // still the embellishment it was opened on
        if let Some(editing) = form.params.get("editing").filter(|e| !e.is_null()) {
            fresh["editing"] = editing.clone();
        }
        form.params = fresh;
        form.error = None;
        form.warning = None;
        form.dirty = true;
    }

    /// Cancel on a tool's form: everything done since the form opened is undone (and can be
    /// redone), and the form closes. What was changed in the toolpath list beside it
    /// meanwhile is not the form's: it is made again after (and Redo then has nothing).
    pub(super) fn cancel_form(&mut self) {
        let Some(form) = self.form.take() else { return };
        while self.history.undo_depth() > form.undo_from && self.history.undo(&mut self.doc) {}
        for (id, params) in &form.list_steps {
            if let Some(params) = self.list_step_again(id, params)
                && let Err(e) = self.apply(id, &params)
            {
                self.status.set_message(&e);
            }
        }
        self.pages.done_with(form.page);
        self.status.set_message(&cut_craft_i18n::tr!(
            "status-cancelled",
            tool = crate::tool_names::form_name(&self.registry, form.id, form.label)
        ));
    }

    /// A form's heading: its tool's icon and name, then what to do first while the tool has
    /// nothing to work on, or how to draw with it (see [`Self::form_guidance`]).
    pub(super) fn form_heading(&self, ui: &mut egui::Ui) {
        let Some(open) = &self.form else { return };
        ui.horizontal(|ui| {
            if let Some(path) = self.icons.find(open.icon, open.id) {
                let edge = ui.spacing().interact_size.y;
                ui.add(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                );
            }
            ui.strong(crate::tool_names::form_name(
                &self.registry,
                open.id,
                open.label,
            ));
        });
        let step = self.form_guidance();
        if FIXED_TEXT_FORMS.contains(&open.id) {
            // a fixed place for the Next line, there or not, so the form never jumps
            let rows = ui.text_style_height(&egui::TextStyle::Body) * NEXT_LINE_ROWS;
            crate::form::fixed_area(ui, egui::vec2(ui.available_width(), rows), |ui| {
                if let Some(step) = step {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(tr!("form-next-step"));
                        ui.label(step);
                    });
                }
            });
        } else if let Some(step) = step {
            ui.horizontal_wrapped(|ui| {
                ui.strong(tr!("form-next-step"));
                ui.label(step);
            });
        }
        // what a toolpath (or the Vector Selector) will be made from, and what it leaves out
        if open.kind.is_toolpath() || open.id == "vector_selector" {
            let count = self.doc.selected_vector_count();
            if let Some(text) = count.text() {
                ui.weak(text)
                    .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
            if closed_only_toolpath(open.id, &open.params)
                && let Some(left_out) = count.left_out(open.label)
            {
                ui.colored_label(ui.visuals().warn_fg_color, left_out)
                    .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
            if let Some(area) = texture_area_note(open.id, &open.params, count) {
                if count.open > 0 {
                    ui.colored_label(ui.visuals().error_fg_color, area)
                } else {
                    ui.weak(area)
                }
                .on_hover_hint(super::panels::SELECTION_COUNT_HINT);
            }
        }
    }

    /// The open form filling `ui`: its settings scroll, while what went wrong and the
    /// buttons stay pinned at the foot, so Apply (or Calculate) and the reason it failed are
    /// never scrolled out of sight.
    pub(super) fn form_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("form_foot").show(ui, |ui| {
            ui.add_space(ui.spacing().item_spacing.y);
            self.form_error(ui);
            self.form_buttons(ui);
        });
        // Notes fill the panel with their own editor and Variable Explorer
        if self.form.as_ref().is_some_and(|f| f.id == "notes") {
            cut_craft_bug_report::mark(ui, "Tool form: Notes");
            self.form_heading(ui);
            self.notes_editor(ui);
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| self.command_form(ui));
    }

    /// The warning for a toolpath form whose tool is a drill (it cuts only straight down)
    /// when the toolpath is not Drilling.
    fn drill_sideways(form: &super::OpenForm) -> Option<String> {
        let tool = form.params.get("tool")?;
        if !form.kind.is_toolpath() || tool.get("drill_only") != Some(&Value::Bool(true)) {
            return None;
        }
        let tool = cut_craft_core::toolpath::Tool {
            name: tool.get("name")?.as_str()?.to_owned(),
            drill_only: true,
            ..Default::default()
        };
        cut_craft_core::preflight::drill_sideways(form.id, &tool)
    }

    /// The warning for a toolpath form whose tool is the wrong kind: a router bit on a laser
    /// toolpath, or a laser on a router's (as soon as it is picked; never refused).
    fn wrong_kind_of_tool(form: &super::OpenForm) -> Option<String> {
        if !form.kind.is_toolpath() {
            return None;
        }
        let tool: cut_craft_core::toolpath::Tool =
            serde_json::from_value(form.params.get("tool")?.clone()).ok()?;
        let laser_form = form.kind.needs() == cut_craft_core::Needs::Laser;
        if laser_form && !tool.is_laser() {
            Some(format!(
                "{} is not a laser: this toolpath takes its diameter as the kerf and its feed \
                 as the laser's speed. Select… a Laser from the Tool Database",
                tool.name
            ))
        } else if !laser_form && tool.shape == cut_craft_core::toolpath::ToolShape::Laser {
            Some(format!(
                "{} is a laser, which cannot cut this toolpath: Select… a bit",
                tool.name
            ))
        } else {
            None
        }
    }

    /// The warning for a Toolpath Tiling form whose tiles, each with its overlap all round
    /// (what a tile's file covers), are bigger than the active machine's bed; none when the
    /// machine's bed size was not given.
    fn tiles_past_bed(&self, form: &super::OpenForm) -> Option<String> {
        if form.id != "toolpath_tiling" || form.params["enabled"] != Value::Bool(true) {
            return None;
        }
        // a bed whose size was not given (0) is nothing to check against
        let machine = self
            .machines
            .active()
            .filter(|m| m.width > 0.0 && m.height > 0.0)?;
        let tiling: cut_craft_core::tiling::Tiling =
            serde_json::from_value(form.params["tiling"].clone()).ok()?;
        let file = tiling.largest_file(self.doc.job.area());
        if file.x <= machine.width + 1e-9 && file.y <= machine.height + 1e-9 {
            return None;
        }
        let units = self.doc.job.units;
        let length = |mm: f64| super::trimmed(mm / units.per());
        Some(format!(
            "A tile with its overlap is {} × {} {}, bigger than {}'s bed ({} × {}): make the \
             tiles or the overlap smaller",
            length(file.x),
            length(file.y),
            units.suffix(),
            machine.name,
            length(machine.width),
            length(machine.height),
        ))
    }

    /// What Model Position in Material needs beside its form's settings: the job's
    /// material and the model's height (measured only while a form showing it is open).
    pub(super) fn placement_room(&self) -> super::placement::Room {
        let shows = self
            .form
            .as_ref()
            .is_some_and(|f| matches!(f.id, "material_setup" | "orientate_3d_model"));
        let tall = if shows && !self.doc.components.is_empty() {
            let revision = self.history.revision();
            match self.model_tall.get() {
                Some((at, tall)) if at == revision => tall,
                _ => {
                    let tall = cmd_scale_model_z_height::model_height(&self.doc);
                    let tall = if tall.is_finite() { tall } else { 0.0 };
                    self.model_tall.set(Some((revision, tall)));
                    tall
                }
            }
        } else {
            0.0
        };
        super::placement::Room {
            thickness: self.doc.job.thickness,
            tall,
            units: self.doc.job.units,
            rotary: self.doc.job.rotary.is_some(),
        }
    }

    /// Why the open form's last Apply failed, or what it warned of, in red, over its buttons.
    fn form_error(&self, ui: &mut egui::Ui) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        if FIXED_TEXT_FORMS.contains(&form.id) {
            // what the setting under the pointer does, in a box of its own under the
            // settings, always the same size
            let rows = ui.text_style_height(&egui::TextStyle::Body) * HELP_BOX_ROWS;
            let said = form::pointed_at_help(ui.ctx());
            ui.group(|ui| {
                crate::form::fixed_area(
                    ui,
                    egui::vec2(ui.available_width(), rows),
                    |ui| match said {
                        Some(help) => {
                            ui.label(help);
                        }
                        None => {
                            ui.weak(tr!("form-point-at-setting"));
                        }
                    },
                );
            });
            // a fixed place for the red text, shown or not, so the settings above never
            // jump as it comes and goes
            let rows = ui.text_style_height(&egui::TextStyle::Body) * WARNING_ROWS;
            crate::form::fixed_area(ui, egui::vec2(ui.available_width(), rows), |ui| {
                self.form_error_text(ui);
            });
        } else {
            self.form_error_text(ui);
        }
    }

    fn form_error_text(&self, ui: &mut egui::Ui) {
        let Some(form) = self.form.as_ref() else {
            return;
        };
        // a drill picked for a toolpath that cuts sideways: said as soon as it is picked
        if let Some(drill) = Self::drill_sideways(form) {
            ui.colored_label(ui.visuals().error_fg_color, drill);
        }
        // a router bit on a laser toolpath, or a laser on a router's: said as it is picked
        if let Some(wrong) = Self::wrong_kind_of_tool(form) {
            ui.colored_label(ui.visuals().error_fg_color, wrong);
        }
        // a laser or rotary toolpath on a machine without one (one already in the job)
        if let Some(lacks) = self.machine_lacks(form.kind.needs()) {
            ui.colored_label(ui.visuals().error_fg_color, lacks);
        }
        // tiles that, with their overlap, will not fit the machine: said as they are set
        if let Some(past) = self.tiles_past_bed(form) {
            ui.colored_label(ui.visuals().error_fg_color, past);
        }
        // a model poking out of the material where Model Position in Material puts it
        if let Some(out) = super::placement::warning(&form.params, self.placement_room()) {
            ui.colored_label(ui.visuals().error_fg_color, out);
        }
        // Calculate found nothing changed: said plainly, not as a warning
        if let Some(note) = self.form_note() {
            ui.label(note);
        }
        // what the last Apply made is not quite what was asked: said in red, never a pop-up
        if let Some(warning) = form.warning.as_ref().filter(|_| form.error.is_none()) {
            ui.colored_label(ui.visuals().error_fg_color, warning);
        }
        let Some(error) = form.error.as_ref() else {
            return;
        };
        let shown = ui.colored_label(ui.visuals().error_fg_color, error);
        // a toolpath kept out of a zone: how to let it be clipped instead
        if error.starts_with(KEEP_OUT_REFUSED) {
            shown.on_hover_text(CLIP_HOW);
        }
    }
}

/// Whether toolpath `id` with `params` cuts only the closed vectors selected, leaving the
/// open ones out: the area toolpaths, and Profile inside or outside (on the line cuts open
/// ones too).
pub(super) fn closed_only_toolpath(id: &str, params: &Value) -> bool {
    match id {
        "profile_toolpath" => params["side"].as_str() != Some("on"),
        // a texture fills closed vectors, but follows any vector used as its pattern
        "texture_toolpath" => params["use_selected_vectors_as_pattern"] != Value::Bool(true),
        _ => CLOSED_ONLY_TOOLPATHS.contains(&id),
    }
}

/// What a Texture Toolpath form filling closed vectors will texture when no closed vector is
/// selected: the whole material, said plainly (in red when vectors are selected but open).
pub(super) fn texture_area_note(
    id: &str,
    params: &Value,
    count: cut_craft_core::document::VectorCount,
) -> Option<String> {
    if id != "texture_toolpath"
        || params["use_selected_vectors_as_pattern"] == Value::Bool(true)
        || count.closed > 0
    {
        return None;
    }
    Some(if count.open > 0 {
        "Nothing closed is selected, so the whole material will be textured: close the open \
         vectors, or clear the selection"
            .to_owned()
    } else {
        "Nothing selected: the whole material will be textured".to_owned()
    })
}

/// The toolpaths that take only closed vectors from the selection.
pub(super) const CLOSED_ONLY_TOOLPATHS: [&str; 9] = [
    "chamfer_toolpath",
    "drill_bank_toolpath",
    "drilling_toolpath",
    "inlay_toolpath",
    "pocket_toolpath",
    "prism_toolpath",
    "sketch_carving_toolpath",
    "vcarve_engrave_toolpath",
    "vcarve_inlay_toolpath",
];

/// What turning clipping on does, and how: the tooltip on a toolpath refused for a zone.
pub(super) const CLIP_HOW: &str = "Keep-out zones (red outlines, such as clamps) are kept \
    safe: a toolpath that would cut into one is not made.\n\nTo make it anyway with the part \
    under the zone left uncut, open Edit ▸ Options, and under Toolpath Settings tick \
    \"Clip toolpaths at keep-out zones\". The tool then lifts over each zone and comes back \
    down past it, as if the zone were cut out of whatever is under it.\n\nTo change a zone \
    instead: Toolpaths ▸ Keep-Out Zones… for the job's, Machine Configuration for the \
    machine's.";

/// A form with no commit button: why there is none, and on a drawing tool whose shape follows
/// the form, New Shape to let it go and type another; whether New Shape was pressed.
/// Forms with a Reset button: their settings back to the defaults, the look made again as
/// if new.
pub(super) const RESET_FORMS: &[&str] = &["embellish"];

/// Forms whose changing text (the Next line, the red text at the foot) has a fixed place,
/// so the form never grows, shrinks or flickers as it changes.
pub(super) const FIXED_TEXT_FORMS: &[&str] = &["embellish"];

/// Lines kept for a fixed form's Next line.
pub(super) const NEXT_LINE_ROWS: f32 = 2.0;

/// Lines kept for what the setting under the pointer does.
pub(super) const HELP_BOX_ROWS: f32 = 4.0;

/// Lines kept for a fixed form's red text at its foot.
pub(super) const WARNING_ROWS: f32 = 3.0;

/// A form's Reset button, for the forms that have one.
pub(super) fn reset_button(ui: &mut egui::Ui, id: &str) -> bool {
    RESET_FORMS.contains(&id)
        && ui
            .button(tr!("form-reset"))
            .on_hover_hint(tr!("form-reset.tooltip"))
            .clicked()
}

pub(super) fn live_row(ui: &mut egui::Ui, kind: Form, drawing: bool, made: bool) -> bool {
    ui.horizontal_wrapped(|ui| {
        ui.weak(commit::live_note(kind, drawing));
        drawing
            && made
            && ui
                .button(tr!("form-new-shape"))
                .on_hover_hint(tr!("form-new-shape.tooltip"))
                .clicked()
    })
    .inner
}

/// A toolpath form's Save Settings… and Load Settings…: whether each was pressed.
pub(super) fn settings_file_row(ui: &mut egui::Ui) -> (bool, bool) {
    ui.horizontal(|ui| {
        let save = ui
            .button(tr!("form-save-settings"))
            .on_hover_hint(tr!("form-save-settings.tooltip"))
            .clicked();
        let load = ui
            .button(tr!("form-load-settings"))
            .on_hover_hint(tr!("form-load-settings.tooltip"))
            .clicked();
        (save, load)
    })
    .inner
}

/// After Calculate: Calculate as New, another toolpath on purpose, the one made kept. A row
/// of its own over the buttons, so Calculate, Cancel and Close stay where they were;
/// whether it was pressed.
pub(super) fn calculate_as_new_row(ui: &mut egui::Ui) -> bool {
    ui.button(tr!("form-calculate-as-new"))
        .on_hover_hint(tr!("form-calculate-as-new.tooltip"))
        .clicked()
}

/// A form's commit button, marked for the help's step pictures; whether it was pressed.
pub(super) fn marked_commit_button(ui: &mut egui::Ui, label: &str, hint: &str) -> bool {
    let button = ui.button(label);
    crate::marks::note(ui, &format!("button:{label}"), button.rect);
    button.on_hover_hint(hint).clicked()
}
