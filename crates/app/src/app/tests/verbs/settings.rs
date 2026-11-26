//! Settings: Options, the window layout, custom toolbars and backups of the settings.

use super::super::*;

impl Harness {
    // ---- Options and the window layout ----

    /// Options ▸ Window Layout's `id` (`layout_ribbon`, `layout_canvas`, ...).
    pub(in super::super) fn set_layout(&mut self, id: &str) {
        self.app.set_layout(id);
    }

    /// Opens Options on the page holding the setting `key`.
    pub(in super::super) fn open_options_at(&mut self, key: &'static str) {
        self.app.options_editor.open_at(key);
    }

    /// Options' Apply with `options` as its window held them when it opened.
    pub(in super::super) fn options_changed(&mut self, options: crate::options::Options) {
        let ctx = self.ctx.clone();
        self.app.options_changed(&ctx, options);
    }

    /// The Ribbon tab showing (Design, Model, ...).
    pub(in super::super) fn ribbon_tab(&self) -> &'static str {
        RIBBON_TABS[self.app.ribbon_tab]
    }

    // ---- custom toolbars ----

    /// The custom toolbar the Ribbon shows as a tab, if one is.
    pub(in super::super) fn ribbon_toolbar(&self) -> Option<usize> {
        self.app.toolbars.ribbon
    }

    /// The custom toolbar the Rail lists, if one is.
    pub(in super::super) fn rail_toolbar(&self) -> Option<usize> {
        self.app.toolbars.rail
    }

    /// Opens custom toolbar `i` in the Rail, as its button there does.
    pub(in super::super) fn show_rail_toolbar(&mut self, i: usize) {
        self.app.toolbars.rail = Some(i);
    }

    /// The group open on a custom toolbar: the toolbar and its place on it.
    pub(in super::super) fn toolbar_group_open(&self) -> Option<(usize, usize)> {
        self.app.toolbars.group
    }

    /// Where custom toolbar `n`'s bar was drawn last, while it is snapped or floating.
    pub(in super::super) fn toolbar_bar_rect(&self, n: usize) -> Option<egui::Rect> {
        let name = &self.app.options.toolbars.get(n)?.name;
        let id = crate::app::custom_toolbars::toolbar_bar_id(n, name);
        self.ctx.memory(|m| m.area_rect(id))
    }

    /// Where the Favourites bar was drawn last, while it floats or lies over the drawing.
    pub(in super::super) fn favourites_bar_rect(&self) -> Option<egui::Rect> {
        let id = crate::app::favourites_bar::favourites_bar_id();
        self.ctx.memory(|m| m.area_rect(id))
    }

    /// The Favourites bar's tools and groups, in order.
    pub(in super::super) fn favourite_items(&self) -> Vec<crate::custom_toolbars::Item> {
        self.app.favourite_items()
    }

    /// A Canvas palette group's right-click menu.
    pub(in super::super) fn group_menu(&self, name: &str) -> Vec<gui::MenuItem> {
        self.app.group_menu(name)
    }

    /// The Favourites bar's grip menu.
    pub(in super::super) fn favourites_menu(&self) -> Vec<gui::MenuItem> {
        self.app.favourites_menu()
    }

    /// Picks `item` on the Customize Toolbars window's left, as a click on it does.
    pub(in super::super) fn customize_pick(&mut self, item: crate::custom_toolbars::Item) {
        self.app.customize.left = Some(item);
    }

    /// Picks bar `bar` (an index of yours, or Favourites') on the window's right, and its
    /// item `at`.
    pub(in super::super) fn customize_pick_bar(&mut self, bar: usize, at: Option<usize>) {
        self.app.customize.bar = bar;
        self.app.customize.right = at;
    }

    /// Presses the window's button `button`.
    pub(in super::super) fn customize_press(&mut self, button: &str) {
        let ctx = self.ctx.clone();
        self.app.customize_press(&ctx, button);
    }

    /// The item picked on the window's right.
    pub(in super::super) fn customize_picked(&self) -> Option<usize> {
        self.app.customize.right
    }

    /// An edit waiting in the Options window, not yet applied.
    pub(in super::super) fn options_edit_waiting(
        &mut self,
        edit: impl FnOnce(&mut crate::options::Options),
    ) {
        let mut edited = self.app.edited_options().clone();
        edit(&mut edited);
        self.app.options_edited(edited);
    }

    /// Whether the window's button `button` can do something now.
    pub(in super::super) fn customize_can(&self, button: &str) -> bool {
        self.app.customize_action(button).is_some()
    }

    /// Whether the Customize Toolbars window is open.
    pub(in super::super) fn customizing(&self) -> bool {
        self.app.customize.open
    }

    /// Where the 2D view was drawn last.
    pub(in super::super) fn view_rect(&self) -> egui::Rect {
        self.app
            .views_drawn
            .area_2d()
            .expect("the 2D view has been drawn")
    }

    /// Where the Canvas palette was drawn last, while it floats or lies over the drawing.
    pub(in super::super) fn palette_rect(&self) -> Option<egui::Rect> {
        let id = crate::app::canvas_layout::palette_id();
        self.ctx.memory(|m| m.area_rect(id))
    }

    /// Whether a toolbar's name is being typed.
    pub(in super::super) fn renaming_toolbar(&self) -> bool {
        self.app.toolbars.rename.is_some()
    }

    // ---- backups of the settings ----

    /// Keeps the settings (and their backups) in `folder`, as a real start does.
    pub(in super::super) fn given_config_folder(&mut self, folder: std::path::PathBuf) {
        self.app.backups.config = Some(folder);
    }

    /// The automatic backups, if the settings have somewhere to go.
    pub(in super::super) fn automatic_backups(&self) -> Option<cut_craft_backup::Automatic> {
        self.app.automatic_backups()
    }
}
