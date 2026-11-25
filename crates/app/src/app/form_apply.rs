//! Applying a tool's form: recording the change, out-of-date toolpaths, crossings, live forms and
//! redrawing what a tool drew.

use super::toolpath_output::CLIP_SETTING;
use super::{
    APP_NAME, Change, CutCraftApp, Dialog, EntityId, OpenForm, TOOL_ICON_ROWS, VECTOR_EXPORTERS,
    Value, egui, files, filtered_dialog, message_dialog, on_the_desktop,
};
use crate::options::IntersectionCheck;
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_core::Form;
use cut_craft_i18n::tr;

impl CutCraftApp {
    /// Applies a computed change for undo, reporting how it went in the status bar.
    pub(super) fn record(
        &mut self,
        change: Result<Change, cut_craft_core::CommandError>,
        done: &str,
    ) {
        let (mut made, mut notes) = (vec![], vec![]);
        let result = change.map_err(|e| e.to_string()).and_then(|c| {
            made = c.toolpaths_made();
            notes.clone_from(&c.notes);
            self.history
                .apply(&mut self.doc, c)
                .map_err(|e| e.to_string())
        });
        if result.is_ok() {
            notes.extend_from_slice(self.history.last_notes());
            notes.dedup();
        }
        self.status
            .set_message(&result.map_or_else(|e| e, |()| done.to_owned()));
        self.warn(&made, &notes);
    }

    /// More toolpaths out of date than last frame, after an edit (not as a job opens, nor
    /// while a drag goes on, as a height handle remakes the model): the status bar says how
    /// many, and that they and any G-code saved from them want making again. Toolpaths are
    /// never remade by themselves.
    pub(super) fn note_stale(&mut self) {
        if self.drag_3d.is_some() {
            return;
        }
        let stale = self.doc.toolpaths.values().filter(|t| t.stale).count();
        let more = stale.saturating_sub(self.stale_known);
        self.stale_known = stale;
        if more == 0 || self.history.undo_depth() == 0 {
            return;
        }
        // asked about once the edit is done (a form's, once it closes)
        self.stale_while_form = true;
        let message = self.status.message.trim_end_matches('.').to_owned();
        self.status.set_message(&tr!(
            "recalculate-out-of-date",
            message = message,
            count = more
        ));
    }

    /// Once a form that left toolpaths out of date has closed: asks whether to recalculate
    /// them all now. No just leaves them marked; Save Toolpaths still warns.
    pub(super) fn ask_to_recalculate(&mut self, ctx: &egui::Context) {
        let done = self.form.is_none() && self.drag_3d.is_none();
        if done && std::mem::take(&mut self.stale_while_form) {
            let stale = self.doc.toolpaths.values().filter(|t| t.stale).count();
            let ask = self.options.confirmations.ask_to_recalculate;
            self.recalculate_ask = (ask && stale > 0).then_some((stale, false));
        }
        // recalculated meanwhile (Plate Production remakes its own straight away)
        let now = self.doc.toolpaths.values().filter(|t| t.stale).count();
        if now == 0 {
            self.recalculate_ask = None;
        }
        let Some((asked, mut silence)) = self.recalculate_ask else {
            return;
        };
        let stale = asked.min(now);
        let (mut yes, mut no) = (false, false);
        egui::Modal::new(egui::Id::unique("recalculate_ask")).show(ctx, |ui| {
            ui.set_max_width(ui.spacing().interact_size.x * 6.0);
            ui.heading(tr!("recalculate-ask-title"));
            ui.label(recalculate_question(stale));
            ui.checkbox(&mut silence, tr!("recalculate-dont-ask"))
                .on_hover_hint(tr!("recalculate-dont-ask.tooltip"));
            ui.separator();
            ui.horizontal(|ui| {
                yes = ui
                    .button(tr!("recalculate-all"))
                    .on_hover_hint(tr!("recalculate-all.tooltip"))
                    .clicked();
                no = ui
                    .button(tr!("recalculate-not-now"))
                    .on_hover_hint(tr!("recalculate-not-now.tooltip"))
                    .clicked();
            });
        });
        self.recalculate_ask = Some((asked, silence));
        if yes || no {
            self.recalculate_ask = None;
            if silence {
                self.options.confirmations.ask_to_recalculate = false;
                self.save_options();
            }
        }
        if yes {
            self.invoke(ctx, "recalculate_all_toolpaths");
        }
    }

