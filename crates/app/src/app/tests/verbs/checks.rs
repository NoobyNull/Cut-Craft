//! The checks on toolpaths: keep-out zones, the tool changer, collisions found by a preview,
//! and the checks before cutting that Analyse and Save Toolpaths run.

use super::super::*;

impl Harness {
    // ---- keep-out zones ----

    /// Brings the machine's keep-out zones into the job, as opening it or Apply in Machine
    /// Configuration does: whether they changed.
    pub(in super::super) fn sync_machine_keep_out(&mut self) -> bool {
        self.app.sync_machine_keep_out()
    }

    /// Remakes the toolpaths round keep-out zones that changed, as a change to them does.
    pub(in super::super) fn keep_out_changed(&mut self) {
        self.app.keep_out_changed();
    }

    /// The names of the keep-out zones `toolpath` cuts into.
    pub(in super::super) fn zones_entered(
        &self,
        toolpath: &cut_craft_core::toolpath::Toolpath,
    ) -> Vec<String> {
        self.app.zones_entered(toolpath)
    }

    /// The toolpaths in `plan` that cut into a keep-out zone, as saved.
    pub(in super::super) fn not_kept_out(&self, plan: &[Vec<EntityId>]) -> Vec<String> {
        self.app.not_kept_out(&self.app.doc, plan)
    }

    /// Why `change` would be refused for cutting into a keep-out zone, if it would be.
    pub(in super::super) fn keep_out_refusal(&self, change: &Change) -> Option<String> {
        self.app.keep_out_refusal(change)
    }

    // ---- the view's display choices ----

    /// What the View menu shows and hides.
    pub(in super::super) fn display(&self) -> &crate::display::Display {
        &self.app.display
    }

    // ---- machines and the tool changer ----

    /// A machine set up as `machine`, now the one in use.
    pub(in super::super) fn given_machine_like(&mut self, machine: cut_craft_machines::Machine) {
        self.app.machines.machines.push(machine);
        self.app.machines.active = self.app.machines.machines.len() - 1;
    }

    /// Takes in a change to the tool changer's holders, as its window's OK does.
    pub(in super::super) fn tool_changer_changed(&mut self) {
        self.app.tool_changer_changed();
    }

    /// Whether the Tool Changer window is open.
    pub(in super::super) fn tool_changer_open(&self) -> bool {
        self.app.tool_changer_window
    }

    /// What the job sheet shows of the job, machine and post (it writes a recovery copy of
    /// an unsaved job).
    pub(in super::super) fn job_sheet(&mut self) -> fmt_create_job_sheet::Setup {
        self.app.job_setup()
    }

    /// Where the recovery copy of the open work goes.
    pub(in super::super) fn given_recovery_path(&mut self, path: std::path::PathBuf) {
        self.app.recovery_path = Some(path);
    }

    /// The machine's rapid rate, as the job uses it.
    pub(in super::super) fn rapid_rate(&self) -> f64 {
        self.app.summary.rapid
    }

    /// The machine's limits the checks before cutting go by.
    pub(in super::super) fn machine_limits(&self) -> cut_craft_core::preflight::Limits {
        self.app.limits()
    }

    // ---- a cut deeper than the material ----

    /// Whether Warning: deeper than the material is asked.
    pub(in super::super) fn asking_deeper_cut(&self) -> bool {
        self.app.deep_cut.is_some()
    }

    // ---- the checks before cutting ----

    /// Whether the checks before cutting are running.
    pub(in super::super) fn checking(&self) -> bool {
        self.app.checking()
    }

    /// How many sets of checks are kept (one per plan, job and machine).
    pub(in super::super) fn checks_kept(&self) -> usize {
        self.app.save_checks.len()
    }

    /// Whether the first set of checks kept has finished.
    pub(in super::super) fn first_checks_done(&self) -> bool {
        self.app
            .save_checks
            .first()
            .is_some_and(|c| c.found.is_some())
    }

    /// The first set of checks as Cancel leaves it: the plan known, nothing found.
    pub(in super::super) fn given_first_checks_cancelled(&mut self) {
        self.app.save_checks[0].found = None;
    }

    /// The checks for `form` through `post` as Cancel leaves them.
    pub(in super::super) fn given_checks_cancelled_for(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
    ) {
        self.given_checks_found(form, post, vec![]);
        self.given_first_checks_cancelled();
    }

    /// No checks kept, as a change to the job leaves them.
    pub(in super::super) fn given_no_checks_kept(&mut self) {
        self.app.save_checks.clear();
    }

    /// Forgets the checks, as Machine Configuration's Apply does.
    pub(in super::super) fn forget_save_checks(&mut self) {
        self.app.forget_save_checks();
    }

    /// The progress window shows only after a long wait.
    pub(in super::super) fn given_progress_shown_late(&mut self) {
        self.app.wait_before_showing = std::time::Duration::from_mins(10);
    }

    /// Waits for the slow job running now and takes in what it made.
    pub(in super::super) fn finish_slow_job(&mut self) {
        if let Some(running) = self.app.busy.take() {
            running.finish(&mut self.app);
        }
    }

    // ---- the preview's collisions and colours ----

    /// What the preview found toolpath `id`'s holder or collet hitting, as its sentence.
    pub(in super::super) fn collision(&self, id: EntityId) -> Option<&str> {
        self.app
            .preview
            .collisions
            .get(&id)
            .map(|c| c.sentence.as_str())
    }

    /// How many of the collisions found still hold for the toolpaths as they are.
    pub(in super::super) fn current_collisions(&self) -> usize {
        self.app.current_collisions().count()
    }

    /// The mark toolpath `id`'s cut cells carry in the preview.
    pub(in super::super) fn preview_mark(&self, id: EntityId) -> u32 {
        self.app.preview_mark(id)
    }

    /// The colour each preview mark is shown in.
    pub(in super::super) fn preview_fills(
        &self,
    ) -> std::collections::HashMap<u32, Option<[u8; 3]>> {
        self.app.preview_fills()
    }

    // ---- the Analyse tab ----

    /// The tab shown in the browser beside the view (Checks, ...).
    pub(in super::super) fn inspector_tab(&self) -> &'static str {
        INSPECTOR_TABS[self.app.inspector_tab]
    }
}
