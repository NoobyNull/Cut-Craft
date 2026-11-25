//! The job: what it holds before a test's steps, and things made in it as the tools make them.

use super::super::*;

impl Harness {
    // ---- the job as it stands before the test's steps ----

    /// The job's drawing: what the user has made so far.
    pub(in super::super) fn doc(&self) -> &Document {
        &self.app.doc
    }

    /// Opens `doc` as the job, as if loaded from a file made earlier.
    pub(in super::super) fn open_document(&mut self, doc: Document) {
        self.app.doc = doc;
    }

    /// Makes `change` to the drawing before the test's own steps (no undo step).
    pub(in super::super) fn given(&mut self, change: &Change) {
        self.app
            .doc
            .apply(change)
            .expect("the given change applies");
    }

    /// Edits the drawing before the test's own steps, as a file made earlier would hold it.
    pub(in super::super) fn given_document(&mut self, edit: impl FnOnce(&mut Document)) {
        edit(&mut self.app.doc);
    }

    /// Changes the saved options, as an earlier session would have left them.
    pub(in super::super) fn given_options(
        &mut self,
        edit: impl FnOnce(&mut crate::options::Options),
    ) {
        edit(&mut self.app.options);
    }

    /// The options in force.
    pub(in super::super) fn options(&self) -> &crate::options::Options {
        &self.app.options
    }

    /// Closes the job unsaved and starts another, as New does once the question is answered.
    pub(in super::super) fn start_another_job(&mut self) {
        self.app.reset_job();
    }

    // ---- making things, as the tools do ----

    /// Makes `change` as a tool does: one undo step named `name`, reported in the status bar.
    pub(in super::super) fn make(&mut self, change: Change, name: &str) {
        self.app.record(Ok(change), name);
    }

    /// Selects `ids`, as clicking them does.
    pub(in super::super) fn select(&mut self, ids: impl IntoIterator<Item = EntityId>) {
        self.app.doc.selection = ids.into_iter().collect();
    }

    /// Selects the components `ids`, as clicking them in the 3D view or Components list does.
    pub(in super::super) fn select_components(&mut self, ids: impl IntoIterator<Item = EntityId>) {
        self.app.doc.selected_components = ids.into_iter().collect();
    }

    /// The open form's settings, all of them, as they now stand.
    pub(in super::super) fn form_settings(&self) -> &Value {
        &self.app.form.as_ref().expect("a form is open").params
    }

    /// Recalculates toolpath `id`, as its Recalculate does.
    pub(in super::super) fn recalculate(&mut self, id: EntityId) {
        let again = self
            .app
            .registry
            .recalculate(&self.app.doc, id)
            .expect("it recalculates");
        self.app.record(Ok(again), "recalculate");
    }

    /// Counts the job put in with `open_document` as a newly opened one, as opening a file does.
    pub(in super::super) fn given_it_opened_as_a_new_job(&mut self) {
        self.app.job_epoch += 1;
    }
}
