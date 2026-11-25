//! The Rail layout's list beside its column: which one is open (a group's tools, the
//! favourites or the gadgets; at most one at a time), and whether its pin keeps it open
//! after a tool is picked. A toolbar of your own opens in the same place, but is kept with
//! the toolbars.

/// Which list is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum List {
    #[default]
    Closed,
    /// A group's tools, by its place in `all_groups`.
    Group(usize),
    Favourites,
    Gadgets,
}

#[derive(Default)]
pub(super) struct RailList {
    pub(super) open: List,
    /// The pin is in: the list stays open after a tool is picked.
    pub(super) pinned: bool,
}

impl RailList {
    /// The column's button for `list` was pressed: opens it in place of any other, or
    /// closes it if it was the one open.
    pub(super) fn toggle(&mut self, list: List) {
        self.open = if self.open == list {
            List::Closed
        } else {
            list
        };
    }

    /// Closes whatever is open.
    pub(super) fn close(&mut self) {
        self.open = List::Closed;
    }

    /// A tool was picked from the list: it closes unless pinned. Whether it closed.
    pub(super) fn picked(&mut self) -> bool {
        if !self.pinned {
            self.close();
        }
        !self.pinned
    }

    /// The open group, if a group's list is open.
    pub(super) fn group(&self) -> Option<usize> {
        match self.open {
            List::Group(i) => Some(i),
            _ => None,
        }
    }

    /// Some list is open.
    pub(super) fn is_open(&self) -> bool {
        self.open != List::Closed
    }

    /// The gadgets' list closes once there is nothing installed to list.
    pub(super) fn keep_gadgets(&mut self, any: bool) {
        if self.open == List::Gadgets && !any {
            self.close();
        }
    }
}

#[cfg(test)]
mod tests;
