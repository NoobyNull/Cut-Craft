//! The 3D view: its handles, buttons, view cube, detail and what it keeps.

use super::super::*;

impl Harness {
    // ---- where the 3D view is and what it shows ----

    /// The rectangle the 3D view fits the job into, as last drawn.
    pub(in super::super) fn fit_3d(&self) -> Rect {
        self.app.fit_3d.expect("the 3D view has been drawn")
    }

    /// Pans and zooms the 3D view to `framing`, as the wheel and middle drag do.
    pub(in super::super) fn given_framing_3d(&mut self, framing: view3d::Framing) {
        self.app.framing = framing;
    }

    /// The 3D view's right-click menu with nothing under the pointer.
    pub(in super::super) fn view_3d_menu(&self) -> Vec<gui::MenuItem> {
        self.app.menus.view_3d_menu(&self.app.doc, &[])
    }

    /// The view cube in the 3D view's corner, as last drawn.
    pub(in super::super) fn view_cube(&self) -> view3d::ViewCube {
        self.app.view_cube.expect("the 3D view draws its view cube")
    }

    /// Whether the 3D view draws the vectors (its Vectors button).
    pub(in super::super) fn vectors_in_3d(&self) -> bool {
        self.app.display.vectors_3d
    }

    /// Whether Auto-rotate is spinning the 3D view.
    pub(in super::super) fn spinning_3d(&self) -> bool {
        self.app.spinning_3d.is_some()
    }

    /// The 3D view last turned, panned or zoomed at `time` (the harness's clock).
    pub(in super::super) fn given_3d_moved_at(&mut self, time: f64) {
        self.app.moved_3d = time;
    }

    /// A 3D mouse plugged in, its movements coming from `input`.
    pub(in super::super) fn given_space_mouse(
        &mut self,
        input: std::sync::mpsc::Receiver<crate::spacemouse::Input>,
    ) {
        self.app.space_mouse = Some(input);
    }

    /// File ▸ Save Shaded Image, saving to `path`.
    pub(in super::super) fn save_shaded_image(&mut self, path: std::path::PathBuf) {
        self.app.save_shaded_image(path);
    }

    /// The status bar's frame rate note.
    pub(in super::super) fn frame_rate_note(&self) -> String {
        self.app.frame_rate_note()
    }

    // ---- the handles over the selection ----

    /// The handles drawn over the selection in the 3D view, and where.
    pub(in super::super) fn handles_3d(&self) -> &[(Handle3d, Pos2)] {
        &self.app.handles_3d
    }

    /// Whether a handle in the 3D view is being dragged.
    pub(in super::super) fn dragging_in_3d(&self) -> bool {
        self.app.drag_3d.is_some()
    }

    /// Picks up `tool` (Transform, Measure, ...), as its button does.
    pub(in super::super) fn pick_view_tool(&mut self, tool: ViewTool) {
        self.app.view_tool = Some(tool);
    }

    /// The 3D settings popup's Set: whose anchors are being clicked, and the first once
    /// clicked.
    pub(in super::super) fn anchors_3d(&self) -> Option<(Anchor, Option<Point>)> {
        self.app
            .settings_3d
            .as_ref()
            .expect("the 3D settings are open")
            .anchors
    }

    /// How many points the polyline being drawn has so far (none once finished).
    pub(in super::super) fn polyline_points(&self) -> usize {
        self.app.polyline.points.len()
    }

    // ---- detail and what is kept ----

    /// The surface the drawing tools draw on in the 3D view.
    pub(in super::super) fn drawing_surface(&mut self) -> Option<std::sync::Arc<Material>> {
        self.app.drawing_surface()
    }

    /// The cells a side the 3D view draws with once still, while the frame rate allows.
    pub(in super::super) fn turning_cells(&self) -> usize {
        self.app.turning_cells()
    }

    /// The fine mesh the still 3D view draws, in triangles, once made.
    pub(in super::super) fn fine_triangles(&self) -> Option<usize> {
        self.app
            .meshes
            .fine
            .value()
            .map(|mesh| mesh.indices.len() / 3)
    }

    /// What the fine mesh was made for, once made.
    pub(in super::super) fn fine_made_for(&self) -> Option<FineKey> {
        self.app.meshes.fine.key().cloned()
    }

    /// What the quick surface of the model was made for, once made.
    pub(in super::super) fn quick_made_for(&self) -> Option<RoughKey> {
        self.app.meshes.rough.key().copied()
    }

    // ---- Orientate 3D Model's rings ----

    /// The turning rings drawn round an imported model (each axis and its outline) and the
    /// sphere inside them (its centre and radius).
    pub(in super::super) fn turn_rings(&self) -> Rings3d {
        self.app.turn_rings.drawn()
    }

    /// Whether a ring or the sphere is being dragged.
    pub(in super::super) fn turning_a_ring(&self) -> bool {
        self.app.turn_rings.held().is_some()
    }
}
