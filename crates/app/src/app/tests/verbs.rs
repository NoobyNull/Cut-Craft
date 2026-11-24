//! What a test does to the app and reads back from it, in a user's terms.
//!
//! Tests reach the app only through these verbs and the harness's input ones (`click`,
//! `drag`, `text_at`, ...), never its fields: so the app's insides can be rearranged without
//! touching the tests, which then fail only when what the user sees or makes changes.

use super::*;
use crate::clipart::ToLevel;
use cut_craft_core::document::Layer;
use cut_craft_core::geometry::Point;

impl Harness {
    // ---- the job as it stands before the test's steps ----

    /// The job's drawing: what the user has made so far.
    pub(super) fn doc(&self) -> &Document {
        &self.app.doc
    }

    /// Opens `doc` as the job, as if loaded from a file made earlier.
    pub(super) fn open_document(&mut self, doc: Document) {
        self.app.doc = doc;
    }

    /// Makes `change` to the drawing before the test's own steps (no undo step).
    pub(super) fn given(&mut self, change: &Change) {
        self.app
            .doc
            .apply(change)
            .expect("the given change applies");
    }

    /// Edits the drawing before the test's own steps, as a file made earlier would hold it.
    pub(super) fn given_document(&mut self, edit: impl FnOnce(&mut Document)) {
        edit(&mut self.app.doc);
    }

    /// Changes the saved options, as an earlier session would have left them.
    pub(super) fn given_options(&mut self, edit: impl FnOnce(&mut crate::options::Options)) {
        edit(&mut self.app.options);
    }

    /// The options in force.
    pub(super) fn options(&self) -> &crate::options::Options {
        &self.app.options
    }

    /// Closes the job unsaved and starts another, as New does once the question is answered.
    pub(super) fn start_another_job(&mut self) {
        self.app.reset_job();
    }

    // ---- tools, as their buttons, menus and forms run them ----

    /// Picks `id` as its button or menu item does.
    pub(super) fn invoke(&mut self, id: &str) {
        self.app.invoke(&self.ctx, id);
    }

    /// Runs `id` with `settings`, as OK on its form does.
    pub(super) fn run(&mut self, id: &str, settings: &Value) -> Result<(), String> {
        self.app.apply(id, settings)
    }

    /// Edit ▸ Undo.
    pub(super) fn undo(&mut self) {
        self.invoke("undo");
    }

    /// The open form's tool, if a form is open.
    pub(super) fn form_id(&self) -> Option<&'static str> {
        self.app.form.as_ref().map(|f| f.id)
    }

    /// The open form's setting `key` as it now stands.
    pub(super) fn form_value(&self, key: &str) -> &Value {
        &self.app.form.as_ref().expect("a form is open").params[key]
    }

    /// Types `value` into the open form's setting at `pointer` (`/placing/position`), which
    /// the form then acts on as it does on any edit.
    pub(super) fn set_form_value(&mut self, pointer: &str, value: Value) {
        let form = self.app.form.as_mut().expect("a form is open");
        *form
            .params
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("the form has no {pointer}")) = value;
        form.dirty = true;
    }

    /// Closes the open form, as its Close does.
    pub(super) fn close_form(&mut self) {
        self.app.form = None;
    }

    // ---- files ----

    /// File ▸ Save As to `path`.
    pub(super) fn save_to(&mut self, path: &std::path::Path) {
        self.app.save_to(path);
    }

    /// File ▸ Open of `path`: whether it opened.
    pub(super) fn open(&mut self, path: &std::path::Path) -> bool {
        self.app.open_path(path)
    }

    /// Clipart dropped at `at` on the view (or imported), onto `level` if given.
    pub(super) fn drop_clipart(
        &mut self,
        path: &std::path::Path,
        at: Option<Point>,
        level: Option<ToLevel>,
    ) {
        self.app.import_clipart(path, at, level);
    }

    // ---- pages and panels ----

    /// Shows the Design page's tab named `name` (Drawing, Layers, ...).
    pub(super) fn show_design_tab(&mut self, name: &str) {
        let tab = DESIGN_TABS
            .iter()
            .position(|&t| t == name)
            .unwrap_or_else(|| panic!("no Design tab {name}"));
        self.app.set_design_tab(tab);
    }

    /// Pins `page` open, as its pin button does.
    pub(super) fn pin(&mut self, page: Page) {
        self.app.pages.set_pinned(page, true);
    }

    /// Clicks `action`'s control (its eye, lock, ...) on `layer`'s row of the Layers panel.
    pub(super) fn click_layer(&mut self, layer: LayerId, action: &str) {
        self.app.layer_action(&self.ctx, layer, action);
    }

    /// The right-click menu of the job's `i`th layer.
    pub(super) fn layer_menu(&self, i: usize) -> Vec<gui::MenuItem> {
        let layer: &Layer = &self.doc().layers[i];
        gui::layer_menu(self.doc(), layer)
    }

    /// Right-clicks the job's `i`th layer and picks the item at `path`.
    pub(super) fn choose_on_layer(&mut self, i: usize, path: &[&str]) {
        self.choose(|app| gui::layer_menu(&app.doc, &app.doc.layers[i]), path);
    }

    /// Types `text` in the Notes panel's Variable Explorer search box.
    pub(super) fn search_variables(&mut self, text: &str) {
        self.app.notes_ui.search = text.into();
    }

    /// What the notes variable `key` stands for now.
    pub(super) fn variable_value(&self, key: &str) -> String {
        super::notes_panel::value_now(&self.app, key)
    }
}

/// The test files moved onto the verbs, each with its source: they never reach into the app.
const THROUGH_THE_HARNESS: [(&str, &str); 1] =
    [("layers_sheets.rs", include_str!("layers_sheets.rs"))];

/// The lines of `source` that reach into the app past the harness.
fn reaching_in(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(n, line)| (n + 1, line.trim()))
        .filter(|(_, line)| {
            line.contains("h.app")
                || *line == ".app"
                || line.starts_with(".app.")
                || line.contains("CutCraftApp")
        })
        .collect()
}

#[test]
fn reaching_in_finds_the_app_however_the_line_is_wrapped() {
    let source = "h.app.doc.notes = x;\nlet m = h\n    .app\n    .form;\nh\n    .app.apply(a);\n|app: &CutCraftApp| 1\nh.doc().notes\n";
    let lines: Vec<usize> = reaching_in(source).iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, [1, 3, 6, 7]);
}

#[test]
fn moved_test_files_reach_the_app_only_through_the_harness() {
    for (name, source) in THROUGH_THE_HARNESS {
        let lines = reaching_in(source);
        assert!(
            lines.is_empty(),
            "{name} reaches into the app; use a verb in tests/verbs.rs (add one if none \
             fits):\n{}",
            lines
                .iter()
                .map(|(n, line)| format!("  {name}:{n}: {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
