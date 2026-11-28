//! Saving toolpaths: what the files hold, View G-code, the questions before a save, the
//! toolpath summary's times and notes, and first-time setup.

use super::super::*;

type Post = cut_craft_post::Post;
type Plan = Vec<Vec<EntityId>>;

impl Harness {
    // ---- what saving writes ----

    /// The toolpaths `form` saves through `post`, file by file.
    pub(in super::super) fn plan(&self, form: &SaveForm, post: &Post) -> Result<Plan, String> {
        self.app.plan(form, post)
    }

    /// The files saving with `form` through `post` to `path` writes, once its checks passed.
    pub(in super::super) fn write_toolpath_files(
        &mut self,
        form: &SaveForm,
        post: &Post,
        path: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, String> {
        self.app.write_toolpath_files(form, post, path)
    }

    /// What each file saving with `form` through `post` to `path` would hold, made at `made`.
    pub(in super::super) fn toolpath_texts(
        &self,
        form: &SaveForm,
        post: &Post,
        path: &std::path::Path,
        made: chrono::NaiveDateTime,
    ) -> Result<Vec<(std::path::PathBuf, String)>, String> {
        self.app.toolpath_texts(form, post, path, made)
    }

    /// The checks before cutting passed for the job and machine as they are now.
    pub(in super::super) fn given_checks_passed(&mut self) {
        let now = self.app.now_checked();
        self.app.saving.go(Vec::new(), now);
    }

    /// The checks before cutting kept, one set per plan, job and machine.
    pub(in super::super) fn kept_checks(&self) -> &[SaveChecks] {
        &self.app.save_checks
    }

    /// Save Toolpaths' window's choices as it opens on this job.
    pub(in super::super) fn default_save_form(&self) -> SaveForm {
        self.app.default_save_form()
    }

    /// The toolpath Save Toolpath Now saves: the one calculated last, if it saves at once.
    pub(in super::super) fn immediate_output_toolpath(&self) -> Option<EntityId> {
        self.app.immediate_output_toolpath()
    }

    /// Saves the job sheets beside `path`, one per side.
    pub(in super::super) fn write_job_sheets(&mut self, path: &std::path::Path) {
        self.app.write_job_sheets(path);
    }

    /// The notes' variables as the job stands.
    pub(in super::super) fn note_variables(
        &self,
    ) -> std::rc::Rc<Vec<cut_craft_core::notes::Variable>> {
        self.app.note_variables()
    }

    // ---- View G-code ----

    /// View G-code with Save Toolpaths' window set as `form`.
    pub(in super::super) fn view_gcode(&mut self, form: SaveForm) {
        self.app.view_gcode(form);
    }

    /// The G-code window, if it is open.
    pub(in super::super) fn gcode_view(&self) -> Option<&crate::app::gcode_view::GcodeView> {
        self.app.gcode_view.as_ref()
    }

    // ---- the questions before a save ----

    /// Asks Save Anyway? with `asking`, for `form`.
    pub(in super::super) fn ask_save_anyway(
        &mut self,
        form: &SaveForm,
        asking: crate::app::output::Asking,
        sending: bool,
    ) {
        self.app.ask_save_anyway(form, asking, sending);
    }

    /// Whether a save waits for the Save Toolpaths window to go first.
    pub(in super::super) fn save_waiting(&self) -> bool {
        self.app.save_after.is_some()
    }

    /// A save of `form` waits for the Save Toolpaths window to go, as Save does.
    pub(in super::super) fn given_save_waiting(&mut self, form: SaveForm, sending: bool) {
        self.app.save_after = Some(crate::app::gcode_view::SaveAfter::new(form, sending));
    }

    /// No save waits.
    pub(in super::super) fn given_no_save_waiting(&mut self) {
        self.app.save_after = None;
    }

    /// The Save Toolpaths window, stepped aside as `form`, comes back once a question is
    /// answered.
    pub(in super::super) fn given_form_stepped_aside(&mut self, form: SaveForm) {
        self.app.form_back = Some(form);
    }

    /// Whether the Save Toolpaths window is stepped aside to come back.
    pub(in super::super) fn form_stepped_aside(&self) -> bool {
        self.app.form_back.is_some()
    }

    /// Whether the finding `message` is ticked as read.
    pub(in super::super) fn ticked_as_read(&self, message: &str) -> bool {
        self.app.acknowledged.contains(message)
    }

    // ---- the toolpath summary ----

    /// The toolpath summary's settings: rate, units, scale, the toolpath picked, notes.
    pub(in super::super) fn summary(&self) -> &crate::app::settings::Summary {
        &self.app.summary
    }

    /// Changes the toolpath summary's settings, as its controls do.
    pub(in super::super) fn edit_summary(
        &mut self,
        edit: impl FnOnce(&mut crate::app::settings::Summary),
    ) {
        edit(&mut self.app.summary);
    }

    /// The minutes the summary gives toolpath `t`.
    pub(in super::super) fn estimated_minutes(
        &self,
        t: &cut_craft_core::toolpath::Toolpath,
    ) -> f64 {
        self.app.estimated_minutes(t)
    }

    /// The minutes the whole job takes, home and back.
    pub(in super::super) fn job_minutes(&self) -> f64 {
        self.app.job_minutes()
    }

    /// How the machine in use moves, as job times go by.
    pub(in super::super) fn motion(&self) -> cut_craft_core::cut_time::Motion {
        self.app.motion()
    }

    /// Toolpath `id` worked out again from its settings, with no undo step.
    pub(in super::super) fn given_remade(&mut self, id: EntityId) {
        let change = (self.app.registry)
            .edit(&self.app.doc, id, None)
            .expect("it remakes");
        self.app.doc.apply(&change).expect("it applies");
    }

    /// Another job's start, as far as what is kept for the last job goes.
    pub(in super::super) fn given_new_job_epoch(&mut self) {
        self.app.job_epoch += 1;
    }

    // ---- other windows ----

    /// Whether Print Part Labels is open.
    pub(in super::super) fn part_labels_open(&self) -> bool {
        self.app.part_labels_window
    }

    /// Whether the Job Setup Sheet Editor is open.
    pub(in super::super) fn job_sheet_editor_open(&self) -> bool {
        self.app.job_sheet_window
    }

    /// No crash-report service to send crash logs to.
    pub(in super::super) fn given_no_crash_service(&mut self) {
        self.app.reporter.crash_service = None;
    }

    /// Adds a new machine and makes it the one in use, as Machine Configuration's Add does.
    pub(in super::super) fn add_machine(&mut self) {
        self.app.machines.add();
    }

    // ---- first-time setup ----

    /// Whether first-time setup is open.
    pub(in super::super) fn first_run_open(&self) -> bool {
        self.app.first_run.is_some()
    }

    /// First-time setup as answered so far.
    pub(in super::super) fn first_run(&self) -> &crate::app::first_run::Setup {
        self.app
            .first_run
            .as_ref()
            .expect("first-time setup is open")
    }

    /// Answers first-time setup's questions, as its controls do.
    pub(in super::super) fn edit_first_run(
        &mut self,
        edit: impl FnOnce(&mut crate::app::first_run::Setup),
    ) {
        edit(
            self.app
                .first_run
                .as_mut()
                .expect("first-time setup is open"),
        );
    }

    /// First-time setup's Start using PowerWhittle, without its window's frame.
    pub(in super::super) fn finish_first_run(&mut self) {
        let setup = self.app.first_run.take().expect("first-time setup is open");
        self.app.finish_first_run(&setup);
    }
}
