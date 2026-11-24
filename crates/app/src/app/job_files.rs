//! Job files: File and template actions, opening, saving, recent files, recovery, new jobs and Job
//! Setup's buttons.

use super::actions::{DONT_SAVE, SAVE};
use super::{
    APP_NAME, CutCraftApp, Dialog, Document, History, JOB_EXTENSION, OpenForm, Screen, Session,
    Shown, TEMPLATE, Value, View, egui, file_dialog, file_name, files, filtered_dialog, form, gui,
    job_dialog, message_dialog, saving_path, template_dialog, toolpath_template_dialog,
};

impl CutCraftApp {
    pub(super) fn file_action(&mut self, id: &str) {
        match id {
            // a new job opens blank, made with Options ▸ Job Defaults; the classic flow
            // starts it in Job Setup
            "new" if !self.options.job_defaults.classic_startup => {
                if self.confirm_discard() {
                    self.start_blank_job();
                    self.status
                        .set_message(&format!("New job: {}", self.material_summary()));
                }
            }
            "new" => {
                if self.confirm_discard() {
                    self.reset_job();
                    // the material being set up shows in the 2D view
                    self.view = View::TwoD;
                    self.start_command("job_setup");
                    if let Some(form) = &mut self.form {
                        form.new_job = true;
                    }
                    self.screen = Screen::NewJob;
                }
            }
            "open" => {
                if self.confirm_discard()
                    && let Some(path) = job_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            "close" => {
                if self.confirm_discard() {
                    if self.options.job_defaults.classic_startup {
                        self.reset_job();
                        self.screen = Screen::Start;
                    } else {
                        // no start page: a blank job to carry on with
                        self.start_blank_job();
                    }
                }
            }
            "save" => match self.session.path.clone() {
                Some(path) => self.save_to(&path),
                None => self.file_action("save_as"),
            },
            "save_as" => self.save_as(),
            "import_vectors_bitmap" | "import_component_3d_model" => {
                let which = if id == "import_component_3d_model" {
                    Dialog::ImportModel
                } else {
                    Dialog::ImportVectorsOrPicture
                };
                if let Some(path) = filtered_dialog(which, &self.registry).pick_file() {
                    self.import_file(&path);
                }
            }
            "import_cut_list" => {
                if let Some(path) =
                    filtered_dialog(Dialog::ImportCutList, &self.registry).pick_file()
                {
                    self.import_cut_list(&path);
                }
            }
            "add_clipart_folder" => {
                if let Some(folder) = file_dialog().pick_folder()
                    && let Err(e) = self.clipart.add(folder)
                {
                    self.status.set_message(&e);
                }
            }
            "segment_3d_model_file" => {
                // the segmenting form, on an STL picked first
                if let Some(path) =
                    filtered_dialog(Dialog::SegmentModel, &self.registry).pick_file()
                {
                    self.start_command("segment_3d_model");
                    if let Some(form) = self.form.as_mut().filter(|f| f.id == "segment_3d_model") {
                        form.params["file"] = Value::from(path.display().to_string());
                    }
                }
            }
            "import_toolpaths_file" => {
                // G-code read back as toolpaths: the form opens on the file picked, to say
                // which tool cut it
                if let Some(path) =
                    filtered_dialog(Dialog::ImportToolpaths, &self.registry).pick_file()
                {
                    self.import_toolpaths_from(&path);
                }
            }
            "install_post_processor" => {
                if let Some(path) =
                    filtered_dialog(Dialog::InstallPostProcessor, &self.registry).pick_file()
                {
                    self.install_post(&path);
                }
            }
            _ => unreachable!("not a file action: {id}"),
        }
    }

    /// Job and toolpath templates, saved and loaded through the templates folder.
    pub(super) fn template_action(&mut self, id: &str) {
        match id {
            "save_as_template" => {
                if let Some(path) = template_dialog()
                    .set_file_name(format!("template.{TEMPLATE}"))
                    .save_file()
                    && let Some(path) = saving_path(&path, &[TEMPLATE])
                {
                    match cut_craft_core::file::write_atomic(
                        &path,
                        &cut_craft_core::file::save_job(&self.doc),
                    ) {
                        Ok(()) => self
                            .status
                            .set_message(&format!("Saved template {}", path.display())),
                        Err(e) => self
                            .status
                            .set_message(&crate::messages::could_not_save(&path, &e)),
                    }
                }
            }
            "save_selected_toolpath_as_template" | "save_all_visible_toolpaths_as_template" => {
                let ids: Vec<_> = if id == "save_selected_toolpath_as_template" {
                    self.selected_toolpath.into_iter().collect()
                } else {
                    self.doc
                        .toolpaths
                        .iter()
                        .filter(|(_, t)| t.visible)
                        .map(|(&i, _)| i)
                        .collect()
                };
                if let Some(path) = toolpath_template_dialog()
                    .set_file_name(format!("toolpaths.{}", files::TOOLPATH_TEMPLATE_EXTENSION))
                    .save_file()
                    && let Some(path) = saving_path(&path, &[files::TOOLPATH_TEMPLATE_EXTENSION])
                {
                    let template = files::toolpath_template(&self.doc, &ids);
                    let message = match cut_craft_core::file::write_atomic(&path, &template) {
                        Ok(()) => format!("Saved toolpath template {}", path.display()),
                        Err(e) => crate::messages::could_not_save(&path, &e),
                    };
                    self.status.set_message(&message);
                }
            }
            "load_template" => {
                if let Some(path) = toolpath_template_dialog().pick_file() {
                    let change = std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| files::load_toolpath_template(&self.doc, &bytes))
                        .map_err(cut_craft_core::CommandError::Failed);
                    self.record(change, &format!("Loaded {}", path.display()));
                }
            }
            "new_from_template" => {
                // like New and Open: unsaved work is asked about first
                if self.confirm_discard()
                    && let Some(path) = template_dialog().pick_file()
                {
                    self.open_path(&path);
                }
            }
            _ => unreachable!("not a template action: {id}"),
        }
    }

