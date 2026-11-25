//! The Bit Library and the Materials Library: their windows, saving them, and the bits forms
//! pick from them.

use super::super::*;
use cut_craft_tool_database::Manager;

impl Harness {
    // ---- the bits kept ----

    /// Edits the kept bits before the test's own steps, as an earlier session left them.
    pub(in super::super) fn given_bits(&mut self, edit: impl FnOnce(&mut ToolDatabase)) {
        edit(&mut self.app.tools);
    }

    /// Imports the bit library file at `path`, as File ▸ Import Bit Library does.
    pub(in super::super) fn import_tools(&mut self, path: &std::path::Path) {
        self.app.import_tools(path);
    }

    /// Reads the bits from `path` (or the usual place for `None`), as starting up does.
    pub(in super::super) fn load_tools_at(&mut self, path: Option<std::path::PathBuf>) {
        self.app.load_tools_at(path);
    }

    /// Keeps the bits in `path` from now on, without reading it first.
    pub(in super::super) fn given_tools_path(&mut self, path: std::path::PathBuf) {
        self.app.tools_file.at(Some(path));
    }

    /// Reads the materials from `path`, as starting up does.
    pub(in super::super) fn load_materials_at(&mut self, path: Option<std::path::PathBuf>) {
        self.app.load_materials_at(path);
    }

    /// Saves the kept bits, saying `done` in the status bar when it works.
    pub(in super::super) fn tools_changed(&mut self, done: &str) {
        self.app.tools_changed(done);
    }

    /// Saves the kept bits after the window's OK, renaming the bits in `renamed` in the job.
    pub(in super::super) fn tools_kept(&mut self, done: &str, renamed: &[(String, String)]) {
        self.app.tools_kept(done, renamed);
    }

    /// Saves the Materials Library.
    pub(in super::super) fn save_materials(&self) -> Result<(), String> {
        self.app.save_materials()
    }

    /// Renames the bits in `renamed` in the job's toolpaths, as saving the Bit Library does.
    pub(in super::super) fn rename_toolpath_tools(&mut self, renamed: &[(String, String)]) {
        self.app.rename_toolpath_tools(renamed, "Bit library saved");
    }

    // ---- the Bit Library window ----

    /// The open form's Edit… on its bit `name`.
    pub(in super::super) fn edit_bit(&mut self, name: &str) {
        self.app.edit_tool(&self.ctx, Some(name.into()));
    }

    /// The Bit Library window, if it is open.
    pub(in super::super) fn bit_library(&self) -> Option<&Manager> {
        self.app.tool_manager.as_ref()
    }

    /// Works in the open Bit Library window: picks, edits and its buttons there.
    pub(in super::super) fn in_bit_library(&mut self, work: impl FnOnce(&mut Manager)) {
        work(
            self.app
                .tool_manager
                .as_mut()
                .expect("the Bit Library is open"),
        );
    }

    /// Keeps what the open Bit Library window holds, as its Apply does, and the open form
    /// takes its bit's new settings.
    pub(in super::super) fn keep_bit_library(&mut self) {
        let kept = self
            .bit_library()
            .expect("the Bit Library is open")
            .draft
            .clone();
        self.app.tools = kept;
        self.app.refresh_form_tool();
    }

    /// A form's list (Rest Machining, Bits that finished before, ...) asks the Bit Library
    /// for a bit, as its Add a bit… or Select… does.
    pub(in super::super) fn ask_listed(&mut self, list: ToolList, ask: ListAsk) {
        self.app.ask_listed(&self.ctx, list, ask);
    }

    /// The Bit Library opened to pick a bit for the Materials Library's Add Tool.
    pub(in super::super) fn given_bit_picker_for_material(&mut self, picker: Manager) {
        self.app.tool_manager = Some(picker);
        self.app.picking = Some(super::super::PickFor::Material);
    }

    /// The Bit Library window's `outcome` (Select, OK, ...); whether it closed.
    pub(in super::super) fn bit_library_outcome(
        &mut self,
        outcome: &cut_craft_tool_database::Outcome,
    ) -> bool {
        self.app.library_outcome(outcome)
    }

    // ---- the Materials Library window ----

    /// The Materials Library opened as `window`.
    pub(in super::super) fn given_materials_window(
        &mut self,
        window: cut_craft_tool_database::materials::MaterialsWindow,
    ) {
        self.app.materials_window = Some(window);
    }

    /// A Materials Library window on the kept materials and bits.
    pub(in super::super) fn materials_window(
        &self,
    ) -> cut_craft_tool_database::materials::MaterialsWindow {
        cut_craft_tool_database::materials::MaterialsWindow::new(
            &self.app.materials,
            &self.app.tools,
        )
    }

    /// The Materials Library's OK (`close`) or Apply.
    pub(in super::super) fn keep_materials(&mut self, close: bool) {
        self.app.materials_kept(close);
    }

    // ---- bits on forms ----

    /// The open form's line on how deep its bit reaches, if it shows one.
    pub(in super::super) fn form_reach(&mut self) -> Option<cut_craft_core::reach::Line> {
        self.app.form_reach()
    }

    /// The names of the bits Suggest clearing bits would pick from.
    pub(in super::super) fn suggested_bits(&self) -> Vec<String> {
        (self.app.suggest_candidates().into_iter())
            .map(|c| c.tool.name)
            .collect()
    }
}
