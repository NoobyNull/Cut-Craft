//! Snapping, its options window, the polyline being drawn, and guides with their properties.

use super::super::*;

impl Harness {
    // ---- snapping ----

    /// The snap the pointer at `at` finds with no button held, as the crosshair shows it.
    pub(in super::super) fn snap_at(&mut self, at: Point) -> Option<cut_craft_core::snap::Snap> {
        let scale = self.zoom_2d();
        let ctx = self.ctx.clone();
        let mut found = None;
        headless(ctx.run_ui(RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                found = self.app.snapping(ui, None, scale, Some(at)).1;
            });
        }));
        found
    }

    /// Moves the pointer to `at` for one frame: the pointer's shape there.
    pub(in super::super) fn hover(&mut self, at: Pos2) -> egui::CursorIcon {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.window)),
            time: Some(self.time),
            events: vec![Event::PointerMoved(at)],
            ..Default::default()
        };
        let ctx = self.ctx.clone();
        let out = headless(ctx.run_ui(input, |ui| self.app.pass(ui)));
        out.platform_output.cursor_icon
    }

    /// The snap mark picked on the view (by its place in the job's snaps), if one is.
    pub(in super::super) fn picked_snap(&self) -> Option<usize> {
        self.app.picked_snap()
    }

    /// Whether the Snapping Options window is open.
    pub(in super::super) fn snap_options_open(&self) -> bool {
        self.app.snap.open
    }

    /// Opens the Snapping Options window.
    pub(in super::super) fn open_snap_options(&mut self) {
        self.app.snap.open = true;
    }

    /// Changes what the open Snapping Options window shows, as its tick boxes do (Apply uses
    /// it).
    pub(in super::super) fn edit_snap_options(
        &mut self,
        edit: impl FnOnce(&mut cut_craft_core::snap::SnapOptions),
    ) {
        let applied = &self.app.options.snapping;
        let mut edited = self.app.snap.shown(applied).clone();
        edit(&mut edited);
        self.app.snap.edited(applied, edited);
    }

    // ---- the polyline being drawn ----

    /// The points of the polyline being drawn so far.
    pub(in super::super) fn polyline_drawn(&self) -> &[Point] {
        &self.app.polyline.points
    }

    /// Places the polyline's next point at `at`, as a click there does.
    pub(in super::super) fn polyline_click(&mut self, at: Point) {
        self.app.polyline.click(at);
    }

    /// Finishes the polyline being drawn as an open line, as Enter does.
    pub(in super::super) fn finish_polyline(&mut self) {
        self.app.finish_polyline(false);
    }

    /// A Shift-click at `at` with `id`'s form open: picks the shape it made there to edit.
    pub(in super::super) fn shift_click_shape(&mut self, id: &'static str, at: Point) {
        self.app.shift_click_shape(id, at, 1.0);
    }

    /// Whether the open form holds a shape it edits.
    pub(in super::super) fn form_has_shape(&self) -> bool {
        self.app
            .form
            .as_ref()
            .expect("a form is open")
            .drawn
            .is_some()
    }

    // ---- guides ----

    /// Where the right-click menu opened on the job, as a right-click at `at` leaves it.
    pub(in super::super) fn given_menu_at(&mut self, at: Point) {
        self.app.menu_at = Some(at);
    }

    /// Opens Guide Properties on guide `index` with what it shows.
    pub(in super::super) fn given_guide_props(&mut self, props: GuideProps) {
        self.app.guide_props = Some(props);
    }

    /// The guide Guide Properties is open on, if it is open.
    pub(in super::super) fn guide_props_open_on(&self) -> Option<usize> {
        self.app.guide_props.as_ref().map(|p| p.index)
    }

    /// Guide Properties' Apply.
    pub(in super::super) fn guide_props_apply(&mut self) {
        self.app.guide_props_apply();
    }

    /// Guide Properties' Create (the parallel guides).
    pub(in super::super) fn guide_props_create(&mut self) {
        self.app.guide_props_create();
    }

    /// Guide Properties' Delete.
    pub(in super::super) fn guide_props_delete(&mut self) {
        self.app.guide_props_delete();
    }
}