    /// Runs a registered command: straight away if it has no settings, else opens its form.
    pub(super) fn start_command(&mut self, id: &str) {
        // unsigned code runs only once the user has said yes to it, however it is started
        if !self.may_run_gadget(id) {
            return;
        }
        self.import_level = None;
        // a new job's setup is accepted, as OK would, before another form takes its place:
        // the new-job screen shows only that form, so it would be left bare (issue #17)
        if id != "job_setup" && self.form.as_ref().is_some_and(|f| f.new_job) {
            self.job_setup_ok();
            // settings it refuses keep it open, saying why
            if self.screen == Screen::NewJob {
                return;
            }
        }
        let Some(cmd) = self.registry.get(id) else {
            self.status
                .set_message(&format!("{}: not implemented yet", gui::title_case(id)));
            return;
        };
        let page = self.page_of(cmd.id());
        let (id, interactive) = (cmd.id(), cmd.interactive());
        let (meta, fields, mut params) = (
            cmd.meta(),
            form::fields(&cmd.schema()),
            cmd.initial_params(&self.doc),
        );
        // Orientate 3D Model offers what Options ▸ Importing and Exporting says; one kind alone
        // is the kind, with no choice shown
        if cmd.id() == "orientate_3d_model" {
            let offered = match self.options.import.model_types {
                crate::options::ModelTypes::Full => "full",
                crate::options::ModelTypes::Flat => "flat",
                crate::options::ModelTypes::Both => "both",
            };
            params["offered"] = Value::from(offered);
            if offered != "both" {
                params["model_type"] = Value::from(offered);
            }
            params["rotary"] = Value::from(self.doc.job.rotary.is_some());
        }
        self.fill_drill_bank(cmd.id(), &mut params);
        // the untouched default bit given the sizes a real one has, so a new form warns of
        // nothing the beginner did not set
        starter_tool(&mut params);
        // a toolpath's form in the standard order
        let fields = if fields.iter().any(|f| f.key == "tool") {
            form::arrange(fields)
        } else {
            fields
        };
        meta.label.clone_into(&mut self.status.current_tool);
        self.view_tool = None;
        if fields.is_empty() {
            if let Err(e) = self.apply(cmd.id(), &params) {
                self.status.set_message(&e);
            }
        } else {
            // a placing tool opened over a form keeps that form, edits and all, for Esc to go
            // back to; any other form lets it go
            if self.placing(id) {
                if let Some(behind) =
                    (self.form.take()).filter(|f| f.id != id && !f.new_job && !self.placing(f.id))
                {
                    self.form_behind = Some(behind);
                }
            } else {
                self.form_behind = None;
            }
            self.form = Some(OpenForm {
                id: cmd.id(),
                label: meta.label,
                icon: meta.icon,
                kind: meta.form,
                fields,
                params,
                error: None,
                warning: None,
                new_job: false,
                page,
                editing: None,
                drawn: None,
                dirty: false,
                suggested: None,
                shown_size: None,
                typed_size: [None; 3],
                shown_place: [None; 3],
                dragged: None,
                model: None,
                model_whole: true,
                beyond: None,
                applied: None,
                remade: None,
                undo_from: self.history.undo_depth(),
                list_steps: Vec::new(),
                in_list: false,
                way: crate::flyout::DragWay::AsIs,
                made: None,
                note: None,
                brief: false,
            });
            // a form shows in its page, so bring an auto-hidden page out
            if self.pages.shown(page) == Shown::Hidden {
                self.pages.open(page);
            }
            // a shape this tool draws, selected as it opens: the form edits that shape
            if let Some((shape, params)) = self.drawn_by_tool(id).filter(|_| interactive) {
                self.edit_shape(shape, id, params);
            }
        }
    }

