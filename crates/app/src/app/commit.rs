//! A tool form's commit button: named for what pressing it does, and shown only when
//! pressing it would do something.
//!
//! - A form whose result already follows its settings (a drawn shape still selected, a model
//!   made as the form opens, an ornament remade as it changes) has no commit button: the
//!   change is already on screen.
//! - A form that makes something new says **Create**; a toolpath's says **Calculate**.
//! - A form that changes what is already there (the selection, a layer's or a job's
//!   settings) says **Apply**.
//!
//! Close (keep what is there) and Cancel (take back what the form did) are always offered.

use crate::pages::Page;
use cut_craft_core::Form;
use cut_craft_i18n::tr;

/// What a form's commit button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Commit {
    /// Makes a new shape, copy, pattern or piece.
    Create,
    /// Works out a toolpath (again, for one being edited).
    Calculate,
    /// Changes what is already there.
    Apply,
    /// Asks an add-on to make something new for the Library (Generate Border).
    Generate,
}

impl Commit {
    /// Its words' key (`locales/en/forms.ftl`).
    fn key(self) -> &'static str {
        match self {
            Self::Create => "commit-create",
            Self::Calculate => "commit-calculate",
            Self::Apply => "commit-apply",
            Self::Generate => "commit-generate",
        }
    }

    /// Its words, in the language in use.
    pub(super) fn label(self) -> String {
        tr!(self.key())
    }
}

/// The commit button a form of `kind` shows, if any (each command says its kind in its
/// [`Meta`](cut_craft_core::Meta)). `drawing`: the tool can be drawn with the mouse; `made`:
/// what the form made still follows its settings, or the form was opened on something it
/// made earlier.
pub(super) fn commit(kind: Form, page: Page, drawing: bool, made: bool) -> Option<Commit> {
    match kind {
        Form::Live { create_first: true } => (!made).then_some(Commit::Create),
        Form::Gesture | Form::Live { .. } => None,
        // Embellish keeps an Apply once made, for changes to be seen to go on the one there
        // (Matthew's ask: tweak it later without deleting and remaking it)
        Form::Remake { keeps_apply: true } if made => Some(Commit::Apply),
        Form::Remake { .. } => (!made).then_some(Commit::Create),
        _ if drawing => (!made).then_some(Commit::Create),
        Form::Toolpath if page == Page::Toolpaths => Some(Commit::Calculate),
        Form::Generate => Some(Commit::Generate),
        Form::Create => Some(Commit::Create),
        Form::Apply | Form::Toolpath => Some(Commit::Apply),
    }
}

/// What the commit button's hint says it will do.
pub(super) fn hint(commit: Commit, drawing: bool, editing: bool) -> String {
    let key = commit.key();
    match commit {
        Commit::Create if drawing => tr!(&format!("{key}.tooltip-drawing")),
        Commit::Calculate if editing => tr!(&format!("{key}.tooltip-again")),
        _ => tr!(&format!("{key}.tooltip")),
    }
}

/// The line a form with no commit button shows where the button would be, saying why there
/// is none.
pub(super) fn live_note(kind: Form, drawing: bool) -> String {
    if kind == Form::Gesture {
        tr!("commit-none-canvas")
    } else if drawing {
        tr!("commit-none-drawing")
    } else {
        tr!("commit-none")
    }
}

#[cfg(test)]
mod tests;
