//! Selecting, grabbing and changing what is selected; tool tips and guidance; the clipart
//! library; Import Into Shape; and the Ribbon's groups.

use super::super::*;

impl Harness {
    // ---- selecting ----

    /// Picks `path` on the selection's right-click menu.
    pub(in super::super) fn choose_on_selection_menu(&mut self, path: &[&str]) {
        self.choose(|app| app.menus.selection_menu(&app.doc), path);
    }

    /// The outlines a drag from `grab` shows moving.
    pub(in super::super) fn grabbed_outlines(&self, grab: Grab) -> Vec<Path> {
        self.app.grabbed_outlines(grab)
    }

    /// Takes back the last undo step straight from the history: whether there was one.
    pub(in super::super) fn undo_step(&mut self) -> bool {
        self.app.history.undo(&mut self.app.doc)
    }

    /// Whether Transform mode is rotating (after a second click on the middle).
    pub(in super::super) fn rotating(&self) -> bool {
        self.app.rotating
    }

    /// The drawing tool in hand, if one is.
    pub(in super::super) fn drawing_tool(&self) -> Option<&'static str> {
        self.app.drawing_tool()
    }

    /// The tool that drew `id` and its settings for it, if a drawing tool did.
    pub(in super::super) fn drawn_by(&self, id: EntityId) -> Option<(&'static str, Value)> {
        self.app
            .registry
            .drawn_by(&self.app.doc, &self.app.doc.vectors[&id])
    }

    /// Redraws the shape `old` with `id`'s `settings`, as its form does on a change.
    pub(in super::super) fn redraw(
        &mut self,
        id: &str,
        settings: &Value,
        old: EntityId,
    ) -> Result<(), String> {
        self.app.redraw(id, settings, old)
    }

    /// The shape the open form edits, if it edits one.
    pub(in super::super) fn form_drawn(&self) -> Option<EntityId> {
        self.app.form.as_ref().expect("a form is open").drawn
    }

    // ---- tool tips and guidance ----

    /// The tip shown over `id`'s button, which says `about` it.
    pub(in super::super) fn tool_tip(&self, id: &str, about: &str) -> guide::Tip {
        self.app.tool_tip(id, about)
    }

    /// The open form's Next: line, if it shows one.
    pub(in super::super) fn form_guidance(&self) -> Option<String> {
        self.app.form_guidance()
    }

    /// The next step the status bar says, if any.
    pub(in super::super) fn next_step(&self) -> Option<String> {
        self.app.next_step()
    }

    /// What a bug report says of the app's state.
    pub(in super::super) fn bug_state(&self) -> String {
        self.app.bug_state()
    }

    /// The Rail group whose list is open, if one is.
    pub(in super::super) fn rail_group_open(&self) -> Option<usize> {
        self.app.rail.group()
    }

    // ---- the clipart library ----

    /// The library's folders are `folders`.
    pub(in super::super) fn given_clipart_folders(&mut self, folders: Vec<std::path::PathBuf>) {
        self.app.clipart.folders = folders;
    }

    /// Reads the library's folders now, listing what can be imported.
    pub(in super::super) fn read_clipart_now(&mut self, importable: &crate::clipart::Importable) {
        self.app.clipart.read_now(importable);
    }

    /// Opens the library's Properties window on `file`.
    pub(in super::super) fn open_clipart_properties(&mut self, file: &std::path::Path) {
        self.app.clipart.open_properties(file);
    }

    /// Closes the library's Properties window: whether it was open.
    pub(in super::super) fn close_clipart_properties(&mut self) -> bool {
        self.app.clipart.close_properties()
    }

    // ---- Import Into Shape ----

    /// Import Into Shape of `path` into `shape`.
    pub(in super::super) fn import_into(&mut self, path: &std::path::Path, shape: EntityId) {
        self.app.import_into(path, shape);
    }

    /// Whether the Import Into Shape window is open.
    pub(in super::super) fn fit_import_open(&self) -> bool {
        self.app.fit_import.is_some()
    }

    /// Picks `way` in the Import Into Shape window, which places the drawing again.
    pub(in super::super) fn fit_import_way(&mut self, way: crate::app::fit_import::Way) {
        self.app
            .fit_import
            .as_mut()
            .expect("the window is open")
            .way = way;
        self.app.place_fit_import();
    }

    // ---- the Ribbon ----

    /// The Ribbon tab's groups, and the page they come from.
    pub(in super::super) fn ribbon_groups(&self) -> (Vec<gui::ToolGroup>, Page) {
        self.app.ribbon_groups()
    }

    /// Whether `id` is offered on this machine (a laser tool needs a laser, ...).
    pub(in super::super) fn offered(&self, id: &str) -> bool {
        self.app.offered(id)
    }
}
