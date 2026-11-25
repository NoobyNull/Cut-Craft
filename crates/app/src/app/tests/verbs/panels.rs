//! Pages, panels, the menu bar and Find a Tool.

use super::super::*;
use cut_craft_core::document::Layer;

impl Harness {
    // ---- pages and panels ----

    /// Shows the Design page's tab named `name` (Drawing, Layers, ...).
    pub(in super::super) fn show_design_tab(&mut self, name: &str) {
        let tab = DESIGN_TABS
            .iter()
            .position(|&t| t == name)
            .unwrap_or_else(|| panic!("no Design tab {name}"));
        self.app.set_design_tab(tab);
    }

    /// Pins `page` open, as its pin button does.
    pub(in super::super) fn pin(&mut self, page: Page) {
        self.app.pages.set_pinned(page, true);
    }

    /// Clicks `action`'s control (its eye, lock, ...) on `layer`'s row of the Layers panel.
    pub(in super::super) fn click_layer(&mut self, layer: LayerId, action: &str) {
        self.app.layer_action(&self.ctx, layer, action);
    }

    /// The right-click menu of the job's `i`th layer.
    pub(in super::super) fn layer_menu(&self, i: usize) -> Vec<gui::MenuItem> {
        let layer: &Layer = &self.doc().layers[i];
        gui::layer_menu(self.doc(), layer)
    }

    /// Right-clicks the job's `i`th layer and picks the item at `path`.
    pub(in super::super) fn choose_on_layer(&mut self, i: usize, path: &[&str]) {
        self.choose(|app| gui::layer_menu(&app.doc, &app.doc.layers[i]), path);
    }

    /// Types `text` in the Notes panel's Variable Explorer search box.
    pub(in super::super) fn search_variables(&mut self, text: &str) {
        self.app.notes_ui.search = text.into();
    }

    /// What the notes variable `key` stands for now.
    pub(in super::super) fn variable_value(&self, key: &str) -> String {
        crate::app::notes_panel::value_now(&self.app, key)
    }

    // ---- pages, menus and finding tools ----

    /// Which pages are open and pinned.
    pub(in super::super) fn pages(&self) -> &Pages {
        &self.app.pages
    }

    /// The menu bar as it stands (the start screen's or a job's).
    pub(in super::super) fn menu_bar(&self) -> &gui::MenuSystem {
        self.app.current_menus()
    }

    /// Find a Tool's matches for `text`: each tool's id and name.
    pub(in super::super) fn find_tools(&self, text: &str) -> Vec<(&'static str, String)> {
        self.app.search_matches(text)
    }

    /// The lathe form's note on how fast A turns, and whether the machine caps it.
    pub(in super::super) fn lathe_a_speed_note(&self) -> Option<(String, bool)> {
        self.app
            .lathe_a_speed(self.app.form.as_ref().expect("a form is open"))
    }
}
