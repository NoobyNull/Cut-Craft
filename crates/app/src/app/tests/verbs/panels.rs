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

    // ---- the Toolpaths page ----

    /// Which of the Toolpaths page's tabs (Create, Preview, ...) is open.
    pub(in super::super) fn toolpath_tab(&self) -> &'static str {
        gui::TOOLPATH_SUBTABS[self.app.toolpath_subtab].name
    }

    /// Opens the Toolpaths page's tab named `name`.
    pub(in super::super) fn show_toolpath_tab(&mut self, name: &str) {
        self.app.toolpath_subtab = gui::TOOLPATH_SUBTABS
            .iter()
            .position(|t| t.name == name)
            .unwrap_or_else(|| panic!("no Toolpaths tab {name}"));
    }

    /// The toolpath group picked in the Toolpaths list, if one is.
    pub(in super::super) fn selected_toolpath_group(&self) -> Option<&str> {
        self.app.selected_group.as_deref()
    }

    /// Puts the toolpaths in a new order, as a drag in the Toolpaths list does.
    pub(in super::super) fn arrange_toolpaths(
        &mut self,
        arranged: crate::toolpath_order::Arranged,
    ) {
        self.app.arrange(Some(arranged));
    }

    /// Ticks the Layers panel's "only layers on this sheet".
    pub(in super::super) fn show_only_layers_on_sheet(&mut self) {
        self.app.layers_on_sheet = true;
    }

    /// The right-click menu of `id`'s row in the Rail layout.
    pub(in super::super) fn row_menu(&self, id: &str) -> Vec<gui::MenuItem> {
        self.app.row_menu(id)
    }

    /// Whether the Rail's favourites list (its star) is open.
    pub(in super::super) fn rail_favourites_open(&self) -> bool {
        self.app.rail_favourites
    }

    /// The Design page's tab now showing.
    pub(in super::super) fn design_tab(&self) -> &'static str {
        DESIGN_TABS[self.app.design_tab]
    }

    /// Flies `page` out, as its side tab does.
    pub(in super::super) fn open_page(&mut self, page: Page) {
        self.app.pages.open(page);
    }

    /// Picks the tool `id` on `page`, as its button there does.
    pub(in super::super) fn pick_on_page(&mut self, page: Page, id: &str) {
        self.app.pick(&self.ctx, page, id);
    }

    /// Picks the item at `path` in the menu bar's menu `menu` (File, Help, ...).
    pub(in super::super) fn choose_on_menu_bar(&mut self, menu: &str, path: &[&str]) {
        let items = |app: &CutCraftApp| {
            (app.current_menus().menus.iter())
                .find(|m| m.name == menu)
                .unwrap_or_else(|| panic!("no {menu} menu"))
                .items
                .clone()
        };
        self.choose(items, path);
    }
}