    /// A toolbox variant of a drawing tool (Draw Circle from Edge): its command's form, named
    /// for the variant, reading the drag the variant's way.
    pub(super) fn start_variant(&mut self, id: &str) {
        let Some(variant) = crate::flyout::variant(id) else {
            return;
        };
        self.start_command(variant.command);
        if let Some(form) = self.form.as_mut().filter(|f| f.id == variant.command) {
            form.way = variant.way;
            form.label = variant.label;
            variant.label.clone_into(&mut self.status.current_tool);
        }
    }

    pub(super) fn save_to(&mut self, path: &std::path::Path) {
        // imported models' files found again from the job's folder, should the two move
        if let Some(folder) = path.parent() {
            for source in self
                .doc
                .components
                .values_mut()
                .filter_map(|k| k.source.as_mut())
            {
                source.relate_to(folder);
            }
        }
        match self.session.save(&self.doc, &self.history, path) {
            Ok(()) => {
                let mut saved = format!("Saved {}", path.display());
                // said again as it happens: the opening warning is long gone by now
                if !self.session.newer_settings.is_empty() {
                    saved = format!(
                        "{saved}. Warning: settings this version of Cut Craft does not know \
                         were left out ({}). Update Cut Craft (Help ▸ Check for Updates) and \
                         open the original to keep them",
                        self.session.newer_settings.join(", ")
                    );
                }
                self.status.set_message(&saved);
                self.remember(path);
                self.clear_recovery();
            }
            Err(e) => self.status.set_message(&e),
        }
    }

    /// Opens a job file in place of whatever is open (already confirmed); whether it opened
    /// (or, a big one, is still opening). A big job is read off the window's thread, its
    /// progress shown; one that will not open leaves the recent files.
    pub(super) fn open_path(&mut self, path: &std::path::Path) -> bool {
        let outcome = std::rc::Rc::new(std::cell::Cell::new(None));
        let told = std::rc::Rc::clone(&outcome);
        let (reading, path) = (path.to_owned(), path.to_owned());
        let what = format!("Opening {}", super::file_name(&path));
        self.in_background(
            &what,
            "open",
            move || {
                let mut session = Session::default();
                session
                    .open(&reading)
                    .map(|(doc, history)| (session, doc, history))
            },
            move |app, opened| told.set(Some(app.opened(&path, opened))),
        );
        outcome.get().unwrap_or(true)
    }

