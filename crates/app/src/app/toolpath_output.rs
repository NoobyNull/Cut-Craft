//! Saving toolpaths: choosing the post, the checks before writing (limits, keep-out zones, tool
//! changers) and writing the files.

use super::{
    Change, CutCraftApp, Dialog, EntityId, SaveChecks, SaveForm, Value, file_name, files,
    filtered_dialog, output,
};

impl CutCraftApp {
    /// The active machine's keep-out zones put on the job, as job millimetres (the G-code's,
    /// which they are given in), and whether Options allow clipping toolpaths at them; true
    /// when either changed.
    pub(super) fn sync_machine_keep_out(&mut self) -> bool {
        let zones: Vec<_> = self
            .machines
            .active()
            .map_or_default(|m| m.keep_out.as_slice())
            .iter()
            .enumerate()
            .filter(|(_, z)| z.width > 0.0 && z.height > 0.0)
            .map(|(i, z)| {
                cut_craft_core::keep_out::KeepOutZone::new(z.name_at(i), z.path())
                    .standing(z.stands)
            })
            .collect();
        let clip = self.options.toolpaths.clip_at_keep_out_zones;
        let changed = zones != self.doc.machine_keep_out || clip != self.doc.clip_keep_out;
        self.doc.machine_keep_out = zones;
        self.doc.clip_keep_out = clip;
        changed
    }

    /// The keep-out zones changed: every toolpath is made again round them
    /// are, and any that still cannot keep out is warned about.
    pub(super) fn keep_out_changed(&mut self) {
        if !self.doc.toolpaths.is_empty() {
            let ids: Vec<EntityId> = self.doc.toolpaths.keys().copied().collect();
            let warned = ids.clone();
            self.recalculate_in_background(ids, "All toolpaths recalculated", move |app| {
                app.warn(&warned, &[]);
            });
        }
    }

    /// The toolpaths of `plan` in `doc` that pass over a keep-out zone, by name.
    #[cfg(test)]
    pub(super) fn not_kept_out(
        &self,
        doc: &super::Document,
        plan: &[Vec<EntityId>],
    ) -> Vec<String> {
        self.output().not_kept_out(doc, plan)
    }

    /// The job with its toolpaths thinned to their tolerances, as they are saved; refused
    /// when a toolpath of `plan`, as saved, would cut into a keep-out zone.
    #[cfg(test)]
    pub(super) fn thinned_for_saving(
        &self,
        plan: &[Vec<EntityId>],
    ) -> Result<super::Document, String> {
        self.output().thinned(plan)
    }

    /// The names of the keep-out zones `toolpath` cuts into (its cutter's edge reaching
    /// one) or rapids over.
    pub(super) fn zones_entered(
        &self,
        toolpath: &cut_craft_core::toolpath::Toolpath,
    ) -> Vec<String> {
        output::zones_entered(&self.doc, toolpath)
    }

