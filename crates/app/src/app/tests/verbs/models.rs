//! 3D models and components: the level and component menus, what the 3D view picks,
//! Relief from Photo's depth estimate, and the model's windows.

use super::super::*;

impl Harness {
    // ---- the level and component menus ----

    /// The first level's right-click menu.
    pub(in super::super) fn level_menu(&self) -> Vec<gui::MenuItem> {
        first_level_menu(&self.app)
    }

    /// Picks `path` on the first level's right-click menu.
    pub(in super::super) fn choose_on_level(&mut self, path: &[&str]) {
        self.choose(first_level_menu, path);
    }

    /// The first component's right-click menu.
    pub(in super::super) fn component_menu(&self) -> Vec<gui::MenuItem> {
        first_component_menu(&self.app)
    }

    /// Picks `path` on the first component's right-click menu.
    pub(in super::super) fn choose_on_component(&mut self, path: &[&str]) {
        self.choose(first_component_menu, path);
    }

    /// Picks `path` on the 3D view's right-click menu, for what is under the pointer.
    pub(in super::super) fn choose_on_view_3d_menu(&mut self, path: &[&str]) {
        self.choose(
            |app| app.menus.view_3d_menu(&app.doc, app.drawn_3d.under()),
            path,
        );
    }

    /// The 3D view's right-click menu, for what is under the pointer.
    pub(in super::super) fn view_3d_menu_here(&self) -> Vec<gui::MenuItem> {
        (self.app.menus).view_3d_menu(&self.app.doc, self.app.drawn_3d.under())
    }

    /// The components under the pointer in the 3D view at the last right-click.
    pub(in super::super) fn under_pointer_3d(&self) -> &[EntityId] {
        self.app.drawn_3d.under()
    }

    /// Collapses `level` in the Components list.
    pub(in super::super) fn collapse_level(&mut self, level: u64) {
        self.app.collapsed_levels.insert(level);
    }

    /// The level `change` puts new components on, when they all go on one; the status bar
    /// names it.
    pub(in super::super) fn new_components_level(change: &Change) -> Option<u64> {
        CutCraftApp::new_components_level(change)
    }

    /// Whether Model ▸ Resolution's item `id` is ticked, or `None` when it is not one.
    pub(in super::super) fn resolution_checked(&self, id: &str) -> Option<bool> {
        self.app.resolution_checked(id)
    }

    // ---- the 3D view ----

    /// The picked components' outline in the 3D view, ring by ring, if one is drawn.
    pub(in super::super) fn picked_outline(&self) -> Option<&[Vec<Point>]> {
        self.app
            .picked_outline
            .as_ref()
            .map(|(_, rings)| rings.as_slice())
    }

    /// The component a double-click at `at` on the 3D view drawn in `fit` picks, if any.
    pub(in super::super) fn component_at_3d(&mut self, fit: Rect, at: Pos2) -> Option<EntityId> {
        self.app.component_at_3d(fit, at)
    }

    /// View ▸ Show Vectors in 3D, on or off.
    pub(in super::super) fn given_vectors_in_3d(&mut self, on: bool) {
        self.app.display.vectors_3d = on;
    }

    /// Whether a 3D handle drag holds the settings it started from.
    pub(in super::super) fn dragging_3d_handle(&self) -> bool {
        self.app.handle_drag.is_setting_height()
    }

    /// Where the 3D settings popup draws the radio for `combine`.
    pub(in super::super) fn combine_radio_at(
        &self,
        combine: cut_craft_core::component::Combine,
    ) -> Pos2 {
        (self
            .app
            .settings_3d
            .as_ref()
            .expect("the settings popup")
            .radios)
            .iter()
            .find(|(m, _)| *m == combine)
            .map(|(_, at)| *at)
            .expect("the combine radios, drawn as icons")
    }

    // ---- components ----

    /// Components copied in this window, as Copy keeps them.
    pub(in super::super) fn given_copied_components(
        &mut self,
        components: Vec<cut_craft_core::component::Component>,
    ) {
        self.app.clipboard_components = components;
    }

    /// Saves component `id` as a sculpting brush at `path`.
    pub(in super::super) fn save_brush_to(
        &mut self,
        id: EntityId,
        path: &std::path::Path,
    ) -> Result<(), String> {
        self.app.save_brush_to(id, path)
    }

    /// Whether Export Model as STL is open.
    pub(in super::super) fn stl_window_open(&self) -> bool {
        self.app.stl_window
    }

    // ---- Relief from Photo's depth ----

    #[cfg(unix)]
    /// An unsigned add-on asks nothing: it runs this time, as answering so does.
    pub(in super::super) fn given_unsigned_runs_this_time(&mut self) {
        self.app.trust.ask = |_| crate::trust::Answer::RunThisTime;
    }

    #[cfg(unix)]
    /// Starts the Depth add-on `installed` estimating picture `id`, as `photo`.
    pub(in super::super) fn start_depth_estimate(
        &mut self,
        installed: &crate::depth::Installed,
        id: EntityId,
        photo: &cut_craft_core::bitmap::Bitmap,
    ) {
        let estimate = crate::depth::Estimating::start(installed, id, photo, &self.ctx);
        self.app.photo_depth.estimate = Some(estimate.expect("the add-on starts"));
    }

    #[cfg(unix)]
    /// Whether a depth estimate is still running.
    pub(in super::super) fn estimating_depth(&self) -> bool {
        self.app.photo_depth.estimating()
    }
}

fn first_level_menu(app: &CutCraftApp) -> Vec<gui::MenuItem> {
    let (&id, level) = app.doc.levels.iter().next().expect("a level");
    gui::level_menu(&app.doc, id, level)
}

fn first_component_menu(app: &CutCraftApp) -> Vec<gui::MenuItem> {
    let (&id, component) = app.doc.components.iter().next().expect("a component");
    gui::component_menu(&app.doc, id, component)
}
