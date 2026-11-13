//! The two tool pages, Design (left) and Toolpaths (right): pinned pages stay docked open,
//! unpinned ones hide to a side tab and fly out over the view one at a time.

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
    /// Starts in the design layout.
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

/// The shape the view between the pages is kept to, width over height: 4:3.
pub const VIEW_ASPECT: f32 = 4.0 / 3.0;
/// The most of the room beside the side tabs one page may take, so a small window still
/// shows its view.
pub const PAGE_SHARE: f32 = 1.0 / 3.0;

/// How wide each tool page is, outer edge to outer edge, in a `room` (width × height)
/// between the side tabs whose top `view_top` goes to the bar above the views.
///
/// The width comes from the window alone, never from what the page shows, so a page
/// never bounces as forms change. Both pages share what the view leaves over at
/// [`VIEW_ASPECT`], whether one is pinned or both, so pinning or unpinning one page
/// never moves the other; with one pinned, the view gets the unused side too. A page is
/// never narrower than `least` (room for its tool buttons), unless that would take more
/// than [`PAGE_SHARE`] of the room.
pub fn page_width(room: (f32, f32), view_top: f32, least: f32) -> f32 {
    let (width, height) = room;
    let view = VIEW_ASPECT * (height - view_top).max(0.0);
    let even = (width - view) / 2.0;
    even.max(least).min(PAGE_SHARE * width).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Page::{Design, Toolpaths};

    #[test]
    fn a_wide_window_widens_both_pages_until_the_view_is_four_by_three() {
        let (room, top, least) = ((2400.0, 1000.0), 40.0, 260.0);
        let page = page_width(room, top, least);
        let view = (room.0 - 2.0 * page) / (room.1 - top);
        assert!((view - VIEW_ASPECT).abs() < 1e-4, "view {view}");
        assert!(page > least, "{page}");
    }

    #[test]
    fn pages_grow_and_shrink_with_the_window() {
        let at = |w: f32| page_width((w, 1000.0), 40.0, 260.0);
        assert!(at(2600.0) > at(2400.0));
        assert!(at(2400.0) > at(2200.0));
    }

    #[test]
    fn a_page_keeps_room_for_its_buttons_when_the_window_is_tall() {
        // 4:3 would leave the pages nothing: the view takes what the pages leave
        assert_eq!(page_width((1280.0, 760.0), 40.0, 260.0), 260.0);
    }

    #[test]
    fn a_small_window_still_shows_its_view() {
        let page = page_width((600.0, 500.0), 40.0, 260.0);
        assert_eq!(page, 200.0, "a third of the room each");
    }

    #[test]
    fn nothing_odd_comes_of_a_window_with_no_room() {
        assert_eq!(page_width((0.0, 0.0), 40.0, 260.0), 0.0);
        let tall = page_width((800.0, 10.0), 40.0, 260.0);
        assert!((tall - 800.0 * PAGE_SHARE).abs() < 1e-3, "{tall}");
    }

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
