//! The window: starting up, slow jobs, shortcuts and crashes.

use super::super::*;

impl Harness {
    // ---- the window, starting up and slow jobs ----

    /// The window's title.
    pub(in super::super) fn window_title(&self) -> &str {
        &self.app.title
    }

    /// The tool the status bar names as in use.
    pub(in super::super) fn current_tool(&self) -> &str {
        &self.app.status.current_tool
    }

    /// The job is the file at `path`, as if opened from it.
    pub(in super::super) fn given_job_path(&mut self, path: std::path::PathBuf) {
        self.app.session.path = Some(path);
    }

    /// Starts PowerWhittle with no file to open, as a plain launch does.
    pub(in super::super) fn start_up(&mut self) {
        self.app.open_at_start(None);
    }

    /// The screen showing: the start screen or a job.
    pub(in super::super) fn screen(&self) -> Screen {
        self.app.screen
    }

    /// The progress window shows as soon as a slow job starts, not after a wait.
    pub(in super::super) fn given_progress_shown_at_once(&mut self) {
        self.app.wait_before_showing = std::time::Duration::ZERO;
    }

    /// Starts `work` as a slow job titled `title`, as a toolpath that takes a while does;
    /// once done (and still wanted), the status bar says "used".
    pub(in super::super) fn start_slow_job(
        &mut self,
        title: &str,
        work: impl FnOnce() + Send + 'static,
    ) {
        self.app.in_background(title, "test:slow", work, |app, ()| {
            app.status.set_message("used");
        });
    }

    /// Whether a slow job is running (its progress window up).
    pub(in super::super) fn busy(&self) -> bool {
        self.app.busy.is_some()
    }

    // ---- the window ----

    /// The desktop's own title bar, as chosen at start (the menus then start at the left).
    pub(in super::super) fn given_desktop_title_bar(&mut self) {
        self.app.title_bar_at_start = true;
    }

    // ---- shortcuts ----

    /// Opens Options at its Shortcuts page.
    pub(in super::super) fn open_shortcut_options(&mut self) {
        self.app
            .options_editor
            .open_at(crate::app::shortcuts_page::CATEGORY);
    }

    /// Whether the Options window is open.
    pub(in super::super) fn options_window_open(&self) -> bool {
        self.app.options_editor.window.open
    }

    /// A press on the Shortcuts page: Add…, Reset, Move it here, ...
    pub(in super::super) fn edit_shortcuts(&mut self, edit: crate::app::shortcuts_page::Edit) {
        self.app.apply_shortcut_edit(edit);
    }

    /// The Shortcuts page waiting for the next key or button, if it is.
    pub(in super::super) fn waiting_for_shortcut(
        &self,
    ) -> Option<&crate::app::settings::ShortcutCapture> {
        self.app.capture.as_ref()
    }

    /// The keys and buttons the Shortcuts page lists for `id` (applied or not).
    pub(in super::super) fn shortcuts_of(&self, id: &str) -> Vec<shortcut::Trigger> {
        self.app.shortcuts_of(id)
    }

    /// Adds `trigger` to `id` on the Shortcuts page (used once applied).
    pub(in super::super) fn add_shortcut(&mut self, id: &str, trigger: shortcut::Trigger) {
        self.app.add_shortcut(id, trigger);
    }

    /// Takes `trigger` off `id` on the Shortcuts page (gone once applied).
    pub(in super::super) fn remove_shortcut(&mut self, id: &str, trigger: shortcut::Trigger) {
        self.app.remove_shortcut(id, trigger);
    }

    /// Every key and button in use now, with the action each runs.
    pub(in super::super) fn shortcuts_in_use(&self) -> Vec<(String, shortcut::Trigger)> {
        self.app.bindings(&self.ctx)
    }

    /// `id`'s shortcut as the menus show it.
    pub(in super::super) fn shortcut_text(&self, id: &str) -> Option<String> {
        self.app.shortcut_text(id)
    }

    /// The Shortcuts page's rows when its search box holds `search`.
    pub(in super::super) fn shortcut_rows(
        &self,
        search: &str,
    ) -> Vec<crate::app::shortcuts_page::Row> {
        self.app.shortcut_rows(search)
    }

    // ---- crashes ----

    /// Starts after a crash that left `crash` as its log.
    pub(in super::super) fn crashed_last_time(&mut self, crash: String) {
        let ctx = self.ctx.clone();
        self.app.offer_crash(&ctx, crash);
    }

    /// Whether the bug report window is open.
    pub(in super::super) fn bug_report_open(&self) -> bool {
        self.app.reporter.is_open()
    }

    /// Starts on the job screen, past the start screen.
    pub(in super::super) fn given_job_screen(&mut self) {
        self.app.screen = Screen::Job;
    }

    /// Closes the Options window, as its Close does.
    pub(in super::super) fn close_options_window(&mut self) {
        self.app.options_editor.window.open = false;
    }
}
