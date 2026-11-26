//! Machines and post processors.

use super::super::*;

impl Harness {
    // ---- machines and post processors ----

    /// Adds a machine (Machine ▸ Add Machine, its settings left as they come), now the one
    /// in use.
    pub(in super::super) fn given_machine(&mut self) {
        self.app.machines.add();
    }

    /// Changes the machine in use, as its Machine Configuration would.
    pub(in super::super) fn edit_machine(
        &mut self,
        edit: impl FnOnce(&mut cut_craft_machines::Machine),
    ) {
        edit(self.app.machines.active_mut().expect("a machine in use"));
    }

    /// The machine in use, if there is one.
    pub(in super::super) fn machine(&self) -> Option<&cut_craft_machines::Machine> {
        self.app.machines.active()
    }

    /// Every machine Cut Craft keeps.
    pub(in super::super) fn machines(&self) -> &[cut_craft_machines::Machine] {
        &self.app.machines.machines
    }

    /// The post processor File ▸ Save Toolpaths would save through now, or why there is none.
    pub(in super::super) fn output_post(&self) -> Result<cut_craft_post::Post, String> {
        self.app.output_post()
    }

    /// The job's toolpaths saved through `post` to `path`, as Save Toolpaths does.
    pub(in super::super) fn save_through(
        &self,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<(), String> {
        files::save_through(post, &self.app.slots(), &self.app.doc, path, None)
    }

    /// Save Toolpaths' window set as `form`, saving through `post` to `path`: what it said.
    pub(in super::super) fn save_toolpaths_to(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        self.app.save_toolpaths_to(form, post, path)
    }

    /// The post Save Toolpaths' window set as `form` saves through.
    pub(in super::super) fn form_post(
        &self,
        form: &SaveForm,
    ) -> Result<cut_craft_post::Post, String> {
        self.app.form_post(form)
    }

    /// Send to Machine with Save Toolpaths' window set as `form`, through `post`, to
    /// `store`; `earlier` answers whether to replace files already there. What it said.
    pub(in super::super) fn send_to(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        store: &dyn cut_craft_machines::send::Store,
        earlier: &dyn Fn(&[String]) -> Option<crate::options::EarlierGcode>,
    ) -> Result<String, String> {
        self.app.send_to(form, post, store, earlier)
    }

    /// The custom post processors kept in `folder`, as the settings folder holds them.
    pub(in super::super) fn given_posts_folder(&mut self, folder: std::path::PathBuf) {
        self.app.posts = cut_craft_machines::Library::new(Some(folder));
    }

    /// The post processors Cut Craft has: its own and the ones installed.
    pub(in super::super) fn posts(&self) -> &cut_craft_machines::Library {
        &self.app.posts
    }

    /// Installs the post processor file at `path`, as Post Processor Management's Install does.
    pub(in super::super) fn install_post(&mut self, path: &std::path::Path) {
        self.app.install_post(path);
    }

    /// Installs the offline machine package at `path` (Machine ▸ Install Offline Package).
    pub(in super::super) fn install_package(&mut self, path: &std::path::Path) {
        self.app.install_package(path);
    }

    /// Install Offline Package from inside Machine Configuration, with the package at `path`.
    pub(in super::super) fn install_package_in_machine_window(&mut self, path: &std::path::Path) {
        self.app.machine_outcome_install(path);
    }

    /// Picks a post in Post Processor Management (Select, ...).
    pub(in super::super) fn pick_post(&mut self, outcome: cut_craft_machines::PostOutcome) {
        self.app.post_outcome(outcome);
    }

    /// Reads the machines from the file at `path`, as starting does.
    pub(in super::super) fn load_machines_from(&mut self, path: &std::path::Path) {
        self.app.load_machines_at(path);
    }

    /// Keeps a change to the machines (saving them), the status bar saying `done`.
    pub(in super::super) fn keep_machines(&mut self, done: &str) {
        self.app.machines_changed(done);
    }

    /// Machine Configuration's Add, on the machines it is editing (kept once applied).
    pub(in super::super) fn add_machine_in_window(&mut self) {
        self.app
            .machine_window
            .as_mut()
            .expect("Machine Configuration is open")
            .draft
            .add();
    }

    /// The machines Machine Configuration is editing, if it is open.
    pub(in super::super) fn machines_being_edited(&self) -> Option<&cut_craft_machines::Machines> {
        self.app.machine_window.as_ref().map(|w| &w.draft)
    }

    /// Whether Post Processor Management is open.
    pub(in super::super) fn post_window_open(&self) -> bool {
        self.app.post_window.is_some()
    }

    /// Whether Machine Files is open.
    pub(in super::super) fn machine_files_open(&self) -> bool {
        self.app.machine_files.is_some()
    }

    /// The bit database the forms pick bits from.
    pub(in super::super) fn tool_database(&self) -> &ToolDatabase {
        &self.app.tools
    }

    // ---- bits and materials ----

    /// Puts `tool` in the Bit Library, as saving it there does.
    pub(in super::super) fn given_tool(&mut self, tool: Tool) {
        self.app.tools.put(tool);
    }

    /// Makes `material` the Bit Library's active material.
    pub(in super::super) fn given_active_material(&mut self, material: &str) {
        self.app.tools.material = material.into();
    }

    /// Opens the Materials Library.
    pub(in super::super) fn open_materials_library(&mut self) {
        let window = cut_craft_tool_database::materials::MaterialsWindow::new(
            &self.app.materials,
            &self.app.tools,
        );
        self.app.materials_window = Some(window);
    }

    /// The Materials Library's New Material.
    pub(in super::super) fn new_material_in_library(&mut self) {
        (self.app.materials_window.as_mut())
            .expect("the Materials Library is open")
            .new_material();
    }

    /// How many materials are kept.
    pub(in super::super) fn materials_kept(&self) -> usize {
        self.app.materials.materials.len()
    }

    /// The checks before cutting on the toolpaths in `plan`, in that order.
    pub(in super::super) fn preflight(
        &self,
        plan: &[Vec<EntityId>],
        changes: crate::files::ToolChanges,
    ) -> Vec<cut_craft_core::preflight::Finding> {
        self.app.preflight(plan, changes)
    }

    // ---- saving toolpaths: earlier files and the checks before cutting ----

    /// Whether the machine in use has a tool changer to offer.
    pub(in super::super) fn has_tool_changer(&self) -> bool {
        self.app.has_tool_changer()
    }

    /// The checks Save Toolpaths runs on `plan`, once they have run.
    pub(in super::super) fn save_checks(
        &mut self,
        plan: &[Vec<EntityId>],
        changes: crate::files::ToolChanges,
    ) -> Option<Vec<cut_craft_core::preflight::Finding>> {
        self.app.save_checks(plan, changes).map(<[_]>::to_vec)
    }

    /// Save Toolpaths' window's choices, while it is open.
    pub(in super::super) fn save_form(&self) -> SaveForm {
        self.app.save_form.clone().expect("Save Toolpaths is open")
    }

    /// The checks before cutting for `form` through `post` as having found `found`.
    pub(in super::super) fn given_checks_found(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        found: Vec<cut_craft_core::preflight::Finding>,
    ) {
        self.app.save_checks = vec![SaveChecks {
            plan: self.app.plan(form, post).expect("a plan"),
            changes: form.changes,
            revision: self.app.history.revision(),
            machine: self.app.machines.active,
            found: Some(found),
        }];
    }

    /// Ticks the finding `message` as read, as its box on Save Toolpaths does.
    pub(in super::super) fn tick_as_read(&mut self, message: &str) {
        self.app.acknowledged.insert(message.to_owned());
    }

    /// Save or Send pressed with `form`: the post it saves through, or `None` when it
    /// stops to ask (or cannot save).
    pub(in super::super) fn ready_to_save(
        &mut self,
        form: &SaveForm,
        sending: bool,
    ) -> Option<cut_craft_post::Post> {
        self.app.ready_to_save(form, sending)
    }

    /// Whether Save Anyway? is asked.
    pub(in super::super) fn asking_save_anyway(&self) -> bool {
        self.app.save_anyway.is_some()
    }

    /// Saves with `form` through `post` to `path`, asking first where an earlier save left
    /// files of its own there.
    pub(in super::super) fn save_over_earlier(
        &mut self,
        form: &SaveForm,
        post: cut_craft_post::Post,
        path: std::path::PathBuf,
    ) {
        self.app.save_over_earlier(form, post, path);
    }

    /// Send to Machine with `form` through `post`, doing `choice` with earlier files.
    pub(in super::super) fn send_choosing(
        &mut self,
        form: &SaveForm,
        post: cut_craft_post::Post,
        choice: crate::options::EarlierGcode,
    ) {
        self.app.send_choosing(form, post, choice);
    }

    /// The question about an earlier save's files, while it is asked.
    pub(in super::super) fn earlier_asked(&self) -> Option<&EarlierAsk> {
        self.app.earlier_ask.as_ref()
    }

    /// Answers the earlier files question: `None` cancels; `remember` keeps the answer.
    pub(in super::super) fn answer_earlier(
        &mut self,
        choice: Option<crate::options::EarlierGcode>,
        remember: bool,
    ) {
        self.app.answer_earlier(choice, remember);
    }
}
