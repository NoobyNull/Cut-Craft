//! What a save of toolpaths carries from the checks before cutting to the files it writes:
//! the problems Save Anyway was answered for, the errors the files then say at the top, and
//! the job and machine it was checked for, so a change while it waits stops it.

/// The job revision and the active machine a save was checked for.
pub(super) type Checked = (u64, usize);

#[derive(Default)]
pub(super) struct SavingChecks {
    /// The problems Save Anyway was answered for: the next save goes on despite them, and
    /// asks again if the checks find any not among them. Empty when not answered.
    accepted: Vec<String>,
    /// The errors (Must fix) the save under way was answered Save Anyway for: said at the
    /// top of each file it writes. Set as each save starts, never kept for the next.
    errors: Vec<String>,
    /// What the save under way was checked for: a change while it waits on a question
    /// (the Web API can make one) stops it, to be checked again.
    checked: Option<Checked>,
}

impl SavingChecks {
    /// Answered Save Anyway for `problems`, and no others: the next save goes on despite them.
    pub(super) fn accept(&mut self, problems: Vec<String>) {
        self.accepted = problems;
    }

    /// The problems Save Anyway was answered for.
    #[cfg(test)]
    pub(super) fn accepted(&self) -> &[String] {
        &self.accepted
    }

    /// Forgets a Save Anyway answer: it was for one save only, whatever stopped it short.
    pub(super) fn forget(&mut self) {
        self.accepted.clear();
    }

    /// A save starts: nothing from the last one is kept. The Save Anyway answer, which is
    /// for this save only, is handed over.
    pub(super) fn start(&mut self) -> Vec<String> {
        self.errors.clear();
        self.checked = None;
        std::mem::take(&mut self.accepted)
    }

    /// The checks let the save go on, despite `errors`, for the job and machine `now`.
    pub(super) fn go(&mut self, errors: Vec<String>, now: Checked) {
        self.errors = errors;
        self.checked = Some(now);
    }

    /// The errors the save under way was answered Save Anyway for.
    pub(super) fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Whether the job and machine are still as `now` was when the save under way was
    /// checked: an error, and nothing written, when they changed while it waited.
    pub(super) fn still_as_checked(&self, now: Checked) -> Result<(), String> {
        match self.checked {
            Some(checked) if checked != now => Err(
                "Nothing saved: the job changed while saving. Save again, so the checks \
                     before cutting look at it as it is now"
                    .to_owned(),
            ),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests;
