//! The window's pages: the menu bar, the start page, the Design and Toolpaths pages, their tabs and
//! sub-tabs.

use super::actions::start_task_key;
use super::{
    Align, CutCraftApp, DESIGN_TABS, Info, LINKS, Layout, Menus, Page, RECENT_FILE_ICON,
    SPLASH_LOGO, SPLASH_SHARE, STARTUP_TASKS, Screen, Shown, WHATS_NEW, egui, file_name, gui,
    menu_items, page_width, show_page, side_tabs, side_tabs_width,
};
use crate::tool_names::{page_tab_name, subtab_hint, subtab_name};
use cut_craft_bug_report::hint::HoverHint;
use cut_craft_i18n::tr;

impl CutCraftApp {
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Menu bar");
        let mut clicked = None;
        let recent: Vec<gui::MenuItem> = self
            .recent_shown()
            .iter()
            .enumerate()
            .map(|(i, path)| {
                gui::MenuItem::with_action(&file_name(path), &format!("open_recent:{i}"))
            })
            .collect();
        // on a Mac without its title bar, the menus share the title bar's row with the window's
        // traffic-light buttons, which sit at the left
        let mac_row = super::mac::is_mac(ui.ctx()) && !self.title_bar_at_start;
        egui::MenuBar::new().ui(ui, |ui| {
            if mac_row {
                ui.allocate_space(egui::vec2(
                    super::mac::TRAFFIC_LIGHTS,
                    super::mac::TITLE_BAR_HEIGHT - 2.0 * f32::from(ui.spacing().menu_margin.top),
                ));
            }
            for menu in &self.current_menus().menus {
                ui.menu_button(tr!(&super::menu_key(&menu.name)), |ui| {
                    menu_items(
                        ui,
                        &menu.items,
                        &Menus {
                            available: &|id| self.available(id),
                            why: &|id| self.unavailable_why(id),
                            checked: &|id| {
                                self.display
                                    .checked(id)
                                    .or_else(|| self.resolution_checked(id))
                                    .or_else(|| self.units_checked(id))
                                    .or_else(|| self.layout_checked(id))
                            },
                            shortcut_of: &|id| self.shortcut_text(id),
                            recent: &recent,
                            image: &|path| self.icons.find(path, "").map(|p| self.icons.image(p)),
                        },
                        &mut clicked,
                    );
                });
            }
            if mac_row {
                super::title_bar::title_and_drag(ui, &self.title);
            } else if !self.options.appearance.system_title_bar && !super::mac::is_mac(ui.ctx()) {
                super::title_bar::controls(ui, &self.title);
            }
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The menu bar with or without a job open.
    pub(super) fn current_menus(&self) -> &gui::MenuSystem {
        if self.screen == Screen::Start {
            &self.start_menus
        } else {
            &self.menus
        }
    }

    #[cfg(test)]
    pub(super) fn menu_names(&self) -> Vec<&str> {
        self.current_menus()
            .menus
            .iter()
            .map(|m| m.name.as_str())
            .collect()
    }

    /// No job open: the startup tasks and the jobs opened lately.
    pub(super) fn start_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Start page");
        let edge = ui.spacing().interact_size.y;
        let mut clicked = None;
        ui.strong(tr!("start-tasks"));
        for (id, icon) in STARTUP_TASKS {
            let label = tr!(&start_task_key(id));
            let button = match self.icons.find(icon, id) {
                Some(path) => egui::Button::image_and_text(
                    self.icons
                        .image(path)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                    label,
                ),
                None => egui::Button::new(label),
            }
            .frame(false);
            let response = ui
                .add_enabled(self.available(id), button)
                .on_disabled_hover_text(self.unavailable_why(id));
            if response.clicked() {
                clicked = Some(id.to_owned());
            }
        }
        ui.separator();
        ui.strong(tr!("start-recent"));
        if self.recent.is_empty() {
            ui.weak(tr!("start-recent-none"));
        }
        let glyph = self.icons.find(RECENT_FILE_ICON, "");
        for (i, path) in self.recent_shown().iter().enumerate() {
            let name = file_name(path);
            let button = match glyph {
                Some(glyph) => egui::Button::image_and_text(
                    self.icons
                        .image(glyph)
                        .fit_to_exact_size(egui::vec2(edge, edge)),
                    name,
                ),
                None => egui::Button::new(name),
            }
            .frame(false);
            if ui
                .add(button)
                .on_hover_hint(path.display().to_string())
                .clicked()
            {
                clicked = Some(format!("open_recent:{i}"));
            }
        }
        ui.separator();
        ui.strong(tr!("start-whats-new", version = env!("CARGO_PKG_VERSION")));
        for change in WHATS_NEW.lines().filter(|l| !l.trim().is_empty()) {
            // a bullet, with the change wrapping beside it
            ui.horizontal_top(|ui| {
                ui.label("•");
                ui.add(egui::Label::new(change.trim_start_matches("- ")).wrap());
            });
        }
        ui.separator();
        // the rest of the community links are in Help ▸ Community, out of the way
        let (label, url, _) = LINKS[0];
        ui.horizontal_wrapped(|ui| {
            ui.weak(tr!("start-questions"));
            ui.hyperlink_to(tr!("start-community-link", site = label), url);
        });
        if let Some(id) = clicked {
            self.invoke(ui.ctx(), &id);
        }
    }

    /// The middle of the window with no job open: the Cut Craft logo, which carries the
    /// name itself, so no heading under it.
    pub(super) fn splash(ui: &mut egui::Ui) {
        let space = ui.available_size();
        let edge = space.min_elem() * SPLASH_SHARE;
        ui.vertical_centered(|ui| {
            ui.add_space((space.y - edge) / 2.0);
            ui.add(
                egui::Image::from_bytes("bytes://splash-logo.jpg", SPLASH_LOGO)
                    .fit_to_exact_size(egui::vec2(edge, edge))
                    .corner_radius(edge * 0.06),
            );
        });
    }

    /// The Design page: the chosen design tab's tools, under any form opened from them.
    pub(super) fn design_page(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design page");
        self.page_header(ui, Page::Design, DESIGN_TABS[self.design_tab]);
        let tab = DESIGN_TABS[self.design_tab];
        // a tool's form takes the page's place
        if self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
            self.form_panel(ui);
            return;
        }
        // room kept at the foot for a tool's red note, so the icons above never move for it
        let note = self.tool_note_height(ui);
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - note)
            .show(ui, |ui| {
                match tab {
                    "Design" => {
                        if self.options.favourites.of(Page::Design).1 {
                            // favourites name every tool, whatever sub-tab it is on
                            let groups = self.design_groups("Design");
                            self.tools_or_favourites(ui, &groups, Page::Design);
                        } else {
                            self.design_subtabs(ui);
                            self.gadgets_section(ui);
                        }
                    }
                    "Sheets" => self.sheets_panel(ui),
                    "Library" => self.clipart_panel(ui),
                    "Layers" => self.layers_panel(ui),
                    "Components" => self.component_list(ui),
                    tab => {
                        ui.weak(tr!("tab-not-implemented", tab = tab));
                    }
                }
            });
        self.tool_note_label(ui);
        // the hovered tool's guidance, or the tool in hand's, in the room left under the icons
        if tab == "Design" {
            self.tip_box(ui);
        }
    }

    /// What the last tool with no form warned of (Group turning text into curves), in red,
    /// while nothing has changed since.
    pub(super) fn tool_note_label(&self, ui: &mut egui::Ui) {
        if let Some(note) = self.tool_note() {
            ui.colored_label(ui.visuals().error_fg_color, note);
        }
    }

    /// The red note [`Self::tool_note_label`] shows, if any.
    pub(super) fn tool_note(&self) -> Option<&str> {
        let (job, revision, note) = self.tool_note.as_ref()?;
        ((*job, *revision) == (self.job_epoch, self.history.revision())).then_some(note.as_str())
    }

    /// How tall [`Self::tool_note_label`] draws at `ui`'s width, with the gap over it.
    fn tool_note_height(&self, ui: &egui::Ui) -> f32 {
        self.tool_note().map_or(0.0, |note| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let colour = egui::Color32::PLACEHOLDER;
            let galley = ui
                .painter()
                .layout(note.to_owned(), font, colour, ui.available_width());
            galley.size().y + ui.spacing().item_spacing.y
        })
    }

    /// The tool groups a design tab shows: Design has the drawing then the modeling tools.
    pub(super) fn design_groups(&self, tab: &str) -> Vec<gui::ToolGroup> {
        match tab {
            "Design" => self.toolbar.all_design_groups(),
            _ => vec![],
        }
    }

    /// The Design tab's sub-tabs, under the groups always shown: Create, Edit, Arrange,
    /// Model, Import (see [`Self::subtabs`]).
    pub(super) fn design_subtabs(&mut self, ui: &mut egui::Ui) {
        cut_craft_bug_report::mark(ui, "Design sub-tabs");
        self.subtabs(ui, Page::Design);
    }

    /// A page's sub-tabs, under the groups always shown. Each names what its tools are for on
    /// hover, so a beginner looks under a word rather than down one long page of icons. A
    /// right-click moves one out of the row into a section of its own below it (or back), so
    /// tools used together can all show at once; the choice is kept in the options. The
    /// Design and Toolpaths pages follow the same rules.
    pub(super) fn subtabs(&mut self, ui: &mut egui::Ui, page: Page) {
        let open = self.subtab_head(ui, page);
        self.subtab_body(ui, page, open);
    }

    /// The top of a page with sub-tabs: the groups always shown, then the tab row. Returns
    /// the sub-tab open in the row, if any. The Toolpaths page keeps this in place while the
    /// sub-tab under it scrolls, so the tabs are always there to go back with.
    pub(super) fn subtab_head(&mut self, ui: &mut egui::Ui, page: Page) -> Option<usize> {
        /// The share of the usual padding and spacing the sub-tabs keep between them.
        const SUBTAB_PADDING: f32 = 0.5;
        let set = subtab_set(page);
        let always = match page {
            Page::Design => self.toolbar.named(set.always),
            Page::Toolpaths => self.toolbar.toolpath_named(set.always),
        };
        self.tool_groups(ui, &always, page);
        ui.separator();
        let sections = self.subtab_sections(page).to_vec();
        let (row, _) = set.layout(&sections);
        let open = set.open(*self.open_subtab_mut(page), &sections);
        let (mut picked, mut chosen) = (None, None);
        if !row.is_empty() {
            ui.horizontal_wrapped(|ui| {
                // a little tighter than a button row, so all five fit across the page
                ui.spacing_mut().button_padding.x *= SUBTAB_PADDING;
                ui.spacing_mut().item_spacing.x *= SUBTAB_PADDING;
                for &i in &row {
                    let tab = &set.tabs[i];
                    let response = ui
                        .selectable_label(Some(i) == open, subtab_name(page, tab))
                        .on_hover_hint(tr!("subtab-in-row", hint = subtab_hint(page, tab)));
                    if response.clicked() {
                        picked = Some(i);
                    }
                    if let Some(action) = self.popup(&response, || set.menu(tab.name, &sections)) {
                        chosen = Some(action);
                    }
                }
            });
            ui.add_space(ui.spacing().item_spacing.y);
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
        let open = picked.or(open);
        if let Some(i) = open {
            *self.open_subtab_mut(page) = i;
        }
        open
    }

    /// Under the tab row: the open sub-tab's tools, then each sub-tab shown as a section of
    /// its own.
    pub(super) fn subtab_body(&mut self, ui: &mut egui::Ui, page: Page, open: Option<usize>) {
        let set = subtab_set(page);
        let sections = self.subtab_sections(page).to_vec();
        let (_, apart) = set.layout(&sections);
        let mut chosen = None;
        if let Some(i) = open {
            self.subtab_contents(ui, page, i);
        }
        for i in apart {
            let tab = &set.tabs[i];
            ui.separator();
            // a tool picked elsewhere unfolds its section once; it can be folded again
            let unfold = (*self.unfold_mut(page) == Some(tab.name)).then_some(true);
            if unfold.is_some() {
                *self.unfold_mut(page) = None;
            }
            let label = egui::RichText::new(subtab_name(page, tab)).strong();
            let header = egui::CollapsingHeader::new(label)
                .id_salt((page_key(page), "section", tab.name))
                .default_open(true)
                .open(unfold)
                .show(ui, |ui| self.subtab_contents(ui, page, i));
            let response = header
                .header_response
                .on_hover_hint(tr!("subtab-as-section", hint = subtab_hint(page, tab)));
            if let Some(action) = self.popup(&response, || set.menu(tab.name, &sections)) {
                chosen = Some(action);
            }
        }
        if let Some(action) = chosen {
            self.invoke(ui.ctx(), &action);
        }
    }

    /// What sub-tab `i` of `page` shows: its groups' tools, or the Toolpaths page's own.
    fn subtab_contents(&mut self, ui: &mut egui::Ui, page: Page, i: usize) {
        match page {
            Page::Design => {
                let groups = self.toolbar.subtab_groups(i);
                self.tool_groups(ui, &groups, Page::Design);
            }
            Page::Toolpaths => self.toolpath_subtab(ui, i),
        }
    }

    /// The sub-tabs of `page` shown as sections of their own.
    fn subtab_sections(&self, page: Page) -> &[String] {
        match page {
            Page::Design => &self.options.design_sections,
            Page::Toolpaths => &self.options.toolpath_sections,
        }
    }

    /// The open sub-tab of `page`.
    fn open_subtab_mut(&mut self, page: Page) -> &mut usize {
        match page {
            Page::Design => &mut self.design_subtab,
            Page::Toolpaths => &mut self.toolpath_subtab,
        }
    }

    /// The section of `page` to unfold next frame.
    fn unfold_mut(&mut self, page: Page) -> &mut Option<&'static str> {
        match page {
            Page::Design => &mut self.design_unfold,
            Page::Toolpaths => &mut self.toolpath_unfold,
        }
    }

    /// Opens the sub-tab `id` is on, so a tool picked from Find Tool, a menu or a shortcut
    /// shows highlighted where it lives: its tab in the row, or its section unfolded.
    pub(super) fn show_subtab_of(&mut self, id: &str) {
        let design = self.toolbar.subtab_of(id, self.design_subtab);
        let toolpaths = self.toolbar.toolpath_subtab_of(id, self.toolpath_subtab);
        for (page, tab) in [(Page::Design, design), (Page::Toolpaths, toolpaths)] {
            if let Some(tab) = tab {
                self.open_subtab(page, tab);
            }
        }
    }

    /// Opens sub-tab `tab` of `page`: in the row, or its section unfolded.
    pub(super) fn open_subtab(&mut self, page: Page, tab: usize) {
        let name = subtab_set(page).tabs[tab].name;
        if self.subtab_sections(page).iter().any(|s| s == name) {
            *self.unfold_mut(page) = Some(name);
        } else {
            *self.open_subtab_mut(page) = tab;
        }
    }

    /// A page's title with its push-pin and the Switch Layout button.
    pub(super) fn page_header(&mut self, ui: &mut egui::Ui, page: Page, title: &str) {
        let row = ui.spacing().interact_size.y;
        let (mut pin, mut switch, mut help) = (false, false, false);
        ui.horizontal(|ui| {
            ui.strong(page_tab_name(title));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pinned = self.pages.pinned(page);
                pin = ui
                    .add(self.pin_button(pinned, row))
                    .on_hover_hint(tr!(if pinned { "page-unpin" } else { "page-pin" }))
                    .clicked();
                switch = ui
                    .add(self.icon_button("view/interface_layout.png", "", "Switch Layout", row))
                    .on_hover_hint(tr!("page-switch-layout"))
                    .clicked();
                help = ui.button("?").on_hover_hint(tr!("page-help")).clicked();
            });
        });
        ui.separator();
        if pin {
            let pinned = self.pages.pinned(page);
            self.pages.set_pinned(page, !pinned);
        }
        if switch {
            self.pages.switch_layout();
        }
        if help {
            self.info = Some(Info::Help(match page {
                Page::Design => "design",
                Page::Toolpaths => "toolpaths",
            }));
        }
    }

    /// The side tabs and the docked pages around the view; flown-out pages over it.
    pub(super) fn job_pages(&mut self, ui: &mut egui::Ui) {
        let ctx = &ui.ctx().clone();
        let style = ctx.global_style();
        // from the window's room alone, before anything is drawn in it
        let width = page_width(ui);
        let margin = style.spacing.item_spacing.y;
        let strip = || egui::Frame::side_top_panel(&style).inner_margin(margin);
        let left_tabs = egui::Panel::left("design_tabs")
            .resizable(false)
            .exact_size(side_tabs_width(&style))
            .frame(strip())
            .show(ui, |ui| self.design_tabs(ui))
            .response
            .rect;
        let right_tabs = egui::Panel::right("toolpath_tabs")
            .resizable(false)
            .exact_size(side_tabs_width(&style))
            .frame(strip())
            .show(ui, |ui| self.toolpath_tabs(ui))
            .response
            .rect;
        self.page_rects = [None, None];
        self.flyout_rect = None;
        if self.pages.shown(Page::Design) == Shown::Docked {
            let rect = show_page(egui::Panel::left("design_panel"), width, ui, |ui| {
                self.design_page(ui);
            });
            self.page_rects[0] = Some(rect);
        }
        if self.pages.shown(Page::Toolpaths) == Shown::Docked {
            let rect = show_page(egui::Panel::right("toolpaths_panel"), width, ui, |ui| {
                self.toolpaths_page(ui);
            });
            self.page_rects[1] = Some(rect);
        }
        let room = ui.available_rect_before_wrap();
        // a click on the view puts a flown-out page away, unless its form is open
        let pressed = ctx.input(|i| {
            i.pointer
                .primary_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        for (page, tabs) in [(Page::Design, left_tabs), (Page::Toolpaths, right_tabs)] {
            if self.pages.shown(page) != Shown::Flyout {
                continue;
            }
            let out = self.flyout(ctx, page, room, width);
            self.flyout_rect = Some(out);
            let form_here = self.form.as_ref().is_some_and(|f| f.page == page);
            let on_view = |at: egui::Pos2| {
                room.contains(at)
                    && ctx.layer_id_at(at).map(|l| l.order) != Some(egui::Order::Foreground)
            };
            if let Some(at) = pressed
                && on_view(at)
                && !out.contains(at)
                && !tabs.contains(at)
                && !form_here
            {
                self.pages.done_with(page);
            }
        }
    }

    /// A tool picked from a page: a page that flew out for it goes away again, unless the
    /// tool opened a form there.
    pub(super) fn pick(&mut self, ctx: &egui::Context, page: Page, id: &str) {
        self.invoke(ctx, id);
        // a button that opens another design tab leaves the page out to show it
        let navigates = matches!(id, "clipart_library" | "component_levels");
        if self.form.is_none() && !navigates {
            self.pages.done_with(page);
        }
    }

    /// The design tabs, down the left edge.
    pub(super) fn design_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Design) != Shown::Hidden;
        let only = self.options.favourites.of(Page::Design).1;
        let labels = DESIGN_TABS.map(|t| {
            let name = page_tab_name(t);
            if t == "Design" && only {
                format!("{name} ★")
            } else {
                name
            }
        });
        let labels = labels.each_ref().map(String::as_str);
        let tabs = side_tabs(ui, &labels, shown.then_some(self.design_tab));
        let (hovered, clicked) = (
            tabs.iter().position(egui::Response::hovered),
            tabs.iter().position(egui::Response::clicked),
        );
        let design = DESIGN_TABS.iter().position(|&t| t == "Design");
        if let Some(action) =
            design.and_then(|i| self.popup(&tabs[i], || gui::favourites_menu(Page::Design, only)))
        {
            self.invoke(ui.ctx(), &action);
        }
        // an auto-hidden page comes out on hover; a docked one changes tab on click
        let unpinned = !self.pages.pinned(Page::Design);
        if let Some(tab) = clicked.or(hovered.filter(|_| unpinned)) {
            self.set_design_tab(tab);
            self.pages.open(Page::Design);
        }
    }

    /// A favourites action from a tool's or a tab's menu, kept in the options at once.
    pub(super) fn favourites_action(&mut self, action: &str) {
        if let Some(rest) = action.strip_prefix(gui::FAVOURITES_ONLY)
            && let Some((page, on)) = rest.split_once(':')
            && let Some(page) = gui::page_of(page)
        {
            *self.options.favourites.of_mut(page).1 = on == "true";
        } else if let Some(rest) = action.strip_prefix(gui::FAVOURITE)
            && let [page, what, id] = rest.splitn(3, ':').collect::<Vec<_>>()[..]
            && let Some(page) = gui::page_of(page)
        {
            let list = self.options.favourites.of_mut(page).0;
            let at = list.iter().position(|f| f == id);
            match (what, at) {
                ("add", None) => list.push(id.to_owned()),
                ("remove", Some(i)) => {
                    list.remove(i);
                }
                ("earlier", Some(i)) if i > 0 => list.swap(i, i - 1),
                ("later", Some(i)) if i + 1 < list.len() => list.swap(i, i + 1),
                _ => return,
            }
        } else {
            return;
        }
        self.save_options();
    }

    /// Going to another design tab closes a form open on the Design page.
    pub(super) fn set_design_tab(&mut self, tab: usize) {
        if tab != self.design_tab && self.form.as_ref().is_some_and(|f| f.page == Page::Design) {
            self.form = None;
        }
        self.design_tab = tab;
    }

    /// The Toolpaths tab, down the right edge.
    pub(super) fn toolpath_tabs(&mut self, ui: &mut egui::Ui) {
        let shown = self.pages.shown(Page::Toolpaths) != Shown::Hidden;
        let only = self.options.favourites.of(Page::Toolpaths).1;
        let label = if only { "Toolpaths ★" } else { "Toolpaths" };
        let tabs = side_tabs(ui, &[label], shown.then_some(0));
        let (hovered, clicked) = (
            tabs.iter().position(egui::Response::hovered),
            tabs.iter().position(egui::Response::clicked),
        );
        if let Some(action) = self.popup(&tabs[0], || gui::favourites_menu(Page::Toolpaths, only)) {
            self.invoke(ui.ctx(), &action);
        }
        let unpinned = !self.pages.pinned(Page::Toolpaths);
        if clicked.or(hovered.filter(|_| unpinned)).is_some() {
            self.pages.open(Page::Toolpaths);
        }
    }

    /// An auto-hidden page flown out over the view from its side, `outer` wide as a docked
    /// page is; returns where it is.
    pub(super) fn flyout(
        &mut self,
        ctx: &egui::Context,
        page: Page,
        room: egui::Rect,
        outer: f32,
    ) -> egui::Rect {
        let frame = egui::Frame::side_top_panel(&ctx.global_style());
        let margins = frame.total_margin().sum();
        let (width, height) = (outer - margins.x, room.height() - margins.y);
        let left = match page {
            Page::Design => room.left(),
            Page::Toolpaths => room.right() - outer,
        };
        egui::Area::new(egui::Id::unique(("flyout", page)))
            .fixed_pos(egui::pos2(left, room.top()))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                frame.show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(height);
                    // laid out apart, so contents asking for more room never widen it
                    let room =
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, height));
                    let mut inside = ui.new_child(egui::UiBuilder::new().max_rect(room));
                    inside.shrink_clip_rect(room);
                    match page {
                        Page::Design => self.design_page(&mut inside),
                        Page::Toolpaths => self.toolpaths_page(&mut inside),
                    }
                });
            })
            .response
            .rect
    }
}

/// The sub-tabs of `page`.
pub(super) fn subtab_set(page: Page) -> gui::SubTabSet {
    crate::tool_model::ToolModel::sections(page)
}

/// `page`'s name in ids.
pub(super) fn page_key(page: Page) -> &'static str {
    match page {
        Page::Design => "design",
        Page::Toolpaths => "toolpaths",
    }
}
