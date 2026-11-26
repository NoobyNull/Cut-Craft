//! Toolpath forms and the Toolpaths list: their settings files, the area V-Carve shades and the
//! menus of the toolpaths made.

use super::super::*;

impl Harness {
    // ---- the open form ----

    /// Edits the open form's settings in place, as its fields and pictures do.
    pub(in super::super) fn edit_form(&mut self, edit: impl FnOnce(&mut Value)) {
        edit(&mut self.app.form.as_mut().expect("a form is open").params);
    }

    /// Whether the open form holds changes it is yet to apply.
    pub(in super::super) fn form_dirty(&self) -> bool {
        self.app.form.as_ref().expect("a form is open").dirty
    }

    /// The area V-Carve shades on the 2D view while its form is open, as triangles.
    pub(in super::super) fn carve_area(&self) -> Option<&[[Point; 3]]> {
        self.app.carve_area.as_ref().map(|a| a.triangles.as_slice())
    }

    /// Whether the job's Clearing Settings window is open.
    pub(in super::super) fn clearing_window_open(&self) -> bool {
        self.app.clearing_window
    }

    // ---- settings files ----

    /// Save Texture on the open Texture Toolpath form.
    pub(in super::super) fn save_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.app.save_texture(path)
    }

    /// Load Texture on the open Texture Toolpath form.
    pub(in super::super) fn load_texture(&mut self, path: &std::path::Path) -> Result<(), String> {
        self.app.load_texture(path)
    }

    /// Save Settings on the open toolpath form.
    pub(in super::super) fn save_form_settings(&mut self, path: &std::path::Path) {
        self.app.save_form_settings(path);
    }

    /// Load Settings on the open toolpath form.
    pub(in super::super) fn load_form_settings(&mut self, path: &std::path::Path) {
        self.app.load_form_settings(path);
    }

    /// Import Toolpaths, with `path` picked in its file window.
    pub(in super::super) fn import_toolpaths_from(&mut self, path: &std::path::Path) {
        self.app.import_toolpaths_from(path);
    }

    // ---- the Toolpaths list ----

    /// Picks `path` on the menu of the job's `i`th toolpath, as a right-click on its row opens.
    pub(in super::super) fn choose_on_toolpath(&mut self, i: usize, path: &[&str]) {
        self.choose(
            |app| {
                let (&id, t) = app.doc.toolpaths.iter().nth(i).expect("that toolpath");
                gui::toolpath_menu(&app.doc, id, t)
            },
            path,
        );
    }

    /// Picks no toolpath in the Toolpaths list, as a click on its empty space does.
    pub(in super::super) fn given_no_toolpath_picked(&mut self) {
        self.app.toolpath_pick.set(None);
    }

    /// The toolpaths picked in the Toolpaths list, in its order.
    pub(in super::super) fn picked_toolpaths(&self) -> Vec<EntityId> {
        self.app.picked_toolpaths()
    }

    /// The tool groups the Design side tab `tab` shows.
    pub(in super::super) fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        self.app.design_groups(tab)
    }

    /// Turns View ▸ Toolpath Previews in 2D on or off.
    pub(in super::super) fn given_2d_previews(&mut self, on: bool) {
        self.app.display.previews_2d = on;
    }
}
