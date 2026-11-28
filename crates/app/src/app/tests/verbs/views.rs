//! The 2D and 3D views, layouts and what the mouse moves in them.

use super::super::*;

impl Harness {
    // ---- views, layouts and floating lists ----

    /// One frame with the screen reader's tree turned on: the tree it was handed.
    pub(in super::super) fn accessibility_tree(&mut self) -> egui::accesskit::TreeUpdate {
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
    pub(in super::super) fn view(&self) -> &View {
        &self.app.view
    }

    /// How the views are split, if they are.
    pub(in super::super) fn split(&self) -> Option<Split> {
        self.app.split
    }

    /// The views split as `split`, as an earlier session left them.
    pub(in super::super) fn given_split(&mut self, split: Option<Split>) {
        self.app.split = split;
    }

    /// Side by side, with the 3D view on the left.
    pub(in super::super) fn given_3d_view_first(&mut self) {
        self.app.split_layout.three_d_first = true;
    }

    /// The corner the small view sits in.
    pub(in super::super) fn small_view_corner(&self) -> egui::Align2 {
        self.app.split_layout.corner
    }

    /// The left view's share of the width, side by side.
    pub(in super::super) fn split_share(&self) -> f32 {
        self.app.split_layout.share
    }

    /// Whether the 3D view is on the left, side by side.
    pub(in super::super) fn three_d_view_first(&self) -> bool {
        self.app.split_layout.three_d_first
    }

    /// Zooms the 2D view by `factor` about the screen point `at`, as the wheel does.
    pub(in super::super) fn zoom_2d_about(&mut self, at: Pos2, factor: f32) {
        self.app.camera = self
            .app
            .views_drawn
            .transform()
            .map(|t| t.zoomed_about(at, factor));
    }

    /// Where the 2D view's area starts on screen.
    pub(in super::super) fn view_2d_corner(&self) -> Pos2 {
        self.app.views_drawn.at_2d().expect("the 2D view is drawn")
    }

    /// Shows the ribbon (or Canvas palette) tab named `name`.
    pub(in super::super) fn show_ribbon_tab(&mut self, name: &str) {
        self.app.ribbon_tab = RIBBON_TABS
            .iter()
            .position(|t| *t == name)
            .unwrap_or_else(|| panic!("no tab {name}"));
    }

    /// The Canvas modeling bar's groups, in order.
    pub(in super::super) fn canvas_groups(&self) -> Vec<gui::ToolGroup> {
        self.app.canvas_groups()
    }

    /// The Canvas toolpaths bar's groups, in order.
    pub(in super::super) fn canvas_toolpath_groups(&self) -> Vec<gui::ToolGroup> {
        self.app
            .canvas_groups_of(crate::app::canvas_layout::CanvasBar::Toolpaths)
    }

    /// The Canvas palette group whose list is open, if one is.
    pub(in super::super) fn open_canvas_group(&self) -> Option<&'static str> {
        self.app.canvas.group
    }

    /// Opens the Canvas palette group `name`'s list, as pointing at it does.
    pub(in super::super) fn open_canvas_group_list(&mut self, name: &'static str) {
        self.app.canvas.group = Some(name);
    }

    /// The Canvas chip (Layers, Library, ...) whose list is open, as its tab's place.
    pub(in super::super) fn open_canvas_chip(&self) -> Option<usize> {
        self.app.canvas.chip
    }

    /// Whether Find a Tool's search box is open.
    pub(in super::super) fn finding_a_tool(&self) -> bool {
        self.app.search.is_some()
    }

    // ---- the 2D view ----

    /// Zoom to Job: the next frame fits the job in the view.
    pub(in super::super) fn zoom_to_job(&mut self) {
        self.invoke("zoom_to_job");
    }

    /// The 2D view's area on screen.
    pub(in super::super) fn view_2d_area(&self) -> Rect {
        self.app
            .views_drawn
            .area_2d()
            .expect("the 2D view was drawn")
    }

    /// How many screen points a millimetre is in the 2D view.
    pub(in super::super) fn zoom_2d(&self) -> f32 {
        self.app
            .views_drawn
            .transform()
            .expect("the 2D view was drawn")
            .scale
    }

