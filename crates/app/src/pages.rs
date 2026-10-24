//! The two tool pages, Design (left) and Toolpaths (right): pinned pages stay docked open,
//! unpinned ones hide to a side tab and fly out over the view one at a time, as Aspire's do.

/// One of the two tool pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Page {
    Design,
    Toolpaths,
}

/// How a page is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// Pinned: a panel beside the view.
    Docked,
    /// Unpinned and opened from its tab: over the view until done with.
    Flyout,
    /// Unpinned: just its tab.
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pages {
    pub design_pinned: bool,
    pub toolpaths_pinned: bool,
    /// The unpinned page flown out, if any; only one at a time.
    open: Option<Page>,
}

impl Default for Pages {
    /// Aspire starts in its design layout.
    fn default() -> Self {
        Self::design_layout()
    }
}

impl Pages {
    /// Designing: Design pinned, Toolpaths hidden (F11).
    pub fn design_layout() -> Self {
        Self {
            design_pinned: true,
            toolpaths_pinned: false,
            open: None,
        }
    }

    /// Machining: Toolpaths pinned, Design hidden (F12).
    pub fn machining_layout() -> Self {
        Self {
            design_pinned: false,
            toolpaths_pinned: true,
            open: None,
        }
    }

    pub fn shown(self, page: Page) -> Shown {
        if self.pinned(page) {
            Shown::Docked
        } else if self.open == Some(page) {
            Shown::Flyout
        } else {
            Shown::Hidden
        }
    }

    pub fn pinned(self, page: Page) -> bool {
        match page {
            Page::Design => self.design_pinned,
            Page::Toolpaths => self.toolpaths_pinned,
        }
    }

    /// Its tab was hovered or clicked: an unpinned page flies out, closing the other.
    pub fn open(&mut self, page: Page) {
        self.open = (!self.pinned(page)).then_some(page);
    }

    /// A tool was picked from it, or the user went elsewhere: an unpinned page hides.
    pub fn done_with(&mut self, page: Page) {
        if self.open == Some(page) {
            self.open = None;
        }
    }

    pub fn set_pinned(&mut self, page: Page, pinned: bool) {
        match page {
            Page::Design => self.design_pinned = pinned,
            Page::Toolpaths => self.toolpaths_pinned = pinned,
        }
        self.done_with(page);
    }

    /// The Switch Layout button: from designing to machining and back.
    pub fn switch_layout(&mut self) {
        *self = if self.toolpaths_pinned && !self.design_pinned {
            Self::design_layout()
        } else {
            Self::machining_layout()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Page::{Design, Toolpaths};

    fn unpinned() -> Pages {
        let mut p = Pages::design_layout();
        p.set_pinned(Design, false);
        p
    }

    #[test]
    fn it_starts_designing_with_toolpaths_hidden() {
        let p = Pages::default();
        assert_eq!(
            (p.shown(Design), p.shown(Toolpaths)),
            (Shown::Docked, Shown::Hidden)
        );
    }

    #[test]
    fn machining_is_the_other_way_round_and_switching_swaps_them() {
        let mut p = Pages::machining_layout();
        assert_eq!(
            (p.shown(Design), p.shown(Toolpaths)),
            (Shown::Hidden, Shown::Docked)
        );
        p.switch_layout();
        assert_eq!(p, Pages::design_layout());
        p.switch_layout();
        assert_eq!(p, Pages::machining_layout());
    }

    #[test]
    fn opening_an_unpinned_page_flies_it_out_and_closes_the_other() {
        let mut p = unpinned();
        p.open(Design);
        assert_eq!(p.shown(Design), Shown::Flyout);
        p.open(Toolpaths);
        assert_eq!(
            (p.shown(Design), p.shown(Toolpaths)),
            (Shown::Hidden, Shown::Flyout)
        );
    }

    #[test]
    fn a_pinned_page_stays_open() {
        let mut p = Pages::design_layout();
        p.open(Toolpaths);
        assert_eq!(
            (p.shown(Design), p.shown(Toolpaths)),
            (Shown::Docked, Shown::Flyout)
        );
        p.open(Design);
        assert_eq!(
            p.shown(Design),
            Shown::Docked,
            "opening a docked page leaves it docked"
        );
        assert_eq!(
            p.shown(Toolpaths),
            Shown::Hidden,
            "and closes the flown-out one"
        );
    }

    #[test]
    fn done_with_hides_a_flown_out_page_only() {
        let mut p = Pages::design_layout();
        p.open(Toolpaths);
        p.done_with(Toolpaths);
        assert_eq!(p.shown(Toolpaths), Shown::Hidden);
        p.done_with(Design);
        assert_eq!(
            p.shown(Design),
            Shown::Docked,
            "pinned pages are never done with"
        );
    }

    #[test]
    fn pinning_docks_a_page_and_unpinning_hides_it() {
        let mut p = Pages::design_layout();
        p.open(Toolpaths);
        p.set_pinned(Toolpaths, true);
        assert_eq!(p.shown(Toolpaths), Shown::Docked);
        assert!(
            p.pinned(Toolpaths) && p.pinned(Design),
            "both may be pinned"
        );
        p.set_pinned(Design, false);
        assert_eq!(p.shown(Design), Shown::Hidden);
    }
}
