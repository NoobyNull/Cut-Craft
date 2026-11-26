//! Jobs and files: the units, starting and closing jobs, recovery copies, save questions,
//! crash reports, adding from the library, Job Defaults, and what is installed.

use super::super::*;

impl Harness {
    // ---- the job's units and the pointer ----

    /// Whether Edit ▸ Units' item `id` is ticked, or `None` when it is not one of them.
    pub(in super::super) fn units_checked(&self, id: &str) -> Option<bool> {
        self.app.units_checked(id)
    }

    /// The pointer is over the job at `at`.
    pub(in super::super) fn given_pointer_at(&mut self, at: Point) {
        self.app.cursor = Some(at);
    }

    /// The status bar's coordinates of the pointer, and their unit.
    pub(in super::super) fn coordinates(&self) -> Option<(String, &'static str)> {
        self.app.coordinates()
    }

    // ---- starting and closing jobs ----

    /// The menu bar's menus, by name.
    pub(in super::super) fn menu_names(&self) -> Vec<&str> {
        self.app.menu_names()
    }

    /// The menus with a job open.
    pub(in super::super) fn job_menus(&self) -> &gui::MenuSystem {
        &self.app.menus
    }

    /// The menus on the Start page.
    pub(in super::super) fn start_menus(&self) -> &gui::MenuSystem {
        &self.app.start_menus
    }

    /// Whether there is a 2D view laid out (none before there is a job).
    pub(in super::super) fn has_2d_view(&self) -> bool {
        self.app.view_transform.is_some()
    }

    /// The job the 2D view shows: the one being set up while Job Setup is open.
    pub(in super::super) fn shown_job(&self) -> cut_craft_core::document::Job {
        self.app.shown_job()
    }

    /// Whether the open form sets up a new job.
    pub(in super::super) fn form_for_new_job(&self) -> bool {
        self.app.form.as_ref().is_some_and(|f| f.new_job)
    }

    /// Starts tool `id` as its button does.
    pub(in super::super) fn start_command(&mut self, id: &str) {
        self.app.start_command(id);
    }

    /// Whether Redo has something to redo.
    pub(in super::super) fn can_redo(&self) -> bool {
        self.app.history.can_redo()
    }

    /// Starts with nothing named on the command line.
    pub(in super::super) fn open_given_nothing(&mut self) {
        self.app.open_given(None);
    }

    /// The recent files are kept in `path`.
    pub(in super::super) fn given_recent_list_at(&mut self, path: std::path::PathBuf) {
        self.app.recent_path = Some(path);
    }

    // ---- recovery copies ----

    /// Takes this run's recovery copy in `dir`, numbered `run`, as a window does as it starts.
    pub(in super::super) fn claim_recovery(&mut self, dir: &std::path::Path, run: &str) {
        self.app.claim_recovery(dir, run);
    }

    /// Where this run's recovery copy goes, if anywhere.
    pub(in super::super) fn recovery_path(&self) -> Option<&std::path::Path> {
        self.app.recovery_path.as_deref()
    }

    /// The work is let go of, as Don't Save does.
    pub(in super::super) fn discard_changes(&mut self) {
        self.app.session.discard_changes(&self.app.history);
    }

    // ---- the save questions ----

    /// Answers Save changes? with `answer`: whether the work may go.
    pub(in super::super) fn discard_answered(&mut self, answer: &rfd::MessageDialogResult) -> bool {
        self.app.discard_answered(answer)
    }

    /// What Save changes? asks.
    pub(in super::super) fn save_changes_question(&self) -> String {
        self.app.save_changes_question()
    }

    /// The folder and name Save As suggests.
    pub(in super::super) fn save_as_suggestion(&self) -> (Option<std::path::PathBuf>, String) {
        self.app.save_as_suggestion()
    }

    /// The window's title for the job as it stands, under the program name `app`.
    pub(in super::super) fn job_title(&self, app: &str) -> String {
        self.app.session.title(app, &self.app.history)
    }

    /// Whether Replace the earlier G-code? is asked.
    pub(in super::super) fn asking_earlier(&self) -> bool {
        self.app.earlier_ask.is_some()
    }

    /// Save Anyway? as asked, if it is.
    pub(in super::super) fn save_anyway(&self) -> Option<&crate::app::gcode_view::SaveAnyway> {
        self.app.save_anyway.as_ref()
    }