    /// A job file read (or why it could not be) put in place of whatever is open; whether
    /// it opened.
    fn opened(
        &mut self,
        path: &std::path::Path,
        opened: Result<(Session, Document, History), String>,
    ) -> bool {
        match opened {
            Ok((mut session, doc, history)) => {
                self.reset_job();
                let template = path
                    .extension()
                    .is_some_and(|e| e == cut_craft_core::file::TEMPLATE_EXTENSION);
                if template {
                    // a template opens as a new file: saving asks where, the template stays
                    session.path = None;
                }
                (self.doc, self.history, self.session) = (doc, history, session);
                // a fresh history counts its revisions from the start again
                self.picked_constraint = None;
                self.screen = Screen::Job;
                if !template {
                    self.remember(path);
                }
                if self.doc.notes.starts_with('.') {
                    self.start_command("notes");
                }
                // a job saved before one of its bits was renamed takes the new name
                let renamed: Vec<(String, String)> = (self.doc.toolpaths.values())
                    .filter_map(|t| {
                        let now = self.tools.renamed_now(&t.tool)?;
                        Some((t.tool.name.clone(), now.to_owned()))
                    })
                    .collect();
                let mut opened = format!("Opened {}", super::file_name(path));
                if !self.session.newer_settings.is_empty() {
                    opened = format!(
                        "{opened}. Warning: it was saved by a newer version of Cut Craft, and \
                         this one does not know some of its settings ({}): saving it here \
                         leaves them out. Update Cut Craft (Help ▸ Check for Updates) to keep \
                         them",
                        self.session.newer_settings.join(", ")
                    );
                }
                self.status.set_message(&opened);
                self.rename_toolpath_tools(&renamed, &opened);
                true
            }
            Err(e) => {
                self.status.set_message(&e);
                // only a file that has gone leaves Recent Files: a damaged one (perhaps a
                // sync still going) or one needing a newer Cut Craft stays to try again
                if !path.exists() && self.recent.iter().any(|p| p == path) {
                    self.recent.retain(|p| p != path);
                    self.save_recent();
                }
                false
            }
        }
    }

    /// Opens one of the recent files; one that no longer opens leaves the list.
    pub(super) fn open_recent(&mut self, index: usize) {
        let Some(path) = self.recent.get(index).cloned() else {
            return;
        };
        if self.confirm_discard() {
            self.open_path(&path);
        }
    }

    /// The recent files listed: as many as Options lists. More are kept (up to
    /// [`files::RECENT_KEPT`]), so a smaller list only hides the rest until it grows again.
    pub(super) fn recent_shown(&self) -> &[std::path::PathBuf] {
        &self.recent[..self.recent.len().min(self.options.recent_count())]
    }

    pub(super) fn remember(&mut self, path: &std::path::Path) {
        files::remember(&mut self.recent, path, files::RECENT_KEPT);
        self.save_recent();
    }

    pub(super) fn save_recent(&mut self) {
        if let Some(path) = &self.recent_path
            && let Err(e) = files::save_recent(&self.recent, path)
        {
            self.status.set_message(&e);
        }
    }

    pub(super) fn load_recent(&mut self) {
        self.recent_path = files::recent_files_path();
        if let Some(path) = &self.recent_path {
            self.recent = files::load_recent(path);
        }
    }