    /// A warning in the status bar about what was just made: the command's `notes`, and for
    /// each toolpath in `made`, crossing a keep-out zone and the problems the checks before
    /// cutting find.
    pub(super) fn warn(&mut self, made: &[EntityId], notes: &[String]) {
        let mut warnings = notes.to_vec();
        for t in made.iter().filter_map(|id| self.doc.toolpaths.get(id)) {
            let zones = self.zones_entered(t);
            if !zones.is_empty() {
                warnings.push(format!(
                    "{} cuts into keep-out zone {} and will not be saved: move the part or the \
                     zone, or turn on clipping ({CLIP_SETTING})",
                    t.name,
                    zones.join(" and ")
                ));
            }
            warnings.extend(cut_craft_core::checks::problems_over(
                &self.doc,
                t,
                self.options.confirmations.left(),
            ));
        }
        if !warnings.is_empty() {
            self.status.set_message(&cut_craft_i18n::tr!(
                "status-warning",
                warnings = warnings.join("; ")
            ));
        }
    }

    /// Asks whether to save toolpaths the checks before cutting found `problems` in.
    /// Crossing vectors chosen for a V-carve or pocket toolpath, or for an offset: what the
    /// Options' confirmation choices say to do. An error stops the command.
    pub(super) fn crossings_allowed(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let c = &self.options.confirmations;
        let check = match id {
            "pocket_toolpath" | "vcarve_engrave_toolpath" | "vcarve_inlay_toolpath" => {
                c.toolpath_intersections
            }
            "offset_vectors" if params["corners"] == "sharp" => c.sharp_offset_intersections,
            "offset_vectors" => c.radiused_offset_intersections,
            _ => return Ok(()),
        };
        if check == IntersectionCheck::DontCheck || self.doc.selection.is_empty() {
            return Ok(());
        }
        let only_crossings = cmd_vector_validator::Params {
            overlapping: false,
            intersections: true,
            zero_length_spans: false,
            open: false,
            ..cmd_vector_validator::Params::default()
        };
        // the validator fails when it finds nothing
        let Ok(crossing) = <cmd_vector_validator::VectorValidator as cut_craft_core::Command>::run(
            &self.doc,
            &only_crossings,
        ) else {
            return Ok(());
        };
        let validator = match check {
            IntersectionCheck::Ask => match Self::crossing_choice() {
                Some(validator) => validator,
                None => return Ok(()),
            },
            IntersectionCheck::OpenVectorValidator => true,
            IntersectionCheck::DontCheck => unreachable!("returned above"),
        };
        if validator {
            self.history
                .apply(&mut self.doc, crossing)
                .map_err(|e| e.to_string())?;
            self.start_command("vector_validator");
            return Err(
                "The selected vectors cross: they are selected for the Vector Validator".into(),
            );
        }
        Err("Stopped: the selected vectors cross".into())
    }

