//! Help: its windows and pages, the search, the tips, and each tool's first step.

use super::super::*;

impl Harness {
    // ---- the Help windows ----

    /// The Help window open (About, Credits, a help page, ...), if one is.
    pub(in super::super) fn info(&self) -> Option<Info> {
        self.app.info
    }

    /// Opens the Help window `info`, as its Help menu item does.
    pub(in super::super) fn open_info(&mut self, info: Info) {
        self.app.info = Some(info);
    }

    /// Types `text` in the Credits window's search box.
    pub(in super::super) fn search_credits(&mut self, text: &str) {
        text.clone_into(&mut self.app.credits.search);
    }

    /// Puts `text` on the status bar, as the app's own messages do.
    pub(in super::super) fn given_status(&mut self, text: &str) {
        self.app.status.set_message(text);
    }

    // ---- the help pages ----

    /// The help page `topic` as the Help window shows it, in Markdown.
    pub(in super::super) fn help_markdown(&self, topic: &str) -> String {
        self.app.help_markdown(topic)
    }

    /// Every help page there is.
    pub(in super::super) fn all_help_topics(&self) -> Vec<&'static str> {
        self.app.all_help_topics()
    }

    /// The help pages of the tools on the pages and menus.
    pub(in super::super) fn help_topics(&self) -> Vec<&'static str> {
        self.app.help_topics()
    }

    /// Where the help finds `id`'s icon, `edge` points square, if it has one.
    pub(in super::super) fn help_icon_uri(&self, id: &str, edge: u16) -> Option<String> {
        self.app.help_icon_uri(id, edge)
    }

    /// What the page `topic` lists beside it (Related pages, Other tools like this).
    pub(in super::super) fn help_sidebar(&self, topic: &str) -> cut_craft_help::wiki::Sidebar {
        self.app.help_sidebar(topic, &self.app.help_page(topic))
    }

    /// The help search's results for `query`: (topic, title, the line that matched).
    pub(in super::super) fn help_search_results(
        &mut self,
        query: &str,
    ) -> Vec<(&'static str, String, String)> {
        self.app.help_search_results(query)
    }

    /// The pages the help search reads, once it has searched: (topic, title, words).
    pub(in super::super) fn help_searched_pages(&self) -> &[(&'static str, String, String)] {
        self.app.help.pages()
    }

    /// Types `text` in the Help window's search box.
    pub(in super::super) fn search_help(&mut self, text: &str) {
        text.clone_into(&mut self.app.help.search);
    }

    /// What the Help window's search box holds.
    pub(in super::super) fn help_search(&self) -> &str {
        &self.app.help.search
    }

    // ---- guidance ----

    /// What `id`'s tip box says to select first, if it needs a selection.
    pub(in super::super) fn first_step_of(&self, id: &str) -> Option<String> {
        self.app.first_step_of(id)
    }

    /// Every tool on the Design and Toolpaths pages.
    pub(in super::super) fn page_tools(&self) -> Vec<&'static str> {
        let toolbar = &self.app.toolbar;
        toolbar
            .all_design_groups()
            .iter()
            .chain(&toolbar.toolpath_groups)
            .flat_map(|g| g.tools.iter().copied())
            .collect()
    }

    // ---- Did You Know? ----

    /// The tip the Did You Know? window shows, if it is open.
    pub(in super::super) fn tip(&self) -> Option<usize> {
        self.app.tip
    }

    /// Opens Did You Know? on tip `n`.
    pub(in super::super) fn open_tips(&mut self, n: usize) {
        self.app.open_tips(n);
    }

    /// Closes Did You Know?, as its Close does.
    pub(in super::super) fn close_tips(&mut self) {
        self.app.tip = None;
    }

    /// Cut Craft starting: the tip it shows then, unless turned off.
    pub(in super::super) fn tip_at_startup(&mut self) {
        self.app.tip_at_startup();
    }

    /// Opens first-time setup, as the first launch does.
    pub(in super::super) fn open_first_run(&mut self) {
        self.app.open_first_run();
    }

    /// The icon a tip about the tool `id` shows, if there is one.
    pub(in super::super) fn tip_icon(&self, id: &str) -> Option<&str> {
        self.app.tip_icon(id)
    }

    /// The picture Did You Know? shows above `tip`.
    pub(in super::super) fn tip_picture(&self, tip: &crate::app::tips::Tip) -> Option<&str> {
        self.app.tip_picture(tip)
    }
}
