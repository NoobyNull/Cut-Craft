//! Editing what is in the job: cut lists, part labels, job sheets, editing a drawn object again,
//! applying a command and the toolpath summary.

use super::{
    Change, CutCraftApp, Dialog, EntityId, Value, files, filtered_dialog, message_dialog,
    on_the_desktop,
};

impl CutCraftApp {
    /// File ▸ Import ▸ Import Cut List: the list's parts drawn and selected, then Nest Parts
    /// opened on them to put them on the material (moving them, not copying). A mistake in
    /// the file is shown in a message box, since the person has to go and fix the file.
    pub(super) fn import_cut_list(&mut self, path: &std::path::Path) {
        let read = std::fs::read(path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))
            .and_then(|bytes| {
                <fmt_cut_list::ImportCutList as cut_craft_core::Importer>::import(&self.doc, &bytes)
                    .map_err(|e| e.to_string())
            });
        match read {
            Ok(change) => {
                self.record(Ok(change), &format!("Imported {}", path.display()));
                self.start_command("nest_parts");
                if let Some(form) = self.form.as_mut().filter(|f| f.id == "nest_parts") {
                    form.params["remove_original_parts"] = Value::from(true);
                }
            }
            Err(e) => {
                message_dialog()
                    .set_title("Import Cut List")
                    .set_description(format!("The cut list could not be imported.\n\n{e}"))
                    .show();
            }
        }
    }

    /// Toolpaths ▸ Print Part Labels ▸ Open Labels to Print: the labels, in the window's
    /// style, opened in the web browser, which prints them.
    pub(super) fn open_part_labels(&mut self) {
        let job = self
            .session
            .path
            .as_deref()
            .and_then(std::path::Path::file_stem)
            .map_or_default(|s| s.to_string_lossy().into_owned());
        let path = std::env::temp_dir().join("cut-craft-part-labels.html");
        let opened = fmt_part_labels::labels(&self.doc, &self.options.part_labels, &job)
            .map_err(|e| e.to_string())
            .and_then(|html| {
                cut_craft_core::file::write_atomic(&path, html.as_bytes())
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                on_the_desktop();
                files::show_folder(&path).map_err(|e| e.to_string())
            });
        self.status.set_message(&opened.map_or_else(
            |e| e,
            |()| "Part labels opened in your web browser: print them from there".to_owned(),
        ));
    }

    /// Toolpaths ▸ Create Job Sheet: the page for each sheet with toolpaths (and the side
    /// worked on), in the Job Setup Sheet Editor's style, beside the file named.
    pub(super) fn create_job_sheet(&mut self) {
        let Some(path) = filtered_dialog(Dialog::JobSheet, &self.registry).save_file() else {
            return;
        };
        self.write_job_sheets(&path);
    }

    /// Toolpaths ▸ Print Job Sheet: the job sheet for the active sheet and side, opened in
    /// the web browser, which prints it.
    pub(super) fn print_job_sheet(&mut self) {
        let path = std::env::temp_dir().join("cut-craft-job-sheet.html");
        let setup = self.job_setup();
        let opened = fmt_create_job_sheet::sheet_for(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| e.to_string())
            .and_then(|html| {
                cut_craft_core::file::write_atomic(&path, html.as_bytes())
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                on_the_desktop();
                files::show_folder(&path).map_err(|e| e.to_string())
            });
        self.status.set_message(&opened.map_or_else(
            |e| e,
            |()| "Job sheet opened in your web browser: print it from there".to_owned(),
        ));
    }

    /// What the job sheet says about where the job is cut: the job's file (when it is on
    /// this computer; else the job is copied to the recovery file, and the sheet says
    /// where), the active machine, its post and tool changer, and the Toolpath Summary's
    /// rapid rate (and the machine's accelerations when the summary allows for them).
    pub(super) fn job_setup(&mut self) -> fmt_create_job_sheet::Setup {
        let job_file = self
            .session
            .path
            .as_deref()
            .filter(|p| p.is_file())
            .map_or_default(|p| p.display().to_string());
        let job_copy = match (&self.recovery_path, job_file.is_empty()) {
            (Some(path), true) => {
                cut_craft_core::file::write_atomic(path, &cut_craft_core::file::save_job(&self.doc))
                    .map_or_else(|_| String::new(), |()| path.display().to_string())
            }
            _ => String::new(),
        };
        let machine = self.machines.active();
        fmt_create_job_sheet::Setup {
            job_file,
            job_copy,
            machine: machine.map_or_default(|m| m.name.clone()),
            post: machine
                .and_then(|m| m.post())
                .unwrap_or_default()
                .to_owned(),
            outputs: vec![],
            slots: machine.map_or_default(|m| m.slots.clone()),
            printed: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            motion: self.motion(),
            scale: self.summary.scale,
            model_height: self.model_height_now(),
            holding: self.holding(),
            variables: (*self.note_variables()).clone(),
        }
    }

    /// The job sheets written beside `path`, one a page, the page's suffix after its name.
    pub(super) fn write_job_sheets(&mut self, path: &std::path::Path) {
        let stem = path
            .file_stem()
            .map_or_else(|| "job".into(), |s| s.to_string_lossy().into_owned());
        let setup = self.job_setup();
        let result = fmt_create_job_sheet::sheets(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| e.to_string())
            .and_then(|pages| {
                for (suffix, html) in &pages {
                    let file = path.with_file_name(format!("{stem}{suffix}.html"));
                    cut_craft_core::file::write_atomic(&file, html.as_bytes())
                        .map_err(|e| format!("could not write {}: {e}", file.display()))?;
                }
                Ok(pages.len())
            });
        self.status.set_message(&result.map_or_else(
            |e| e,
            |n| format!("Saved {n} job sheet{}", if n == 1 { "" } else { "s" }),
        ));
    }

    /// Edit Object (E): the one selected vector, if a drawing tool drew it, with that tool's
    /// id and the settings that draw it again.
    pub(super) fn object_to_edit(&self) -> Option<(EntityId, &'static str, Value)> {
        let [id] = self.doc.selection.iter().copied().collect::<Vec<_>>()[..] else {
            return None;
        };
        let path = self.doc.vectors.get(&id)?;
        let (command, params) = self.registry.drawn_by(&self.doc, path)?;
        Some((id, command, params))
    }

    /// Import Toolpaths' form, open on G-code file `path` and named after it.
    pub(super) fn import_toolpaths_from(&mut self, path: &std::path::Path) {
        let id = <cmd_import_toolpaths::ImportToolpaths as cut_craft_core::Command>::ID;
        self.start_command(id);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id) {
            form.params["file"] = Value::from(path.display().to_string());
            if let Some(stem) = path.file_stem() {
                form.params["name"] = Value::from(stem.to_string_lossy().into_owned());
            }
        }
    }

    /// Object Properties on the selection: the form that edits what is selected, the one
    /// way a beginner can always reach: the shape's own tool, a picture's, a component's,
    /// else the selected vectors' position and size.
    pub(super) fn object_properties(&self) -> Option<&'static str> {
        let doc = &self.doc;
        if self.object_to_edit().is_some() || self.shape_to_edit() {
            Some("edit_object")
        } else if !doc.selected_bitmaps.is_empty() {
            Some("bitmap_properties")
        } else if doc.selection.is_empty() && !doc.selected_components.is_empty() {
            Some("component_properties")
        } else if !doc.selection.is_empty() {
            Some("set_size")
        } else {
            None
        }
    }

    /// The tool that made what is picked and keeps its settings, so E edits it in the form:
    /// Embellish for an embellishment (its component, or any of its pieces), Create Shape
    /// for a component it made (and no vector picked).
    pub(super) fn made_to_edit(&self) -> Option<&'static str> {
        use cut_craft_core::Command as _;
        if cmd_embellish::edits_selection(&self.doc) {
            return Some(cmd_embellish::Embellish::ID);
        }
        let shaped = self.doc.selection.is_empty()
            && self.doc.selected_components.iter().any(|id| {
                self.doc
                    .components
                    .get(id)
                    .and_then(|k| k.shape.as_ref())
                    .is_some_and(cmd_create_shape::made_here)
            });
        shaped.then_some(cmd_create_shape::CreateShape::ID)
    }

    /// See [`Self::made_to_edit`].
    pub(super) fn shape_to_edit(&self) -> bool {
        self.made_to_edit().is_some()
    }

    /// The one vector selected, if drawing tool `id` draws shapes like it, with the settings
    /// that draw it again (a square reads as Draw Rectangle's, or as Draw Polygon's).
    pub(super) fn drawn_by_tool(&self, id: &str) -> Option<(EntityId, Value)> {
        let [shape] = self.doc.selection.iter().copied().collect::<Vec<_>>()[..] else {
            return None;
        };
        let path = self.doc.vectors.get(&shape)?;
        let cmd = self.registry.get(id)?;
        let params = cmd.params_from(path, &cmd.initial_params(&self.doc))?;
        Some((shape, params))
    }

    /// Opens drawing tool `command` on vector `id` with `params`: changing the form redraws
    /// that shape rather than drawing another (E, or Shift-click).
    pub(super) fn edit_shape(&mut self, id: EntityId, command: &'static str, params: Value) {
        if self.form.as_ref().is_none_or(|f| f.id != command) {
            self.start_command(command);
        }
        self.doc.selection = [id].into();
        if let Some(form) = self.form.as_mut().filter(|f| f.id == command) {
            form.params = params.clone();
            form.drawn = Some(id);
            form.applied = Some(params);
        }
        let label = self.registry.get(command).map_or("", |c| c.meta().label);
        self.status.set_message(&format!(
            "Editing the shape in {label}: change the form to change it"
        ));
    }

    /// Once the job has turned over (Swap Sides, its undo or its redo), the preview turns
    /// with it: its other side on top, the side that was up underneath. A side never
    /// previewed comes up uncut, so the one that was up is kept underneath, not lost.
    pub(super) fn follow_side(&mut self) {
        if self.doc.bottom_side == self.preview_side {
            return;
        }
        self.preview_side = self.doc.bottom_side;
        if self.simulation.is_none() {
            return;
        }
        self.finish_playback();
        let Some(mut top) = self.simulation.take() else {
            return;
        };
        let mut now_up = if let Some(under) = top.other_side.take() {
            std::sync::Arc::unwrap_or_clone(under)
        } else {
            let mut fresh = top.clone();
            fresh.heights.fill(0.0);
            fresh.cut_by.fill(0);
            (fresh.paint, fresh.under) = (vec![], vec![]);
            fresh
        };
        now_up.other_side = Some(std::sync::Arc::new(top));
        self.simulation = Some(now_up);
        (self.simulation_coarse, self.fine_3d, self.surface_gpu) = (None, None, None);
    }

    /// `change` with the components Orientate 3D Model makes put on the level clipart was
    /// imported to, if it was (Import to Level).
    pub(super) fn onto_import_level(&self, id: &str, change: Change) -> Change {
        match self.import_level {
            Some(level) if id == "orientate_3d_model" => crate::clipart::onto_level(change, level),
            _ => change,
        }
    }

    /// Runs a command and records it for undo.
    pub(super) fn apply(&mut self, id: &str, params: &Value) -> Result<(), String> {
        self.crossings_allowed(id, params)?;
        let surrounded = self.with_surroundings(id, params);
        let change = super::background::command_change(&self.registry, &self.doc, id, &surrounded);
        let depth = self.history.undo_depth();
        self.applied(id, params, change)?;
        // made in the toolpath list beside an open form: noted, for its Cancel to make again
        if self.history.undo_depth() > depth
            && let Some(form) = self.form.as_mut().filter(|f| f.in_list)
        {
            form.list_steps.push((id.to_owned(), params.clone()));
        }
        Ok(())
    }

    /// A toolpath list change's settings made to fit the job as it is now, after Cancel took
    /// back what the form did: the toolpaths it names that are gone are left out (or the
    /// change, when it was on one of them), and a show/hide keeps each name as it now is.
    pub(super) fn list_step_again(&self, id: &str, params: &Value) -> Option<Value> {
        let toolpaths = &self.doc.toolpaths;
        let exists = |v: &Value| {
            v.as_u64()
                .is_some_and(|n| toolpaths.contains_key(&EntityId(n)))
        };
        let mut params = params.clone();
        if let Some(one) = params.get("toolpath") {
            let n = one.as_u64().filter(|_| exists(one))?;
            if id == "toolpath_properties" {
                params["name"] = Value::from(toolpaths[&EntityId(n)].name.clone());
            }
        }
        if let Some(Value::Array(ids)) = params.get_mut("toolpaths") {
            ids.retain(exists);
        }
        if let Some(Value::Object(groups)) = params.get_mut("groups") {
            groups.retain(|key, _| {
                key.parse()
                    .is_ok_and(|n| toolpaths.contains_key(&EntityId(n)))
            });
        }
        if let Some(Value::Array(order)) = params.get_mut("order") {
            // every toolpath once, as the order needs: ones it did not name go at the end
            order.retain(exists);
            for id in toolpaths.keys() {
                if !order.iter().any(|v| v.as_u64() == Some(id.0)) {
                    order.push(Value::from(id.0));
                }
            }
        }
        Some(params)
    }

    /// `change` with the points it placed on snaps kept there: a drawn shape's snapped
    /// points (a line's nodes stretch to stay, other shapes move whole), or a node dragged
    /// onto a snap. Only for the drawing tool in use, or Node Editing's Move Node.
    pub(super) fn kept_on_snaps(&mut self, id: &str, mut change: Change) -> Change {
        use cut_craft_core::Edit;
        use cut_craft_core::constraint::{merged, pins};
        let placed = std::mem::take(&mut self.placed);
        let drawn = self.drawing_tool() == Some(id) && id != "draw_text";
        if placed.is_empty() || !(drawn || id == "move_node") {
            return change;
        }
        let stretch = matches!(id, "draw_polyline" | "move_node");
        let mut made = vec![];
        for edit in &change.edits {
            match edit {
                Edit::Add { id, path } if drawn => made.extend(pins(path, *id, &placed, stretch)),
                Edit::Replace { id, after, .. } if !drawn => {
                    made.extend(pins(after, *id, &placed, stretch));
                }
                _ => {}
            }
        }
        if !made.is_empty() {
            let before = self.doc.constraints.clone();
            let after = merged(&before, made);
            change.edits.push(Edit::SetConstraints { before, after });
        }
        change
    }

    /// The level `change`'s new components all went on, for the status bar to name. None
    /// when it makes no new component, shares them over several levels, sets out levels
    /// itself (Slice Model, Bake: it says what it showed and hid), or replaces components
    /// by removing and adding them (Split, Emboss, Sculpt): there is no new component then.
    pub(super) fn new_components_level(change: &Change) -> Option<u64> {
        use cut_craft_core::Edit;
        let mut levels = change.edits.iter().filter_map(|e| match e {
            Edit::AddComponent { component, .. } => Some(component.level),
            _ => None,
        });
        let first = levels.next()?;
        let replaces = change
            .edits
            .iter()
            .any(|e| matches!(e, Edit::RemoveComponent { .. } | Edit::SetLevel { .. }));
        (!replaces && levels.all(|l| l == first)).then_some(first)
    }

    /// ": new component on level <name>" for the status bar, saying when that level is
    /// hidden so a new shape that cannot be seen is not taken for lost.
    pub(super) fn placed_on(&self, level: u64) -> String {
        self.doc.levels.get(&level).map_or_else(String::new, |l| {
            let hidden = if l.visible {
                ""
            } else {
                " (hidden: show the level to see it)"
            };
            format!(": new component on level {}{hidden}", l.name)
        })
    }

    /// What command `id` made with `params` (or why it made nothing) put into the job: onto
    /// the import level, into the picked toolpath group, as one undo; then whatever the
    /// command goes on to do.
    pub(super) fn applied(
        &mut self,
        id: &str,
        params: &Value,
        change: Result<Change, String>,
    ) -> Result<(), String> {
        let change = self.kept_on_snaps(id, change?);
        if let Some(why) = self.keep_out_refusal(&change) {
            return Err(why);
        }
        let cmd = self
            .registry
            .get(id)
            .expect("only registered commands are applied");
        let change = self.onto_import_level(id, change);
        // a toolpath made while a group is picked in the list joins it
        let change = match &self.selected_group {
            Some(group) => change.into_toolpath_group(group),
            None => change,
        };
        let (made, notes) = (change.toolpaths_made(), change.notes.clone());
        // the level the new components went on, for the status bar
        let new_level = Self::new_components_level(&change);
        self.history
            .apply(&mut self.doc, change)
            .map_err(|e| e.to_string())?;
        let kept = self.history.last_notes().to_vec();
        // a form that edits what it made (Draw Text) goes on editing it, not adding another
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id)
            && let Some(made) = cmd.initial_params(&self.doc).get("replaces").cloned()
        {
            form.params["replaces"] = made;
        }
        // Bake made the shaping permanent and reset it: the form shows that reset (100 %,
        // Bake unticked), so pressing Apply again does not bake it in a second time
        if id == "component_properties"
            && params.get("bake") == Some(&Value::Bool(true))
            && let Ok(p) =
                serde_json::from_value::<cmd_component_properties::Params>(params.clone())
            && let Some(k) = self.doc.components.get(&p.component)
            && let Some(form) = self.form.as_mut().filter(|f| f.id == id)
            && let Ok(fresh) =
                serde_json::to_value(cmd_component_properties::Params::of(p.component, k))
        {
            form.params = fresh;
        }
        let placed = new_level.map_or_else(String::new, |l| self.placed_on(l));
        self.status
            .set_message(&format!("{} applied{placed}", cmd.meta().label));
        // new keep-out zones: the toolpaths are made again round them
        if id == "keep_out_zones" {
            self.keep_out_changed();
        }
        // the spiral gadget says how many times round each strand goes
        if id == "wrapped_spiral_layout"
            && let Ok(p) = serde_json::from_value(params.clone())
            && let Some(turns) = cmd_wrapped_spiral_layout::revolutions(&self.doc, &p)
        {
            self.status
                .set_message(&format!("Each strand makes {turns:.2} revolutions"));
        }
        // a toolpath made already out of date (Plate Production's, cutting each plate's own
        // text) is recalculated straight away, then warned about
        let stale: Vec<EntityId> = made
            .iter()
            .copied()
            .filter(|t| self.doc.toolpaths.get(t).is_some_and(|t| t.stale))
            .collect();
        let mut notes = [notes, kept].concat();
        notes.dedup();
        // the form's own command: its warnings stay in red at the form's foot until its
        // next Apply
        if let Some(form) = self.form.as_mut().filter(|f| f.id == id) {
            form.warning = (!notes.is_empty()).then(|| notes.join("\n"));
        } else {
            // a tool with no form: in red at the foot of the tool panel until the next change
            self.tool_note = (!notes.is_empty())
                .then(|| (self.job_epoch, self.history.revision(), notes.join("\n")));
        }
        if stale.is_empty() {
            self.warn(&made, &notes);
        } else {
            self.recalculate_in_background(stale, "Toolpaths recalculated", move |app| {
                app.warn(&made, &notes);
            });
        }
        // the preview turns over with the job (follow_side)
        self.follow_side();
        Ok(())
    }

    /// The Toolpath Summary's rows: each toolpath with its estimated time and notes.
    pub(super) fn summary_rows(&self) -> Vec<super::SummaryRow> {
        let times = self.cut_times();
        self.doc
            .toolpaths
            .iter()
            .map(|(&id, t)| super::SummaryRow {
                id,
                name: t.name.clone(),
                minutes: times.each.get(&id).copied().unwrap_or_default(),
                visible: t.visible,
                notes: t.summary_notes().to_owned(),
            })
            .collect()
    }

    /// A toolpath's time for the Toolpath Summary, minutes: see [`super::CutTimes`].
    pub(super) fn estimated_minutes(&self, t: &cut_craft_core::toolpath::Toolpath) -> f64 {
        let times = self.cut_times();
        self.doc
            .toolpaths
            .iter()
            .find(|(_, v)| std::ptr::eq(*v, t))
            .and_then(|(id, _)| times.each.get(id).copied())
            .unwrap_or_else(|| {
                cut_craft_core::cut_time::toolpath(t, &self.motion()).total() / 60.0
                    * self.summary.scale
            })
    }

    /// The whole job's cutting time, minutes: every shown toolpath, the moves between them,
    /// the tool changes and the trip back home.
    pub(super) fn job_minutes(&self) -> f64 {
        self.cut_times().total
    }

    /// What the cutting times go by: the Toolpath Summary's rapid rate, the active
    /// machine's top speeds, cornering and (unless the Toolpath Summary is told not to)
    /// accelerations, its tool changer, and the tool change times in Options.
    pub(super) fn motion(&self) -> cut_craft_core::cut_time::Motion {
        let machine = self.machines.active();
        let o = &self.options.toolpaths;
        cut_craft_core::cut_time::Motion {
            rapid: self.summary.rapid,
            max_rate: machine.map_or([0.0; 3], |m| m.max_rate),
            acceleration: machine
                .filter(|_| self.summary.acceleration)
                .map_or([0.0; 3], |m| m.acceleration),
            junction_deviation: machine.map_or(0.0, |m| m.junction_deviation),
            change_by_hand: o.change_by_hand * 60.0,
            change_by_changer: o.change_by_changer * 60.0,
            loaded: machine.map_or_default(|m| m.slots.iter().flatten().cloned().collect()),
        }
    }

    /// Every toolpath's cutting time and the job's, worked out again only when the job or
    /// what the times go by changes.
    pub(super) fn cut_times(&self) -> std::rc::Rc<super::CutTimes> {
        let motion = self.motion();
        let shown: Vec<(EntityId, bool)> = self
            .doc
            .toolpaths
            .iter()
            .map(|(&id, t)| (id, t.visible))
            .collect();
        let key = format!(
            "{}|{}|{motion:?}|{}|{shown:?}",
            self.job_epoch,
            self.history.revision(),
            self.summary.scale
        );
        if let Some((at, times)) = self.cut_times.borrow().as_ref()
            && *at == key
        {
            return times.clone();
        }
        let ctx = cut_craft_core::notes::Context {
            motion,
            scale: self.summary.scale,
            ..cut_craft_core::notes::Context::default()
        };
        let (each, total) = cut_craft_core::notes::job_minutes(&self.doc, &ctx);
        let times = std::rc::Rc::new(super::CutTimes { each, total });
        *self.cut_times.borrow_mut() = Some((key, times.clone()));
        times
    }
}