    /// Asks what to do about crossing vectors: `None` to go on, `Some(true)` for the Vector
    /// Validator, `Some(false)` to stop.
    fn crossing_choice() -> Option<bool> {
        let (go_on, validator) = ("Continue", "Vector Validator");
        let answer = message_dialog()
            .set_title(APP_NAME)
            .set_level(rfd::MessageLevel::Warning)
            .set_description(
                "Some of the selected vectors cross themselves or each other, which can spoil \
                 the result. Continue anyway, or select them in the Vector Validator?",
            )
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                go_on.into(),
                validator.into(),
                "Cancel".into(),
            ))
            .show();
        match answer {
            rfd::MessageDialogResult::Custom(choice) if choice == go_on => None,
            rfd::MessageDialogResult::Custom(choice) => Some(choice == validator),
            rfd::MessageDialogResult::Yes => None,
            rfd::MessageDialogResult::No => Some(true),
            _ => Some(false),
        }
    }

    /// The icons over the toolpath list: edit and hide the selected toolpath, and with
    /// `templates` save it, or every visible one, as a template (the Toolpaths page has those
    /// under the list, with the other ways to reuse toolpaths).
    pub(super) fn toolpath_list_buttons(&mut self, ui: &mut egui::Ui, templates: bool) {
        let edge = ui.spacing().interact_size.y * TOOL_ICON_ROWS;
        let mut clicked = None;
        ui.horizontal(|ui| {
            for (id, icon, tip) in [
                (
                    "edit_toolpath",
                    "toolpaths/edit_toolpath.png",
                    "Edit Toolpath",
                ),
                (
                    "hide_toolpath",
                    "toolpaths/hide_toolpaths.png",
                    "Show or Hide Toolpath",
                ),
                (
                    "save_selected_toolpath_as_template",
                    "toolpaths/save_toolpath_template.png",
                    "Save Toolpath as Template",
                ),
                (
                    "save_all_visible_toolpaths_as_template",
                    "toolpaths/save_all_as_template.png",
                    "Save All Visible Toolpaths as Template",
                ),
            ]
            .into_iter()
            .take(if templates { 4 } else { 2 })
            {
                let available =
                    id == "hide_toolpath" && self.selected_toolpath.is_some() || self.available(id);
                let button = self.icon_button(icon, "", tip, edge);
                if ui
                    .add_enabled(available, button)
                    .on_hover_hint(tip)
                    .clicked()
                {
                    clicked = Some(id);
                }
            }
        });
        match clicked {
            Some("hide_toolpath") => {
                if let Some((id, t)) = self
                    .selected_toolpath
                    .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?)))
                {
                    let params = serde_json::json!({ "toolpath": id, "name": t.name, "visible": !t.visible });
                    if let Err(e) = self.apply("toolpath_properties", &params) {
                        self.status.set_message(&e);
                    }
                }
            }
            Some(id) => self.invoke(ui.ctx(), id),
            None => {}
        }
    }

    /// Export Vectors: the vectors saved in the format the file name's extension picks, DXF
    /// when it has none.
    pub(super) fn export_vectors_picked(&mut self) {
        let result = {
            let dialog = filtered_dialog(Dialog::ExportVectors, &self.registry);
            let exporters: Vec<_> = VECTOR_EXPORTERS
                .iter()
                .filter_map(|id| self.registry.exporter(id))
                .map(|e| (e.id, e.format))
                .collect();
            match dialog.save_file() {
                // the extension picks the format; DXF when there is none
                Some(path) => {
                    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
                    let (id, path) = exporters
                        .iter()
                        .find(|(_, f)| ext.as_deref().is_some_and(|x| f.extensions.contains(&x)))
                        .map_or(
                            ("export_dxf", files::named_for_saving(&path, &["dxf"])),
                            |(id, _)| (*id, path.clone()),
                        );
                    files::export(&self.registry, id, &self.doc, &path).map(|()| {
                        self.status.set_message(&cut_craft_i18n::tr!(
                            "status-exported",
                            path = path.display().to_string()
                        ));
                    })
                }
                None => Ok(()),
            }
        };
        self.say_failure(result);
    }

    /// Open Folder: the job's folder, else the settings folder before the job is saved.
    pub(super) fn open_job_folder(&mut self) {
        let result = {
            let folder = self
                .session
                .path
                .as_deref()
                .and_then(std::path::Path::parent)
                .map(std::path::Path::to_path_buf)
                .or_else(files::config_dir);
            folder.map_or(Err("no folder to open".to_owned()), |f| {
                on_the_desktop();
                files::show_folder(&f).map_err(|e| e.to_string())
            })
        };
        self.say_failure(result);
    }

    /// Open Clipart Folder: the folder looked in, or the first when looking in all of them.
    pub(super) fn open_clipart_folder(&mut self) {
        let result = match (self.clipart.open.as_ref()).or(self.clipart.folders.first()) {
            Some(folder) => {
                on_the_desktop();
                files::show_folder(folder).map_err(|e| e.to_string())
            }
            None => Err("add a folder to the Library first".to_owned()),
        };
        self.say_failure(result);
    }

    fn say_failure(&mut self, result: Result<(), String>) {
        if let Err(e) = result {
            self.status.set_message(&e);
        }
    }

    /// Edit Toolpath: the selected toolpath's form, filled in with its settings.
    pub(super) fn edit_toolpath(&mut self) {
        let Some((id, toolpath)) = self
            .selected_toolpath
            .and_then(|id| Some((id, self.doc.toolpaths.get(&id)?.clone())))
        else {
            return;
        };
        self.start_command(&toolpath.strategy);
        let mut params = toolpath.params.clone();
        // the bank as the machine has it now, so a bank changed since is used
        self.fill_drill_bank(&toolpath.strategy, &mut params);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == toolpath.strategy) {
            // Calculate with nothing changed keeps it (a drill bank changed since is a change)
            form.made = Some(super::calculate_again::Made {
                ids: vec![id],
                kept: vec![toolpath.params.clone()],
                params: toolpath.params.clone(),
                selection: None,
            });
            form.params = params;
            form.editing = Some(id);
        }
    }

    /// A Drill Bank Toolpath's form drills with the active machine's bank (none when it has
    /// none, which the toolpath then says how to set up).
    pub(super) fn fill_drill_bank(&self, command: &str, params: &mut Value) {
        if command != "drill_bank_toolpath" {
            return;
        }
        let bank = self
            .machines
            .active()
            .and_then(|m| m.drill_bank.clone())
            .unwrap_or_default();
        params["bank"] = serde_json::to_value(bank).expect("a drill bank serialises");
    }

    /// A form's Apply: its command, or for an edited toolpath that toolpath made again with
    /// the form's settings, in its place.
    pub(super) fn apply_form(&mut self, id: &str, params: &Value) -> Result<(), String> {
        if let Some(form) = &mut self.form {
            form.note = None;
        }
        // Generate Border's Generate: the add-on runs in the background, from the next frame
        if id == super::generate_border::ID {
            self.border.pending = Some(params.clone());
            return Ok(());
        }
        let editing = self.form.as_ref().and_then(|f| f.editing);
        // Calculate pressed again: kept when nothing changed, else updated in place
        match self.calculate_again(params) {
            super::calculate_again::Again::UpToDate => {
                self.say_up_to_date(params);
                return Ok(());
            }
            super::calculate_again::Again::Update(made) if editing.is_none() => {
                return self.calculate_in_background(id, params, Some(made));
            }
            _ => {}
        }
        if let Some(toolpath) = editing {
            self.edit_in_background(id, toolpath, params);
            return Ok(());
        }
        if matches!(self.form_kind(id), Form::Remake { .. }) {
            return self.remake(id, params);
        }
        let before = self.entity_ids();
        let same = self.form.as_ref().and_then(|f| f.applied.as_ref()) == Some(params);
        let first = self.drawn_shape().is_none();
        match self.drawn_shape() {
            // the shape already follows these settings: nothing to do again
            Some(_) if same => return Ok(()),
            Some(old) => self.redraw(id, params, old)?,
            // a shape or model the form goes on changing is made in place; anything else
            // (a toolpath, a relief) may take a while, so it shows its progress
            None if self.makes_one(id) => self.apply(id, params)?,
            None => self.apply_in_background(id, params)?,
        }
        self.remember_drawn(&before);
        if let Some(form) = &mut self.form {
            form.applied = form.drawn.map(|_| params.clone());
        }
        // a relief is a 3D model: the first one made is shown where it can be seen
        if first && id == "relief_from_photo" && self.drawn_shape().is_some() {
            self.view = super::View::ThreeD;
        }
        Ok(())
    }

    /// A remaking form's Apply: its pieces made again with `params` in place of the ones it
    /// last made, while that is still the last thing done (one undo); else a new one.
    pub(super) fn remake(&mut self, id: &str, params: &Value) -> Result<(), String> {
        let mut taken_back = None;
        if let Some(applied) = self.remade_last() {
            if applied == *params {
                return Ok(());
            }
            if self.history.undo(&mut self.doc) {
                taken_back = Some(applied);
            }
        }
        if let Some(form) = &mut self.form {
            form.remade = None;
        }
        if let Err(e) = self.apply(id, params) {
            // what it last made comes back, still the one it remakes in place
            if let Some(applied) = taken_back
                && self.history.redo(&mut self.doc)
            {
                let after = (self.history.revision(), self.doc.selection.clone());
                if let Some(form) = &mut self.form {
                    form.remade = Some((after.0, after.1, applied));
                }
            }
            return Err(e);
        }
        let after = (self.history.revision(), self.doc.selection.clone());
        if let Some(form) = &mut self.form {
            form.remade = Some((after.0, after.1, params.clone()));
        }
        Ok(())
    }

    /// The settings the open remaking form last made its pieces with, while nothing has been done or picked
    /// since.
    pub(super) fn remade_last(&self) -> Option<Value> {
        let (revision, selection, applied) = self.form.as_ref()?.remade.as_ref()?;
        (self.history.revision() == *revision && self.doc.selection == *selection)
            .then(|| applied.clone())
    }

    /// Every vector and component, to tell what a command made.
    pub(super) fn entity_ids(&self) -> std::collections::BTreeSet<EntityId> {
        self.doc
            .vectors
            .keys()
            .chain(self.doc.components.keys())
            .copied()
            .collect()
    }

    /// Whether form `id` makes one shape or model that it goes on to change: the drawing
    /// tools, and the forms whose model follows them as they change.
    pub(super) fn makes_one(&self, id: &str) -> bool {
        self.drawing_tool() == Some(id) || matches!(self.form_kind(id), Form::Live { .. })
    }

    /// How command `id`'s form behaves, as the command says ([`Form`]); Apply for an id that
    /// is no command.
    pub(super) fn form_kind(&self, id: &str) -> Form {
        self.registry.get(id).map_or(Form::Apply, |c| c.meta().form)
    }

    /// Once a frame: a form's shape or model made again with its settings once they have
    /// changed, a dragged value as it goes (the drag one undo).
    pub(super) fn follow_form(&mut self, ctx: &egui::Context) {
        let pressed = ctx.input(|i| i.pointer.any_down());
        let Some(form) = self.form.as_mut() else {
            return;
        };
        if !pressed {
            form.dragged = None;
        }
        if !form.dirty {
            return;
        }
        form.dirty = false;
        let dragged = form.dragged.take();
        // a remaking form's pieces follow its settings once made, a drag only remaking it (one undo)
        if self.remade_last().is_some() {
            self.apply_live();
            return;
        }
        if self.drawn_shape().is_none() {
            return;
        }
        // the last frame's step of this drag taken back first, so the drag is one undo
        if let Some((revision, drawn, applied)) = dragged
            && self.history.revision() == revision
            && self.history.undo(&mut self.doc)
            && let Some(form) = self.form.as_mut()
        {
            form.drawn = drawn;
            form.applied = applied;
        }
        let Some(form) = &self.form else { return };
        let before = (self.history.undo_depth(), form.drawn, form.applied.clone());
        self.apply_live();
        let depth = self.history.undo_depth();
        let revision = self.history.revision();
        if let (true, Some(form)) = (pressed && depth > before.0, self.form.as_mut()) {
            form.dragged = Some((revision, before.1, before.2));
        }
    }

    /// Applies the open form, Orientate 3D Model's Model Size following the model's own
    /// but for the sizes the user typed.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a model grid's columns and rows"
    )]
    #[expect(
        clippy::float_cmp,
        reason = "a size is edited when not the very value shown"
    )]
    pub(super) fn apply_live(&mut self) {
        const SIZES: [&str; 3] = ["x", "y", "z"];
        let Some(form) = &mut self.form else { return };
        let (id, mut params) = (form.id, form.params.clone());
        let sized = params["size"].is_object();
        if sized && id == "orientate_3d_model" {
            typed_into_scale(form, &mut params, &self.doc);
        } else if sized {
            typed_sizes(form, &mut params);
        }
        if id == "orientate_3d_model" {
            forget_typed_alignment(form);
            params["placing"]["align"] = form.params["placing"]["align"].clone();
        }
        let wrapped = self.doc.job.rotary.is_some() && params["model_type"] != "flat";
        let result = self.apply_form(id, &params);
        // the component the settings just made judges what a cutter cannot reach, so the
        // model is rasterised once per change, not twice
        let top = self
            .form
            .as_ref()
            .and_then(|f| f.drawn)
            .and_then(|id| self.doc.components.get(&id));
        let model = serde_json::from_value(params)
            .ok()
            .filter(|_| id == "orientate_3d_model")
            .and_then(|p| {
                cmd_orientate_3d_model::preview_over(&self.doc, &p, top)
                    .ok()
                    .flatten()
            });
        // the model's size: its triangles' box, or (wrapped) its component's
        let made = model.as_deref().and_then(model_size);
        let cap = usize::try_from(self.options.import.preview_faces).unwrap_or(usize::MAX);
        // on a flat job, what is past the material's edges is cut off to show as a ghost;
        // past the preview limit, every so many of its faces only (faint, it still reads)
        let (model, beyond) = match model {
            Some(all) if self.doc.job.rotary.is_none() => {
                let (over, past) = crate::view3d::split_at_edges(&all, self.doc.job.area());
                let every = past.len().div_ceil(cap.max(1)).max(1);
                let past = past
                    .into_iter()
                    .step_by(every)
                    .map(|t| (t, false))
                    .collect();
                (Some(over), Some(std::sync::Arc::new(past)))
            }
            all => (all, None),
        };
        // a huge model is shown as the surface it makes, only its unreachable faces drawn
        let (model, whole) = match model {
            Some(all) if all.len() > cap => (
                Some(all.into_iter().filter(|(_, red)| *red).collect()),
                false,
            ),
            all => (all, true),
        };
        let model = model.map(std::sync::Arc::new);
        let made = made.or_else(|| {
            let k = self.doc.components.get(&self.form.as_ref()?.drawn?)?;
            let (lo, hi) = k.height_range().unwrap_or_default();
            Some([k.cell * k.columns as f64, k.cell * k.rows as f64, hi - lo])
        });
        let (area, thickness) = (self.doc.job.area(), self.doc.job.thickness);
        if let Some(form) = &mut self.form {
            form.error = result.err();
            form.model = model;
            form.model_whole = whole;
            form.beyond = beyond;
            if let (true, Some(made)) = (sized, made) {
                // to the hundredth of a millimetre shown, not the cell's float noise
                let round = |v: f64| (v * 100.0).round() / 100.0;
                let shown = std::array::from_fn(|i| form.typed_size[i].unwrap_or(round(made[i])));
                for (key, value) in SIZES.iter().zip(shown) {
                    form.params["size"][key] = Value::from(value);
                }
                // Depth Below Top reads the model's height while the plane is at its
                // bottom (0, or where its bottom was before it was sized)
                // does; a depth typed above the bottom stays, and one past it lifts the
                // model off the plane, up to the material's top
                let depth = &mut form.params["placing"]["depth_below_top"];
                let (at, tall) = (depth.as_f64().unwrap_or(0.0), shown[2]);
                let was_bottom = form.shown_size.is_some_and(|s| s[2] == at);
                if at <= 0.0 || was_bottom {
                    *depth = Value::from(tall);
                } else if at > tall.max(thickness) {
                    *depth = Value::from(tall.max(thickness));
                }
                if !wrapped {
                    show_alignment(form, area, thickness, shown);
                }
                form.shown_size = Some(shown);
            }
        }
    }

    /// The shape (while it is the one selected) or model the open form made.
    pub(super) fn drawn_shape(&self) -> Option<EntityId> {
        let form = self.form.as_ref()?;
        let id = form.drawn?;
        let only = self.doc.selection.len() == 1 && self.doc.selection.contains(&id);
        let vector = only && self.doc.vectors.contains_key(&id);
        (vector || self.doc.components.contains_key(&id)).then_some(id)
    }

    /// After a form that makes one shape or model makes it, the form goes on to change it.
    pub(super) fn remember_drawn(&mut self, before: &std::collections::BTreeSet<EntityId>) {
        let made: Vec<EntityId> = self.entity_ids().difference(before).copied().collect();
        let one = self.form.as_ref().is_some_and(|f| self.makes_one(f.id));
        let ids = self.entity_ids();
        if let (true, Some(form)) = (one, self.form.as_mut()) {
            form.drawn = match made[..] {
                [made] => Some(made),
                // made again in its own place, it keeps its id
                [] => form.drawn.filter(|id| ids.contains(id)),
                _ => None,
            };
        }
    }

    /// `id`'s command run again with `params`, its shape taking the place of `old`: one undo.
    pub(super) fn redraw(&mut self, id: &str, params: &Value, old: EntityId) -> Result<(), String> {
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let mut taken = Change::builder(&self.doc);
        if self.doc.components.contains_key(&old) {
            taken.remove_component(old);
        } else {
            taken.remove(old);
        }
        let taken = taken.finish();
        let mut without = self.doc.clone();
        without.apply(&taken).map_err(|e| e.to_string())?;
        let made = cmd.run_json(&without, params).map_err(|e| e.to_string())?;
        // a shape changed inside a group stays in that group
        let mut regrouped = Vec::new();
        if let Some(&group) = self.doc.vector_groups.get(&old) {
            let mut after = without.clone();
            after.apply(&made).map_err(|e| e.to_string())?;
            let mut c = Change::builder(&after);
            for edit in &made.edits {
                if let cut_craft_core::Edit::Add { id, .. } = edit {
                    c.set_group(*id, Some(group));
                }
            }
            regrouped = c.finish().edits;
        }
        let change = Change {
            edits: (taken.edits.into_iter().chain(made.edits))
                .chain(regrouped)
                .collect(),
            notes: made.notes,
            unit_guess: made.unit_guess,
        };
        let change = self.onto_import_level(id, change);
        let (toolpaths, notes) = (change.toolpaths_made(), change.notes.clone());
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        self.status.set_message(&cut_craft_i18n::tr!(
            "status-changed",
            tool = crate::tool_names::name_of(&self.registry, id)
        ));
        self.warn(&toolpaths, &notes);
        Ok(())
    }
}