    /// Puts Save Anyway? away unanswered: whether it was asked.
    pub(in super::super) fn dismiss_save_anyway(&mut self) -> bool {
        self.app.save_anyway.take().is_some()
    }

    /// Save Anyway answered for `problems`, as its button does.
    pub(in super::super) fn given_save_anyway_agreed(&mut self, problems: &[&str]) {
        (self.app.saving).accept(problems.iter().map(|&p| p.to_owned()).collect());
    }

    /// The problems Save Anyway was answered for, not yet used by a save.
    pub(in super::super) fn save_anyway_agreed(&self) -> &[String] {
        self.app.saving.accepted()
    }

    // ---- crash reports ----

    /// Crash logs go to `url` through `post`, those that may go later kept in `retry_dir`.
    pub(in super::super) fn given_crash_service(
        &mut self,
        url: &str,
        post: fn(&str, &str) -> Result<(), cut_craft_bug_report::relay::Failure>,
        retry_dir: std::path::PathBuf,
    ) {
        let reporter = &mut self.app.reporter;
        reporter.crash_service = Some(url.into());
        reporter.post = post;
        // a light proof of work, quick in a test
        reporter.work_bits = 4;
        reporter.retry_dir = Some(retry_dir);
    }

    // ---- adding from the library ----

    /// Add to Project with `files` picked.
    pub(in super::super) fn add_from_library(&mut self, files: Vec<std::path::PathBuf>) {
        self.app.add_from_library(files, None);
    }

    /// How many picked files wait their turn to come in.
    pub(in super::super) fn imports_waiting(&self) -> usize {
        self.app.import_queue.len()
    }

    /// The 2D view's zoom, pixels a millimetre, once it is laid out.
    pub(in super::super) fn zoom_2d_scale(&self) -> Option<f32> {
        self.app.camera.map(|c| c.scale)
    }

    // ---- Job Defaults ----

    /// Opens Options at Job Defaults.
    pub(in super::super) fn open_job_defaults(&mut self) {
        self.app.open_job_defaults();
    }

    /// Job Defaults' settings as shown, and why the last change was refused, if it was.
    pub(in super::super) fn job_defaults_shown(&self) -> (Value, Option<String>) {
        let (_, params, why) = self.app.job_defaults_state();
        (params, why)
    }

    /// Changes Job Defaults' settings to `params`, as its fields do.
    pub(in super::super) fn edit_job_defaults(&mut self, params: Value) {
        self.app.job_defaults_edited(params);
    }

    /// The Options window's Apply.
    pub(in super::super) fn apply_options_window(&mut self) {
        let ctx = self.ctx.clone();
        self.app
            .options_button(&ctx, crate::app::options_apply::Button::Apply);
    }

    // ---- what is installed ----

    /// Reads the add-ons folder again, as installing or removing one does.
    pub(in super::super) fn refresh_installed(&mut self) {
        self.app.refresh_installed();
    }

    /// The tools the installed gadgets and add-ons bring.
    pub(in super::super) fn gadget_tools(&self) -> Vec<&'static str> {
        self.app.gadget_tools()
    }

    /// The gadgets in the My Gadgets folder are `gadgets` (name and tool).
    pub(in super::super) fn given_my_gadgets(&mut self, gadgets: Vec<(String, &'static str)>) {
        self.app.my_gadgets = gadgets;
    }

    /// The gadget `id` is not signed.
    pub(in super::super) fn given_gadget_not_signed(&mut self, id: &str, name: &str) {
        let code = crate::trust::Code::gadget(id, name, b"");
        let unsigned = crate::gadgets::Unsigned {
            code,
            waiting: None,
        };
        self.app.unsigned_gadgets.insert(id.to_owned(), unsigned);
    }

    /// The add-ons installed are `addons` (name, tool, signed).
    pub(in super::super) fn given_installed_addons(
        &mut self,
        addons: Vec<(String, Option<&'static str>, bool)>,
    ) {
        self.app.installed_addons = addons;
    }

    /// Opens the Rail's Gadgets list, as its button does.
    pub(in super::super) fn open_rail_gadgets(&mut self) {
        self.app.rail.open = crate::app::rail_list::List::Gadgets;
    }

    /// Whether a Rail list is open.
    pub(in super::super) fn rail_list_open(&self) -> bool {
        self.app.rail.is_open()
    }
}