    /// Once a frame: unsaved work copied to the recovery file as often as Options says,
    /// on a thread of its own so a big job does not stall the window.
    pub(super) fn autosave(&mut self, ctx: &egui::Context) {
        if let Some((revision, writing)) = self.autosave.take() {
            if writing.is_finished() {
                let outcome = writing
                    .join()
                    .unwrap_or_else(|_| Err("the recovery copy could not be written".into()));
                match outcome {
                    // kept: no copy again until something changes
                    Ok(()) => self.autosaved.0 = revision,
                    // tried again once the interval is up
                    Err(e) => self.status.set_message(&e),
                }
            } else {
                self.autosave = Some((revision, writing));
            }
        }
        let now = ctx.input(|i| i.time);
        // once kept for the new version, that copy stays as it is
        let unsaved = self.screen == Screen::Job
            && !self.work_kept_for_update
            && self.session.is_dirty(&self.history)
            && self.history.revision() != self.autosaved.0;
        if unsaved
            && now - self.autosaved.1 >= self.options.general.autosave_interval()
            && self.autosave.is_none()
            && let Some(path) = self.recovery_path.clone()
        {
            // the time of the try spaces out the next; the revision counts once written
            self.autosaved.1 = now;
            let doc = self.doc.clone();
            // the job's own file goes with it, so recovered work saves back there
            let note = files::RecoveryNote {
                path: self.session.path.clone(),
                after_update: false,
            };
            let writing = std::thread::spawn(move || {
                files::write_recovery(&path, &cut_craft_core::file::save_job(&doc), &note)
                    .map_err(|e| format!("could not keep a recovery copy: {e}"))
            });
            self.autosave = Some((self.history.revision(), writing));
        }
    }

    /// The work is saved or let go on purpose: nothing is left to recover.
    pub(super) fn clear_recovery(&mut self) {
        if let Some((_, writing)) = self.autosave.take() {
            let _ = writing.join();
        }
        if let Some(path) = &self.recovery_path {
            files::remove_recovery(path, false);
        }
        self.autosaved.0 = self.history.revision();
    }

    /// Install Now is closing Cut Craft to start the new version: unsaved work is written to
    /// this run's recovery copy, with a note of the job's own file, for the new version to
    /// open just as it was, still unsaved; the job's own file is not touched. Nothing is
    /// written when nothing is unsaved, and the work is not kept when the copy cannot be
    /// written (closing then asks "Save changes?" as before).
    pub(super) fn hand_off_for_update(&mut self) -> HandOff {
        if self.screen != Screen::Job || !self.session.is_dirty(&self.history) {
            return HandOff::NothingUnsaved;
        }
        let Some(path) = self.recovery_path.clone() else {
            return HandOff::NotKept;
        };
        // a copy still being written would finish over this one
        if let Some((_, writing)) = self.autosave.take() {
            let _ = writing.join();
        }
        let note = files::RecoveryNote {
            path: self.session.path.clone(),
            after_update: true,
        };
        match files::write_recovery(&path, &cut_craft_core::file::save_job(&self.doc), &note) {
            Ok(()) => {
                self.work_kept_for_update = true;
                // let go now: the new version may start before this one has closed
                self.recovery_lock = None;
                HandOff::Kept
            }
            Err(e) => {
                log::warn!("could not keep the work for the new version: {e}");
                HandOff::NotKept
            }
        }
    }

    /// At startup: this window's recovery copy claimed in the settings folder, and a crashed
    /// run's work opened if one left any.
    pub(super) fn start_recovery(&mut self) {
        if let Some(dir) = files::config_dir() {
            self.claim_recovery(&dir, &std::process::id().to_string());
        }
        self.open_recovery();
    }

    /// A file named on the command line (Open With, or dragged onto the launcher), handled
    /// much as if dropped on the window: a job opens; a drawing or picture is imported into
    /// the open job or, with none open (the start page), into a new one once Job Setup's OK
    /// has made it.
    pub(super) fn open_given(&mut self, path: Option<&std::path::Path>) {
        let Some(path) = path else {
            return;
        };
        if self.screen == Screen::Job || !Self::drop_imports(path) {
            self.drop_file(path);
            return;
        }
        if !self.options.job_defaults.classic_startup {
            // a blank job to bring it into, as if dropped on the window
            self.start_blank_job();
            self.drop_file(path);
            return;
        }
        self.file_action("new");
        if self.screen == Screen::NewJob {
            self.import_after_setup = Some(path.to_owned());
            self.status.set_message(&format!(
                "Set up the material, then OK makes the job and imports {} into it",
                file_name(path)
            ));
        }
    }

