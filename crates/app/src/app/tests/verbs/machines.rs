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
}
