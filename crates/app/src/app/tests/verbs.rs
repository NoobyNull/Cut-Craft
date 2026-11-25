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

    // ---- making things, as the tools do ----

    /// Makes `change` as a tool does: one undo step named `name`, reported in the status bar.
    pub(super) fn make(&mut self, change: Change, name: &str) {
        self.app.record(Ok(change), name);
    }

    /// Selects `ids`, as clicking them does.
    pub(super) fn select(&mut self, ids: impl IntoIterator<Item = EntityId>) {
        self.app.doc.selection = ids.into_iter().collect();
    }

    /// The open form's settings, all of them, as they now stand.
    pub(super) fn form_settings(&self) -> &Value {
        &self.app.form.as_ref().expect("a form is open").params
    }

    /// Recalculates toolpath `id`, as its Recalculate does.
    pub(super) fn recalculate(&mut self, id: EntityId) {
        let again = self
            .app
            .registry
            .recalculate(&self.app.doc, id)
            .expect("it recalculates");
        self.app.record(Ok(again), "recalculate");
    }

    // ---- the 2D view ----

    /// Zoom to Job: the next frame fits the job in the view.
    pub(super) fn zoom_to_job(&mut self) {
        self.invoke("zoom_to_job");
    }

    /// The 2D view's area on screen.
    pub(super) fn view_2d_area(&self) -> Rect {
        self.app.view_area.expect("the 2D view was drawn")
    }

    /// How many screen points a millimetre is in the 2D view.
    pub(super) fn zoom_2d(&self) -> f32 {
        self.app
            .view_transform
            .expect("the 2D view was drawn")
            .scale
    }

    /// Where a drawing tool's drag lands once snapped (to ends, ghosts, ...): its gesture.
    pub(super) fn snap_drag(&mut self, drag: Drag) -> Option<Gesture> {
        let scale = self.zoom_2d();
        let ctx = self.ctx.clone();
        let mut snapped = None;
        headless(ctx.run_ui(RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                snapped = self.app.snapped(ui, Some(Gesture::Dragging(drag)), scale);
            });
        }));
        snapped
    }

    // ---- the 3D view and imported models ----

    /// Shows `view` (View ▸ 2D or 3D).
    pub(super) fn show_view(&mut self, view: View) {
        self.app.view = view;
    }

    /// Whether View ▸ Auto Wrapping is on: a rotary job drawn round its axis.
    pub(super) fn auto_wrapping(&self) -> bool {
        self.app.display.auto_wrapping
    }

    /// Ticks or clears View ▸ Auto Wrapping.
    pub(super) fn set_auto_wrapping(&mut self, on: bool) {
        self.app.display.auto_wrapping = on;
    }

    /// Which way the 3D view looks.
    pub(super) fn orbit(&self) -> view3d::Orbit {
        self.app.orbit
    }

    /// Imports the model or picture file at `path`, as File ▸ Import does.
    pub(super) fn import(&mut self, path: &std::path::Path) {
        self.app.import_file(path);
    }

    /// The model the import form shows while it is open, triangle by triangle.
    pub(super) fn importing_model(&self) -> Option<&[cmd_orientate_3d_model::Seen]> {
        self.app
            .form
            .as_ref()
            .and_then(|f| f.model.as_deref())
            .map(Vec::as_slice)
    }

    /// Whether the open form has changes its model or shape has yet to follow.
    pub(super) fn form_waiting_to_follow(&self) -> bool {
        self.app.form.as_ref().is_some_and(|f| f.dirty)
    }

    /// The component the open form made, if it made one.
    pub(super) fn form_made(&self) -> Option<EntityId> {
        self.app.form.as_ref().and_then(|f| f.drawn)
    }

    /// The heights the 3D view draws the models' surface from, `cells` a side.
    pub(super) fn model_shape(&mut self, cells: usize) -> Material {
        self.app.model_shape(cells)
    }

    /// Whether the 3D view shows the job turned round its rotary axis.
    pub(super) fn shows_turned(&self) -> bool {
        self.app.shows_turned()
    }

    /// The rotary job's full models as the 3D view draws them, triangle by triangle.
    pub(super) fn rotary_models(&self) -> Vec<cmd_orientate_3d_model::Seen> {
        self.app.rotary_models(None)
    }

    /// A drag of `delta` in a 3D view of `size` while a model is placed: whether the form
    /// took it (turning or moving the model) rather than the view.
    pub(super) fn drag_3d(
        &mut self,
        mode: view3d::DragMode,
        delta: egui::Vec2,
        size: egui::Vec2,
    ) -> bool {
        self.app.import_drag(Some(mode), delta, size)
    }

    /// The 3D view's detail now, and whether it is the coarser one used while turning.
    pub(super) fn detail_3d(&self) -> (crate::options::Detail, bool) {
        self.app.detail_3d
    }

    /// The cells a side the 3D view draws with while it turns.
    pub(super) fn moving_cells(&self) -> usize {
        self.app.moving_cells()
    }

    // ---- unsigned code ----

    /// The answer the user gives whenever Cut Craft asks whether to run unsigned code.
    pub(super) fn answer_unsigned_with(
        &mut self,
        answer: fn(&crate::trust::Code) -> crate::trust::Answer,
    ) {
        self.app.trust.ask = answer;
    }

    /// Puts the unsigned gadget `source`, from the file `file`, in My Gadgets as Cut Craft
    /// finds it on starting: unread, by its file's name. Its command id.
    pub(super) fn given_unsigned_gadget(&mut self, file: &str, source: &str) -> &'static str {
        let commands = std::sync::Arc::new(crate::commands::registry());
        let (gadget, waiting) = cut_craft_scripting::waiting_gadget(file, source, commands);
        let id = cut_craft_core::DynCommand::id(&gadget);
        std::sync::Arc::get_mut(&mut self.app.registry)
            .expect("nothing else holds the commands between frames")
            .add_dyn(Box::new(gadget));
        self.app.my_gadgets.push((file.to_owned(), id));
        let code = crate::trust::Code::gadget(id, file, source.as_bytes());
        let waiting = Some(waiting);
        self.app
            .unsigned_gadgets
            .insert(id.to_owned(), crate::gadgets::Unsigned { code, waiting });
        id
    }

    /// My Gadgets, as its list names them.
    pub(super) fn my_gadgets(&self) -> Vec<String> {
        self.app
            .installed_items()
            .into_iter()
            .map(|(n, _)| n)
            .collect()
    }

    /// Add-ons kept in `folder`, as a settings folder holds them.
    pub(super) fn given_add_ons_folder(&mut self, folder: std::path::PathBuf) {
        self.app.addons = crate::addons::Addons::new(Some(folder));
    }

    /// Starting the add-on `id`'s `program`: whether it may run (asking if unsigned).
    pub(super) fn may_run_add_on(
        &mut self,
        id: &str,
        name: &str,
        program: &std::path::Path,
    ) -> bool {
        self.app.may_run_add_on(id, name, program)
    }

    // ---- menus, the clipboard and undo ----

    /// The items of the menu bar's menu `name` (File, Edit, Gadgets, ...).
    pub(super) fn menu_items(&self, name: &str) -> &[gui::MenuItem] {
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
    pub(super) fn available(&self, id: &str) -> bool {
        self.app.available(id)
    }

    /// How many pieces Copy or Cut put on the clipboard.
    pub(super) fn on_clipboard(&self) -> usize {
        self.app.clipboard.len()
    }

    /// Whether Edit ▸ Undo has anything to take back.
    pub(super) fn can_undo(&self) -> bool {
        self.app.history.can_undo()
    }

    /// How many steps Edit ▸ Undo could take back.
    pub(super) fn undo_steps(&self) -> usize {
        self.app.history.undo_depth()
    }

    /// A count that moves on with every change to the job, undo and redo too.
    pub(super) fn job_revision(&self) -> u64 {
        self.app.history.revision()
    }

    // ---- forms, as their buttons run them ----

    /// The form's Apply, Calculate or Create with `settings`.
    pub(super) fn apply_form(&mut self, id: &str, settings: &Value) -> Result<(), String> {
        self.app.apply_form(id, settings)
    }

    /// Replaces all the open form's settings with `settings`, as typing each would.
    pub(super) fn set_form_settings(&mut self, settings: Value) {
        self.set_form_value("", settings);
    }

    /// The settings `id`'s form opens with on this job.
    pub(super) fn opening_settings(&self, id: &str) -> Value {
        self.app
            .registry
            .get(id)
            .unwrap_or_else(|| panic!("no tool {id}"))
            .initial_params(&self.app.doc)
    }

    /// `id`'s own default settings.
    pub(super) fn default_settings(&self, id: &str) -> Value {
        self.app
            .registry
            .get(id)
            .unwrap_or_else(|| panic!("no tool {id}"))
            .default_params()
    }

    /// The note under the open form's buttons ("Already up to date", ...), if it shows one.
    pub(super) fn form_note(&self) -> Option<&str> {
        self.app.form_note()
    }

    /// Calculate as New on a toolpath form.
    pub(super) fn calculate_as_new(&mut self) {
        self.app.calculate_as_new();
    }

    /// Picks toolpath `id` in the Toolpaths list.
    pub(super) fn select_toolpath(&mut self, id: EntityId) {
        self.app.selected_toolpath = Some(id);
    }

    // ---- the window, starting up and slow jobs ----

    /// The window's title.
    pub(super) fn window_title(&self) -> &str {
        &self.app.title
    }

    /// The tool the status bar names as in use.
    pub(super) fn current_tool(&self) -> &str {
        &self.app.status.current_tool
    }

    /// The job is the file at `path`, as if opened from it.
    pub(super) fn given_job_path(&mut self, path: std::path::PathBuf) {
        self.app.session.path = Some(path);
    }

    /// Starts Cut Craft with no file to open, as a plain launch does.
    pub(super) fn start_up(&mut self) {
        self.app.open_at_start(None);
    }

    /// The screen showing: the start screen or a job.
    pub(super) fn screen(&self) -> Screen {
        self.app.screen
    }

    /// The progress window shows as soon as a slow job starts, not after a wait.
    pub(super) fn given_progress_shown_at_once(&mut self) {
        self.app.wait_before_showing = std::time::Duration::ZERO;
    }

    /// Starts `work` as a slow job titled `title`, as a toolpath that takes a while does;
    /// once done (and still wanted), the status bar says "used".
    pub(super) fn start_slow_job(&mut self, title: &str, work: impl FnOnce() + Send + 'static) {
        self.app.in_background(title, "test:slow", work, |app, ()| {
            app.status.set_message("used");
        });
    }

    /// Whether a slow job is running (its progress window up).
    pub(super) fn busy(&self) -> bool {
        self.app.busy.is_some()
    }

    // ---- the mouse and the views it moves ----

    /// How the 2D view maps the job to the screen, as last drawn.
    pub(super) fn view_2d_transform(&self) -> ViewTransform {
        self.app.view_transform.expect("the 2D view was drawn")
    }

    /// The 3D view's area on screen.
    pub(super) fn view_3d_area(&self) -> Rect {
        self.app.view_3d_area.expect("the 3D view is drawn")
    }

    /// How the 3D view is panned and zoomed.
    pub(super) fn framing_3d(&self) -> view3d::Framing {
        self.app.framing
    }

    /// The view tool in hand (Pan, Zoom Box, ...), if one is.
    pub(super) fn view_tool(&self) -> Option<ViewTool> {
        self.app.view_tool
    }

    /// Puts the view tool down, back to selecting.
    pub(super) fn put_down_view_tool(&mut self) {
        self.app.view_tool = None;
    }

    /// The buttons drawn as words, as where the icons are missing.
    pub(super) fn given_no_icons(&mut self) {
        self.app.icons = Icons::load(std::path::Path::new("/no/icons"));
    }

    /// What Copy or Cut put on Cut Craft's clipboard.
    pub(super) fn clipboard(&self) -> &[Path] {
        &self.app.clipboard
    }

    // ---- the window ----

    /// The desktop's own title bar, as chosen at start (the menus then start at the left).
    pub(super) fn given_desktop_title_bar(&mut self) {
        self.app.title_bar_at_start = true;
    }

    // ---- shortcuts ----

    /// Opens Options at its Shortcuts page.
    pub(super) fn open_shortcut_options(&mut self) {
        self.app.options_window = true;
        self.app.options_goto = Some(super::super::shortcuts_page::CATEGORY);
    }

    /// Whether the Options window is open.
    pub(super) fn options_window_open(&self) -> bool {
        self.app.options_window
    }

    /// A press on the Shortcuts page: Add…, Reset, Move it here, ...
    pub(super) fn edit_shortcuts(&mut self, edit: super::super::shortcuts_page::Edit) {
        self.app.apply_shortcut_edit(edit);
    }

    /// The Shortcuts page waiting for the next key or button, if it is.
    pub(super) fn waiting_for_shortcut(&self) -> Option<&super::super::settings::ShortcutCapture> {
        self.app.capture.as_ref()
    }

    /// The keys and buttons the Shortcuts page lists for `id` (applied or not).
    pub(super) fn shortcuts_of(&self, id: &str) -> Vec<shortcut::Trigger> {
        self.app.shortcuts_of(id)
    }

    /// Adds `trigger` to `id` on the Shortcuts page (used once applied).
    pub(super) fn add_shortcut(&mut self, id: &str, trigger: shortcut::Trigger) {
        self.app.add_shortcut(id, trigger);
    }

    /// Takes `trigger` off `id` on the Shortcuts page (gone once applied).
    pub(super) fn remove_shortcut(&mut self, id: &str, trigger: shortcut::Trigger) {
        self.app.remove_shortcut(id, trigger);
    }

    /// Every key and button in use now, with the action each runs.
    pub(super) fn shortcuts_in_use(&self) -> Vec<(String, shortcut::Trigger)> {
        self.app.bindings(&self.ctx)
    }

    /// `id`'s shortcut as the menus show it.
    pub(super) fn shortcut_text(&self, id: &str) -> Option<String> {
        self.app.shortcut_text(id)
    }

    /// The Shortcuts page's rows when its search box holds `search`.
    pub(super) fn shortcut_rows(&self, search: &str) -> Vec<super::super::shortcuts_page::Row> {
        self.app.shortcut_rows(search)
    }

    // ---- machines and post processors ----

    /// Adds a machine (Machine ▸ Add Machine, its settings left as they come), now the one
    /// in use.
    pub(super) fn given_machine(&mut self) {
        self.app.machines.add();
    }

    /// Changes the machine in use, as its Machine Configuration would.
    pub(super) fn edit_machine(&mut self, edit: impl FnOnce(&mut cut_craft_machines::Machine)) {
        edit(self.app.machines.active_mut().expect("a machine in use"));
    }

    /// The machine in use, if there is one.
    pub(super) fn machine(&self) -> Option<&cut_craft_machines::Machine> {
        self.app.machines.active()
    }

    /// Every machine Cut Craft keeps.
    pub(super) fn machines(&self) -> &[cut_craft_machines::Machine] {
        &self.app.machines.machines
    }

    /// The post processor File ▸ Save Toolpaths would save through now, or why there is none.
    pub(super) fn output_post(&self) -> Result<cut_craft_post::Post, String> {
        self.app.output_post()
    }

    /// The job's toolpaths saved through `post` to `path`, as Save Toolpaths does.
    pub(super) fn save_through(
        &self,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<(), String> {
        files::save_through(post, &self.app.slots(), &self.app.doc, path, None)
    }

    /// Save Toolpaths' window set as `form`, saving through `post` to `path`: what it said.
    pub(super) fn save_toolpaths_to(
        &self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        path: &std::path::Path,
    ) -> Result<String, String> {
        self.app.save_toolpaths_to(form, post, path)
    }

    /// The post Save Toolpaths' window set as `form` saves through.
    pub(super) fn form_post(&self, form: &SaveForm) -> Result<cut_craft_post::Post, String> {
        self.app.form_post(form)
    }

    /// Send to Machine with Save Toolpaths' window set as `form`, through `post`, to
    /// `store`; `earlier` answers whether to replace files already there. What it said.
    pub(super) fn send_to(
        &mut self,
        form: &SaveForm,
        post: &cut_craft_post::Post,
        store: &dyn cut_craft_machines::send::Store,
        earlier: &dyn Fn(&[String]) -> Option<crate::options::EarlierGcode>,
    ) -> Result<String, String> {
        self.app.send_to(form, post, store, earlier)
    }

    /// The custom post processors kept in `folder`, as the settings folder holds them.
    pub(super) fn given_posts_folder(&mut self, folder: std::path::PathBuf) {
        self.app.posts = cut_craft_machines::Library::new(Some(folder));
    }

    /// The post processors Cut Craft has: its own and the ones installed.
    pub(super) fn posts(&self) -> &cut_craft_machines::Library {
        &self.app.posts
    }

    /// Installs the post processor file at `path`, as Post Processor Management's Install does.
    pub(super) fn install_post(&mut self, path: &std::path::Path) {
        self.app.install_post(path);
    }

    /// Installs the offline machine package at `path` (Machine ▸ Install Offline Package).
    pub(super) fn install_package(&mut self, path: &std::path::Path) {
        self.app.install_package(path);
    }

    /// Install Offline Package from inside Machine Configuration, with the package at `path`.
    pub(super) fn install_package_in_machine_window(&mut self, path: &std::path::Path) {
        self.app.machine_outcome_install(path);
    }

    /// Picks a post in Post Processor Management (Select, ...).
    pub(super) fn pick_post(&mut self, outcome: cut_craft_machines::PostOutcome) {
        self.app.post_outcome(outcome);
    }

    /// Reads the machines from the file at `path`, as starting does.
    pub(super) fn load_machines_from(&mut self, path: &std::path::Path) {
        self.app.load_machines_at(path);
    }

    /// Keeps a change to the machines (saving them), the status bar saying `done`.
    pub(super) fn keep_machines(&mut self, done: &str) {
        self.app.machines_changed(done);
    }

    /// Machine Configuration's Add, on the machines it is editing (kept once applied).
    pub(super) fn add_machine_in_window(&mut self) {
        self.app
            .machine_window
            .as_mut()
            .expect("Machine Configuration is open")
            .draft
            .add();
    }

    /// The machines Machine Configuration is editing, if it is open.
    pub(super) fn machines_being_edited(&self) -> Option<&cut_craft_machines::Machines> {
        self.app.machine_window.as_ref().map(|w| &w.draft)
    }

    /// Whether Post Processor Management is open.
    pub(super) fn post_window_open(&self) -> bool {
        self.app.post_window.is_some()
    }

    /// Whether Machine Files is open.
    pub(super) fn machine_files_open(&self) -> bool {
        self.app.machine_files.is_some()
    }

    /// The bit database the forms pick bits from.
    pub(super) fn tool_database(&self) -> &ToolDatabase {
        &self.app.tools
    }

    // ---- pages, menus and finding tools ----

    /// Which pages are open and pinned.
    pub(super) fn pages(&self) -> &Pages {
        &self.app.pages
    }

    /// The menu bar as it stands (the start screen's or a job's).
    pub(super) fn menu_bar(&self) -> &gui::MenuSystem {
        self.app.current_menus()
    }

    /// Find a Tool's matches for `text`: each tool's id and name.
    pub(super) fn find_tools(&self, text: &str) -> Vec<(&'static str, String)> {
        self.app.search_matches(text)
    }

    /// The lathe form's note on how fast A turns, and whether the machine caps it.
    pub(super) fn lathe_a_speed_note(&self) -> Option<(String, bool)> {
        self.app
            .lathe_a_speed(self.app.form.as_ref().expect("a form is open"))
    }

    // ---- crashes ----

    /// Starts after a crash that left `crash` as its log.
    pub(super) fn crashed_last_time(&mut self, crash: String) {
        let ctx = self.ctx.clone();
        self.app.offer_crash(&ctx, crash);
    }

    /// Whether the bug report window is open.
    pub(super) fn bug_report_open(&self) -> bool {
        self.app.reporter.is_open()
    }
}

/// The test files moved onto the verbs, each with its source: they never reach into the app.
const THROUGH_THE_HARNESS: [(&str, &str); 13] = [
    ("layers_sheets.rs", include_str!("layers_sheets.rs")),
    (
        "accessibility_tests.rs",
        include_str!("../accessibility_tests.rs"),
    ),
    ("canvas.rs", include_str!("canvas.rs")),
    ("split_views.rs", include_str!("split_views.rs")),
    ("updating.rs", include_str!("updating.rs")),
    ("locked_face.rs", include_str!("locked_face.rs")),
    ("rotary_wrap.rs", include_str!("rotary_wrap.rs")),
    ("unsigned_code.rs", include_str!("unsigned_code.rs")),
    ("rotary.rs", include_str!("rotary.rs")),
    ("moving.rs", include_str!("moving.rs")),
    ("calculating.rs", include_str!("calculating.rs")),
    ("keys_and_mouse.rs", include_str!("keys_and_mouse.rs")),
    ("machines.rs", include_str!("machines.rs")),
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

/// Whether `line` names a harness's `.app` (`h.app.doc`, `next.app`, `hs[0].app`, a wrapped
/// `.app.form`), not a path such as `crate::app::` or a name such as `h.app_state`.
fn reaches_app(line: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(".app").any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + ".app".len()..].chars().next();
        after.is_none_or(|c| !word(c)) && before.is_none_or(|c| word(c) || c == ')' || c == ']')
    })
}

#[test]
fn reaching_in_finds_the_app_however_the_line_is_wrapped() {
    let source = "h.app.doc.notes = x;\nlet m = h\n    .app\n    .form;\nh\n    .app.apply(a);\n|app: &CutCraftApp| 1\nh.doc().notes\nnext.app.session\nuse crate::app::job_files;\nh.app_state()\nharnesses[0].app.doc\n";
    let lines: Vec<usize> = reaching_in(source).iter().map(|(n, _)| *n).collect();
    assert_eq!(lines, [1, 3, 6, 7, 9, 12]);
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
