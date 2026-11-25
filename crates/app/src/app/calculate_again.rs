//! Calculate pressed again on a toolpath form that has already made its toolpath.
//!
//! - Nothing changed since (the same settings, the same vectors picked, and the toolpath
//!   neither edited, undone nor out of date): the toolpath is kept as it is and the form
//!   says "Already up to date". No second copy is made.
//! - Something changed: the toolpath is worked out again **in its place** (its spot in the
//!   cut order and its group kept), as one undo.
//! - Other vectors picked, or the toolpath deleted or undone away: a new one is made, as
//!   on the first Calculate, and the form follows that one from then on.
//! - **Calculate as New** makes another toolpath on purpose, keeping the one there.
//!
//! Edit Toolpath's form follows the toolpath it opened on in the same way.

use super::CutCraftApp;
use cut_craft_core::document::EntityId;
use cut_craft_core::selection::Selection;
use serde_json::Value;

/// What a toolpath form made last: its toolpaths and what they were made from.
#[derive(Clone, Debug)]
pub(super) struct Made {
    /// The toolpaths, in the order made.
    pub ids: Vec<EntityId>,
    /// Each toolpath's own settings just after it was made: different now when it has been
    /// edited or undone since.
    pub kept: Vec<Value>,
    /// The form's settings they were made with.
    pub params: Value,
    /// The vectors picked when they were made; `None` for Edit Toolpath, which uses the
    /// toolpath's own.
    pub selection: Option<Selection>,
}

/// What Calculate does on a form that has made toolpaths before.
#[derive(Debug)]
pub(super) enum Again {
    /// Nothing changed: keep them.
    UpToDate,
    /// Work them out again in their place.
    Update(Made),
    /// Make new ones (nothing made yet, other vectors picked, or they are gone).
    New,
}

/// What the form says when Calculate finds nothing to do.
pub(super) const UP_TO_DATE: &str = "Already up to date: nothing changed, so the toolpath \
     was kept. Calculate as New adds another.";

/// What Edit Toolpath's form says when Calculate finds nothing to do.
const UP_TO_DATE_EDITING: &str = "Already up to date: nothing changed, so the toolpath \
     was kept.";

impl CutCraftApp {
    /// What Calculate with `params` should do on the open form.
    pub(super) fn calculate_again(&self, params: &Value) -> Again {
        let Some(made) = self.form.as_ref().and_then(|f| f.made.as_ref()) else {
            return Again::New;
        };
        let Some(there) = made
            .ids
            .iter()
            .map(|id| self.doc.toolpaths.get(id))
            .collect::<Option<Vec<_>>>()
        else {
            return Again::New;
        };
        // other vectors picked: a toolpath for them, the one made kept (nothing picked
        // means the same ones)
        if let Some(picked) = &made.selection
            && !self.doc.selection.is_empty()
            && self.doc.selection != *picked
        {
            return Again::New;
        }
        let unchanged = made.params == *params
            && there
                .iter()
                .zip(&made.kept)
                .all(|(t, kept)| !t.stale && t.params == *kept);
        if unchanged {
            Again::UpToDate
        } else {
            Again::Update(made.clone())
        }
    }

    /// Calculate found nothing changed: said on the form and in the status bar.
    pub(super) fn say_up_to_date(&mut self, params: &Value) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        let note = if form.editing.is_some() {
            UP_TO_DATE_EDITING
        } else {
            UP_TO_DATE
        };
        form.note = Some((params.clone(), note.to_owned()));
        self.status
            .set_message("Already up to date: nothing changed, so nothing was calculated");
    }

    /// The open form's note, while its settings are the ones it was said for.
    pub(super) fn form_note(&self) -> Option<&str> {
        let form = self.form.as_ref()?;
        let (said_for, note) = form.note.as_ref()?;
        (*said_for == form.params).then_some(note.as_str())
    }

    /// Toolpaths `ids` just made by form `id` with `params` from `selection`: its next
    /// Calculate updates them rather than adding more. Only a toolpath form's Calculate
    /// follows what it made.
    pub(super) fn follow_made(
        &mut self,
        id: &str,
        params: &Value,
        ids: Vec<EntityId>,
        selection: Option<Selection>,
    ) {
        let kept: Option<Vec<Value>> = ids
            .iter()
            .map(|t| self.doc.toolpaths.get(t).map(|t| t.params.clone()))
            .collect();
        let Some(form) = self
            .form
            .as_mut()
            .filter(|f| f.id == id && f.page == crate::pages::Page::Toolpaths)
            .filter(|f| f.kind.is_toolpath())
        else {
            return;
        };
        form.made = match kept {
            Some(kept) if !ids.is_empty() => Some(Made {
                ids,
                kept,
                params: params.clone(),
                selection,
            }),
            _ => None,
        };
    }

    /// Whether the open form shows Calculate as New: a toolpath form (not Edit Toolpath's)
    /// whose toolpath is there to keep.
    pub(super) fn offers_calculate_as_new(&self) -> bool {
        self.form.as_ref().is_some_and(|f| {
            f.editing.is_none()
                && f.made
                    .as_ref()
                    .is_some_and(|m| m.ids.iter().all(|t| self.doc.toolpaths.contains_key(t)))
        })
    }

    /// Calculate as New: another toolpath from the form's settings, the one it made kept.
    /// The form follows the new one from then on.
    pub(super) fn calculate_as_new(&mut self) {
        let Some(form) = self.form.as_mut() else {
            return;
        };
        form.made = None;
        let (id, params) = (form.id, form.params.clone());
        self.calculate_checked(id, &params);
    }
}