    /// Where a drawing tool's drag lands once snapped (to ends, ghosts, ...): its gesture.
    pub(in super::super) fn snap_drag(&mut self, drag: Drag) -> Option<Gesture> {
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
    pub(in super::super) fn show_view(&mut self, view: View) {
        self.app.view = view;
    }

    /// Whether View ▸ Auto Wrapping is on: a rotary job drawn round its axis.
    pub(in super::super) fn auto_wrapping(&self) -> bool {
        self.app.display.auto_wrapping
    }

    /// Ticks or clears View ▸ Auto Wrapping.
    pub(in super::super) fn set_auto_wrapping(&mut self, on: bool) {
        self.app.display.auto_wrapping = on;
    }

    /// Which way the 3D view looks.
    pub(in super::super) fn orbit(&self) -> view3d::Orbit {
        self.app.orbit
    }

    /// Imports the model or picture file at `path`, as File ▸ Import does.
    pub(in super::super) fn import(&mut self, path: &std::path::Path) {
        self.app.import_file(path);
    }

    /// The model the import form shows while it is open, triangle by triangle.
    pub(in super::super) fn importing_model(&self) -> Option<&[cmd_orientate_3d_model::Seen]> {
        self.app
            .form
            .as_ref()
            .and_then(|f| f.model.as_deref())
            .map(Vec::as_slice)
    }

    /// Whether the open form has changes its model or shape has yet to follow.
    pub(in super::super) fn form_waiting_to_follow(&self) -> bool {
        self.app.form.as_ref().is_some_and(|f| f.dirty)
    }

    /// The component the open form made, if it made one.
    pub(in super::super) fn form_made(&self) -> Option<EntityId> {
        self.app.form.as_ref().and_then(|f| f.drawn)
    }

    /// The heights the 3D view draws the models' surface from, `cells` a side.
    pub(in super::super) fn model_shape(&mut self, cells: usize) -> Material {
        self.app.model_shape(cells)
    }

    /// Whether the 3D view shows the job turned round its rotary axis.
    pub(in super::super) fn shows_turned(&self) -> bool {
        self.app.shows_turned()
    }

    /// The rotary job's full models as the 3D view draws them, triangle by triangle.
    pub(in super::super) fn rotary_models(&self) -> Vec<cmd_orientate_3d_model::Seen> {
        self.app.rotary_models(None)
    }

    /// A drag of `delta` in a 3D view of `size` while a model is placed: whether the form
    /// took it (turning or moving the model) rather than the view.
    pub(in super::super) fn drag_3d(
        &mut self,
        mode: view3d::DragMode,
        delta: egui::Vec2,
        size: egui::Vec2,
    ) -> bool {
        self.app.import_drag(Some(mode), delta, size)
    }

    /// The 3D view's detail now, and whether it is the coarser one used while turning.
    pub(in super::super) fn detail_3d(&self) -> (crate::options::Detail, bool) {
        self.app.motion_3d.detail()
    }

    /// The cells a side the 3D view draws with while it turns.
    pub(in super::super) fn moving_cells(&self) -> usize {
        self.app.moving_cells()
    }

    // ---- the mouse and the views it moves ----

    /// How the 2D view maps the job to the screen, as last drawn.
    pub(in super::super) fn view_2d_transform(&self) -> ViewTransform {
        self.app
            .views_drawn
            .transform()
            .expect("the 2D view was drawn")
    }

    /// The 3D view's area on screen.
    pub(in super::super) fn view_3d_area(&self) -> Rect {
        self.app
            .views_drawn
            .area_3d()
            .expect("the 3D view is drawn")
    }

    /// How the 3D view is panned and zoomed.
    pub(in super::super) fn framing_3d(&self) -> view3d::Framing {
        self.app.framing
    }

    /// The view tool in hand (Pan, Zoom Box, ...), if one is.
    pub(in super::super) fn view_tool(&self) -> Option<ViewTool> {
        self.app.view_tool
    }

    /// Puts the view tool down, back to selecting.
    pub(in super::super) fn put_down_view_tool(&mut self) {
        self.app.view_tool = None;
    }

    /// The buttons drawn as words, as where the icons are missing.
    pub(in super::super) fn given_no_icons(&mut self) {
        self.app.icons = Icons::load(std::path::Path::new("/no/icons"));
    }

    /// What Copy or Cut put on PowerWhittle's clipboard.
    pub(in super::super) fn clipboard(&self) -> &[Path] {
        &self.app.clipboard
    }

    /// Turns the 3D view to `orbit`, as its view buttons do.
    pub(in super::super) fn given_orbit(&mut self, orbit: view3d::Orbit) {
        self.app.orbit = orbit;
    }

    /// Changes the open 3D settings popup's values, as dragging them does.
    pub(in super::super) fn edit_3d_settings(
        &mut self,
        edit: impl FnOnce(&mut cmd_component_properties::Params),
    ) {
        let s = self
            .app
            .settings_3d
            .as_mut()
            .expect("the 3D settings are open");
        edit(&mut s.params);
        s.changed(true);
    }

    /// Whether the icon set has the picture `icon`.
    pub(in super::super) fn has_icon(&self, icon: &str) -> bool {
        self.app.icons.find(icon, "").is_some()
    }
}