/// A form's Model Size following the model's own but for the sizes the user typed: those
/// are kept and given to the command (locked, the first typed sets the others).
#[expect(
    clippy::float_cmp,
    reason = "a size is edited when not the very value shown"
)]
pub(super) fn typed_sizes(form: &mut OpenForm, params: &mut Value) {
    const SIZES: [&str; 3] = ["x", "y", "z"];
    let lock = params["size"]["lock_ratio"].as_bool().unwrap_or(false);
    for (i, key) in SIZES.iter().enumerate() {
        let value = params["size"][key].as_f64().unwrap_or(0.0);
        let edited = form
            .shown_size
            .map_or(value != 0.0, |shown| value != shown[i]);
        if edited {
            // locked, the size just typed sets the others
            if lock {
                form.typed_size = [None; 3];
            }
            form.typed_size[i] = (value > 0.0).then_some(value);
        }
    }
    if lock {
        let first = form.typed_size.iter().position(Option::is_some);
        form.typed_size = std::array::from_fn(|i| form.typed_size[i].filter(|_| Some(i) == first));
    }
    for (key, typed) in SIZES.iter().zip(form.typed_size) {
        params["size"][key] = Value::from(typed.unwrap_or(0.0));
    }
}

/// Import 3D Model's sizes: a size typed scales the model once, by how much it differs
/// from the size shown (all three alike while the ratio is locked, and always on a rotary
/// job), kept as the model's scale so turning the model after does not size it again. The
/// command is given the scale, never the sizes.
#[expect(
    clippy::float_cmp,
    reason = "a size is edited when not the very value shown"
)]
pub(super) fn typed_into_scale(
    form: &mut OpenForm,
    params: &mut Value,
    doc: &cut_craft_core::document::Document,
) {
    const SIZES: [&str; 3] = ["x", "y", "z"];
    let size = &params["size"];
    let mut scale: [f64; 3] = serde_json::from_value(size["scale"].clone()).unwrap_or([1.0; 3]);
    let uniform = size["lock_ratio"].as_bool().unwrap_or(false) || params["rotary"] == true;
    // the model's size as it is now, turned as the settings now turn it, at its scale
    let now = || {
        let mut now = params.clone();
        for key in SIZES {
            now["size"][key] = Value::from(0.0);
        }
        let p = serde_json::from_value(now).ok()?;
        model_size(&cmd_orientate_3d_model::preview(doc, &p).ok()??)
    };
    let edited =
        |i: usize, value: f64| value > 0.0 && form.shown_size.is_none_or(|shown| value != shown[i]);
    let typed = SIZES
        .iter()
        .enumerate()
        .any(|(i, key)| edited(i, size[*key].as_f64().unwrap_or(0.0)));
    if let Some(shown) = typed.then(now).flatten() {
        for (i, key) in SIZES.iter().enumerate() {
            let value = size[key].as_f64().unwrap_or(0.0);
            if !edited(i, value) || shown[i] <= 0.0 {
                continue;
            }
            let by = value / shown[i];
            if uniform {
                scale = scale.map(|s| s * by);
                break;
            }
            scale[i] *= by;
        }
    }
    form.typed_size = [None; 3];
    form.params["size"]["scale"] = serde_json::json!(scale);
    params["size"]["scale"] = serde_json::json!(scale);
    for key in SIZES {
        params["size"][key] = Value::from(0.0);
    }
}

