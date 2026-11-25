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

    // ---- views, layouts and floating lists ----

    /// One frame with the screen reader's tree turned on: the tree it was handed.
    pub(super) fn accessibility_tree(&mut self) -> egui::accesskit::TreeUpdate {
        self.ctx.enable_accesskit();
        // the tree is built from the frame after it is turned on
        for _ in 0..2 {
            self.frame(vec![]);
        }
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.window)),
            time: Some(self.time),
            ..Default::default()
        };
        let out = headless(self.ctx.run_ui(input, |ui| self.app.pass(ui)));
        out.platform_output.accesskit_update.expect("a tree")
    }

    /// The view that has the window (or the big part of a split).
    pub(super) fn view(&self) -> &View {
        &self.app.view
    }

    /// How the views are split, if they are.
    pub(super) fn split(&self) -> Option<Split> {
        self.app.split
    }

    /// The views split as `split`, as an earlier session left them.
    pub(super) fn given_split(&mut self, split: Option<Split>) {
        self.app.split = split;
    }

    /// Side by side, with the 3D view on the left.
    pub(super) fn given_3d_view_first(&mut self) {
        self.app.split_layout.three_d_first = true;
    }

    /// The corner the small view sits in.
    pub(super) fn small_view_corner(&self) -> egui::Align2 {
        self.app.split_layout.corner
    }

    /// The left view's share of the width, side by side.
    pub(super) fn split_share(&self) -> f32 {
        self.app.split_layout.share
    }

    /// Whether the 3D view is on the left, side by side.
    pub(super) fn three_d_view_first(&self) -> bool {
        self.app.split_layout.three_d_first
    }

    /// Zooms the 2D view by `factor` about the screen point `at`, as the wheel does.
    pub(super) fn zoom_2d_about(&mut self, at: Pos2, factor: f32) {
        self.app.camera = self.app.view_transform.map(|t| t.zoomed_about(at, factor));
    }

    /// Where the 2D view's area starts on screen.
    pub(super) fn view_2d_corner(&self) -> Pos2 {
        self.app.view_2d_at.expect("the 2D view is drawn")
    }

    /// Shows the ribbon (or Canvas palette) tab named `name`.
    pub(super) fn show_ribbon_tab(&mut self, name: &str) {
        self.app.ribbon_tab = RIBBON_TABS
            .iter()
            .position(|t| *t == name)
            .unwrap_or_else(|| panic!("no tab {name}"));
    }

    /// The Canvas palette's groups, in order.
    pub(super) fn canvas_groups(&self) -> Vec<gui::ToolGroup> {
        self.app.canvas_groups()
    }

    /// The Canvas palette group whose list is open, if one is.
    pub(super) fn open_canvas_group(&self) -> Option<&'static str> {
        self.app.canvas.group
    }

    /// Opens the Canvas palette group `name`'s list, as pointing at it does.
    pub(super) fn open_canvas_group_list(&mut self, name: &'static str) {
        self.app.canvas.group = Some(name);
    }

    /// The Canvas chip (Layers, Library, ...) whose list is open, as its tab's place.
    pub(super) fn open_canvas_chip(&self) -> Option<usize> {
        self.app.canvas.chip
    }

    /// Whether Find a Tool's search box is open.
    pub(super) fn finding_a_tool(&self) -> bool {
        self.app.search.is_some()
    }

    // ---- updating, recovery and closing ----

    /// Whether the Updates window is open.
    pub(super) fn updates_window_open(&self) -> bool {
        self.app.updates.is_open()
    }

    /// Starts this run as run `run` with its settings in `settings`: where its recovery copy
    /// goes.
    pub(super) fn given_settings_folder(&mut self, settings: &std::path::Path, run: &str) {
        self.app.claim_recovery(settings, run);
    }

    /// No folder for recovery copies (the settings folder could not be made).
    pub(super) fn given_no_recovery_folder(&mut self) {
        self.app.recovery_path = None;
    }

    /// Opens the work an earlier run left, as starting does: whether there was any.
    pub(super) fn open_recovered_work(&mut self) -> bool {
        self.app.open_recovery()
    }

    /// Install Now, as far as handing the open work over to the new version.
    pub(super) fn hand_off_for_update(&mut self) -> crate::app::job_files::HandOff {
        self.app.hand_off_for_update()
    }

    /// The file the new version opens after an update, if it opens one.
    pub(super) fn job_to_reopen(&self) -> Option<std::path::PathBuf> {
        self.app.job_to_reopen()
    }

    /// The window's close button: whether it closes.
    pub(super) fn close_window(&mut self) -> bool {
        self.app.close_requested()
    }

    /// The job's file, if it has one (None: Untitled).
    pub(super) fn job_path(&self) -> Option<&std::path::Path> {
        self.app.session.path.as_deref()
    }

    /// Whether the job has changes not yet saved.
    pub(super) fn has_unsaved_changes(&self) -> bool {
        self.app.session.is_dirty(&self.app.history)
    }

    /// The status bar's message.
    pub(super) fn status_message(&self) -> &str {
        &self.app.status.message
    }

    /// Whether a recovery copy is being written now.
    pub(super) fn writing_recovery_copy(&self) -> bool {
        self.app.autosave.is_some()
    }

    /// Waits for the recovery copy being written to be done.
    pub(super) fn finish_recovery_copy(&mut self) {
        let (_, writing) = self.app.autosave.take().expect("a copy is being written");
        writing.join().unwrap().unwrap();
    }
}

/// The test files moved onto the verbs, each with its source: they never reach into the app.
const THROUGH_THE_HARNESS: [(&str, &str); 5] = [
    ("layers_sheets.rs", include_str!("layers_sheets.rs")),
    (
        "accessibility_tests.rs",
        include_str!("../accessibility_tests.rs"),
    ),
    ("canvas.rs", include_str!("canvas.rs")),
    ("split_views.rs", include_str!("split_views.rs")),
    ("updating.rs", include_str!("updating.rs")),
];

/// The lines of `source` that reach into the app past the harness.
fn reaching_in(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .map(|(n, line)| (n + 1, line.trim()))
        .filter(|(_, line)| reaches_app(line) || line.contains("CutCraftApp"))
        .collect()
}

/// Whether `line` names a harness's `.app` (`h.app.doc`, `next.app`, a wrapped `.app.form`),
/// not a path such as `crate::app::` or a name such as `h.app_state`.
fn reaches_app(line: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(".app").any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + ".app".len()..].chars().next();
        after.is_none_or(|c| !word(c)) && before.is_none_or(|c| word(c) || c == ')')
    })
}

#[test]
fn reaching_in_finds_the_app_however_the_line_is_wrapped() {
    let source = "h.app.doc.notes = x;\nlet m = h\n    .app\n    .form;\nh\n    .app.apply(a);\n|app: &CutCraftApp| 1\nh.doc().notes\nnext.app.session\nuse crate::app::job_files;\nh.app_state()\n";
    let lines: Vec<usize> = reaching_in(source).iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, [1, 3, 6, 7, 9]);
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
