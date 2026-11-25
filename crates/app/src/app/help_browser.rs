//! What the Help window remembers between frames: the pages it has drawn and the `help:`
//! links it watches, the search box and its index, and the pages read before and after the
//! one shown (Back and Forward).

use cut_craft_help::search::{Doc, Index};
use cut_craft_help::wiki::History;

/// Every help page as (topic, title, Markdown).
pub(super) type Pages = Vec<(&'static str, String, String)>;

/// Which way through the pages read the Back or Forward button goes.
#[derive(Clone, Copy)]
pub(super) enum Step {
    Back,
    Forward,
}

/// The Help window's state.
#[derive(Default)]
pub(super) struct HelpBrowser {
    /// The rendered pages and the `help:` links watched.
    pub(super) cache: egui_commonmark::CommonMarkCache,
    /// What is typed in the search box.
    pub(super) search: String,
    /// Every help page, made the first time help is searched.
    index: Pages,
    /// The search's ranked index of [`Self::index`], made with it.
    ranked: Option<Index>,
    /// The pages read before and after the one shown.
    history: History<&'static str>,
}

impl HelpBrowser {
    /// Whether the pages still have to be given ([`Self::index_pages`]) before a search.
    pub(super) fn needs_pages(&self) -> bool {
        self.index.is_empty()
    }

    /// `pages` to search from now on, ranked afresh.
    pub(super) fn index_pages(&mut self, pages: Pages) {
        self.index = pages;
        self.ranked = None;
    }

    /// The pages searched: empty until [`Self::index_pages`].
    #[cfg(test)]
    pub(super) fn pages(&self) -> &[(&'static str, String, String)] {
        &self.index
    }

    /// The pages answering `query`, as (topic, title, the first line saying one of its
    /// words), best first.
    pub(super) fn results(&mut self, query: &str) -> Pages {
        let index = &self.index;
        let ranked = self.ranked.get_or_insert_with(|| {
            let docs: Vec<Doc> = index
                .iter()
                .map(|(t, title, md)| Doc {
                    id: (*t).to_owned(),
                    title: title.clone(),
                    text: md.clone(),
                })
                .collect();
            Index::new(&docs)
        });
        super::help::search(index, ranked, query)
    }

    /// Whether Back and Forward each have a page to go to.
    pub(super) fn can_step(&self) -> (bool, bool) {
        (self.history.can_go_back(), self.history.can_go_forward())
    }

    /// The page to show after `shown`: the one `step` goes to, else the one `clicked` (kept
    /// for Back when it is another page), else none.
    pub(super) fn go(
        &mut self,
        shown: &'static str,
        step: Option<Step>,
        clicked: Option<&'static str>,
    ) -> Option<&'static str> {
        match (step, clicked) {
            (Some(Step::Back), _) => self.history.back(shown),
            (Some(Step::Forward), _) => self.history.forward(shown),
            (None, Some(t)) if t != shown => {
                self.history.visit(shown);
                Some(t)
            }
            (None, other) => other,
        }
    }
}

#[cfg(test)]
mod tests;