    /// This window's own recovery copy, in `dir`, named for `run` (the process), and locked
    /// for as long as the window is open (where the folder can lock files). A folder that
    /// cannot be written says so in the status bar.
    pub(super) fn claim_recovery(&mut self, dir: &std::path::Path, run: &str) {
        if let Some((own, lock)) = files::claim_recovery(dir, run) {
            (self.recovery_path, self.recovery_lock) = (Some(own), lock);
        } else {
            (self.recovery_path, self.recovery_lock) = (None, None);
            self.status.set_message(&format!(
                "Warning: unsaved work cannot be copied for recovery, because {} cannot be \
                 written. Save often",
                dir.display()
            ));
        }
    }

    /// The recovery copy a run that ended badly left, opened as an unsaved job; whether
    /// there was one. Another window's copy, still running, is left alone. The work becomes
    /// this window's copy, kept until the job is saved or closed.
    pub(super) fn open_recovery(&mut self) -> bool {
        let Some(own) = self.recovery_path.clone() else {
            return false;
        };
        let Some(left) = own
            .parent()
            .and_then(|dir| files::orphaned_recovery(dir, &own))
        else {
            return false;
        };
        let note = files::recovery_note(&left);
        let Some((bytes, doc)) = std::fs::read(&left).ok().and_then(|bytes| {
            let doc = cut_craft_core::file::load_job(&bytes).ok()?;
            Some((bytes, doc))
        }) else {
            return false;
        };
        self.reset_job();
        // kept as this run's copy, so the work is still there if this run ends badly too
        // before anything is changed (autosave writes it again then)
        let kept_note = files::RecoveryNote {
            after_update: false,
            ..note.clone()
        };
        match files::write_recovery(&own, &bytes, &kept_note) {
            Ok(()) => files::remove_recovery(&left, true),
            Err(e) => self
                .status
                .set_message(&format!("could not keep the recovery copy: {e}")),
        }
        self.doc = doc;
        self.session = Session::unsaved_at(note.path);
        self.screen = Screen::Job;
        if note.after_update {
            self.status
                .set_message(&super::tr!("update-your-unsaved-work-is-back"));
        } else {
            // Save goes back to the job's own file when it had one
            let save = if self.session.path.is_some() {
                "Save"
            } else {
                "Save As"
            };
            self.status.set_message(&format!(
                "Recovered unsaved work from the last run: save it (File ▸ {save}) or close it"
            ));
        }
        true
    }

    /// Back to nothing: no job, no form, no preview.
    pub(super) fn reset_job(&mut self) {
        (self.doc, self.history, self.session, self.form) = Default::default();
        // a form kept behind a placing tool was the last job's
        self.form_behind = None;
        self.picked_constraint = None;
        self.picked_outline = None;
        self.job_epoch += 1;
        // measured for the job before: a new history can be at the same revision
        self.model_tall.set(None);
        *self.note_vars.borrow_mut() = None;
        if let Some(job) = self.options.default_job {
            self.doc.job = job;
        }
        (self.camera, self.view_tool) = (None, None);
        (
            self.selected_toolpath,
            self.simulation,
            self.simulation_coarse,
            self.fine_3d,
            self.surface_gpu,
            self.rough_3d,
        ) = (None, None, None, None, None, None);
        self.turned_3d = None;
        // the old job's preview playback, its controls and its undo go with it
        (self.playback, self.kept_playback, self.before_preview) = (None, None, None);
        (self.playing, self.playback_goal, self.playback_shown) = (false, None, None);
        (self.playback_view, self.playback_gpu) = (None, None);
        self.preview_side = self.doc.bottom_side;
        // the G-code shown, and a save waiting or asked about, were the job that was open
        self.gcode_view = None;
        self.save_anyway = None;
        self.save_after = None;
        self.form_back = None;
        self.checks_accepted.clear();
        self.clear_recovery();
    }