/// Import 3D Model's alignment in the material as kept by its settings.
pub(super) fn kept_alignment(params: &Value) -> cmd_orientate_3d_model::Align {
    serde_json::from_value(params["placing"]["align"].clone()).unwrap_or_default()
}

/// Import 3D Model's alignment dropped where its number was typed since it was shown: a
/// position X or Y, or Depth Below Top, typed leaves the model there.
pub(super) fn forget_typed_alignment(form: &mut OpenForm) {
    const KEYS: [(&str, &str); 3] = [("across", "x"), ("up", "y"), ("height", "")];
    for ((align, at), shown) in KEYS.into_iter().zip(form.shown_place) {
        let placing = &form.params["placing"];
        let now = if at.is_empty() {
            placing["depth_below_top"].as_f64()
        } else {
            placing["position"][at].as_f64()
        };
        if shown.is_some() && now != shown {
            form.params["placing"]["align"][align] = Value::Null;
        }
    }
}

/// Import 3D Model's position and Depth Below Top shown where the kept alignment puts a
/// model `shown` big, measured from the material's corner, so the numbers say where it is
/// (a centred model kept centred from now on).
pub(super) fn show_alignment(
    form: &mut OpenForm,
    area: cut_craft_core::geometry::Rect,
    thickness: f64,
    shown: [f64; 3],
) {
    let round = |v: f64| (v * 100.0).round() / 100.0;
    // centred, it is kept centred as Align in Material's centre would, its bottom on the
    // zero plane however it is sized after, and the corner it gives is shown, for the
    // numbers to move it from there
    let placing = &mut form.params["placing"];
    if placing["center"] == true {
        placing["align"]["across"] = Value::from(0.5);
        placing["align"]["up"] = Value::from(0.5);
        if placing["align"]["height"].is_null() {
            placing["align"]["height"] = Value::from(0.0);
        }
        placing["center"] = Value::Bool(false);
    }
    let align = kept_alignment(&form.params);
    let placing = &mut form.params["placing"];
    let typed = cut_craft_core::geometry::Point::new(
        placing["position"]["x"].as_f64().unwrap_or(0.0),
        placing["position"]["y"].as_f64().unwrap_or(0.0),
    );
    let footprint = cut_craft_core::geometry::Point::new(shown[0], shown[1]);
    let corner = align.corner(area, footprint, typed);
    if align.across.is_some() {
        placing["position"]["x"] = Value::from(round(corner.x));
    }
    if align.up.is_some() {
        placing["position"]["y"] = Value::from(round(corner.y));
    }
    if let Some(depth) = align.depth(shown[2], thickness) {
        placing["depth_below_top"] = Value::from(round(depth));
    }
    form.shown_place = [
        placing["position"]["x"].as_f64(),
        placing["position"]["y"].as_f64(),
        placing["depth_below_top"].as_f64(),
    ];
}

/// The size of the box round a model's `faces`.
pub(super) fn model_size(faces: &[cmd_orientate_3d_model::Seen]) -> Option<[f64; 3]> {
    let corners = faces.iter().flat_map(|(t, _)| t);
    let lo = corners
        .clone()
        .copied()
        .reduce(cut_craft_core::glam::DVec3::min)?;
    let hi = corners.copied().reduce(cut_craft_core::glam::DVec3::max)?;
    Some((hi - lo).to_array())
}

/// What the "recalculate all?" question says for `stale` toolpaths out of date.
pub(super) fn recalculate_question(stale: usize) -> String {
    tr!("recalculate-ask", count = stale)
}
