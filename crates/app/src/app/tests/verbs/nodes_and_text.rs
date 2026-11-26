//! Node editing, drawing with the pointer (polylines, freehand, text), fillets and joining
//! open ends, spelling, and the Import Into Shape side.

use super::super::*;

type JobRect = cut_craft_core::geometry::Rect;
type NodeAt = cut_craft_core::nodes::NodeAt;

impl Harness {
    // ---- node editing ----

    /// The note in red under the tools, if one shows.
    pub(in super::super) fn tool_note(&self) -> Option<&str> {
        self.app.tool_note()
    }

    /// The node or span of a selected vector within `reach` of `at`, as a right-click finds.
    pub(in super::super) fn node_target(&self, at: Point, reach: f64) -> Option<NodeAt> {
        self.app.node_target(at, reach)
    }

    /// What the view draws over the selected vectors' nodes while `gesture` goes on.
    pub(in super::super) fn node_overlay(
        &self,
        gesture: Option<Gesture>,
        reach: f64,
    ) -> crate::view2d::Overlay {
        self.app.node_overlay(gesture, reach)
    }

    /// The nodes picked.
    pub(in super::super) fn picked_nodes(&self) -> &[NodeAt] {
        &self.app.picked_nodes
    }

    /// Picks the nodes inside `rect`, as a box dragged round them does.
    pub(in super::super) fn pick_nodes_in(&mut self, rect: JobRect) {
        self.app.picked_nodes = self.app.nodes_in(rect);
    }

    /// Which mirror lines (H, V) node editing has on.
    pub(in super::super) fn node_mirror(&self) -> [bool; 2] {
        self.app.node_mirror
    }

    // ---- drawing with the pointer ----

    /// The areas the open Sketch Carving form would carve, if it found any.
    pub(in super::super) fn sketch_areas(&self) -> Option<&[JobRect]> {
        self.app.sketch_areas.as_ref().map(|s| s.areas.as_slice())
    }

    /// Whether the polyline being drawn makes curves (C) instead of straight lines.
    pub(in super::super) fn polyline_curved(&self) -> bool {
        self.app.polyline.curved
    }

    /// Whether Create Fillets is waiting for corners to round.
    pub(in super::super) fn filleting(&self) -> bool {
        self.app.filleting()
    }

    /// Whether Join Open Ends is waiting for ends to join.
    pub(in super::super) fn joining_ends(&self) -> bool {
        self.app.joining_ends()
    }

    /// The first end Join Open Ends has picked, if it has.
    pub(in super::super) fn join_end_picked(&self) -> Option<Point> {
        self.app.join_ends_first()
    }

    /// The freehand stroke drawn so far is `points`.
    pub(in super::super) fn given_stroke(&mut self, points: Vec<Point>) {
        self.app.stroke = points;
    }

    /// What `id` would draw for `drag`, shown while dragging.
    pub(in super::super) fn drawing_preview(&self, id: &str, drag: Drag) -> Vec<Path> {
        self.app.preview(id, drag)
    }

    /// Draws with `id` as `drag` does on the view.
    pub(in super::super) fn draw(&mut self, id: &'static str, drag: Drag) {
        self.app.draw(id, drag);
    }

    /// Hands `gesture` to the drawing tool `id`, as the view does.
    pub(in super::super) fn drawing_gesture(
        &mut self,
        id: &'static str,
        gesture: Option<Gesture>,
        reach: f64,
        modifiers: Modifiers,
    ) {
        self.app.drawing_gesture(id, gesture, reach, modifiers);
    }

    /// A click at `at` with Draw Text open, holding `modifiers`.
    pub(in super::super) fn text_click(&mut self, at: Point, reach: f64, modifiers: Modifiers) {
        self.app.text_click(at, reach, modifiers);
    }

    /// The selected shape E would edit: its id, the tool that drew it and its settings.
    pub(in super::super) fn object_to_edit(&self) -> Option<(EntityId, &'static str, Value)> {
        self.app.object_to_edit()
    }

    // ---- spelling ----

    /// Spelling is checked with `speller`.
    pub(in super::super) fn given_speller(&mut self, speller: cut_craft_core::spell::Speller) {
        self.app.spelling.given(speller);
    }

    /// The speller in use.
    pub(in super::super) fn speller(&self) -> &cut_craft_core::spell::Speller {
        self.app.spelling.found().expect("a speller")
    }

    /// Picks `to` for the misspelt `from`, as the suggestion's menu does.
    pub(in super::super) fn fix_spelling(&mut self, from: &str, to: &str) {
        self.app.fix_spelling(from, to);
    }

    /// Adds `word` to (or takes it from) the user's own words.
    pub(in super::super) fn user_word(&mut self, word: &str, add: bool) {
        self.app.user_word(word, add);
    }

    // ---- other windows ----

    /// Whether the Add-ons window is open.
    pub(in super::super) fn addons_open(&self) -> bool {
        self.app.addons.open
    }

    /// The Start screen shows, as closing the job leaves it.
    pub(in super::super) fn given_start_screen(&mut self) {
        self.app.screen = Screen::Start;
    }

    /// The job was never saved, so it has no file.
    pub(in super::super) fn given_no_job_path(&mut self) {
        self.app.session.path = None;
    }

    /// Picks `side` in the Import Into Shape window, which places the drawing again.
    pub(in super::super) fn fit_import_side(&mut self, side: cut_craft_core::bend::Side) {
        self.app
            .fit_import
            .as_mut()
            .expect("the window is open")
            .side = side;
        self.app.place_fit_import();
    }
}