    /// A blank job made with Options ▸ Job Defaults, open to draw on, with nothing to undo
    /// or save yet.
    pub(super) fn start_blank_job(&mut self) {
        self.reset_job();
        self.view = View::TwoD;
        self.screen = Screen::Job;
        "Select".clone_into(&mut self.status.current_tool);
    }

    /// What Cut Craft shows once it has started: the file named on the command line, if
    /// one was; else a blank job, unless Classic Startup Flow asks for the start page or a
    /// job is already open (recovered work).
    pub(super) fn open_at_start(&mut self, given: Option<&std::path::Path>) {
        self.open_given(given);
        if self.screen == Screen::Start && !self.options.job_defaults.classic_startup {
            self.start_blank_job();
        }
    }

    /// The material in one line, for the status bar and the foot of the sheet:
    /// "600 × 400 × 20 mm, single sided, Z zero on top, X0 Y0 bottom left".
    pub(super) fn material_summary(&self) -> String {
        super::setup_material::summary(&self.doc.job)
    }

    /// Set as Default in Job Setup: new jobs start with the job its settings make.
    pub(super) fn job_setup_default(&mut self) {
        let Some(form) = &mut self.form else { return };
        // made on an empty document, so only the job's own settings count; it starts from
        // the default kept now, so what is not on the form (Clearing Settings' Use as My
        // Default, the preview colours, where the model sits) is kept as it was
        let mut empty = Document::default();
        if let Some(job) = self.options.default_job {
            empty.job = job;
        }
        let made = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms")
            .run_json(&empty, &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| {
                change
                    .job()
                    .copied()
                    .ok_or_else(|| "no job made".to_owned())
            });
        match made {
            Ok(job) => {
                form.error = None;
                self.options.default_job = Some(job);
                self.save_options();
                self.status
                    .set_message("New jobs will start with these settings");
            }
            Err(e) => form.error = Some(e),
        }
    }

