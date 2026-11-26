//! The window's own parts: layouts, the tool pages and their buttons, favourites, Options and
//! the language.

use super::super::*;

impl Harness {
    // ---- layouts ----

    /// Whether `id`'s menu item is ticked: `None` for an item that is no layout.
    pub(in super::super) fn layout_checked(&self, id: &str) -> Option<bool> {
        self.app.layout_checked(id)
    }

    /// The first start after an update: the one-time layout question.
    pub(in super::super) fn ask_layout_after_update(&mut self) {
        self.app.ask_layout_after_update();
    }

    /// The layout the layout question has picked, while it is asked.
    pub(in super::super) fn layout_question(&self) -> Option<crate::options::WindowLayout> {
        self.app.layout_question.as_ref().map(|q| q.chosen)
    }

    // ---- the tool pages ----

    /// What Find Tool's box holds, while it is open.
    pub(in super::super) fn find_tool_text(&self) -> Option<&str> {
        self.app.search.as_deref()
    }

    /// The tool pages' groups: the Design page's drawing and modeling, and the Toolpaths
    /// page's.
    pub(in super::super) fn toolbar(&self) -> &gui::Toolbar {
        &self.app.toolbar
    }

    /// The Design page's sub-tab showing (Create, Edit, Arrange, Model, ...).
    pub(in super::super) fn design_subtab(&self) -> &'static str {
        gui::DESIGN_SUBTABS[self.app.design_subtab].name
    }

    /// Shows the Design page's sub-tab `name`, as clicking it does.
    pub(in super::super) fn show_design_subtab(&mut self, name: &str) {
        self.app.design_subtab = (gui::DESIGN_SUBTABS.iter())
            .position(|t| t.name == name)
            .unwrap_or_else(|| panic!("no Design sub-tab {name}"));
    }

    /// The tools on `page`'s favourites, as the page lists them.
    pub(in super::super) fn favourite_tools(&self, page: Page) -> Vec<&'static str> {
        (self.app.tool_model()).favourites(page, &self.app.options.favourites)
    }

    /// Whether `page` is hidden, flown out or pinned.
    pub(in super::super) fn page_shown(&self, page: Page) -> Shown {
        self.app.pages.shown(page)
    }

    /// Unpins `page`, as its pin button does.
    pub(in super::super) fn unpin(&mut self, page: Page) {
        self.app.pages.set_pinned(page, false);
    }

    /// Where the docked page `i` (0 the left, 1 the right) was drawn, if it was.
    pub(in super::super) fn page_rect(&self, i: usize) -> Option<Rect> {
        self.app.page_rects[i]
    }

    /// Where the flown-out page was drawn, if one was.
    pub(in super::super) fn flyout_rect(&self) -> Option<Rect> {
        self.app.flyout_rect
    }

    /// How many tool buttons fit side by side on a pinned page, as last drawn.
    pub(in super::super) fn buttons_across(&self) -> f32 {
        self.app.buttons_across.get()
    }

    /// The button's words for the tool `id`.
    pub(in super::super) fn tool_label(&self, id: &str) -> String {
        self.app.tool_label(id)
    }

    /// Whether `id` is a command (not one of the app's own actions).
    pub(in super::super) fn is_command(&self, id: &str) -> bool {
        self.app.registry.get(id).is_some()
    }

    /// Whether `id` saves the job in another format (File ▸ Export ...).
    pub(in super::super) fn is_exporter(&self, id: &str) -> bool {
        self.app.registry.exporter(id).is_some()
    }

    /// Whether the tool `id`'s button shows an icon, not its name.
    pub(in super::super) fn tool_has_icon(&self, id: &str) -> bool {
        self.app
            .icons
            .find(self.app.explicit_icon(id), id)
            .is_some()
    }

    /// Why the greyed item `id` cannot be picked now, as its tooltip says.
    pub(in super::super) fn unavailable_why(&self, id: &str) -> String {
        self.app.unavailable_why(id)
    }

    // ---- menus and controls ----

    /// Every leaf item of the menu bar's menus.
    pub(in super::super) fn menu_actions(&self) -> Vec<String> {
        (self.app.menus.items().iter())
            .map(|i| i.action.clone())
            .collect()
    }

    /// The right-click menu on the selection.
    pub(in super::super) fn selection_menu(&self) -> Vec<gui::MenuItem> {
        self.app.menus.selection_menu(&self.app.doc)
    }

    /// The job as `doc` with `toolpath` picked, and nothing else going on: no undo, form,
    /// view tool or message, as a fresh start on that job.
    pub(in super::super) fn start_over_on(&mut self, doc: Document, toolpath: Option<EntityId>) {
        self.app.doc = doc;
        self.app.toolpath_pick.only(toolpath);
        self.app.history = History::default();
        (self.app.form, self.app.view_tool) = (None, None);
        self.app.status.set_message("");
    }

    /// Closes the open form, handing back its tool and settings, if one was open.
    pub(in super::super) fn take_form(&mut self) -> Option<(&'static str, Value)> {
        self.app.form.take().map(|f| (f.id, f.params))
    }

    /// The way the open form's shape is dragged (from its centre, across an edge, ...).
    pub(in super::super) fn form_way(&self) -> Option<crate::flyout::DragWay> {
        self.app.form.as_ref().map(|f| f.way)
    }

    /// Puts down the drawing tool in hand, dropping a polyline half drawn.
    pub(in super::super) fn put_down_drawing_tool(&mut self) {
        self.app.form = None;
        self.app.polyline = crate::polyline::Draft::default();
    }

    /// View ▸ Units: `units_mm` or `units_inch`.
    pub(in super::super) fn set_units(&mut self, id: &str) {
        self.app.set_units(id);
    }

    // ---- help beside the tools ----

    /// The tools in `id`'s group, in order (none for a page that is no tool's).
    pub(in super::super) fn help_group_of(&self, id: &str) -> Vec<&'static str> {
        self.app.help_group_of(id)
    }

    /// Whether the help shows an icon for `id`.
    pub(in super::super) fn has_help_icon(&self, id: &str) -> bool {
        self.app.help_icon(id).is_some()
    }

    /// The page `topic`'s words, with other pages' names linked.
    pub(in super::super) fn help_page(&self, topic: &str) -> String {
        self.app.help_page(topic)
    }

    /// The page `topic`'s title.
    pub(in super::super) fn help_title(&self, topic: &str) -> String {
        self.app.help_title(topic)
    }

    // ---- Options and the language ----

    /// The options as the Options window now holds them, applied or not.
    pub(in super::super) fn edited_options(&self) -> &crate::options::Options {
        self.app.edited_options()
    }

    /// Whether the Options window holds changes not applied.
    pub(in super::super) fn options_dirty(&self) -> bool {
        self.app.options_dirty()
    }

    /// Uses the language pack `pack` (`""` for English), as starting with it does.
    pub(in super::super) fn use_language(&mut self, pack: &str) {
        self.app.use_language(pack);
    }
}
