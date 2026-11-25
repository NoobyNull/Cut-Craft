//! A settings window whose changes wait for Apply: whether it is open, the changes not
//! applied yet, and Close's question. Edit ▸ Options and Snapping Options both keep one,
//! so they open, apply, cancel and close the same way.

pub(super) struct Unapplied<T> {
    /// The window is open.
    pub(super) open: bool,
    /// Changes made and not applied yet; `None` when it shows the settings as they are.
    changes: Option<T>,
    /// Close was pressed with changes not applied: the window asks Apply or Discard.
    pub(super) closing: bool,
}

impl<T> Default for Unapplied<T> {
    fn default() -> Self {
        Self {
            open: false,
            changes: None,
            closing: false,
        }
    }
}

impl<T: PartialEq> Unapplied<T> {
    /// The settings as the window shows them: `applied` with any changes not applied yet.
    pub(super) fn shown<'a>(&'a self, applied: &'a T) -> &'a T {
        self.changes.as_ref().unwrap_or(applied)
    }

    /// `edited` is what the window now shows: kept until Apply, unless it is `applied`
    /// as it already is.
    pub(super) fn edited(&mut self, applied: &T, edited: T) {
        self.changes = (edited != *applied).then_some(edited);
    }

    /// There are changes that Apply hasn't used yet.
    pub(super) fn dirty(&self) -> bool {
        self.changes.is_some()
    }

    /// The changes waiting, for Apply to use; none wait after.
    pub(super) fn take_changes(&mut self) -> Option<T> {
        self.changes.take()
    }

    /// The changes waiting, if any, to change along with the settings themselves.
    pub(super) fn changes_mut(&mut self) -> Option<&mut T> {
        self.changes.as_mut()
    }

    /// Closes the window, throwing away what was not applied, and Close's question.
    pub(super) fn close(&mut self) {
        (self.changes, self.closing, self.open) = (None, false, false);
    }
}

#[cfg(test)]
mod tests;