    /// The job and machine as they are now, for toolpath output: what every way of saving
    /// toolpaths works from.
    pub(super) fn output(&self) -> output::Output<'_> {
        output::Output {
            doc: &self.doc,
            machine: self.machines.active(),
            posts: &self.posts,
            selected_toolpath: self.selected_toolpath,
            selected_group: self.selected_group.as_deref(),
            job_path: self.session.path.as_deref(),
            tolerances: &self.options.toolpaths,
        }
    }

    /// Why `change` may not be made: a toolpath it makes would cut into a keep-out zone,
    /// Options not allowing it to be clipped there (or it being a laser or drill bank
    /// toolpath, which are not clipped). A clamp is never cut by mistake.
    pub(super) fn keep_out_refusal(&self, change: &Change) -> Option<String> {
        let mut clippable = false;
        let refused: Vec<String> = change
            .toolpaths_making()
            .filter_map(|t| {
                let zones = self.zones_entered(t);
                clippable |= !zones.is_empty() && can_clip(t);
                (!zones.is_empty())
                    .then(|| format!("{} would cut into {}", t.name, zones.join(" and ")))
            })
            .collect();
        // clipping can't help a laser or drill bank toolpath: they are never clipped
        let way = if self.doc.clip_keep_out || !clippable {
            "Move the part or the zone (laser and drill bank toolpaths are not clipped)"
        } else {
            &format!(
                "Move the part or the zone, or let toolpaths be clipped there ({CLIP_SETTING})"
            )
        };
        (!refused.is_empty()).then(|| format!("{KEEP_OUT_REFUSED}: {}. {way}", refused.join("; ")))
    }

    /// Recalculate Toolpath: the selected one, worked out again.
    pub(super) fn recalculate_selected(&mut self) {
        if let Some(id) = self.selected_toolpath {
            self.recalculate_in_background(vec![id], "Toolpath recalculated", |_| {});
        }
    }

    /// Recalculate Visible Toolpaths: the cascade, Recalculate on each shown toolpath.
    pub(super) fn recalculate_shown(&mut self) {
        let shown = self.doc.toolpaths.iter().filter(|(_, t)| t.visible);
        let ids: Vec<EntityId> = shown.map(|(&id, _)| id).collect();
        self.recalculate_in_background(ids, "Visible toolpaths recalculated", |_| {});
    }

    pub(super) fn recalculate_all(&mut self) {
        let ids: Vec<EntityId> = self.doc.toolpaths.keys().copied().collect();
        self.recalculate_in_background(ids, "All toolpaths recalculated", |_| {});
    }

    /// The toolpath list's button: the Delete Toolpath command on the selected one.
    pub(super) fn delete_selected_toolpath(&mut self) {
        if let Some(id) = self.selected_toolpath.take()
            && let Err(e) = self.apply("delete_toolpath", &serde_json::json!({ "toolpath": id }))
        {
            self.status.set_message(&e);
        }
    }

    /// Save Toolpaths' form as it opens: every shown toolpath in one file through the
    /// machine's default post.
    pub(super) fn default_save_form(&self) -> SaveForm {
        SaveForm {
            mode: files::SaveMode::VisibleToOneFile,
            group: false,
            post: None,
            add_side: self.doc.job.double_sided.is_some(),
            changes: self.default_changes(),
            job_sheet: true,
        }
    }

    /// The posts of the active machine that suit this job (flat, or wrapping its side), for
    /// Save Toolpaths to choose from; none without a machine (Cut Craft's own is used).
    pub(super) fn post_choices(&self) -> Vec<String> {
        cut_craft_machines::post_choices(self.machines.active(), &self.doc.job, &self.posts)
    }

    /// Save Toolpaths' Save: the checks before cutting, a file name (the dialog opening in
    /// the project's folder in the machine's G-code programs folder, when it has one), then
    /// each file of the form's plan through its post (and each tile, when the job is tiled),
    /// with the job sheet beside them when the form says so.
    pub(super) fn save_toolpaths(&mut self, form: &SaveForm) {
        let Some(post) = self.ready_to_save(form, false) else {
            return;
        };
        let extension = post.extension.clone();
        let which = Dialog::SaveToolpaths {
            post: &post.name,
            extension: &extension,
        };
        let mut dialog = filtered_dialog(which, &self.registry);
        dialog = dialog.set_file_name(self.output().file_name(form, &post));
        let (key, name) = self.project();
        let programs = self
            .machines
            .active_mut()
            .and_then(|m| m.send_to.programs_folder(&key, &name));
        if let Some(folder) = &programs {
            dialog = dialog.set_directory(folder);
            self.machines_changed("");
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let path = files::with_extension_added(&path, &extension);
        self.save_over_earlier(form, post, path);
    }

    /// Save Toolpaths' Save once the file is named: straight on when no earlier save left
    /// G-code there, else as Options say (Replace or Archive), or held for the question.
    pub(super) fn save_over_earlier(
        &mut self,
        form: &SaveForm,
        post: cut_craft_post::Post,
        path: std::path::PathBuf,
    ) {
        let earlier = files::earlier_save(&path);
        if earlier.is_empty() {
            self.finish_save(form, &post, &path);
            return;
        }
        match self.options.confirmations.earlier_gcode {
            crate::options::EarlierGcode::Ask => {
                self.earlier_ask = Some(super::EarlierAsk {
                    form: form.clone(),
                    post,
                    path,
                    earlier,
                    remember: false,
                    sending: false,
                });
            }
            choice => self.save_with(choice, form, &post, &path, &earlier),
        }
    }

    /// The answer to the earlier-save question: `None` saves nothing. `remember` keeps the
    /// answer as the default (Edit ▸ Options ▸ Confirmation Choices).
    pub(super) fn answer_earlier(
        &mut self,
        choice: Option<crate::options::EarlierGcode>,
        remember: bool,
    ) {
        let Some(ask) = self.earlier_ask.take() else {
            return;
        };
        let Some(choice) = choice else {
            return self
                .status
                .set_message(&cut_craft_i18n::tr!("status-toolpaths-not-saved"));
        };
        if remember {
            self.options.confirmations.earlier_gcode = choice;
            self.save_options();
        }
        if ask.sending {
            return self.send_choosing(&ask.form, ask.post, choice);
        }
        self.save_with(choice, &ask.form, &ask.post, &ask.path, &ask.earlier);
    }

    /// Replaces (deletes) or archives `earlier`, then saves: see [`output::write_over`].
    fn save_with(
        &mut self,
        choice: crate::options::EarlierGcode,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
        earlier: &[std::path::PathBuf],
    ) {
        let archiving = choice == crate::options::EarlierGcode::Archive;
        let made = chrono::Local::now().naive_local();
        let vars = self.note_variables();
        let output = self.output();
        let texts = || output.texts(form, post, path, made, &vars);
        match output::write_over(texts, path, earlier, archiving, made) {
            Ok(over) => {
                let after = over.archived.map_or_else(String::new, |folder| {
                    format!("; the earlier files are in {}", folder.display())
                });
                self.saved(form, path, &over.written, &after);
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Writes the files (and the job sheet) and says what was saved.
    fn finish_save(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) {
        match self.write_toolpath_files(form, post, path) {
            Ok(written) => self.saved(form, path, &written, ""),
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Once the G-code files `written` are saved at `path`: the job sheet beside them when
    /// the form says so, and the status bar saying what was saved, then `after`.
    fn saved(
        &mut self,
        form: &SaveForm,
        path: &std::path::Path,
        written: &[std::path::PathBuf],
        after: &str,
    ) {
        let (_, name) = self.project();
        let done = self.output().saved_message(written, path);
        let result = if form.job_sheet {
            let folder = path.parent().unwrap_or(std::path::Path::new("."));
            let outputs = (written.iter())
                .map(|p| (file_name(p), p.is_file()))
                .collect();
            self.job_sheet_with(outputs, folder, &name)
                .map(|sheet| format!("{done}, with the job sheet {sheet}"))
        } else {
            Ok(done)
        };
        self.status
            .set_message(&result.map_or_else(|e| e, |done| format!("{done}{after}")));
    }

    /// Save Toolpaths' post and the checks before cutting, asked about when they find
    /// something not ticked as read (and so may say no): the post; `None` when not saving,
    /// the status bar saying why.
    pub(super) fn ready_to_save(
        &mut self,
        form: &SaveForm,
        sending: bool,
    ) -> Option<cut_craft_post::Post> {
        // a Save Anyway answer is for this save only, whichever way it ends
        let accepted = std::mem::take(&mut self.checks_accepted);
        let post = match self.form_post(form) {
            Ok(post) => post,
            Err(e) => {
                self.status.set_message(&e);
                return None;
            }
        };
        let plan = match self.plan(form, &post) {
            Ok(plan) => plan,
            Err(e) => {
                self.status.set_message(&e);
                return None;
            }
        };
        let Some(found) = self.save_checks(&plan, form.changes).map(<[_]>::to_vec) else {
            self.status
                .set_message(&cut_craft_i18n::tr!(if self.checking() {
                    "status-still-checking"
                } else if self.busy.is_some() {
                    "status-checked-after-work"
                } else {
                    "status-checks-cancelled"
                }));
            return None;
        };
        match output::verdict(&found, &self.acknowledged, &accepted) {
            output::Verdict::Refused(refused) => {
                self.status.set_message(&refused);
                None
            }
            output::Verdict::Ask(problems) => {
                // asked over the window: the answer saves (or sends) again
                self.ask_save_anyway(form, problems, sending);
                None
            }
            output::Verdict::Go => Some(post),
        }
    }

    /// The project, as its folders for G-code are kept: by its file ("" before it is saved)
    /// and named after it ("Untitled" before it is saved).
    pub(super) fn project(&self) -> (String, String) {
        self.session.path.as_deref().map_or_else(
            || (String::new(), "Untitled".to_owned()),
            |path| {
                let kept = crate::clipart::network::kept(path);
                let name = path
                    .file_stem()
                    .map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned());
                (kept.display().to_string(), name)
            },
        )
    }

    /// The job sheet for the active sheet and side, its list of toolpath files `outputs`
    /// (each with whether it is there), saved in `folder` as `<name> job sheet.html`; its
    /// file name.
    pub(super) fn job_sheet_with(
        &mut self,
        outputs: Vec<(String, bool)>,
        folder: &std::path::Path,
        name: &str,
    ) -> Result<String, String> {
        let mut setup = self.job_setup();
        setup.outputs = outputs;
        let html = fmt_create_job_sheet::sheet_for(&self.doc, &self.options.job_sheet, &setup)
            .map_err(|e| format!("Toolpaths saved, but not the job sheet: {e}"))?;
        let sheet =
            cut_craft_machines::send::safe_file_name(&format!("{name} job sheet.html"), false);
        let path = folder.join(&sheet);
        cut_craft_core::file::write_atomic(&path, html.as_bytes()).map_err(|e| {
            format!(
                "Toolpaths saved, but not the job sheet: could not write {}: {e}",
                path.display()
            )
        })?;
        Ok(sheet)
    }

    /// Saves the form's plan through `post` at `path`, unless the checks before cutting
    /// find an error, and says what was saved: Save Toolpaths' Save short of its dialogs.
    #[cfg(test)]
    pub(super) fn save_toolpaths_to(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let plan = self.plan(form, post)?;
        match output::refused(&self.preflight(&plan, form.changes)) {
            Some(refused) => Err(refused),
            None => self.write_toolpaths(form, post, path),
        }
    }

    /// The active machine has a tool changer (slots in Machine Configuration); with no
    /// machine set up there is none.
    pub(super) fn has_tool_changer(&self) -> bool {
        self.output().has_tool_changer()
    }

    /// How tools are changed unless the form says otherwise: by the tool changer when the
    /// active machine has one, else as its Machine Configuration says (a file for each
    /// tool, or one file that pauses).
    pub(super) fn default_changes(&self) -> files::ToolChanges {
        let machine = self.machines.active();
        if self.has_tool_changer() {
            files::ToolChanges::ToolChanger
        } else if machine.is_some_and(|m| m.hand_changes == cut_craft_machines::HandChanges::Pause)
        {
            files::ToolChanges::Pause
        } else {
            files::ToolChanges::SeparateFiles
        }
    }

    /// The post the form saves through: see [`output::Output::post`].
    pub(super) fn form_post(&self, form: &SaveForm) -> Result<cut_craft_post::Post, String> {
        self.output().post(form)
    }

    /// The form's toolpaths, file by file: see [`output::Output::plan`].
    pub(super) fn plan(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
    ) -> Result<Vec<Vec<EntityId>>, String> {
        self.output().plan(form, post)
    }

    /// The checks before cutting for `plan` (Save Toolpaths', or the Analyse tab's), worked
    /// out again only when the plan, the job or the machine changed. They run off the
    /// window's thread, with their progress and Cancel over the window: a 3D finish of
    /// millions of moves takes a while to check, and the window must keep drawing. `None`
    /// while they run, or once they were cancelled ([`Self::check_again`] runs them again).
    pub(super) fn save_checks(
        &mut self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Option<&[cut_craft_core::preflight::Finding]> {
        let key = SaveChecks {
            plan: plan.to_vec(),
            changes,
            revision: self.history.revision(),
            machine: self.machines.active,
            found: None,
        };
        if !self.save_checks.iter().any(|c| c.same(&key)) {
            // one job at a time: other work (or the other place's checks, as the Analyse
            // tab's and Save Toolpaths' can both ask in one frame) is not waited on here,
            // on the window's thread; these start when asked again once it is done
            if self.busy.is_some() {
                return None;
            }
            // the Analyse tab's and Save Toolpaths' can both be showing: keep one for each
            if self.save_checks.len() >= super::KEPT_CHECKS {
                self.save_checks.remove(0);
            }
            self.save_checks.push(key.clone());
            let doc = self.doc.clone();
            let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
            let limits = self.check_limits(&ids, changes);
            let mut no_changer: Vec<_> = self.no_tool_changer_for(changes).into_iter().collect();
            let machine = self.machines.active();
            no_changer.extend(super::bit_changes::findings(
                &self.doc, machine, plan, changes,
            ));
            let started = key.clone();
            self.in_background(
                super::CHECKING,
                "check",
                move || {
                    let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> =
                        ids.iter().filter_map(|id| doc.toolpaths.get(id)).collect();
                    let mut found = cut_craft_core::preflight::check(&doc, &toolpaths, &limits);
                    found.extend(no_changer);
                    found
                },
                move |app, found| {
                    if let Some(checks) = app.save_checks.iter_mut().find(|c| c.same(&started)) {
                        checks.found = Some(found);
                    }
                },
            );
        }
        self.save_checks
            .iter()
            .find(|c| c.same(&key))?
            .found
            .as_deref()
    }

    /// Whether the checks before cutting are running now (else, when [`Self::save_checks`]
    /// has none, they were cancelled).
    pub(super) fn checking(&self) -> bool {
        self.busy
            .as_ref()
            .is_some_and(|b| b.what() == super::CHECKING)
    }

    /// Forgets the checks before cutting, as the machine they used changed: those made, and
    /// one still running, whose findings were for the machine as it was.
    pub(super) fn forget_save_checks(&mut self) {
        if self.checking()
            && let Some(running) = self.busy.take()
        {
            running.cancel();
        }
        self.save_checks.clear();
    }

    /// Runs the cancelled checks before cutting again, next time they are asked for.
    pub(super) fn check_again(&mut self) {
        self.save_checks.retain(|c| c.found.is_some());
    }

    /// What the checks before cutting find in `plan`'s toolpaths, cut in its order on the
    /// active machine; its tool changer's slots are checked only when it changes the tools.
    #[cfg(test)]
    pub(super) fn preflight(
        &self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Vec<cut_craft_core::preflight::Finding> {
        let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
        let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> = ids
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        let limits = self.check_limits(&ids, changes);
        let mut found = cut_craft_core::preflight::check(&self.doc, &toolpaths, &limits);
        found.extend(self.no_tool_changer_for(changes));
        found
    }

    /// The warning that the tool changer is chosen in `changes` but the active machine has
    /// none set up, if it is.
    pub(super) fn no_tool_changer_for(
        &self,
        changes: files::ToolChanges,
    ) -> Option<cut_craft_core::preflight::Finding> {
        self.output().no_tool_changer_for(changes)
    }

    /// Writes the form's plan through `post` at `path`: each file, or each tile of each file
    /// when the job is tiled, and says what was saved.
    #[cfg(test)]
    fn write_toolpaths(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        let written = self.write_toolpath_files(form, post, path)?;
        Ok(self.output().saved_message(&written, path))
    }

    /// Writes the form's plan through `post` at `path` (and beside it); the files written,
    /// in the order they are run.
    pub(super) fn write_toolpath_files(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        let made = chrono::Local::now().naive_local();
        output::write(&self.toolpath_texts(form, post, path, made)?)
    }

    /// The files the form's plan makes through `post` at `path`, with their G-code as made
    /// at `made`: see [`output::Output::texts`].
    pub(super) fn toolpath_texts(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
        made: chrono::NaiveDateTime,
    ) -> Result<Vec<(std::path::PathBuf, String)>, String> {
        let vars = self.note_variables();
        self.output().texts(form, post, path, made, &vars)
    }

    /// The post toolpaths are saved through when none is picked: the active machine's
    /// choice for this job (see [`cut_craft_machines::post_for`]).
    pub(super) fn output_post(&self) -> Result<cut_craft_post::Post, String> {
        cut_craft_machines::post_for(self.machines.active(), &self.doc.job, &self.posts, None)
    }

    /// What the active machine can do, for the checks before cutting; nothing is checked
    /// against a machine without one.
    pub(super) fn limits(&self) -> cut_craft_core::preflight::Limits {
        let mut limits =
            self.machines
                .active()
                .map_or_default(|m| cut_craft_core::preflight::Limits {
                    name: m.name.clone(),
                    bed: [m.width, m.height],
                    z_travel: m.depth,
                    max_rate: m.max_rate,
                    spindle: m.spindle,
                    slots: m.slots.clone(),
                    left: None,
                    library: None,
                    holding: m.holding(),
                    rotary: m.has_rotary().then_some(m.rotary_axis),
                });
        limits.left = Some(self.options.confirmations.left());
        limits
    }

    /// [`Self::limits`] for checking the toolpaths `ids` saved with `changes`: the tool
    /// changer's slots only when it changes the tools, and the Tool Database's copy of
    /// each of their bits, to compare with.
    pub(super) fn check_limits(
        &self,
        ids: &[EntityId],
        changes: files::ToolChanges,
    ) -> cut_craft_core::preflight::Limits {
        let mut limits = self.limits();
        if changes != files::ToolChanges::ToolChanger {
            limits.slots.clear();
        }
        let mut library = std::collections::BTreeMap::new();
        for toolpath in ids.iter().filter_map(|id| self.doc.toolpaths.get(id)) {
            let name = &toolpath.tool.name;
            // a bit copied in from a maker's catalogue is compared with the catalogue's
            let catalogue =
                (self.tools.tools.iter()).find(|t| t.name == *name && self.tools.is_catalogue(t));
            if let Some(listed) = catalogue.or_else(|| self.tools.get(name)) {
                library.insert(name.clone(), listed.for_material(&self.tools.material));
            }
        }
        limits.library = Some(library);
        limits
    }

    /// The checks before cutting that cannot be made on `plan` saved with `changes`, each
    /// with why.
    pub(super) fn not_checked(
        &self,
        plan: &[Vec<EntityId>],
        changes: files::ToolChanges,
    ) -> Vec<(cut_craft_core::preflight::Check, String)> {
        let ids: Vec<EntityId> = plan.iter().flatten().copied().collect();
        let toolpaths: Vec<&cut_craft_core::toolpath::Toolpath> = ids
            .iter()
            .filter_map(|id| self.doc.toolpaths.get(id))
            .collect();
        cut_craft_core::preflight::not_checked(
            &self.doc,
            &toolpaths,
            &self.check_limits(&ids, changes),
        )
    }

    /// Whether every warning in `found` was ticked as read.
    pub(super) fn all_acknowledged(&self, found: &[cut_craft_core::preflight::Finding]) -> bool {
        output::all_acknowledged(found, &self.acknowledged)
    }

    /// How the active machine holds its bits, for the collet and holder checks (the usual
    /// holder without a machine).
    pub(super) fn holding(&self) -> cut_craft_core::collision::Holding {
        self.machines
            .active()
            .map_or_default(cut_craft_machines::Machine::holding)
    }

    /// `params` for command `id` with what a rotary toolpath must keep its holder clear of
    /// (the active machine's holder for its bit, chuck and tailstock) filled in; other
    /// commands' as they are.
    pub(super) fn with_surroundings(&self, id: &str, params: &Value) -> Value {
        use cut_craft_core::rotary_axis::{RotaryAxis, Surroundings};
        if !ROTARY_SURROUNDED.contains(&id) {
            return params.clone();
        }
        let tool: cut_craft_core::toolpath::Tool = params
            .get("tool")
            .and_then(|t| serde_json::from_value(t.clone()).ok())
            .unwrap_or_default();
        let machine = self.machines.active();
        let axis = machine
            .filter(|m| m.has_rotary())
            .map_or_else(RotaryAxis::default, |m| m.rotary_axis);
        let around = Surroundings::of(&tool, &self.holding(), &axis);
        let mut params = params.clone();
        if let (Value::Object(map), Ok(around)) = (&mut params, serde_json::to_value(around)) {
            map.insert("surroundings".into(), around);
            if ROTARY_CARRIED.contains(&id) {
                map.insert(
                    "whole_blank".into(),
                    self.options.toolpaths.advanced_rotary.into(),
                );
            }
        }
        params
    }
}

/// How a toolpath refused for cutting into a keep-out zone is said to be (the form's message
/// then explains how to allow clipping when hovered).
pub(super) const KEEP_OUT_REFUSED: &str = "Not made, to keep out of a keep-out zone";

/// Whether clipping at keep-out zones can keep `toolpath` out of them: not a laser's (its
/// power goes with each move) or a drill bank's (its head plunges whole).
pub(super) fn can_clip(toolpath: &cut_craft_core::toolpath::Toolpath) -> bool {
    toolpath.power.is_empty() && toolpath.drill_bank.is_none()
}

/// Where clipping at keep-out zones is turned on.
pub(super) const CLIP_SETTING: &str =
    "Edit ▸ Options ▸ Toolpath Settings ▸ Clip toolpaths at keep-out zones";

/// Rotary toolpaths, given the machine's holder, chuck and tailstock to keep clear of.
pub(super) const ROTARY_SURROUNDED: [&str; 6] = [
    "rotary_roughing_toolpath",
    "rotary_finishing_toolpath",
    "rotary_locked_angles_toolpath",
    "rotary_details_toolpath",
    "rotary_machining_toolpath",
    "rotary_lathe_toolpath",
];

/// Rotary toolpaths that cut only what the toolpaths above them leave, unless Settings'
/// advanced rotary mode is on.
pub(super) const ROTARY_CARRIED: [&str; 4] = [
    "rotary_roughing_toolpath",
    "rotary_finishing_toolpath",
    "rotary_locked_angles_toolpath",
    "rotary_details_toolpath",
];
