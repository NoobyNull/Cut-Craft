//! Tools, menus, forms, the clipboard and undo, as their buttons run them.

use super::super::*;

impl Harness {
    // ---- tools, as their buttons, menus and forms run them ----

    /// Picks `id` as its button or menu item does.
    pub(in super::super) fn invoke(&mut self, id: &str) {
        self.app.invoke(&self.ctx, id);
    }

    /// Runs `id` with `settings`, as OK on its form does.
    pub(in super::super) fn run(&mut self, id: &str, settings: &Value) -> Result<(), String> {
        self.app.apply(id, settings)
    }

    /// Edit ▸ Undo.
    pub(in super::super) fn undo(&mut self) {
        self.invoke("undo");
    }

    /// The open form's tool, if a form is open.
    pub(in super::super) fn form_id(&self) -> Option<&'static str> {
        self.app.form.as_ref().map(|f| f.id)
    }

    /// The open form's setting `key` as it now stands.
    pub(in super::super) fn form_value(&self, key: &str) -> &Value {
        &self.app.form.as_ref().expect("a form is open").params[key]
    }

    /// Types `value` into the open form's setting at `pointer` (`/placing/position`), which
    /// the form then acts on as it does on any edit.
    pub(in super::super) fn set_form_value(&mut self, pointer: &str, value: Value) {
        let form = self.app.form.as_mut().expect("a form is open");
        *form
            .params
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("the form has no {pointer}")) = value;
        form.dirty = true;
    }

    /// Closes the open form, as its Close does.
    pub(in super::super) fn close_form(&mut self) {
        self.app.form = None;
    }

    // ---- menus, the clipboard and undo ----

    /// The items of the menu bar's menu `name` (File, Edit, Gadgets, ...).
    pub(in super::super) fn menu_items(&self, name: &str) -> &[gui::MenuItem] {
        &self
            .app
            .menus
            .menus
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("no {name} menu"))
            .items
    }

    /// Whether `id`'s button or menu item can be picked now (not greyed out).
    pub(in super::super) fn available(&self, id: &str) -> bool {
        self.app.available(id)
    }

    /// How many pieces Copy or Cut put on the clipboard.
    pub(in super::super) fn on_clipboard(&self) -> usize {
        self.app.clipboard.len()
    }

    /// Whether Edit ▸ Undo has anything to take back.
    pub(in super::super) fn can_undo(&self) -> bool {
        self.app.history.can_undo()
    }

    /// How many steps Edit ▸ Undo could take back.
    pub(in super::super) fn undo_steps(&self) -> usize {
        self.app.history.undo_depth()
    }

    /// A count that moves on with every change to the job, undo and redo too.
    pub(in super::super) fn job_revision(&self) -> u64 {
        self.app.history.revision()
    }

    // ---- forms, as their buttons run them ----

    /// The form's Apply, Calculate or Create with `settings`.
    pub(in super::super) fn apply_form(
        &mut self,
        id: &str,
        settings: &Value,
    ) -> Result<(), String> {
        self.app.apply_form(id, settings)
    }

    /// Replaces all the open form's settings with `settings`, as typing each would.
    pub(in super::super) fn set_form_settings(&mut self, settings: Value) {
        self.set_form_value("", settings);
    }

    /// The settings `id`'s form opens with on this job.
    pub(in super::super) fn opening_settings(&self, id: &str) -> Value {
        self.app
            .registry
            .get(id)
            .unwrap_or_else(|| panic!("no tool {id}"))
            .initial_params(&self.app.doc)
    }

    /// `id`'s own default settings.
    pub(in super::super) fn default_settings(&self, id: &str) -> Value {
        self.app
            .registry
            .get(id)
            .unwrap_or_else(|| panic!("no tool {id}"))
            .default_params()
    }

    /// The note under the open form's buttons ("Already up to date", ...), if it shows one.
    pub(in super::super) fn form_note(&self) -> Option<&str> {
        self.app.form_note()
    }

    /// Calculate as New on a toolpath form.
    pub(in super::super) fn calculate_as_new(&mut self) {
        self.app.calculate_as_new();
    }

    /// Picks toolpath `id` in the Toolpaths list.
    pub(in super::super) fn select_toolpath(&mut self, id: EntityId) {
        self.app.selected_toolpath = Some(id);
    }

    /// The toolpath picked in the Toolpaths list, if one is.
    pub(in super::super) fn selected_toolpath(&self) -> Option<EntityId> {
        self.app.selected_toolpath
    }

    /// The toolpath the open form edits, if it edits one rather than making a new one.
    pub(in super::super) fn form_editing(&self) -> Option<EntityId> {
        self.app.form.as_ref().expect("a form is open").editing
    }

    /// Edit on the picked toolpath, as a double-click on its row does.
    pub(in super::super) fn edit_toolpath(&mut self) {
        self.app.edit_toolpath();
    }

    /// The open form's Cancel.
    pub(in super::super) fn cancel_form(&mut self) {
        self.app.cancel_form();
    }

    /// How many changes made in the toolpath list the open form noted, to keep on Cancel.
    pub(in super::super) fn form_list_steps(&self) -> usize {
        self.app
            .form
            .as_ref()
            .expect("a form is open")
            .list_steps
            .len()
    }

    /// How many undo steps there were when the open form opened.
    pub(in super::super) fn form_opened_at_step(&self) -> usize {
        self.app.form.as_ref().expect("a form is open").undo_from
    }
}