    /// OK in Job Setup for a new job: the job is made, with nothing to undo yet.
    pub(super) fn job_setup_ok(&mut self) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let cmd = self
            .registry
            .get(form.id)
            .expect("only registered commands have forms");
        let made = cmd
            .run_json(&self.doc, &form.params)
            .map_err(|e| e.to_string())
            .and_then(|change| self.doc.apply(&change).map_err(|e| e.to_string()));
        match made {
            Ok(()) => {
                (self.history, self.session) = Default::default();
                self.picked_constraint = None;
                self.screen = Screen::Job;
                self.status.set_message("New job created");
                "Select".clone_into(&mut self.status.current_tool);
                if let Some(path) = self.import_after_setup.take() {
                    self.import_file(&path);
                }
            }
            Err(e) => {
                form.error = Some(e);
                self.form = Some(form);
            }
        }
    }

    pub(super) fn job_setup_cancel(&mut self) {
        self.import_after_setup = None;
        self.reset_job();
        "Select".clone_into(&mut self.status.current_tool);
        self.screen = Screen::Start;
    }

    pub(super) fn export(&mut self, id: &str) {
        let Some(format) = self.registry.exporter(id).map(|e| e.format) else {
            return;
        };
        let dialog = filtered_dialog(Dialog::Export(id), &self.registry);
        if let Some(path) = dialog.save_file()
            && let Some(path) = saving_path(&path, format.extensions)
        {
            let result = files::export(&self.registry, id, &self.doc, &path);
            self.status.set_message(
                &result.map_or_else(|e| e, |()| format!("Exported {}", path.display())),
            );
        }
    }

    /// True when it is fine to throw the current job away: no unsaved work, or the user chose
    /// to save it or not to. False if they cancelled.
    pub(super) fn confirm_discard(&mut self) -> bool {
        if !self.session.is_dirty(&self.history) {
            return true;
        }
        let answer = message_dialog()
            .set_title(APP_NAME)
            .set_description(self.save_changes_question())
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                SAVE.into(),
                DONT_SAVE.into(),
                "Cancel".into(),
            ))
            .show();
        self.discard_answered(&answer)
    }

    /// File ▸ Save As: the job saved under the name and folder the person picks.
    fn save_as(&mut self) {
        let (folder, name) = self.save_as_suggestion();
        let dialog = filtered_dialog(Dialog::SaveJob, &self.registry).set_file_name(name);
        let dialog = match folder {
            Some(folder) => dialog.set_directory(folder),
            None => dialog,
        };
        if let Some(path) = dialog.save_file()
            && let Some(path) = saving_path(&path, &[JOB_EXTENSION])
        {
            self.save_to(&path);
        }
    }

    /// Where Save As starts: the job's own folder and name when it has a file, else
    /// `untitled.ccjob` wherever file dialogs open.
    pub(super) fn save_as_suggestion(&self) -> (Option<std::path::PathBuf>, String) {
        let Some(path) = &self.session.path else {
            return (None, format!("untitled.{JOB_EXTENSION}"));
        };
        let name = path.file_stem().map_or_else(
            || "untitled".to_owned(),
            |n| n.to_string_lossy().into_owned(),
        );
        (
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(std::path::Path::to_owned),
            format!("{name}.{JOB_EXTENSION}"),
        )
    }

    /// The question asked before unsaved work is replaced: the job's file name alone.
    pub(super) fn save_changes_question(&self) -> String {
        let name = self
            .session
            .path
            .as_deref()
            .and_then(std::path::Path::file_name)
            .map_or_else(
                || "Untitled".to_owned(),
                |n| n.to_string_lossy().into_owned(),
            );
        format!("Save changes to {name}?")
    }

    /// What the answer at "Save changes?" means: true when the job may be replaced. No only
    /// lets the work go: the job stays marked unsaved until something actually replaces it,
    /// so backing out afterwards (a cancelled file picker, a file that will not open) loses
    /// nothing and is asked about again.
    pub(super) fn discard_answered(&mut self, answer: &rfd::MessageDialogResult) -> bool {
        let answer = match answer {
            rfd::MessageDialogResult::Custom(c) if c == SAVE => &rfd::MessageDialogResult::Yes,
            rfd::MessageDialogResult::Custom(c) if c == DONT_SAVE => &rfd::MessageDialogResult::No,
            other => other,
        };
        match answer {
            rfd::MessageDialogResult::Yes => {
                let path = self.session.path.clone().or_else(|| {
                    filtered_dialog(Dialog::SaveJob, &self.registry)
                        .save_file()
                        .and_then(|path| saving_path(&path, &[JOB_EXTENSION]))
                });
                match path {
                    Some(path) => self.save_to(&path),
                    None => return false,
                }
                !self.session.is_dirty(&self.history)
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }
}

/// A form's starting bit, when it is the bare default ([`Tool::default`]), made the
/// starter bit ([`Tool::starter`]): the same 6 mm end mill with its flute length and shank.
pub(super) fn starter_tool(params: &mut Value) {
    use cut_craft_core::toolpath::Tool;
    let bare = serde_json::to_value(Tool::default()).expect("tools serialize");
    if let Some(tool) = params.get_mut("tool").filter(|t| **t == bare) {
        *tool = serde_json::to_value(Tool::starter()).expect("tools serialize");
    }
}

/// What [`CutCraftApp::hand_off_for_update`] did with the open job's unsaved work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HandOff {
    /// Nothing was unsaved: the job opens again from its own file.
    NothingUnsaved,
    /// Kept in the recovery copy for the new version to open, still unsaved.
    Kept,
    /// The copy could not be written: closing asks "Save changes?" as before.
    NotKept,
}
